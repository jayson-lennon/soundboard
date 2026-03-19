//! Soundboard audio playback service.
//!
//! This crate provides audio playback functionality with varlink IPC support.

use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::thread::{self, JoinHandle};

use error_stack::{Report, ResultExt};
use serde::{Deserialize, Serialize};
use tracing::{error, info};
use wherror::Error;
use zlink::introspect;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("no config directory found")]
    NoConfigDir,
    #[error("failed to read config file")]
    Read,
    #[error("failed to parse config file")]
    Parse,
}

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("failed to open audio file")]
    FileOpen,
    #[error("failed to decode audio")]
    Decode,
    #[error("no audio device available")]
    NoDevice,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub output_device: Option<String>,
}

impl Config {
    /// # Errors
    ///
    /// Returns an error if the config directory cannot be determined or the file cannot be read.
    pub fn load() -> Result<Self, Report<ConfigError>> {
        let config_dir = dirs::config_dir().ok_or_else(|| Report::new(ConfigError::NoConfigDir))?;
        let config_path = config_dir.join("soundboard").join("config.toml");

        if !config_path.exists() {
            info!("No config file found, using defaults");
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&config_path)
            .change_context(ConfigError::Read)
            .attach("failed to read config file")?;

        toml::from_str(&content)
            .change_context(ConfigError::Parse)
            .attach("failed to parse config.toml")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, introspect::Type)]
pub struct PlayResponse {
    pub success: bool,
}

#[derive(Debug, Clone, PartialEq, zlink::ReplyError, introspect::ReplyError)]
#[zlink(interface = "io.soundboard")]
pub enum SoundboardError {
    FileNotFound { path: String },
    PlaybackFailed { message: String },
}

#[zlink::proxy("io.soundboard")]
pub trait SoundboardProxy {
    async fn play(&mut self, path: &str) -> zlink::Result<Result<PlayResponse, SoundboardError>>;
}

pub struct AudioPlayer {
    #[allow(dead_code)]
    config: Config,
    _stream: rodio::OutputStream,
    handle: rodio::OutputStreamHandle,
}

impl AudioPlayer {
    pub fn new(config: Config) -> Result<Self, Report<AudioError>> {
        let (stream, handle) = rodio::OutputStream::try_default()
            .change_context(AudioError::NoDevice)
            .attach("failed to get default audio output")?;

        Ok(Self {
            config,
            _stream: stream,
            handle,
        })
    }

    /// # Errors
    ///
    /// Returns an error if the file cannot be opened, decoded, or no audio device is available.
    pub fn play(&self, path: &Path) -> Result<(), Report<AudioError>> {
        let file = File::open(path)
            .change_context(AudioError::FileOpen)
            .attach("failed to open audio file")?;

        let source = rodio::Decoder::new(BufReader::new(file))
            .change_context(AudioError::Decode)
            .attach("failed to decode audio")?;

        let sink = rodio::Sink::try_new(&self.handle)
            .change_context(AudioError::NoDevice)
            .attach("failed to create audio sink")?;

        sink.append(source);
        sink.detach();

        Ok(())
    }
}

#[allow(dead_code)]
enum AudioCommand {
    Play(PathBuf),
    Stop,
}

pub struct AudioThread {
    sender: Sender<AudioCommand>,
    _handle: JoinHandle<()>,
}

impl AudioThread {
    pub fn start(config: Config) -> Result<Self, Report<AudioError>> {
        let (sender, receiver) = mpsc::channel::<AudioCommand>();

        let handle = thread::spawn(move || {
            let player = match AudioPlayer::new(config) {
                Ok(p) => p,
                Err(e) => {
                    error!("Failed to initialize audio player: {}", e);
                    return;
                }
            };

            while let Ok(cmd) = receiver.recv() {
                match cmd {
                    AudioCommand::Play(path) => {
                        if let Err(e) = player.play(&path) {
                            error!("Playback failed for {}: {}", path.display(), e);
                        }
                    }
                    AudioCommand::Stop => break,
                }
            }
        });

        Ok(Self {
            sender,
            _handle: handle,
        })
    }

    pub fn play(&self, path: &Path) -> Result<(), Report<AudioError>> {
        self.sender
            .send(AudioCommand::Play(path.to_path_buf()))
            .map_err(|_| Report::new(AudioError::NoDevice))?;
        Ok(())
    }
}
