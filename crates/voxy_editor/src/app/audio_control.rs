//! Audio source, bus, listener and device control from the editor.
//!
//! Methods of [`App`] split out of lib.rs (plan step C4); they keep the same
//! bodies and private access through `crate::*`.

use crate::*;

impl App {
    pub(crate) fn toggle_audio_source(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let node = *self
            .instances
            .get(self.selected)
            .ok_or("missing selected node")?;
        if self
            .scene
            .component::<voxy_gameplay::AudioSource>(node)?
            .is_some()
        {
            self.scene
                .remove_component::<voxy_gameplay::AudioSource>(node)?;
        } else {
            let asset = self
                .authoring
                .authoring_project
                .audio_assets()?
                .into_iter()
                .next()
                .ok_or("project has no WAV audio asset")?;
            self.scene.insert_component(
                node,
                voxy_gameplay::AudioSource {
                    import_settings: None,
                    asset: asset.0,
                    bus: 0,
                    gain: 1.,
                    looping: false,
                    spatial: true,
                    near: 1.,
                    far: 5.,
                },
            )?;
        }
        self.commit_authoring()
    }

    pub(crate) fn toggle_audio_bus(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let node = *self
            .instances
            .get(self.selected)
            .ok_or("missing selected node")?;
        if self
            .scene
            .component::<voxy_gameplay::AudioBus>(node)?
            .is_some()
        {
            self.scene
                .remove_component::<voxy_gameplay::AudioBus>(node)?;
        } else {
            let used: BTreeSet<_> = self
                .scene
                .active_components::<voxy_gameplay::AudioBus>()
                .map(|(_, bus)| bus.bus)
                .collect();
            let bus = (0..16)
                .find(|bus| !used.contains(bus))
                .ok_or("all audio buses have active owners")?;
            self.scene
                .insert_component(node, voxy_gameplay::AudioBus { bus, gain: 1. })?;
        }
        self.commit_authoring()
    }

    pub(crate) fn toggle_audio_listener(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let node = *self
            .instances
            .get(self.selected)
            .ok_or("missing selected node")?;
        if self
            .scene
            .component::<voxy_gameplay::AudioListener>(node)?
            .is_some()
        {
            self.scene
                .remove_component::<voxy_gameplay::AudioListener>(node)?;
        } else {
            self.scene
                .insert_component(node, voxy_gameplay::AudioListener::default())?;
        }
        self.commit_authoring()
    }

    pub(crate) fn close_play_audio(&mut self) {
        self.audio.close_play_audio();
    }

    pub(crate) fn close_audio_device(&mut self) {
        self.audio.close_audio_device();
    }

    pub(crate) fn poll_audio_device(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match self
            .audio
            .poll(&self.scene, &self.authoring.authoring_project)?
        {
            audio_session::AudioPoll::Ready | audio_session::AudioPoll::RetryPlay => {
                self.toggle_play()?;
            }
            audio_session::AudioPoll::Pending => {}
        }
        Ok(())
    }
}
