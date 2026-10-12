//! The song model: what a user asks for, and what lands in the library.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// How much symbolic planning YuE2 does before singing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// Melody and chords: the most structure, the default.
    #[default]
    Full,
    /// Melody only.
    Melody,
    /// No symbolic plan.
    Off,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Full => "full",
            Mode::Melody => "melody",
            Mode::Off => "off",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "full" => Some(Mode::Full),
            "melody" => Some(Mode::Melody),
            "off" => Some(Mode::Off),
            _ => None,
        }
    }
}

/// Acoustic steps for a quick audition versus a finished song.
pub const DRAFT_STEPS: u32 = 8;
pub const FULL_STEPS: u32 = 32;

/// Everything needed to make one song.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SongRequest {
    pub title: String,
    /// English tags: genre, vocal, instruments, tempo, mood, production.
    pub style: String,
    /// Section-tagged lyrics, or empty for explicit instrumental conditioning.
    pub lyrics: String,
    #[serde(default)]
    pub mode: Mode,
    #[serde(default)]
    pub seed: u64,
    #[serde(default = "default_steps")]
    pub steps: u32,
    /// Optional upper bound in whole seconds (1–360). Omitted keeps the engine default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_duration_seconds: Option<u32>,
    /// Request empty-lyric instrumental conditioning, not guaranteed vocal-free audio.
    #[serde(default, skip_serializing_if = "is_false")]
    pub instrumental: bool,
    /// Optional supplied native ABC score text, retained for loading and remixes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abc: Option<String>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn default_steps() -> u32 {
    FULL_STEPS
}

impl SongRequest {
    pub fn validate(&self) -> anyhow::Result<()> {
        anyhow::ensure!(!self.style.trim().is_empty(), "style cannot be empty");
        self.validate_lyric_conditioning()?;
        anyhow::ensure!((1..=128).contains(&self.steps), "steps must be between 1 and 128");
        self.validate_duration_limit()?;
        self.validate_score_conditioning()?;
        Ok(())
    }

    pub fn validate_score_conditioning(&self) -> anyhow::Result<()> {
        if let Some(abc) = &self.abc {
            anyhow::ensure!(!abc.trim().is_empty(), "supplied ABC score cannot be empty");
            anyhow::ensure!(self.mode != Mode::Off, "supplied ABC score requires full or melody mode; off mode does not accept a score");
        }
        Ok(())
    }

    pub fn validate_lyric_conditioning(&self) -> anyhow::Result<()> {
        if self.instrumental {
            anyhow::ensure!(self.lyrics.trim().is_empty(), "instrumental conditioning requires empty lyrics; omit lyrics and section markers");
        } else {
            anyhow::ensure!(!self.lyrics.trim().is_empty(), "lyrics cannot be empty");
        }
        Ok(())
    }

    pub fn validate_duration_limit(&self) -> anyhow::Result<()> {
        if let Some(seconds) = self.max_duration_seconds {
            anyhow::ensure!((1..=360).contains(&seconds), "max_duration_seconds must be between 1 and 360");
        }
        Ok(())
    }

    pub fn is_draft(&self) -> bool {
        self.steps < FULL_STEPS
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SongStatus {
    Complete,
    Failed,
    Cancelled,
    /// A directory with no result yet: still running, or killed mid-run.
    Incomplete,
}

/// Canto's own notes about a song, kept in `canto.json` inside its directory.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SongMeta {
    pub title: String,
    pub created: u64,
    pub favorite: bool,
    pub steps: Option<u32>,
    /// The idea or instruction the refiner worked from, when one was used.
    pub prompt_source: Option<String>,
    pub status: Option<SongStatus>,
    pub error: Option<String>,
    pub seconds_to_generate: Option<f64>,
    /// What was asked for, so a failed run can still be reused or remixed.
    pub request: Option<SongRequest>,
}

/// A song as the library sees it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Song {
    pub id: String,
    pub dir: PathBuf,
    pub title: String,
    pub style: String,
    pub lyrics: String,
    pub mode: Mode,
    pub seed: u64,
    pub steps: u32,
    pub created: u64,
    pub favorite: bool,
    pub status: SongStatus,
    pub error: Option<String>,
    pub duration_secs: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_duration_seconds: Option<u32>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub instrumental: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abc: Option<String>,
    pub audio: Option<PathBuf>,
    pub mp3: Option<PathBuf>,
    pub seconds_to_generate: Option<f64>,
    pub prompt_source: Option<String>,
}

