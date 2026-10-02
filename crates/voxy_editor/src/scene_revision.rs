//! Exact, bounded observations for legacy scene files outside the asset project.
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
use voxy_scene::SceneDocument;

const MAX_BYTES: usize = 1_048_576;

#[derive(Debug)]
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
