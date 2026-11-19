//! Dedicated headless project simulation with a TCP snapshot endpoint.
use std::borrow::Cow;
use std::io::{self, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::mpsc::{self, SyncSender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::time::{Duration, Instant};
use voxy_editor::{
    CharacterReplicationSnapshot, GameHost, HeadlessGame, editor_component_registry,
};
use voxy_scene::{
    PredictionUpdate, SnapshotBaseline, SnapshotRollbackBuffer, SnapshotUpdate, TickSnapshot,
};
use winit::keyboard::KeyCode;

const MAX_SNAPSHOT_BYTES: usize = 16 * 1024 * 1024;
const MAX_CONCURRENT_REQUESTS: usize = 8;
const HISTORY_TICKS: usize = 120;
const STEP_SECONDS: f64 = 1.0 / 60.0;
const INPUT_HEADER: [u8; 5] = *b"VXSI\x01";
const SEQUENCED_INPUT_HEADER: [u8; 5] = *b"VXSI\x02";
const APPLIED_INPUT_HEADER: [u8; 5] = *b"VXSP\x01";
const INPUT_QUEUE_CAPACITY: usize = 32;

struct LocalInput {
    commands: SyncSender<InputCommand>,
    last: Mutex<Option<(u64, KeyCode)>>,
}

struct PublishedState {
    snapshots: SnapshotRollbackBuffer,
    tick: u64,
    applied: (u64, u64),
    character: Option<Arc<CharacterReplicationSnapshot>>,
}

fn capture(
    game: &HeadlessGame,
    local_input: bool,
) -> Result<(TickSnapshot, Option<Arc<CharacterReplicationSnapshot>>), Box<dyn std::error::Error>> {
    if local_input {
        let character = game.character_replication_snapshot()?;
        Ok((character.scene.clone(), Some(Arc::new(character))))
    } else {
        Ok((game.replication_snapshot()?, None))
    }
}

struct InputCommand {
    key: KeyCode,
    sequence: Option<u64>,
}

impl LocalInput {
    fn accept(&self, sequence: u64, key: KeyCode) -> Result<u64, Box<dyn std::error::Error>> {
        let mut last = self
            .last
            .lock()
            .map_err(|_| "input sequence lock poisoned")?;
        let accepted = last.map_or(0, |(sequence, _)| sequence);
        if sequence == 0 {
            return Err("input sequence must start at one".into());
        }
        if sequence <= accepted {
            if let Some((previous, previous_key)) = *last
                && sequence == previous
                && key != previous_key
            {
                return Err("input retry changed the accepted command".into());
            }
            return Ok(accepted);
        }
        if accepted.checked_add(1) != Some(sequence) {
            return Err("input sequence gap".into());
        }
        self.commands
            .try_send(InputCommand {
                key,
                sequence: Some(sequence),
            })
            .map_err(|error| error.to_string())?;
        *last = Some((sequence, key));
        Ok(sequence)
    }
}

fn input_key(command: u8) -> io::Result<KeyCode> {
    match command {
        0 => Ok(KeyCode::ArrowLeft),
        1 => Ok(KeyCode::ArrowRight),
        2 => Ok(KeyCode::Space),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unknown game input",
        )),
    }
}

fn reply(
    mut connection: TcpStream,
    state: &RwLock<PublishedState>,
    local_input: Option<&LocalInput>,
) -> Result<(), Box<dyn std::error::Error>> {
    let timeout = Some(Duration::from_secs(1));
    connection.set_read_timeout(timeout)?;
    connection.set_write_timeout(timeout)?;
    let mut header = [0; 5];
    connection.read_exact(&mut header)?;
    if header == INPUT_HEADER {
        let input = local_input.ok_or("local input is disabled")?;
        let mut command = [0; 1];
        connection.read_exact(&mut command)?;
        input
            .commands
            .try_send(InputCommand {
                key: input_key(command[0])?,
                sequence: None,
            })
            .map_err(|error| error.to_string())?;
        connection.write_all(&INPUT_HEADER)?;
        return Ok(());
    }
    if header == SEQUENCED_INPUT_HEADER {
        let input = local_input.ok_or("local input is disabled")?;
        let mut sequence = [0; 8];
        let mut command = [0; 1];
        connection.read_exact(&mut sequence)?;
        connection.read_exact(&mut command)?;
        let accepted = input.accept(u64::from_be_bytes(sequence), input_key(command[0])?)?;
        connection.write_all(&SEQUENCED_INPUT_HEADER)?;
        connection.write_all(&accepted.to_be_bytes())?;
        return Ok(());
    }
    if header == APPLIED_INPUT_HEADER {
        local_input.ok_or("local input is disabled")?;
        let (sequence, tick) = state
            .read()
            .map_err(|_| "snapshot state lock poisoned")?
            .applied;
        connection.write_all(&APPLIED_INPUT_HEADER)?;
        connection.write_all(&sequence.to_be_bytes())?;
        connection.write_all(&tick.to_be_bytes())?;
        return Ok(());
    }
    if header == CharacterReplicationSnapshot::HEADER {
        local_input.ok_or("local input is disabled")?;
        let (character, (sequence, applied_tick)) = {
            let state = state.read().map_err(|_| "snapshot state lock poisoned")?;
            let character = Arc::clone(
                state
                    .character
                    .as_ref()
                    .ok_or("character snapshot unavailable")?,
            );
            (character, state.applied)
        };
        connection.write_all(&CharacterReplicationSnapshot::HEADER)?;
        connection.write_all(&sequence.to_be_bytes())?;
        connection.write_all(&applied_tick.to_be_bytes())?;
        character.write_frame(&mut connection, MAX_SNAPSHOT_BYTES)?;
        return Ok(());
    }
    let prediction = header == PredictionUpdate::HEADER;
    let baseline = if prediction {
        local_input.ok_or("local input is disabled")?;
        SnapshotBaseline::read_request(&mut connection)?
    } else {
        SnapshotBaseline::read_request(&mut header.as_slice().chain(&mut connection))?
    };
    let confirmed = match baseline {
        SnapshotBaseline::Full => None,
        SnapshotBaseline::Confirmed { tick, state_crc32c } => Some((tick, state_crc32c)),
    };
    let (update, tick, (sequence, applied_tick)) = {
        let state = state.read().map_err(|_| "snapshot state lock poisoned")?;
        let update = match state
            .snapshots
            .update_for_confirmed(confirmed)
            .ok_or("snapshot history is empty")?
        {
            SnapshotUpdate::Full(snapshot) => {
                SnapshotUpdate::Full(Cow::Owned(snapshot.into_owned()))
            }
            SnapshotUpdate::Delta(delta) => SnapshotUpdate::Delta(delta),
            SnapshotUpdate::UpToDate => SnapshotUpdate::UpToDate,
        };
        (update, state.tick, state.applied)
    };
    if prediction {
        PredictionUpdate {
            applied_sequence: sequence,
            applied_tick,
            state_tick: tick,
            update,
        }
        .write_frame(&mut connection, MAX_SNAPSHOT_BYTES)?;
    } else {
        update.write_frame(&mut connection, MAX_SNAPSHOT_BYTES)?;
    }
    Ok(())
}