impl Song {
    pub fn request(&self) -> SongRequest {
        SongRequest {
            title: self.title.clone(),
            style: self.style.clone(),
            lyrics: self.lyrics.clone(),
            mode: self.mode,
            seed: self.seed,
            steps: self.steps,
            max_duration_seconds: self.max_duration_seconds,
            instrumental: self.instrumental,
            abc: self.abc.clone(),
        }
    }

    pub fn is_draft(&self) -> bool {
        self.steps < FULL_STEPS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_requests_keep_engine_default_and_roundtrip_without_a_limit() {
        let request: SongRequest = serde_json::from_str(r#"{"title":"Old","style":"folk","lyrics":"[intro]"}"#).unwrap();
        assert_eq!(request.max_duration_seconds, None);
        assert!(!request.instrumental);
        assert_eq!(request.abc, None);
        assert_eq!(request.steps, FULL_STEPS);
        assert!(request.validate().is_ok());
        let body = serde_json::to_value(request).unwrap();
        assert!(body.get("max_duration_seconds").is_none());
        assert!(body.get("instrumental").is_none());
        assert!(body.get("abc").is_none());
    }

    #[test]
    fn instrumental_conditioning_requires_explicit_intent_and_no_lyric_text() {
        let mut request = SongRequest { style: "piano, jazz".into(), steps: 32, ..Default::default() };
        assert!(request.validate().unwrap_err().to_string().contains("lyrics cannot be empty"));
        request.instrumental = true;
        request.validate().unwrap();
        let loaded: SongRequest = serde_json::from_value(serde_json::to_value(&request).unwrap()).unwrap();
        assert!(loaded.instrumental);
        for lyrics in ["Sing these words", "[Intro][Verse][Outro]", "[instrumental]"] {
            request.lyrics = lyrics.into();
            assert!(request.validate().unwrap_err().to_string().contains("requires empty lyrics"));
        }
        request.lyrics = "  \n  ".into();
        request.validate().unwrap();
        request.style.clear();
        assert!(request.validate().is_err());
    }

    #[test]
    fn supplied_score_validation_preserves_text_and_refuses_blank_or_off() {
        let score = include_str!("../tests/fixtures/instrumental.abc");
        let mut request = SongRequest { style: "piano".into(), instrumental: true, steps: 32, abc: Some(score.into()), ..Default::default() };
        for mode in [Mode::Full, Mode::Melody] {
            request.mode = mode;
            request.validate().unwrap();
            let loaded: SongRequest = serde_json::from_value(serde_json::to_value(&request).unwrap()).unwrap();
            assert_eq!(loaded.abc.as_deref(), Some(score));
        }
        request.mode = Mode::Off;
        assert!(request.validate().unwrap_err().to_string().contains("requires full or melody"));
        request.abc = None;
        request.validate().unwrap();
        request.mode = Mode::Full;
        for blank in ["", " \n\t"] {
            request.abc = Some(blank.into());
            assert!(request.validate().unwrap_err().to_string().contains("cannot be empty"));
        }
    }

    #[test]
    fn duration_limit_is_bounded_and_survives_request_serialization() {
        let mut request = SongRequest { style: "instrumental".into(), lyrics: "[intro]".into(), steps: 8, ..Default::default() };
        for seconds in [1, 90, 360] {
            request.max_duration_seconds = Some(seconds);
            request.validate().unwrap();
            let loaded: SongRequest = serde_json::from_value(serde_json::to_value(&request).unwrap()).unwrap();
            assert_eq!(loaded.max_duration_seconds, Some(seconds));
        }
        for seconds in [0, 361, u32::MAX] {
            request.max_duration_seconds = Some(seconds);
            assert!(request.validate().is_err());
        }
    }
}
