//! Audio playback for the player bar.

use std::fs::File;
use std::path::Path;
use std::time::Duration;

use anyhow::Result;

pub struct Player {
    device: Option<rodio::MixerDeviceSink>,
    player: Option<rodio::Player>,
    pub song_id: Option<String>,
    pub duration: f32,
    pub volume: f32,
}

impl Player {
    pub fn new() -> Self {
        Self { device: None, player: None, song_id: None, duration: 0.0, volume: 0.8 }
    }

    pub fn load(&mut self, song_id: &str, path: &Path, duration: f32) -> Result<()> {
        if self.device.is_none() {
            self.device = Some(rodio::DeviceSinkBuilder::open_default_sink()?);
        }
        if let Some(old) = self.player.take() {
            old.stop();
        }
        let player = rodio::Player::connect_new(self.device.as_ref().unwrap().mixer());
        player.append(rodio::Decoder::try_from(File::open(path)?)?);
        player.set_volume(self.volume);
        self.player = Some(player);
        self.song_id = Some(song_id.to_string());
        self.duration = duration;
        Ok(())
    }

    pub fn is_playing(&self) -> bool {
        self.player.as_ref().is_some_and(|p| !p.is_paused() && !p.empty())
    }

    pub fn is_loaded(&self) -> bool {
        self.player.is_some()
    }

    pub fn toggle(&mut self) {
        if let Some(p) = &self.player {
            if p.empty() {
                return;
            }
            if p.is_paused() { p.play() } else { p.pause() }
        }
    }

    /// Seconds into the current song; the end once playback finishes.
    pub fn position(&self) -> f32 {
        match &self.player {
            Some(p) if p.empty() => self.duration,
            Some(p) => p.get_pos().as_secs_f32().min(self.duration.max(0.0)),
            None => 0.0,
        }
    }

    pub fn finished(&self) -> bool {
        self.player.as_ref().is_some_and(|p| p.empty())
    }

    pub fn seek(&mut self, fraction: f32) {
        let target = Duration::from_secs_f32((fraction.clamp(0.0, 1.0) * self.duration).max(0.0));
        if let Some(p) = &self.player {
            let _ = p.try_seek(target);
        }
    }

    pub fn set_volume(&mut self, volume: f32) {
        self.volume = volume.clamp(0.0, 1.0);
        if let Some(p) = &self.player {
            p.set_volume(self.volume);
        }
    }

    pub fn stop(&mut self) {
        if let Some(p) = self.player.take() {
            p.stop();
        }
        self.song_id = None;
    }
}