fn serve(
    listener: TcpListener,
    state: Arc<RwLock<PublishedState>>,
    local_input: Option<Arc<LocalInput>>,
) -> io::Result<()> {
    let mut workers: Vec<thread::JoinHandle<()>> = Vec::with_capacity(MAX_CONCURRENT_REQUESTS);
    for connection in listener.incoming() {
        let connection = match connection {
            Ok(connection) => connection,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        workers.retain(|worker| !worker.is_finished());
        if workers.len() == MAX_CONCURRENT_REQUESTS {
            continue;
        }
        let state = Arc::clone(&state);
        let local_input = local_input.clone();
        match thread::Builder::new()
            .name("voxy-snapshot-request".into())
            .spawn(move || {
                if let Err(error) = reply(connection, &state, local_input.as_deref()) {
                    eprintln!("VOXY SERVER request failed: {error}");
                }
            }) {
            Ok(worker) => workers.push(worker),
            Err(error) => eprintln!("VOXY SERVER cannot start request worker: {error}"),
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<_> = std::env::args_os().skip(1).collect();
    let (package, address, local_input) = match arguments.as_slice() {
        [package, address] => (package, address, false),
        [package, address, flag] if flag == "--local-input" => (package, address, true),
        _ => return Err("usage: voxy_server PROJECT.vpak IP:PORT [--local-input]".into()),
    };
    let address: std::net::SocketAddr =
        address.to_str().ok_or("invalid listen address")?.parse()?;
    if local_input && !address.ip().is_loopback() {
        return Err("--local-input requires a loopback listen address".into());
    }
    let listener = TcpListener::bind(address)?;
    let local_address = listener.local_addr()?;
    let host = GameHost::new(editor_component_registry()?);
    let mut game = HeadlessGame::open_package(Path::new(package), host)?;
    let mut history = SnapshotRollbackBuffer::new(HISTORY_TICKS);
    let (initial, character) = capture(&game, local_input)?;
    let mut published_tick = initial.tick;
    history.push(initial)?;
    let state = Arc::new(RwLock::new(PublishedState {
        snapshots: history,
        tick: published_tick,
        applied: (0, 0),
        character,
    }));
    let network_state = Arc::clone(&state);
    let (input, commands) = mpsc::sync_channel(INPUT_QUEUE_CAPACITY);
    let network_input = local_input.then(|| {
        Arc::new(LocalInput {
            commands: input,
            last: Mutex::new(None),
        })
    });
    let network = thread::Builder::new()
        .name("voxy-snapshot-server".into())
        .spawn(move || serve(listener, network_state, network_input))?;
    println!("VOXY SERVER READY {local_address}");

    let step = Duration::from_secs_f64(STEP_SECONDS);
    let mut deadline = Instant::now() + step;
    let mut active_input = None;
    while !network.is_finished() {
        thread::sleep(deadline.saturating_duration_since(Instant::now()));
        if active_input.is_none() {
            active_input = commands.try_recv().ok();
            if let Some(command) = &active_input {
                game.key(command.key, true)?;
            }
        }
        game.advance(STEP_SECONDS)?;
        if game.fixed_ticks() > published_tick {
            let completed_input = active_input.take();
            if let Some(command) = &completed_input {
                game.key(command.key, false)?;
            }
            let (snapshot, character) = capture(&game, local_input)?;
            published_tick = snapshot.tick;
            let mut state = state.write().map_err(|_| "snapshot state lock poisoned")?;
            state.snapshots.push(snapshot)?;
            state.tick = published_tick;
            state.character = character;
            if let Some(sequence) = completed_input.and_then(|command| command.sequence) {
                state.applied = (sequence, published_tick);
            }
        }
        deadline = (deadline + step).max(Instant::now());
    }
    game.stop()?;
    network
        .join()
        .map_err(|_| "snapshot server thread panicked")??;
    Ok(())
}
