//! Bounded archive of an accepted immutable publication and its original inputs.
use super::*;
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug)]
pub struct ImportedArchiveLimits {
    pub max_inputs: usize,
    pub max_input_bytes: usize,
    pub max_archive_bytes: usize,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct InputsWire {
    max_inputs: usize,
    max_bytes: usize,
    records: Vec<Record>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    id: String,
    bytes: Vec<u8>,
    digest: [u8; 32],
}
const HEADER: usize = 56;
fn invalid() -> InputError {
    InputError::Read("invalid imported publication archive".into())
}
impl<T> ImportedAsset<T> {
    /// Encodes the accepted payload with captured inputs; no source read is performed.
    /// The payload codec and its semantic limits belong to the resource type.
    /// # Errors
    /// Rejects input policy limits, invalid observations, codec failure and byte overflow.
    pub fn to_checkpoint_bytes(
        &self,
        encode: impl FnOnce(&T) -> Result<Vec<u8>, InputError>,
        limits: ImportedArchiveLimits,
    ) -> Result<Vec<u8>, InputError> {
        if self.inputs.max_inputs > limits.max_inputs
            || self.inputs.max_bytes > limits.max_input_bytes
            || self.inputs.observations.len() > limits.max_inputs
            || self.inputs.bytes > limits.max_input_bytes
        {
            return Err(InputError::Capacity);
        }
        if let Some(error) = &self.inputs.rejected {
            return Err(error.clone());
        }
        let mut records = Vec::new();
        for (id, input) in &self.inputs.observations {
            let input = input.as_ref().map_err(Clone::clone)?;
            if blake3::hash(&input.bytes).as_bytes() != &input.digest {
                return Err(InputError::Changed(id.clone()));
            }
            records.push(Record {
                id: id.0.clone(),
                bytes: input.bytes.to_vec(),
                digest: input.digest,
            });
        }
        let inputs = serde_json::to_vec(&InputsWire {
            max_inputs: self.inputs.max_inputs,
            max_bytes: self.inputs.max_bytes,
            records,
        })
        .map_err(|_| invalid())?;
        let payload = encode(&self.value)?;
        let size = HEADER
            .checked_add(inputs.len())
            .and_then(|s| s.checked_add(payload.len()))
            .ok_or(InputError::Capacity)?;
        if size > limits.max_archive_bytes {
            return Err(InputError::Capacity);
        }
        let mut bytes = Vec::with_capacity(size);
        bytes.extend_from_slice(b"VOXYIMP1");
        bytes.extend_from_slice(
            &u64::try_from(inputs.len())
                .map_err(|_| InputError::Capacity)?
                .to_le_bytes(),
        );
        bytes.extend_from_slice(
            &u64::try_from(payload.len())
                .map_err(|_| InputError::Capacity)?
                .to_le_bytes(),
        );
        let mut hasher = blake3::Hasher::new();
        hasher.update(&inputs);
        hasher.update(&payload);
        bytes.extend_from_slice(hasher.finalize().as_bytes());
        bytes.extend_from_slice(&inputs);
        bytes.extend_from_slice(&payload);
        Ok(bytes)
    }
    /// Restores from the archive alone. Payload decoding must enforce type-specific limits.
    /// The checksum detects corruption; it is not a signature or trusted provenance.
    /// # Errors
    /// Rejects malformed/corrupt data, policy violations and codec failure.
    pub fn from_checkpoint_bytes(
        bytes: &[u8],
        decode: impl FnOnce(&[u8]) -> Result<T, InputError>,
        limits: ImportedArchiveLimits,
    ) -> Result<Self, InputError> {
        Self::from_checkpoint_bytes_with_inputs(bytes, |payload, _| decode(payload), limits)
    }
    /// Decode a typed payload with read-only access to its already validated
    /// original inputs. The decoder must enforce its own payload/work policies.
    pub fn from_checkpoint_bytes_with_inputs(
        bytes: &[u8],
        decode: impl FnOnce(&[u8], &ImportInputs) -> Result<T, InputError>,
        limits: ImportedArchiveLimits,
    ) -> Result<Self, InputError> {
        if bytes.len() > limits.max_archive_bytes {
            return Err(InputError::Capacity);
        }
        if bytes.len() < HEADER || &bytes[..8] != b"VOXYIMP1" {
            return Err(invalid());
        }
        let input_len = usize::try_from(u64::from_le_bytes(
            bytes[8..16].try_into().map_err(|_| invalid())?,
        ))
        .map_err(|_| InputError::Capacity)?;
        let payload_len = usize::try_from(u64::from_le_bytes(
            bytes[16..24].try_into().map_err(|_| invalid())?,
        ))
        .map_err(|_| InputError::Capacity)?;
        if HEADER
            .checked_add(input_len)
            .and_then(|n| n.checked_add(payload_len))
            != Some(bytes.len())
        {
            return Err(invalid());
        }
        if blake3::hash(&bytes[HEADER..]).as_bytes() != &bytes[24..HEADER] {
            return Err(invalid());
        }
        let wire: InputsWire =
            serde_json::from_slice(&bytes[HEADER..HEADER + input_len]).map_err(|_| invalid())?;
        if wire.max_inputs > limits.max_inputs
            || wire.max_bytes > limits.max_input_bytes
            || wire.records.len() > wire.max_inputs
        {
            return Err(InputError::Capacity);
        }
        let mut inputs = ImportInputs::new(wire.max_inputs, wire.max_bytes);
        for record in wire.records {
            let id = AssetId(record.id);
            if id.0.is_empty() || inputs.observations.contains_key(&id) {
                return Err(InputError::InvalidId);
            }
            inputs.bytes = inputs
                .bytes
                .checked_add(record.bytes.len())
                .ok_or(InputError::Capacity)?;
            if inputs.bytes > inputs.max_bytes {
                return Err(InputError::Capacity);
            }
            if blake3::hash(&record.bytes).as_bytes() != &record.digest {
                return Err(InputError::Changed(id));
            }
            inputs.observations.insert(
                id,
                Ok(InputSnapshot {
                    bytes: record.bytes.into(),
                    digest: record.digest,
                }),
            );
        }
        let value = decode(&bytes[HEADER + input_len..], &inputs)?;
        Ok(Self { value, inputs })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn publication_and_inputs_roundtrip_without_source_files_and_reject_corruption() {
        let mut inputs = ImportInputs::new(2, 100);
        let id = AssetId("sound.wav".into());
        inputs
            .read(id.clone(), |_, _| Ok(vec![1, 2, 3, 4]))
            .unwrap();
        let original = inputs.finish(17_u32, |_, _| Ok(vec![1, 2, 3, 4])).unwrap();
        let limits = ImportedArchiveLimits {
            max_inputs: 2,
            max_input_bytes: 100,
            max_archive_bytes: 4096,
        };
        let bytes = original
            .to_checkpoint_bytes(|v| Ok(v.to_le_bytes().to_vec()), limits)
            .unwrap();
        let decode = |b: &[u8]| Ok(u32::from_le_bytes(b.try_into().map_err(|_| invalid())?));
        let restored = ImportedAsset::from_checkpoint_bytes(&bytes, decode, limits).unwrap();
        assert_eq!(restored.value(), original.value());
        assert_eq!(restored.inputs.observations, original.inputs.observations);
        assert_eq!(restored.inputs.max_inputs, original.inputs.max_inputs);
        assert_eq!(restored.inputs.max_bytes, original.inputs.max_bytes);
        assert_eq!(restored.inputs.bytes, original.inputs.bytes);
        assert_eq!(
            restored
                .to_checkpoint_bytes(|v| Ok(v.to_le_bytes().to_vec()), limits)
                .unwrap(),
            bytes
        );
        for index in [0, 8, 16, 24, HEADER, bytes.len() - 1] {
            let mut bad = bytes.clone();
            bad[index] ^= 1;
            assert!(ImportedAsset::from_checkpoint_bytes(&bad, decode, limits).is_err());
        }
        assert!(
            ImportedAsset::from_checkpoint_bytes(
                &bytes,
                decode,
                ImportedArchiveLimits {
                    max_input_bytes: 99,
                    ..limits
                }
            )
            .is_err()
        );
        assert!(
            ImportedAsset::from_checkpoint_bytes(
                &bytes,
                decode,
                ImportedArchiveLimits {
                    max_archive_bytes: bytes.len() - 1,
                    ..limits
                }
            )
            .is_err()
        );
        let mut inputs = restored.inputs;
        assert_eq!(
            &*inputs
                .read(id, |_, _| panic!("archived read must be cached"))
                .unwrap()
                .bytes,
            &[1, 2, 3, 4]
        );
    }
}

#[cfg(test)]
mod input_aware_tests {
    use super::*;
    #[test]
    fn decoder_shares_verified_input_bytes_and_is_not_called_for_corruption() {
        let id = AssetId("font".into());
        let mut inputs = ImportInputs::new(1, 16);
        inputs.read(id.clone(), |_, _| Ok(vec![1, 2, 3])).unwrap();
        let original = inputs.finish(9_u8, |_, _| Ok(vec![1, 2, 3])).unwrap();
        let limits = ImportedArchiveLimits {
            max_inputs: 1,
            max_input_bytes: 16,
            max_archive_bytes: 4096,
        };
        let bytes = original
            .to_checkpoint_bytes(|value| Ok(vec![*value]), limits)
            .unwrap();
        let restored = ImportedAsset::from_checkpoint_bytes_with_inputs(
            &bytes,
            |payload, inputs| {
                assert_eq!(payload, [9]);
                Ok(inputs.observations()[&id].as_ref().unwrap().bytes.clone())
            },
            limits,
        )
        .unwrap();
        assert!(std::sync::Arc::ptr_eq(
            restored.value(),
            &restored.inputs().observations()[&id]
                .as_ref()
                .unwrap()
                .bytes
        ));
        let called = std::cell::Cell::new(false);
        let mut bad = bytes;
        let last = bad.len() - 1;
        bad[last] ^= 1;
        assert!(
            ImportedAsset::<u8>::from_checkpoint_bytes_with_inputs(
                &bad,
                |_, _| {
                    called.set(true);
                    Ok(0)
                },
                limits
            )
            .is_err()
        );
        assert!(!called.get());
    }
}
