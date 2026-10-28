//! One bounded transport for the original participant codecs, not a second runtime.
use super::*;
use std::io::Read;
const MAGIC: &[u8; 8] = b"VOXYCGR1";
const HEADER: usize = 72;
#[derive(Clone, Copy)]
pub(crate) struct CoordinatedFileLimits {
    pub max_bytes: usize,
    pub max_game_bytes: usize,
    pub ui: crate::ui_live::checkpoint::UiLiveCheckpointLimits,
    pub max_audio_bytes: usize,
    pub max_input_bytes: usize,
}
impl CoordinatedPlayCheckpoint {
    pub(crate) fn to_bytes(&self, limits: CoordinatedFileLimits) -> Result<Vec<u8>, EditorError> {
        let mut remaining = limits
            .max_bytes
            .checked_sub(HEADER)
            .ok_or("coordinated file byte budget")?;
        let game = self.game.to_bytes(limits.max_game_bytes.min(remaining))?;
        remaining -= game.len();
        if remaining == 0 {
            return Err("coordinated file byte budget".into());
        }
        let mut ui_limits = limits.ui;
        ui_limits.max_bytes = ui_limits.max_bytes.min(remaining);
        let ui = self.ui.to_bytes(ui_limits)?;
        remaining -= ui.len();
        if remaining == 0 {
            return Err("coordinated file byte budget".into());
        }
        let audio = self
            .audio
            .to_json(limits.max_audio_bytes.min(remaining))?
            .into_bytes();
        remaining -= audio.len();
        if remaining == 0 {
            return Err("coordinated file byte budget".into());
        }
        let input = self
            .ui_input
            .to_json(limits.max_input_bytes.min(remaining))?
            .into_bytes();
        let sections = [game, ui, audio, input];
        let size = sections.iter().try_fold(HEADER, |size, section| {
            size.checked_add(section.len())
                .ok_or("coordinated file size overflow")
        })?;
        if size > limits.max_bytes {
            return Err("coordinated file byte budget".into());
        }
        let mut output = Vec::with_capacity(size);
        output.extend_from_slice(MAGIC);
        for section in &sections {
            output.extend_from_slice(&u64::try_from(section.len())?.to_le_bytes());
        }
        let mut hash = blake3::Hasher::new();
        hash.update(&output[8..]);
        for section in &sections {
            hash.update(section);
        }
        output.extend_from_slice(hash.finalize().as_bytes());
        for section in sections {
            output.extend_from_slice(&section);
        }
        Ok(output)
    }
    pub(crate) fn from_bytes(
        bytes: &[u8],
        limits: CoordinatedFileLimits,
    ) -> Result<Self, EditorError> {
        if bytes.len() < HEADER || bytes.len() > limits.max_bytes || &bytes[..8] != MAGIC {
            return Err("invalid coordinated file header or budget".into());
        }
        let mut hash = blake3::Hasher::new();
        hash.update(&bytes[8..40]);
        hash.update(&bytes[HEADER..]);
        if hash.finalize().as_bytes() != &bytes[40..HEADER] {
            return Err("corrupt coordinated file".into());
        }
        let caps = [
            limits.max_game_bytes,
            limits.ui.max_bytes,
            limits.max_audio_bytes,
            limits.max_input_bytes,
        ];
        let mut sections = Vec::with_capacity(4);
        let mut offset = HEADER;
        for (index, cap) in caps.into_iter().enumerate() {
            let start = 8 + index * 8;
            let length = usize::try_from(u64::from_le_bytes(
                bytes[start..start + 8]
                    .try_into()
                    .map_err(|_| "invalid coordinated section length")?,
            ))?;
            let end = offset
                .checked_add(length)
                .ok_or("coordinated section size overflow")?;
            if length > cap || end > bytes.len() {
                return Err("coordinated section byte budget or length".into());
            }
            sections.push(&bytes[offset..end]);
            offset = end;
        }
        if offset != bytes.len() {
            return Err("trailing coordinated file data".into());
        }
        Ok(Self {
            game: crate::app::resource_checkpoint::play::PlayResourcesCheckpoint::from_bytes(
                sections[0],
                limits.max_game_bytes,
            )?,
            ui: crate::ui_live::checkpoint::UiLiveCheckpoint::from_bytes(sections[1], limits.ui)?,
            audio: AudioSessionCheckpoint::from_json(
                std::str::from_utf8(sections[2])?,
                limits.max_audio_bytes,
            )?,
            ui_input: crate::game_ui_input::checkpoint::GameUiInputCheckpoint::from_json(
                std::str::from_utf8(sections[3])?,
                limits.max_input_bytes,
            )?,
        })
    }
    /// Blocking trusted-path IO; validation/encoding completes before replacement.
    pub(crate) fn save_file(
        &self,
        path: &std::path::Path,
        limits: CoordinatedFileLimits,
    ) -> Result<(), EditorError> {
        let bytes = self.to_bytes(limits)?;
        voxy_assets::save_atomic_file(path, &bytes, limits.max_bytes).map_err(|e| e.to_string())?;
        Ok(())
    }
    pub(crate) fn read_file(
        path: &std::path::Path,
        limits: CoordinatedFileLimits,
    ) -> Result<Self, EditorError> {
        let file = std::fs::File::open(path)?;
        let max = u64::try_from(limits.max_bytes)?;
        if file.metadata()?.len() > max {
            return Err("coordinated file byte budget".into());
        }
        let mut bytes = Vec::new();
        file.take(
            max.checked_add(1)
                .ok_or("coordinated read limit overflow")?,
        )
        .read_to_end(&mut bytes)?;
        Self::from_bytes(&bytes, limits)
    }
}
