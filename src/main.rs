use std::os::unix::io::{FromRawFd, OwnedFd};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
use sd_notify::NotifyState;
use soundboard::{AudioThread, Config, PlayResponse, SoundboardError, SoundboardProxy};
use tracing::{error, info};
use zlink::{Server, service, unix};

const IDLE_TIMEOUT_SECS: u64 = 300;
const IDLE_CHECK_INTERVAL_SECS: u64 = 30;

#[derive(Parser)]
#[command(name = "soundboard")]
#[command(about = "Soundboard audio player service")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run as server (normally started by systemd)
    Server,
    /// Play a sound file
    Play {
        /// Path to the audio file
        #[arg(value_name = "PATH")]
        path: PathBuf,
    },
}

struct SoundboardService {
    player: AudioThread,
    start_time: Arc<Instant>,
    last_activity: Arc<AtomicU64>,
}

impl SoundboardService {
    fn new(player: AudioThread) -> Self {
        Self {
            player,
            start_time: Arc::new(Instant::now()),
            last_activity: Arc::new(AtomicU64::new(0)),
        }
    }
}

#[service(interface = "io.soundboard")]
impl SoundboardService {
    async fn play(&mut self, path: String) -> Result<PlayResponse, SoundboardError> {
        self.last_activity.store(
            self.start_time.elapsed().as_secs(),
            Ordering::Relaxed,
        );

        let path = PathBuf::from(&path);
        if !path.exists() {
            return Err(SoundboardError::FileNotFound { 
                path: path.display().to_string() 
            });
        }

        info!("Playing audio file: {}", path.display());
        if let Err(e) = self.player.play(&path) {
            return Err(SoundboardError::PlaybackFailed {
                message: e.to_string(),
            });
        }

        Ok(PlayResponse { success: true })
    }
}

fn get_socket_path() -> PathBuf {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR")
        .unwrap_or_else(|_| format!("/run/user/{}", users::get_current_uid()));
    PathBuf::from(runtime_dir).join("soundboard.varlink")
}

fn get_systemd_socket() -> Option<OwnedFd> {
    let listen_fds = std::env::var("LISTEN_FDS").ok()?;
    let count: i32 = listen_fds.parse().ok()?;
    if count >= 1 {
        Some(unsafe { OwnedFd::from_raw_fd(3) })
    } else {
        None
    }
}

fn spawn_idle_watchdog(start_time: Arc<Instant>, last_activity: Arc<AtomicU64>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(IDLE_CHECK_INTERVAL_SECS)).await;
            
            let last = last_activity.load(Ordering::Relaxed);
            let current = start_time.elapsed().as_secs();
            
            if current.saturating_sub(last) >= IDLE_TIMEOUT_SECS {
                info!("Idle timeout reached, exiting");
                std::process::exit(0);
            }
        }
    });
}

fn notify_systemd_ready() {
    if std::env::var("NOTIFY_SOCKET").is_ok() {
        let _ = sd_notify::notify(false, &[NotifyState::Ready]);
    }
}

async fn run_server() {
    let config = Config::load().expect("Failed to load config");
    let player = AudioThread::start(config).expect("Failed to initialize audio player");
    
    let service = SoundboardService::new(player);
    let start_time = Arc::clone(&service.start_time);
    let last_activity = Arc::clone(&service.last_activity);
    
    let socket_path = get_socket_path();
    
    let listener = match get_systemd_socket() {
        Some(fd) => {
            info!("Using socket from systemd");
            unix::Listener::try_from(fd).expect("Failed to convert systemd socket")
        }
        None => {
            info!("Binding to socket: {:?}", socket_path);
            let _ = std::fs::remove_file(&socket_path);
            unix::bind(&socket_path).expect("Failed to bind to socket")
        }
    };
    
    spawn_idle_watchdog(start_time, last_activity);
    notify_systemd_ready();
    
    let server = Server::new(listener, service);
    
    match server.run().await {
        Ok(()) => info!("Server done"),
        Err(e) => error!("Server error: {:?}", e),
    }
}

async fn run_client(path: PathBuf) {
    let socket_path = get_socket_path();
    
    let mut conn = match unix::connect(&socket_path).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to connect to server: {:?}", e);
            std::process::exit(1);
        }
    };
    
    let path_str = path.display().to_string();
    match conn.play(&path_str).await {
        Ok(Ok(_)) => {}
        Ok(Err(SoundboardError::FileNotFound { path })) => {
            eprintln!("File not found: {}", path);
            std::process::exit(1);
        }
        Ok(Err(SoundboardError::PlaybackFailed { message })) => {
            eprintln!("Playback failed: {}", message);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Failed to send play command: {:?}", e);
            std::process::exit(1);
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Server => run_server().await,
        Commands::Play { path } => run_client(path).await,
    }
}
