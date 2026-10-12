//! Music engines. One adapter today: the `mlx-yue` CLI from the MLX port of YuE2.

use std::io::{BufRead as _, BufReader};
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context as _, Result, bail};
use serde::Serialize;

use crate::config::Config;
use crate::song::SongRequest;

/// Shared flag a caller flips to stop a running generation.
#[derive(Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    /// Another generation holds the GPU.
    Waiting,
    Loading,
    /// Composing the melody and chords.
    Planning,
    /// Writing the song's audio tokens.
    Writing,
    /// Turning tokens into sound — the long part.
    Synthesizing,
    Decoding,
    Done,
}

impl Stage {
    pub fn label(self) -> &'static str {
        match self {
            Stage::Waiting => "Waiting for the GPU",
            Stage::Loading => "Loading model",
            Stage::Planning => "Composing",
            Stage::Writing => "Writing the song",
            Stage::Synthesizing => "Synthesizing audio",
            Stage::Decoding => "Mastering",
            Stage::Done => "Done",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Progress {
    pub stage: Stage,
    /// Overall completion, 0.0–1.0, monotone within one run.
    pub fraction: f32,
    pub detail: String,
}

pub enum EngineEvent {
    Progress(Progress),
    Log(String),
}

/// A backend that turns a request into a song directory.
pub trait Engine: Send + Sync {
    fn generate(
        &self,
        request: &SongRequest,
        id: &str,
        out_dir: &Path,
        on_event: &mut dyn FnMut(EngineEvent),
        cancel: &CancelToken,
    ) -> Result<()>;
}

pub struct MlxYue {
    config: Config,
}

impl MlxYue {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    fn write_request(&self, request: &SongRequest, id: &str) -> Result<std::path::PathBuf> {
        let dir = self.config.library_dir.join(".requests");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{id}.json"));
        let body = serde_json::json!({
            "id": id,
            "style": request.style,
            "lyrics": request.lyrics,
            "cot": request.mode.as_str(),
            "seed": request.seed,
            "generation_config": { "ode_steps": request.steps },
        });
        std::fs::write(&path, serde_json::to_string_pretty(&body)? + "\n")?;
        Ok(path)
    }
}

impl Engine for MlxYue {
    fn generate(
        &self,
        request: &SongRequest,
        id: &str,
        out_dir: &Path,
        on_event: &mut dyn FnMut(EngineEvent),
        cancel: &CancelToken,
    ) -> Result<()> {
        let cfg = &self.config;
        let bin = cfg.mlx_yue();
        if !bin.is_file() {
            bail!("mlx-yue not found at {}", bin.display());
        }
        let request_path = self.write_request(request, id)?;
        let mut child = Command::new(&bin)
            .arg("generate")
            .arg(&request_path)
            .args(["--model", "models/converted", "--vae", "models/vae", "--offline"])
            .args(["--precision", &cfg.precision])
            .args(["--vae-core-frames", &cfg.vae_core_frames.to_string()])
            .args(["--memory-budget-gib", &cfg.memory_budget_gib.to_string()])
            .arg("--output")
            .arg(out_dir)
            .current_dir(&cfg.yue_project)
            .env("MLX_ENABLE_TF32", "0")
            .env("PYTHONUNBUFFERED", "1")
            .env("LYRA_VAE", cfg.yue_project.join("models/vae"))
            .env("PATH", tool_path())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("starting {}", bin.display()))?;

        // YuE2 prints progress on stderr and the final summary on stdout; merge both.
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        for stream in [
            child.stdout.take().map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
            child.stderr.take().map(|s| Box::new(s) as Box<dyn std::io::Read + Send>),
        ]
        .into_iter()
        .flatten()
        {
            let tx = tx.clone();
            std::thread::spawn(move || {
                for line in BufReader::new(stream).lines().map_while(Result::ok) {
                    if tx.send(line).is_err() {
                        break;
                    }
                }
            });
        }
        drop(tx);

        let child = Arc::new(Mutex::new(child));
        let mut tracker = ProgressTracker::default();
        let mut log: Vec<String> = Vec::new();
        loop {
            if cancel.is_cancelled() {
                let _ = child.lock().unwrap().kill();
            }
            match rx.recv_timeout(Duration::from_millis(200)) {
                Ok(line) => {
                    if let Some(p) = tracker.observe(&line) {
                        on_event(EngineEvent::Progress(p));
                    }
                    log.push(line.clone());
                    on_event(EngineEvent::Log(line));
                }
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
        let status = child.lock().unwrap().wait()?;
        let _ = std::fs::remove_file(&request_path);
        if std::fs::create_dir_all(out_dir).is_ok() {
            let _ = std::fs::write(out_dir.join(LOG_FILE), log.join("\n") + "\n");
        }
        if cancel.is_cancelled() {
            bail!(Cancelled);
        }
        if !status.success() {
            bail!(explain_failure(&log).unwrap_or_else(|| format!("mlx-yue exited with {status}")));
        }
        Ok(())
    }
}

/// The engine's full output, saved inside each song directory.
pub const LOG_FILE: &str = "canto.log";

/// Turn the last Python exception into a sentence a person can act on.
fn explain_failure(log: &[String]) -> Option<String> {
    let last = log.iter().rev().find(|l| {
        let head = l.split(':').next().unwrap_or("");
        !l.starts_with(' ') && head.ends_with("Error") && !head.contains(' ')
    })?;
    let (kind, message) = last.split_once(':').unwrap_or((last, ""));
    let message = message.trim();
    Some(if kind == "MemoryError" {
        format!(
            "Not enough free memory, so YuE2 stopped to avoid swapping ({message}). \
             Quit memory-heavy apps and try again.{}",
            biggest_process().map(|p| format!(" Largest right now: {p}.")).unwrap_or_default()
        )
    } else {
        format!("{kind}: {message}")
    })
}

/// The process using the most memory, as "name (N GB)".
fn biggest_process() -> Option<String> {
    let out = Command::new("ps").args(["-axo", "rss=,comm="]).output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let (rss, comm) = text
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            let (rss, comm) = l.split_once(' ')?;
            Some((rss.parse::<u64>().ok()?, comm.trim().to_string()))
        })
        .max_by_key(|(rss, _)| *rss)?;
    let name = comm.rsplit('/').next().unwrap_or(&comm).to_string();
    Some(format!("{name} ({:.1} GB)", rss as f64 / 1_048_576.0))
}

/// Marker error for a run the user stopped.
#[derive(Debug)]
pub struct Cancelled;

impl std::fmt::Display for Cancelled {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("cancelled")
    }
}

