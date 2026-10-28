//! File representation of gameplay input; platform compatibility belongs to the game envelope.
use serde::{Deserialize, Serialize};
use voxy_input::{ActionCheckpoint, Control, InputCheckpoint, InputMap};

#[derive(Debug, thiserror::Error)]
pub enum InputCheckpointCodecError {
    #[error("input checkpoint exceeds byte budget")]
    Capacity,
    #[error(transparent)]
    Encoding(#[from] serde_json::Error),
    #[error(transparent)]
    Input(#[from] voxy_input::InputError),
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    version: u32,
    focused: bool,
    actions: Vec<ActionWire>,
    controls: Vec<[u32; 3]>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActionWire {
    name: String,
    bindings: Vec<[u32; 3]>,
    dead_zone_bits: u32,
    value_bits: u32,
    held: bool,
    pressed: bool,
    released: bool,
}
fn pack((control, bits): &(Control, u32)) -> [u32; 3] {
    [control.device, control.code, *bits]
}
fn unpack([device, code, bits]: [u32; 3]) -> (Control, u32) {
    (Control { device, code }, bits)
}
fn validate(
    saved: &InputCheckpoint,
    actions: usize,
    bindings: usize,
) -> Result<(), voxy_input::InputError> {
    InputMap::new(actions, bindings).restore_checkpoint(saved)
}

/// Encode without consuming logical edges or changing the live input owner.
/// Float values use integer bits, including signed zero and subnormal values.
/// # Errors
/// Rejects invalid state, admission limits and oversized output.
pub fn encode_input_checkpoint(
    saved: &InputCheckpoint,
    max_bytes: usize,
    max_actions: usize,
    max_bindings: usize,
) -> Result<String, InputCheckpointCodecError> {
    validate(saved, max_actions, max_bindings)?;
    let wire = Wire {
        version: saved.version,
        focused: saved.focused,
        controls: saved.controls.iter().map(pack).collect(),
        actions: saved
            .actions
            .iter()
            .map(|a| ActionWire {
                name: a.name.clone(),
                bindings: a.bindings.iter().map(pack).collect(),
                dead_zone_bits: a.dead_zone_bits,
                value_bits: a.value_bits,
                held: a.held,
                pressed: a.pressed,
                released: a.released,
            })
            .collect(),
    };
    let text = serde_json::to_string(&wire)?;
    if text.len() > max_bytes {
        return Err(InputCheckpointCodecError::Capacity);
    }
    Ok(text)
}

/// Decode into a separately validated participant for an enclosing save transaction.
/// The caller must bound the file read and verify the physical input adapter identity.
/// # Errors
/// Rejects oversized text before parsing, unknown fields, corrupt state and limits.
pub fn decode_input_checkpoint(
    text: &str,
    max_bytes: usize,
    max_actions: usize,
    max_bindings: usize,
) -> Result<InputCheckpoint, InputCheckpointCodecError> {
    if text.len() > max_bytes {
        return Err(InputCheckpointCodecError::Capacity);
    }
    let wire: Wire = serde_json::from_str(text)?;
    let saved = InputCheckpoint {
        version: wire.version,
        focused: wire.focused,
        controls: wire.controls.into_iter().map(unpack).collect(),
        actions: wire
            .actions
            .into_iter()
            .map(|a| ActionCheckpoint {
                name: a.name,
                bindings: a.bindings.into_iter().map(unpack).collect(),
                dead_zone_bits: a.dead_zone_bits,
                value_bits: a.value_bits,
                held: a.held,
                pressed: a.pressed,
                released: a.released,
            })
            .collect(),
    };
    validate(&saved, max_actions, max_bindings)?;
    Ok(saved)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn file_codec_preserves_analog_and_unconsumed_edges_and_continuation() {
        let mut source = crate::player_input().unwrap();
        let control = source.checkpoint().actions[0].bindings[0].0;
        source.event(control, 0.375).unwrap();
        let action = source.checkpoint().actions[0].name.clone();
        source.activate(&action).unwrap();
        let saved = source.checkpoint();
        let text = encode_input_checkpoint(&saved, 65536, 32, 64).unwrap();
        let decoded = decode_input_checkpoint(&text, 65536, 32, 64).unwrap();
        assert_eq!(saved, decoded);
        let mut restored = InputMap::new(32, 64);
        restored.restore_checkpoint(&decoded).unwrap();
        for i in 0..120 {
            let value = if i % 3 == 0 { 0. } else { 0.375 };
            source.event(control, value).unwrap();
            restored.event(control, value).unwrap();
            assert_eq!(source.checkpoint(), restored.checkpoint());
            source.finish_frame();
            restored.finish_frame();
        }
    }
    #[test]
    fn corrupt_or_oversized_records_cannot_modify_live_input() {
        let source = crate::player_input().unwrap();
        let saved = source.checkpoint();
        let text = encode_input_checkpoint(&saved, 65536, 32, 64).unwrap();
        assert!(decode_input_checkpoint(&text, 1, 32, 64).is_err());
        assert!(decode_input_checkpoint(&text, 65536, 0, 0).is_err());
        assert!(encode_input_checkpoint(&saved, 1, 32, 64).is_err());
        for field in ["version", "value_bits", "extra"] {
            let mut value: serde_json::Value = serde_json::from_str(&text).unwrap();
            match field {
                "version" => value[field] = 9.into(),
                "value_bits" => value["actions"][0][field] = f32::NAN.to_bits().into(),
                _ => value[field] = true.into(),
            }
            assert!(decode_input_checkpoint(&value.to_string(), 65536, 32, 64).is_err());
        }
        assert_eq!(source.checkpoint(), saved);
    }
}
