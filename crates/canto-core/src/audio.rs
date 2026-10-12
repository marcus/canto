//! Audio helpers: waveform peaks for display, MP3 export for sharing.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, Result, bail};

use crate::engine::tool_path;
use crate::song::Song;

const WAVEFORM_FILE: &str = "waveform.json";

/// `buckets` peak amplitudes in 0.0–1.0, cached next to the audio.
pub fn waveform(song: &Song, buckets: usize) -> Result<Vec<f32>> {
    let audio = song.audio.as_ref().context("song has no audio yet")?;
    let cache = audio.with_file_name(WAVEFORM_FILE);
    if let Ok(text) = std::fs::read_to_string(&cache) {
        if let Ok(peaks) = serde_json::from_str::<Vec<f32>>(&text) {
            if peaks.len() == buckets {
                return Ok(peaks);
            }
        }
    }
    let peaks = compute_peaks(audio, buckets)?;
    let _ = std::fs::write(&cache, serde_json::to_string(&peaks)?);
    Ok(peaks)
}

fn compute_peaks(path: &Path, buckets: usize) -> Result<Vec<f32>> {
    let mut reader = claxon::FlacReader::open(path).with_context(|| format!("opening {}", path.display()))?;
    let info = reader.streaminfo();
    let total = info.samples.context("FLAC has no sample count")? as usize * info.channels as usize;
    let scale = (1i64 << (info.bits_per_sample - 1)) as f32;
    let per_bucket = (total / buckets).max(1);
    let mut peaks = vec![0f32; buckets];
    for (i, sample) in reader.samples().enumerate() {
        let v = (sample? as f32 / scale).abs();
        let b = (i / per_bucket).min(buckets - 1);
        if v > peaks[b] {
            peaks[b] = v;
        }
    }
    // Normalise so quiet songs still draw a readable shape.
    let max = peaks.iter().cloned().fold(0f32, f32::max).max(1e-3);
    Ok(peaks.into_iter().map(|p| (p / max).sqrt()).collect())
}

/// Encode a 320 kbps MP3 beside the FLAC; reuses an up-to-date one.
pub fn export_mp3(song: &Song) -> Result<PathBuf> {
    let audio = song.audio.as_ref().context("song has no audio yet")?;
    let target = audio.with_extension("mp3");
    let fresh = |t: &Path| -> Option<bool> {
        Some(std::fs::metadata(t).ok()?.modified().ok()? >= std::fs::metadata(audio).ok()?.modified().ok()?)
    };
    if fresh(&target) == Some(true) {
        return Ok(target);
    }
    let out = Command::new("ffmpeg")
        .env("PATH", tool_path())
        .args(["-y", "-hide_banner", "-loglevel", "error", "-i"])
        .arg(audio)
        .args(["-c:a", "libmp3lame", "-b:a", "320k"])
        .arg(&target)
        .output()
        .context("starting ffmpeg")?;
    if !out.status.success() {
        bail!("ffmpeg failed: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(target)
}
