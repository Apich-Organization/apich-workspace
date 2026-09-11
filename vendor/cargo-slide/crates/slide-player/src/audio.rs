use rodio::Decoder;
use rodio::DeviceSinkBuilder;
use rodio::MixerDeviceSink;
use rodio::Player;
use rodio::Source;
use std::fs::File;
use std::path::Path;
use std::time::Duration;
use std::time::Instant;

/// High-level audio engine supporting background tracks, volume adjustments, and fade effects
pub struct AudioEngine {
    _sink: Option<MixerDeviceSink>,
    player: Option<Player>,
    current_source: Option<String>,
    global_volume: f32,
    is_muted: bool,
    // Fade in / out state
    fade_start_time: Option<Instant>,
    fade_duration: Duration,
    fade_start_vol: f32,
    fade_target_vol: f32,
    stop_after_fade: bool,
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioEngine {
    pub fn new() -> Self {
        let sink = match DeviceSinkBuilder::open_default_sink() {
            | Ok(mut s) => {
                s.log_on_drop(false);
                Some(s)
            },
            | Err(e) => {
                eprintln!(
                    "⚠️ Audio device not available: {}. Continuing without audio.",
                    e
                );
                None
            },
        };

        Self {
            _sink: sink,
            player: None,
            current_source: None,
            global_volume: 0.8,
            is_muted: false,
            fade_start_time: None,
            fade_duration: Duration::ZERO,
            fade_start_vol: 0.8,
            fade_target_vol: 0.8,
            stop_after_fade: false,
        }
    }

    /// Play an audio file with optional loop, volume, and fade-in duration
    pub fn play_track(
        &mut self,
        source_path: &str,
        loop_audio: bool,
        volume: f32,
        fade_in: Duration,
    ) -> std::io::Result<()> {
        let sink = match &self._sink {
            | Some(s) => s,
            | None => return Ok(()), // Silent no-op if no audio device
        };

        let path = Path::new(source_path);
        if !path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Audio file not found: {}", source_path),
            ));
        }

        let file = File::open(path)?;
        let decoder = Decoder::try_from(file).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to decode audio: {}", e),
            )
        })?;

        // Stop previous player
        if let Some(player) = self.player.take() {
            player.stop();
        }

        let player = Player::connect_new(sink.mixer());

        let initial_vol = if fade_in > Duration::ZERO {
            0.0
        } else {
            volume * self.global_volume
        };
        player.set_volume(initial_vol);

        if loop_audio {
            player.append(decoder.repeat_infinite());
        } else {
            player.append(decoder);
        }

        self.player = Some(player);
        self.current_source = Some(source_path.to_string());

        if fade_in > Duration::ZERO {
            self.fade_start_time = Some(Instant::now());
            self.fade_duration = fade_in;
            self.fade_start_vol = 0.0;
            self.fade_target_vol = volume * self.global_volume;
            self.stop_after_fade = false;
        } else {
            self.fade_start_time = None;
        }

        Ok(())
    }

    /// Fade out and stop the current audio
    pub fn stop_with_fade(
        &mut self,
        fade_out: Duration,
    ) {
        if let Some(ref player) = self.player {
            if fade_out > Duration::ZERO {
                self.fade_start_time = Some(Instant::now());
                self.fade_duration = fade_out;
                self.fade_start_vol = player.volume();
                self.fade_target_vol = 0.0;
                self.stop_after_fade = true;
            } else {
                player.stop();
                self.player = None;
                self.current_source = None;
            }
        }
    }

    /// Toggle play / pause
    pub fn toggle_pause(&mut self) {
        if let Some(ref player) = self.player {
            if player.is_paused() {
                player.play();
            } else {
                player.pause();
            }
        }
    }

    pub fn is_playing(&self) -> bool {
        self.player
            .as_ref()
            .map(|p| !p.is_paused() && !p.empty())
            .unwrap_or(false)
    }

    /// Update frame ticks to smoothly apply fade curves
    pub fn update(&mut self) {
        if let (Some(start), Some(player)) = (self.fade_start_time, self.player.as_ref()) {
            let elapsed = start.elapsed();
            if elapsed >= self.fade_duration {
                player.set_volume(self.fade_target_vol);
                self.fade_start_time = None;
                if self.stop_after_fade {
                    player.stop();
                    self.player = None;
                    self.current_source = None;
                }
            } else {
                let t = elapsed.as_secs_f32() / self.fade_duration.as_secs_f32();
                let vol = self.fade_start_vol + (self.fade_target_vol - self.fade_start_vol) * t;
                player.set_volume(vol.max(0.0));
            }
        }
    }

    /// Adjust volume by delta (+0.05 or -0.05)
    pub fn adjust_volume(
        &mut self,
        delta: f32,
    ) {
        self.set_volume(self.global_volume + delta);
    }

    pub fn set_volume(
        &mut self,
        vol: f32,
    ) {
        self.global_volume = vol.clamp(0.0, 1.0);
        if self.is_muted && self.global_volume > 0.0 {
            self.is_muted = false;
        }
        if !self.is_muted
            && let Some(ref player) = self.player
        {
            player.set_volume(self.global_volume);
        }
    }

    pub fn toggle_mute(&mut self) {
        self.is_muted = !self.is_muted;
        if let Some(ref player) = self.player {
            if self.is_muted {
                player.set_volume(0.0);
            } else {
                player.set_volume(self.global_volume);
            }
        }
    }

    pub fn is_muted(&self) -> bool {
        self.is_muted
    }

    pub fn volume(&self) -> f32 {
        self.global_volume
    }

    pub fn volume_percent(&self) -> u32 {
        if self.is_muted {
            0
        } else {
            (self.global_volume.clamp(0.0, 1.0) * 100.0)
                .round()
                .min(100.0) as u32
        }
    }

    pub fn current_source(&self) -> Option<&str> {
        self.current_source.as_deref()
    }
}