impl std::error::Error for Cancelled {}

/// A GUI launch inherits launchd's bare PATH; ffmpeg and friends live in Homebrew.
pub(crate) fn tool_path() -> String {
    let mut parts: Vec<String> = std::env::var("PATH")
        .unwrap_or_default()
        .split(':')
        .filter(|p| !p.is_empty())
        .map(String::from)
        .collect();
    for extra in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin"] {
        if !parts.iter().any(|p| p == extra) {
            parts.push(extra.into());
        }
    }
    parts.join(":")
}

/// Maps YuE2's `[YuE2] Running <Phase>: …` lines onto one overall fraction.
#[derive(Default)]
pub struct ProgressTracker {
    best: f32,
}

impl ProgressTracker {
    pub fn observe(&mut self, line: &str) -> Option<Progress> {
        let rest = line.strip_prefix("[YuE2] ")?;
        let (_, rest) = rest.split_once(' ')?; // Starting / Running / Completed
        let (phase, detail) = rest.split_once(':').unwrap_or((rest, ""));
        let detail = detail.trim();
        let tokens = number_before(detail, " tokens");
        let (stage, fraction) = match phase {
            "Planning score" => (Stage::Planning, 0.05 + 0.10 * saturate(tokens, 600.0)),
            "Generating song" => (Stage::Writing, 0.15 + 0.30 * saturate(tokens, 1500.0)),
            "Synthesizing audio" => {
                let steps = detail.split_whitespace().next().and_then(|s| s.split_once('/'));
                let frac = steps
                    .and_then(|(a, b)| Some(a.parse::<f32>().ok()? / b.parse::<f32>().ok()?.max(1.0)))
                    .unwrap_or(0.0);
                (Stage::Synthesizing, 0.45 + 0.50 * frac)
            }
            p if p.contains("decoder") || p.contains("Decoding") => (Stage::Decoding, 0.96),
            _ => (Stage::Loading, 0.02),
        };
        self.best = self.best.max(fraction);
        Some(Progress { stage, fraction: self.best, detail: detail.to_string() })
    }
}

fn saturate(value: Option<f32>, scale: f32) -> f32 {
    1.0 - (-value.unwrap_or(0.0) / scale).exp()
}

fn number_before(text: &str, unit: &str) -> Option<f32> {
    let idx = text.find(unit)?;
    text[..idx].split_whitespace().last()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_phases_monotonically() {
        let mut t = ProgressTracker::default();
        let p = t.observe("[YuE2] Running Planning score: 258 tokens | 51.5 tokens/s | elapsed 5.0s").unwrap();
        assert_eq!(p.stage, Stage::Planning);
        let p = t.observe("[YuE2] Running Synthesizing audio: 16/32 steps (50%) | elapsed 5.0s").unwrap();
        assert_eq!(p.stage, Stage::Synthesizing);
        assert!((p.fraction - 0.70).abs() < 0.01);
        let p = t.observe("[YuE2] Starting Loading acoustic model: elapsed 0.0s").unwrap();
        assert!(p.fraction >= 0.70, "never goes backwards");
        assert!(t.observe("plain log line").is_none());
    }

    #[test]
    fn explains_python_errors() {
        let log = vec![
            "Traceback (most recent call last):".to_string(),
            "    raise type(error)(*error.args) from error".to_string(),
            "ValueError: lyrics cannot be empty".to_string(),
        ];
        assert_eq!(explain_failure(&log).unwrap(), "ValueError: lyrics cannot be empty");
    }
}
