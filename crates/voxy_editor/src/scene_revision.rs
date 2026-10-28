//! Exact, bounded observations for legacy scene files outside the asset project.
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
use voxy_scene::SceneDocument;

const MAX_BYTES: usize = crate::scene_limits::DOCUMENT_BYTES;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SceneRevision {
    path: PathBuf,
    digest: blake3::Hash,
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_BYTES {
        return Err("scene exceeds document budget".into());
    }
    Ok(bytes)
}

impl SceneRevision {
    pub(super) fn read(path: &Path) -> Result<(SceneDocument, Self), Box<dyn std::error::Error>> {
        let bytes = read_bytes(path)?;
        let document = SceneDocument::from_json(std::str::from_utf8(&bytes)?)?;
        Ok((
            document,
            Self {
                path: path.to_owned(),
                digest: blake3::hash(&bytes),
            },
        ))
    }

    pub(super) fn written(
        path: &Path,
        document: &SceneDocument,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            path: path.to_owned(),
            digest: blake3::hash(document.to_json()?.as_bytes()),
        })
    }

    pub(super) fn validate(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        if path != self.path || blake3::hash(&read_bytes(path)?) != self.digest {
            return Err("scene file changed externally; reload before saving".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SceneRevisionCheckpoint {
    path: PathBuf,
    digest: [u8; 32],
}
impl SceneRevision {
    pub(super) fn checkpoint(&self) -> SceneRevisionCheckpoint {
        SceneRevisionCheckpoint {
            path: self.path.clone(),
            digest: *self.digest.as_bytes(),
        }
    }
}
impl SceneRevisionCheckpoint {
    pub(super) fn prepare(&self, path: Option<&Path>) -> Result<SceneRevision, crate::EditorError> {
        if path != Some(self.path.as_path()) || self.path.as_os_str().is_empty() {
            return Err("scene revision checkpoint path differs".into());
        }
        Ok(SceneRevision {
            path: self.path.clone(),
            digest: blake3::Hash::from_bytes(self.digest),
        })
    }
}
