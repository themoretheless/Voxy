//! Nonblocking operational save barrier. Presentation still needs an App frame
//! boundary; this participant freezes import/watch dispatch, not the swapchain.
#![allow(dead_code)]
use super::*;
use crate::EditorError;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use voxy_assets::{PollWorkerError, SourceDependenciesCheckpoint, SourcePollerCheckpoint};

const PERIOD: Duration = Duration::from_millis(200);
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Sources(
    #[serde(with = "crate::asset_checkpoint_wire::sources")] SourceDependenciesCheckpoint,
);
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Watcher(#[serde(with = "crate::asset_checkpoint_wire::watcher")] SourcePollerCheckpoint);
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct UiReloadCheckpoint {
    version: u32,
    sources: Option<Sources>,
    watcher: Option<Watcher>,
    delay: Option<Duration>,
    deferred: Option<[u64; 4]>,
}
#[derive(Debug)]
pub(super) enum Barrier {
    Draining,
    Requested,
    Ready(Box<UiReloadCheckpoint>),
}
#[derive(Debug)]
pub(crate) struct PreparedUiReload {
    pub(super) sources: Option<voxy_assets::SourceDependencies>,
    pub(super) watcher: Option<voxy_assets::SourcePollWorker>,
    pub(super) last_scan: Option<Instant>,
    pub(super) deferred: Option<(u64, u64, u64, u64)>,
}
impl PreparedUiReload {
    /// Retain the returned join handle for teardown outside frame-critical code.
    pub(crate) fn close(self) -> Option<std::thread::JoinHandle<()>> {
        self.watcher.map(voxy_assets::SourcePollWorker::close)
    }
}
impl UiLive {
    pub(crate) fn begin_checkpoint(&mut self) -> Result<(), EditorError> {
        if self.checkpoint.is_some() {
            return Err("UI reload checkpoint already requested".into());
        }
        self.checkpoint = Some(Barrier::Draining);
        Ok(())
    }
    /// Normal invalidation-before-publication processing drains in-flight work.
    /// No new import or scan is submitted while the barrier is held. Import/GPU
    /// failures are reported normally; the caller may retry polling or cancel.
    pub(crate) fn poll_checkpoint(
        &mut self,
        scene: &SceneGraph,
        project: &AuthoringProject,
        viewport: [f32; 2],
        graphics: &mut Graphics,
    ) -> Result<Option<UiReloadCheckpoint>, Box<dyn std::error::Error>> {
        if self.checkpoint.is_none() {
            return Err("UI reload checkpoint not requested".into());
        }
        self.poll(scene, project, viewport, graphics)?;
        Ok(self.advance_checkpoint()?)
    }
    /// Called after a normal poll. This method only joins already-finished threads
    /// and exchanges bounded watcher messages; it never waits for IO or imports.
    pub(crate) fn advance_checkpoint(&mut self) -> Result<Option<UiReloadCheckpoint>, EditorError> {
        self.reap().map_err(|error| error.to_string())?;
        let Some(barrier) = &self.checkpoint else {
            return Err("UI reload checkpoint not requested".into());
        };
        if let Barrier::Ready(saved) = barrier {
            return Ok(Some((**saved).clone()));
        }
        if self.worker.is_some() || self.scanning || !self.retired.is_empty() {
            return Ok(None);
        }
        if matches!(barrier, Barrier::Draining)
            && let Some(watcher) = &mut self.watcher
        {
            match watcher.request_checkpoint() {
                Ok(()) => self.checkpoint = Some(Barrier::Requested),
                Err(PollWorkerError::Busy) => return Ok(None),
                Err(error) => return Err(error.into()),
            }
        }
        let watcher = if let Some(watcher) = &mut self.watcher {
            let Some(saved) = watcher.try_checkpoint()? else {
                return Ok(None);
            };
            Some(Watcher(saved))
        } else {
            None
        };
        let saved = UiReloadCheckpoint {
            version: 1,
            sources: self
                .sources
                .as_ref()
                .map(|index| Sources(index.checkpoint())),
            watcher,
            delay: self
                .last_scan
                .map(|last| PERIOD.saturating_sub(last.elapsed())),
            deferred: self.deferred.map(|(a, b, c, d)| [a, b, c, d]),
        };
        self.checkpoint = Some(Barrier::Ready(Box::new(saved.clone())));
        Ok(Some(saved))
    }
    /// Cancellation must consume an already-requested watcher snapshot before
    /// opening dispatch again. Returning false means its fence is still pending.
    pub(crate) fn resume_after_checkpoint(&mut self) -> Result<bool, EditorError> {
        if matches!(self.checkpoint, Some(Barrier::Requested)) {
            let watcher = self
                .watcher
                .as_mut()
                .ok_or("missing checkpoint UI watcher")?;
            if watcher.try_checkpoint()?.is_none() {
                return Ok(false);
            }
        }
        if let Some(Barrier::Ready(saved)) = &self.checkpoint {
            self.last_scan = rebase(saved.delay)?;
        }
        self.checkpoint = None;
        Ok(true)
    }
}
fn rebase(delay: Option<Duration>) -> Result<Option<Instant>, EditorError> {
    delay
        .map(|delay| {
            Instant::now()
                .checked_sub(PERIOD.saturating_sub(delay))
                .ok_or_else(|| "UI scan clock rebase overflow".into())
        })
        .transpose()
}
impl UiReloadCheckpoint {
    pub(crate) fn to_json(&self, max_bytes: usize) -> Result<String, EditorError> {
        let json = serde_json::to_string(self)?;
        if json.len() > max_bytes {
            return Err("UI reload checkpoint byte budget".into());
        }
        Ok(json)
    }
    pub(crate) fn from_json(json: &str, max_bytes: usize) -> Result<Self, EditorError> {
        if json.len() > max_bytes {
            return Err("UI reload checkpoint byte budget".into());
        }
        Ok(serde_json::from_str(json)?)
    }
    /// The caller supplies the final restored GPU cache state after staging its
    /// resources. A saved deferred retry is rebound to that state explicitly;
    /// old process cache epochs are never treated as the restored cache's epoch.
    /// Watcher construction starts an idle worker from the original fingerprint
    /// baseline, without taking a fresh filesystem baseline or reading sources.
    pub(crate) fn prepare(
        &self,
        project: &AuthoringProject,
        restored_cache: (u64, u64, u64, u64),
    ) -> Result<PreparedUiReload, EditorError> {
        self.prepare_using(restored_cache, |saved| {
            Ok(voxy_assets::SourcePollWorker::new_with_checkpoint(
                project.input_provider()?,
                saved,
                32,
                16 * 1024 * 1024,
            )?)
        })
    }
    fn prepare_using(
        &self,
        restored_cache: (u64, u64, u64, u64),
        make_watcher: impl FnOnce(
            &SourcePollerCheckpoint,
        ) -> Result<voxy_assets::SourcePollWorker, EditorError>,
    ) -> Result<PreparedUiReload, EditorError> {
        if self.version != 1
            || self.delay.is_some_and(|delay| delay > PERIOD)
            || (self.watcher.is_some() && self.sources.is_none())
        {
            return Err("invalid UI reload checkpoint header".into());
        }
        let sources = self
            .sources
            .as_ref()
            .map(|saved| {
                if saved
                    .0
                    .outputs
                    .iter()
                    .any(|entry| entry.output.0 != "game-ui-text")
                {
                    return Err("unexpected UI reload output".into());
                }
                Ok::<_, EditorError>(saved.0.prepare(1, 32)?)
            })
            .transpose()?;
        if let Some(saved) = &self.watcher {
            saved.0.prepare(32, 16 * 1024 * 1024)?;
        }
        let last_scan = rebase(self.delay)?;
        let watcher = self
            .watcher
            .as_ref()
            .map(|saved| make_watcher(&saved.0))
            .transpose()?;
        Ok(PreparedUiReload {
            sources,
            watcher,
            last_scan,
            deferred: self.deferred.map(|_| restored_cache),
        })
    }
}

#[cfg(test)]
mod tests;
