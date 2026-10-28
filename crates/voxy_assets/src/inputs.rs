//! Bounded, immutable importer inputs. The importer reads through this provider.
use std::{collections::BTreeMap, sync::Arc};

use crate::AssetId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputSnapshot {
    pub bytes: Arc<[u8]>,
    pub digest: [u8; 32],
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InputError {
    InvalidId,
    Capacity,
    Changed(AssetId),
    Read(String),
}
/// A decoded value and its captured inputs share one immutable publication.
/// Use `AssetCatalog<ImportedAsset<T>>` to replace them together.
#[derive(Clone, Debug)]
pub struct ImportedAsset<T> {
    value: T,
    inputs: ImportInputs,
}
impl<T> ImportedAsset<T> {
    #[must_use]
    pub const fn value(&self) -> &T {
        &self.value
    }
    /// Stages a receipt for an owner-authored replacement of exactly one input.
    /// All original inputs are validated first; unchanged snapshots retain their
    /// Arc identities and the decoded value is cloned without re-decoding.
    /// This is detached state: install it only after publishing exactly `after`.
    /// It does not write files or authorize unrelated input changes.
    /// # Errors
    /// Rejects a stale baseline, invalid replacement digest or original quotas.
    pub fn prepare_input_replacement(
        &self,
        id: &AssetId,
        before: &InputSnapshot,
        after: InputSnapshot,
        reader: impl FnMut(&AssetId, usize) -> Result<Vec<u8>, String>,
    ) -> Result<Self, InputError>
    where
        T: Clone,
    {
        if self.inputs.observations.get(id) != Some(&Ok(before.clone())) {
            return Err(InputError::Changed(id.clone()));
        }
        let bytes = self
            .inputs
            .bytes
            .checked_sub(before.bytes.len())
            .and_then(|n| n.checked_add(after.bytes.len()))
            .filter(|n| *n <= self.inputs.max_bytes)
            .ok_or(InputError::Capacity)?;
        if after.digest != *blake3::hash(&after.bytes).as_bytes() {
            return Err(InputError::Changed(id.clone()));
        }
        self.inputs.validate(reader)?;
        let mut inputs = self.inputs.clone();
        inputs.bytes = bytes;
        inputs.observations.insert(id.clone(), Ok(after));
        Ok(Self {
            value: self.value.clone(),
            inputs,
        })
    }
    /// Rejects a decoded candidate while retaining its source observations.
    /// This lets owners enforce compatibility with existing scene references.
    #[must_use]
    pub fn into_failed<E>(self, error: E) -> FailedImport<E> {
        FailedImport {
            error,
            inputs: self.inputs,
        }
    }
    #[must_use]
    pub const fn inputs(&self) -> &ImportInputs {
        &self.inputs
    }
}
/// A failed decoder retains successful and failed source reads for diagnostics.
#[derive(Debug)]
pub struct FailedImport<E> {
    pub error: E,
    pub inputs: ImportInputs,
}
/// A rejected publication retains its decoded value and every input observation.
/// The owner decides when to release these bounded attempt resources.
#[derive(Debug)]
pub struct RejectedImport<T> {
    pub value: T,
    pub inputs: ImportInputs,
    pub error: InputError,
}
/// A read attempt remains observable even when loading fails. Failed reads remain cached for this import attempt; successful reads always return the same captured bytes.
#[derive(Clone, Debug)]
pub struct ImportInputs {
    observations: BTreeMap<AssetId, Result<InputSnapshot, InputError>>,
    max_inputs: usize,
    max_bytes: usize,
    bytes: usize,
    rejected: Option<InputError>,
}
impl ImportInputs {
    #[must_use]
    pub fn new(max_inputs: usize, max_bytes: usize) -> Self {
        Self {
            observations: BTreeMap::new(),
            max_inputs,
            max_bytes,
            bytes: 0,
            rejected: None,
        }
    }
    /// Runs a decoder against owned observations, returning them on both paths.
    /// Success still requires `finish_observed` and catalog ticket validation.
    /// # Errors
    /// Returns the decoder error with all reads performed before that error.
    pub fn decode_observed<T, E>(
        mut self,
        decoder: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<(T, Self), FailedImport<E>> {
        match decoder(&mut self) {
            Ok(value) => Ok((value, self)),
            Err(error) => Err(FailedImport {
                error,
                inputs: self,
            }),
        }
    }

    /// Reads at most the remaining byte budget through a caller-owned provider.
    /// The provider must enforce the supplied limit while reading, rather than
    /// allocating an unbounded buffer first. Provider roots/permissions are its
    /// responsibility. An oversized returned buffer is rejected defensively.
    /// # Errors
    /// Rejects empty identities, count/byte exhaustion, or provider errors.
    pub fn read(
        &mut self,
        id: AssetId,
        reader: impl FnOnce(&AssetId, usize) -> Result<Vec<u8>, String>,
    ) -> Result<InputSnapshot, InputError> {
        if id.0.is_empty() {
            self.rejected.get_or_insert(InputError::InvalidId);
            return Err(InputError::InvalidId);
        }
        if let Some(result) = self.observations.get(&id) {
            return result.clone();
        }
        if self.observations.len() >= self.max_inputs {
            self.rejected.get_or_insert(InputError::Capacity);
            return Err(InputError::Capacity);
        }
        let remaining = self.max_bytes - self.bytes;
        let result = reader(&id, remaining)
            .map_err(InputError::Read)
            .and_then(|bytes| {
                if bytes.len() > remaining {
                    return Err(InputError::Capacity);
                }
                let digest = *blake3::hash(&bytes).as_bytes();
                self.bytes += bytes.len();
                Ok(InputSnapshot {
                    bytes: bytes.into(),
                    digest,
                })
            });
        self.observations.insert(id, result.clone());
        result
    }
    /// Packages a decoded value with its observations after content validation.
    /// This does not lock external files or replace catalog ticket validation.
    /// # Errors
    /// Rejects failed or changed input observations.
    pub fn finish<T>(
        self,
        value: T,
        reader: impl FnMut(&AssetId, usize) -> Result<Vec<u8>, String>,
    ) -> Result<ImportedAsset<T>, InputError> {
        self.validate(reader)?;
        Ok(ImportedAsset {
            value,
            inputs: self,
        })
    }

    /// Validates publication while retaining inputs and value on rejection.
    /// Unlike `finish`, this lets diagnostics and retry owners inspect failures.
    /// # Errors
    /// Returns the rejected attempt when any input fails validation.
    pub fn finish_observed<T>(
        self,
        value: T,
        reader: impl FnMut(&AssetId, usize) -> Result<Vec<u8>, String>,
    ) -> Result<ImportedAsset<T>, RejectedImport<T>> {
        match self.validate(reader) {
            Ok(()) => Ok(ImportedAsset {
                value,
                inputs: self,
            }),
            Err(error) => Err(RejectedImport {
                value,
                inputs: self,
                error,
            }),
        }
    }

    /// Computes a versioned build identity from all successful input digests,
    /// importer identity/version, target and canonical option bytes. Callers must
    /// include every decoding setting and encode options deterministically.
    /// Sorted input IDs make discovery order irrelevant; lengths separate fields.
    /// This identifies inputs, not deterministic execution or an output cache.
    /// # Errors
    /// Rejects empty importer/target identities and failed observations.
    pub fn build_key(
        &self,
        importer: &str,
        version: &str,
        target: &str,
        options: &[u8],
    ) -> Result<[u8; 32], InputError> {
        if importer.is_empty() || version.is_empty() || target.is_empty() {
            return Err(InputError::InvalidId);
        }
        if let Some(error) = &self.rejected {
            return Err(error.clone());
        }
        let mut hash = blake3::Hasher::new_derive_key("voxy.asset.import.v1");
        for field in [
            importer.as_bytes(),
            version.as_bytes(),
            target.as_bytes(),
            options,
        ] {
            hash.update(&(field.len() as u128).to_le_bytes());
            hash.update(field);
        }
        hash.update(&(self.observations.len() as u128).to_le_bytes());
        for (id, observation) in &self.observations {
            let snapshot = observation.as_ref().map_err(Clone::clone)?;
            hash.update(&(id.0.len() as u128).to_le_bytes());
            hash.update(id.0.as_bytes());
            hash.update(&snapshot.digest);
        }
        Ok(*hash.finalize().as_bytes())
    }

    /// Rechecks successful inputs without modifying captured snapshots.
    /// Each provider read is limited to that input's original length. A provider
    /// must reject longer content, including an appended suffix. Errors from the
    /// original import prevent validation. This is an observation, not a lock:
    /// publication still needs owner revision checks against concurrent changes.
    /// # Errors
    /// Returns original read errors, provider failures, or changed content.
    pub fn validate(
        &self,
        mut reader: impl FnMut(&AssetId, usize) -> Result<Vec<u8>, String>,
    ) -> Result<(), InputError> {
        if let Some(error) = &self.rejected {
            return Err(error.clone());
        }
        for (id, observation) in &self.observations {
            let snapshot = observation.as_ref().map_err(Clone::clone)?;
            let bytes = reader(id, snapshot.bytes.len()).map_err(InputError::Read)?;
            if bytes.len() != snapshot.bytes.len()
                || blake3::hash(&bytes).as_bytes() != &snapshot.digest
            {
                return Err(InputError::Changed(id.clone()));
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn observations(&self) -> &BTreeMap<AssetId, Result<InputSnapshot, InputError>> {
        &self.observations
    }
    #[must_use]
    pub const fn captured_bytes(&self) -> usize {
        self.bytes
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn snapshots_and_failed_dependencies_survive_import_failure() {
        let mut inputs = ImportInputs::new(2, 3);
        let id = AssetId("source".into());
        let first = inputs
            .read(id.clone(), |_, limit| {
                assert_eq!(limit, 3);
                Ok(vec![1, 2, 3])
            })
            .unwrap();
        let second = inputs
            .read(id.clone(), |_, _| panic!("duplicate provider read"))
            .unwrap();
        assert!(Arc::ptr_eq(&first.bytes, &second.bytes));
        assert_eq!(first.digest, *blake3::hash(&[1, 2, 3]).as_bytes());
        let missing = AssetId("include".into());
        assert_eq!(
            inputs.read(missing.clone(), |_, limit| {
                assert_eq!(limit, 0);
                Err("missing".into())
            }),
            Err(InputError::Read("missing".into()))
        );
        assert!(inputs.observations().contains_key(&missing));
        assert_eq!(inputs.captured_bytes(), 3);
        assert!(matches!(
            inputs.read(AssetId("third".into()), |_, _| panic!()),
            Err(InputError::Capacity)
        ));
    }
    #[test]
    fn validation_detects_same_length_changes_growth_and_missing_inputs() {
        let mut inputs = ImportInputs::new(1, 2);
        let id = AssetId("source".into());
        let original = inputs.read(id.clone(), |_, _| Ok(vec![1, 2])).unwrap();
        assert_eq!(
            inputs.validate(|_, limit| {
                assert_eq!(limit, 2);
                Ok(vec![1, 2])
            }),
            Ok(())
        );
        assert_eq!(
            inputs.validate(|_, _| Ok(vec![2, 1])),
            Err(InputError::Changed(id.clone()))
        );
        assert_eq!(
            inputs.validate(|_, _| Ok(vec![1, 2, 3])),
            Err(InputError::Changed(id))
        );
        assert_eq!(
            inputs.validate(|_, _| Err("gone".into())),
            Err(InputError::Read("gone".into()))
        );
        assert_eq!(&*original.bytes, &[1, 2]);
        let mut failed = ImportInputs::new(1, 2);
        let _ = failed.read(AssetId("missing".into()), |_, _| Err("missing".into()));
        assert_eq!(
            failed.validate(|_, _| panic!("failed import cannot validate")),
            Err(InputError::Read("missing".into()))
        );
    }

    #[test]
    fn publication_keeps_input_provenance_with_each_version() {
        use crate::AssetCatalog;
        let source = AssetId("source".into());
        let output = AssetId("output".into());
        let mut catalog = AssetCatalog::new(1, 1).unwrap();
        let mut inputs = ImportInputs::new(1, 1);
        inputs.read(source.clone(), |_, _| Ok(vec![1])).unwrap();
        let artifact = inputs.finish(10_u32, |_, _| Ok(vec![1])).unwrap();
        let ticket = catalog.request(output.clone()).unwrap();
        catalog.complete(&ticket, Ok(artifact)).unwrap();
        let old = catalog.snapshot(&output).unwrap();
        let mut inputs = ImportInputs::new(1, 1);
        inputs.read(source.clone(), |_, _| Ok(vec![2])).unwrap();
        assert!(matches!(
            inputs.finish(20, |_, _| Ok(vec![3])),
            Err(InputError::Changed(_))
        ));
        assert_eq!(*catalog.snapshot(&output).unwrap().value(), 10);
        let mut inputs = ImportInputs::new(1, 1);
        inputs.read(source.clone(), |_, _| Ok(vec![2])).unwrap();
        let replacement = inputs.finish(20, |_, _| Ok(vec![2])).unwrap();
        let ticket = catalog.request(output.clone()).unwrap();
        catalog.complete(&ticket, Ok(replacement)).unwrap();
        assert_eq!(*old.value(), 10);
        assert_eq!(
            &*old.inputs().observations()[&source].as_ref().unwrap().bytes,
            &[1]
        );
        assert_eq!(*catalog.snapshot(&output).unwrap().value(), 20);
    }

    #[test]
    fn build_identity_covers_configuration_and_is_order_independent() {
        let make = |ids: [&str; 2]| {
            let mut inputs = ImportInputs::new(2, 2);
            for id in ids {
                inputs.read(AssetId(id.into()), |_, _| Ok(vec![1])).unwrap();
            }
            inputs
        };
        let a = make(["a", "b"]);
        let b = make(["b", "a"]);
        let key = a.build_key("wav", "1", "desktop", &[0]).unwrap();
        let renamed = make(["a", "c"]);
        assert_ne!(key, renamed.build_key("wav", "1", "desktop", &[0]).unwrap());
        let mut changed = ImportInputs::new(2, 2);
        changed
            .read(AssetId("a".into()), |_, _| Ok(vec![2]))
            .unwrap();
        changed
            .read(AssetId("b".into()), |_, _| Ok(vec![1]))
            .unwrap();
        assert_ne!(key, changed.build_key("wav", "1", "desktop", &[0]).unwrap());
        let mut failed = ImportInputs::new(1, 1);
        let _ = failed.read(AssetId("missing".into()), |_, _| Err("missing".into()));
        assert_eq!(
            failed.build_key("wav", "1", "desktop", &[0]),
            Err(InputError::Read("missing".into()))
        );

        assert_eq!(key, b.build_key("wav", "1", "desktop", &[0]).unwrap());
        for (importer, version, target, options) in [
            ("png", "1", "desktop", vec![0]),
            ("wav", "2", "desktop", vec![0]),
            ("wav", "1", "mobile", vec![0]),
            ("wav", "1", "desktop", vec![1]),
        ] {
            assert_ne!(
                key,
                a.build_key(importer, version, target, &options).unwrap()
            );
        }
        assert_ne!(
            a.build_key("ab", "c", "desktop", &[]).unwrap(),
            a.build_key("a", "bc", "desktop", &[]).unwrap()
        );
        assert!(matches!(
            a.build_key("", "1", "desktop", &[]),
            Err(InputError::InvalidId)
        ));
    }

    #[test]
    fn ignored_admission_errors_cannot_publish_partial_imports() {
        let mut inputs = ImportInputs::new(1, 1);
        inputs
            .read(AssetId("valid".into()), |_, _| Ok(vec![1]))
            .unwrap();
        assert_eq!(
            inputs.read(AssetId("extra".into()), |_, _| panic!()),
            Err(InputError::Capacity)
        );
        assert_eq!(
            inputs.build_key("test", "1", "host", &[]),
            Err(InputError::Capacity)
        );
        assert_eq!(inputs.validate(|_, _| panic!()), Err(InputError::Capacity));
        assert!(matches!(
            inputs.finish(1, |_, _| panic!()),
            Err(InputError::Capacity)
        ));
        let mut inputs = ImportInputs::new(1, 1);
        let _ = inputs.read(AssetId(String::new()), |_, _| panic!());
        assert!(matches!(
            inputs.finish(1, |_, _| panic!()),
            Err(InputError::InvalidId)
        ));
    }

    #[test]
    fn rejected_publication_retains_successful_and_failed_observations() {
        let source = AssetId("source".into());
        let missing = AssetId("include".into());
        let mut inputs = ImportInputs::new(2, 2);
        let captured = inputs.read(source.clone(), |_, _| Ok(vec![1, 2])).unwrap();
        let _ = inputs.read(missing.clone(), |_, _| Err("missing include".into()));
        let rejected = inputs
            .finish_observed(42, |_, _| panic!("failed input prevents validation"))
            .unwrap_err();
        assert_eq!(rejected.value, 42);
        assert_eq!(rejected.error, InputError::Read("missing include".into()));
        assert_eq!(rejected.inputs.captured_bytes(), 2);
        assert!(Arc::ptr_eq(
            &captured.bytes,
            &rejected.inputs.observations()[&source]
                .as_ref()
                .unwrap()
                .bytes
        ));
        assert_eq!(
            rejected.inputs.observations()[&missing],
            Err(InputError::Read("missing include".into()))
        );
        let mut inputs = ImportInputs::new(1, 2);
        inputs.read(source.clone(), |_, _| Ok(vec![1, 2])).unwrap();
        let rejected = inputs
            .finish_observed(7, |_, _| Ok(vec![2, 1]))
            .unwrap_err();
        assert_eq!(rejected.error, InputError::Changed(source.clone()));
        assert_eq!(
            &*rejected.inputs.observations()[&source]
                .as_ref()
                .unwrap()
                .bytes,
            &[1, 2]
        );
        let mut inputs = ImportInputs::new(1, 2);
        inputs.read(source, |_, _| Ok(vec![1, 2])).unwrap();
        assert_eq!(
            *inputs
                .finish_observed(9, |_, _| Ok(vec![1, 2]))
                .unwrap()
                .value(),
            9
        );
    }

    #[test]
    fn decoder_failure_retains_all_reads_and_success_requires_validation() {
        let source = AssetId("broken".into());
        let missing = AssetId("include".into());
        let failed = ImportInputs::new(2, 2)
            .decode_observed(|inputs| {
                inputs
                    .read(source.clone(), |_, _| Ok(vec![255, 0]))
                    .unwrap();
                let _ = inputs.read(missing.clone(), |_, _| Err("absent".into()));
                Err::<(), _>("invalid format")
            })
            .unwrap_err();
        assert_eq!(failed.error, "invalid format");
        assert_eq!(failed.inputs.captured_bytes(), 2);
        assert_eq!(
            &*failed.inputs.observations()[&source]
                .as_ref()
                .unwrap()
                .bytes,
            &[255, 0]
        );
        assert_eq!(
            failed.inputs.observations()[&missing],
            Err(InputError::Read("absent".into()))
        );
        let (value, inputs) = ImportInputs::new(1, 1)
            .decode_observed(|inputs| {
                let _ = inputs.read(source, |_, _| Err("absent".into()));
                Ok::<_, ()>(42)
            })
            .unwrap();
        assert!(inputs.finish_observed(value, |_, _| panic!()).is_err());
    }

    #[test]
    fn oversized_input_is_recorded_without_consuming_budget() {
        let mut inputs = ImportInputs::new(2, 1);
        assert!(matches!(
            inputs.read(AssetId("large".into()), |_, _| Ok(vec![0; 2])),
            Err(InputError::Capacity)
        ));
        assert_eq!(inputs.captured_bytes(), 0);
        assert_eq!(inputs.observations().len(), 1);
        assert!(
            inputs
                .read(AssetId("valid".into()), |_, _| Ok(vec![4]))
                .is_ok()
        );
    }
    #[test]
    fn staged_input_replacement_preserves_payload_bits_and_unchanged_snapshot_arcs() {
        let manifest = AssetId("manifest".into());
        let leaf = AssetId("leaf".into());
        let reader = |id: &AssetId, _: usize| {
            Ok(if id == &manifest {
                b"old".to_vec()
            } else {
                b"leaf".to_vec()
            })
        };
        let mut inputs = ImportInputs::new(2, 8);
        let before = inputs.read(manifest.clone(), reader).unwrap();
        let sibling = inputs.read(leaf.clone(), reader).unwrap();
        let payload = Arc::new(vec![f64::from_bits(0x3ff0_0000_0000_0001), -0.0]);
        let original = inputs.finish(payload.clone(), reader).unwrap();
        let after = InputSnapshot {
            bytes: Arc::from(&b"new!"[..]),
            digest: *blake3::hash(b"new!").as_bytes(),
        };
        let staged = original
            .prepare_input_replacement(&manifest, &before, after.clone(), reader)
            .unwrap();
        assert!(Arc::ptr_eq(original.value(), staged.value()));
        assert_eq!(staged.value()[0].to_bits(), 0x3ff0_0000_0000_0001);
        assert_eq!(staged.value()[1].to_bits(), (-0.0_f64).to_bits());
        assert_eq!(original.inputs().captured_bytes(), 7);
        assert_eq!(staged.inputs().captured_bytes(), 8);
        let unchanged = staged
            .inputs()
            .observations()
            .get(&leaf)
            .unwrap()
            .as_ref()
            .unwrap();
        assert!(Arc::ptr_eq(&sibling.bytes, &unchanged.bytes));
        assert!(Arc::ptr_eq(
            &after.bytes,
            &staged.inputs().observations()[&manifest]
                .as_ref()
                .unwrap()
                .bytes
        ));
        assert_eq!(
            staged.inputs().validate(|id, _| Ok(if id == &manifest {
                b"new!".to_vec()
            } else {
                b"leaf".to_vec()
            })),
            Ok(())
        );
    }
    #[test]
    fn staged_input_replacement_rejects_quota_forgery_and_unrelated_input_drift() {
        let id = AssetId("manifest".into());
        let mut inputs = ImportInputs::new(2, 7);
        let read = |key: &AssetId, _: usize| {
            Ok(if key == &id {
                b"old".to_vec()
            } else {
                b"leaf".to_vec()
            })
        };
        let before = inputs.read(id.clone(), read).unwrap();
        inputs.read(AssetId("leaf".into()), read).unwrap();
        let original = inputs.finish((), read).unwrap();
        let oversized = InputSnapshot {
            bytes: Arc::from(&b"four"[..]),
            digest: *blake3::hash(b"four").as_bytes(),
        };
        assert_eq!(
            original
                .prepare_input_replacement(&id, &before, oversized, |_, _| panic!(
                    "quota must reject before IO"
                ))
                .unwrap_err(),
            InputError::Capacity
        );
        let forged = InputSnapshot {
            bytes: Arc::from(&b"new"[..]),
            digest: [0; 32],
        };
        assert_eq!(
            original
                .prepare_input_replacement(&id, &before, forged, |_, _| panic!(
                    "digest must reject before IO"
                ))
                .unwrap_err(),
            InputError::Changed(id.clone())
        );
        let after = InputSnapshot {
            bytes: Arc::from(&b"new"[..]),
            digest: *blake3::hash(b"new").as_bytes(),
        };
        assert!(
            original
                .prepare_input_replacement(&id, &before, after, |key, _| Ok(if key == &id {
                    b"old".to_vec()
                } else {
                    b"xxxx".to_vec()
                }))
                .is_err()
        );
        assert_eq!(original.inputs().captured_bytes(), 7);
        assert_eq!(original.inputs().validate(read), Ok(()));
    }
}

#[path = "imported_archive.rs"]
mod archive;
pub use archive::ImportedArchiveLimits;
