//! Native Linux playback, independent of WebView visibility and autoplay policies.
use crate::{core::SoundCue, files::atomic_write};
use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct Player {
    sender: mpsc::SyncSender<Request>,
}
struct Request {
    cue: SoundCue,
    volume: u32,
    reply: Option<mpsc::Sender<Result<(), String>>>,
}
impl Player {
    pub fn new(dir: &Path) -> Result<Self, String> {
        let dir = dir.join("sounds");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
                .map_err(|e| e.to_string())?;
        }
        atomic_write(
            &dir.join("start.wav"),
            include_bytes!("../../assets/sounds/start.wav"),
        )?;
        atomic_write(
            &dir.join("complete.wav"),
            include_bytes!("../../assets/sounds/complete.wav"),
        )?;
        let (sender, receiver) = mpsc::sync_channel::<Request>(8);
        std::thread::spawn(move || {
            for request in receiver {
                let result = play_file(&dir, request.cue, request.volume);
                if let Some(reply) = request.reply {
                    let _ = reply.send(result);
                } else if let Err(e) = result {
                    eprintln!("FocusFlow audio: {e}");
                }
            }
        });
        Ok(Self { sender })
    }
    pub fn enqueue(&self, cue: SoundCue, volume: u32) {
        if volume > 0 {
            let _ = self.sender.try_send(Request {
                cue,
                volume,
                reply: None,
            });
        }
    }
    pub fn preview(&self, cue: &str, volume: u32) -> Result<(), String> {
        let cue = match cue {
            "start" => SoundCue::Start,
            "complete" => SoundCue::Complete,
            _ => return Err("Geçersiz ses seçimi.".into()),
        };
        if !(1..=100).contains(&volume) {
            return Err("Ses düzeyi 1–100 arasında olmalı.".into());
        }
        let (reply, response) = mpsc::channel();
        self.sender
            .try_send(Request {
                cue,
                volume,
                reply: Some(reply),
            })
            .map_err(|_| "Ses çalınıyor; biraz sonra yeniden deneyin.")?;
        response
            .recv_timeout(Duration::from_secs(75))
            .map_err(|_| "Ses çalma zaman aşımı.".to_string())?
    }
}
fn play_file(dir: &Path, cue: SoundCue, volume: u32) -> Result<(), String> {
    let path: PathBuf = dir.join(match cue {
        SoundCue::Start => "start.wav",
        SoundCue::Complete => "complete.wav",
    });
    for backend in ["pw-play", "paplay"] {
        let mut command = Command::new(backend);
        if backend == "pw-play" {
            command.arg(format!("--volume={:.3}", volume as f32 / 100.0));
        } else {
            command.arg(format!("--volume={}", volume * 65536 / 100));
        }
        let Ok(mut child) = command
            .arg(&path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
        else {
            continue;
        };
        let deadline = Instant::now() + Duration::from_secs(6);
        loop {
            match child.try_wait() {
                Ok(Some(status)) if status.success() => return Ok(()),
                Ok(Some(_)) => break,
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(50))
                }
                _ => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break;
                }
            }
        }
    }
    Err("Ses çalınamadı. Ses çıkışını ve PipeWire/PulseAudio bağlantısını kontrol edin.".into())
}
