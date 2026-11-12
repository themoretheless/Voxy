use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant, SystemTime};
use voxy_editor::{
    CharacterReplicationSnapshot, editor_component_registry, export_project_package,
};
use voxy_scene::{ObjectId, PredictionClient, PredictionUpdate, SnapshotBaseline, SnapshotUpdate};
use winit::keyboard::KeyCode;

struct Server {
    child: Child,
    root: PathBuf,
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn connect(address: SocketAddr) -> std::io::Result<TcpStream> {
    let timeout = Duration::from_secs(2);
    let stream = TcpStream::connect_timeout(&address, timeout)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    Ok(stream)
}

fn send_right(address: SocketAddr, sequence: u64, wait: bool) -> std::io::Result<()> {
    let mut stream = connect(address)?;
    let mut message = *b"VXSI\x02\0\0\0\0\0\0\0\0\x01";
    message[5..13].copy_from_slice(&sequence.to_be_bytes());
    stream.write_all(&message)?;
    if wait {
        let mut reply = [0; 13];
        stream.read_exact(&mut reply)?;
        assert_eq!(&reply[..5], &message[..5]);
        assert_eq!(&reply[5..], &message[5..13]);
    }
    Ok(())
}

#[test]
fn character_prediction_matches_dedicated_server_after_lost_ack()
-> Result<(), Box<dyn std::error::Error>> {
    let nonce = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)?
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("voxy-server-replay-{}-{nonce}", std::process::id()));
    let source = root.join("source");
    std::fs::create_dir_all(&source)?;
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/interactive-project");
    for entry in std::fs::read_dir(fixture)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            std::fs::copy(entry.path(), source.join(entry.file_name()))?;
        }
    }
    let scene = source.join("main.scene.json");
    std::fs::write(
        &scene,
        std::fs::read_to_string(&scene)?.replace("sample.dismiss", "voxy.ui.hide"),
    )?;
    let package = root.join("game.vpak");
    export_project_package(&source.join("project.voxy"), &package)?;
    std::fs::remove_dir_all(source)?;

    let child = Command::new(env!("CARGO_BIN_EXE_voxy_server"))
        .arg(&package)
        .args(["127.0.0.1:0", "--local-input"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut server = Server { child, root };
    let stdout = server.child.stdout.take().ok_or("missing server stdout")?;
    let (ready, address) = std::sync::mpsc::sync_channel(1);
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if let Some(value) = line.strip_prefix("VOXY SERVER READY ") {
                let _ = ready.send(value.parse::<SocketAddr>());
            }
        }
    });
    let address = address.recv_timeout(Duration::from_secs(10))??;
    let registry = editor_component_registry()?;
    let mut client = PredictionClient::default();
    assert!(
        client
            .synchronize(&mut connect(address)?, &registry, 64, 64 * 1024)?
            .is_some()
    );
    let (sequence, _, initial) =
        CharacterReplicationSnapshot::request(&mut connect(address)?, 64 * 1024)?;
    assert_eq!(sequence, 0);
    let predicted =
        initial.replay_character_inputs(&registry, 64, &[Some(KeyCode::ArrowRight); 3], 3)?;
    let player_id = ObjectId("player".into());
    let player = predicted
        .resolve(&player_id)
        .ok_or("missing predicted player")?;
    let expected = predicted.graph.local(player)?.translation.to_array();
    assert!(expected[0] > 0.0);

    send_right(address, 1, false)?;
    for sequence in 1..=3 {
        send_right(address, sequence, true)?;
    }
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut applied = false;
    loop {
        let (sequence, applied_tick, snapshot) =
            CharacterReplicationSnapshot::request(&mut connect(address)?, 64 * 1024)?;
        if sequence == 3 {
            assert!(applied_tick > initial.scene.tick);
            let actual = snapshot
                .scene
                .document
                .objects
                .iter()
                .find(|object| object.id == player_id)
                .ok_or("missing authoritative player")?;
            assert_eq!(actual.translation, expected);
            applied = true;
            if snapshot.scene.tick >= initial.scene.tick + 120 {
                break;
            }
        }
        assert!(
            Instant::now() < deadline,
            "server did not apply input and expire the baseline"
        );
        std::thread::sleep(Duration::from_millis(if applied { 50 } else { 10 }));
    }
    let mut connection = connect(address)?;
    PredictionUpdate::write_request(
        SnapshotBaseline::Confirmed {
            tick: initial.scene.tick,
            state_crc32c: initial.scene.state_crc32c,
        },
        &mut connection,
    )?;
    let reply = PredictionUpdate::read_frame(&mut connection, 64 * 1024)?;
    assert_eq!(reply.applied_sequence, 3);
    assert!(matches!(reply.update, SnapshotUpdate::Full(_)));
    let scene = client
        .synchronize(&mut connect(address)?, &registry, 64, 64 * 1024)?
        .ok_or("reconnect did not install a scene")?;
    assert_eq!(client.applied_input(), (3, reply.applied_tick));
    let player = scene
        .resolve(&player_id)
        .ok_or("missing reconnected player")?;
    assert_eq!(scene.graph.local(player)?.translation.to_array(), expected);
    Ok(())
}
