//! The song library: one directory per song under `library_dir`.
//!
//! YuE2 writes `request.json`, `result.json`, `config.json` and `audio.flac`
//! into each song directory. Canto adds `canto.json` (title, favorite, status)
//! and never edits YuE2's files, so songs made by other tools show up too.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context as _, Result, bail};
use serde_json::Value;

use crate::song::{FULL_STEPS, Mode, Song, SongMeta, SongStatus};

pub const META_FILE: &str = "canto.json";
const TRASH_DIR: &str = ".trash";

pub struct Library {
    dir: PathBuf,
}

impl Library {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Every song, newest first.
    pub fn list(&self) -> Result<Vec<Song>> {
        let mut songs = Vec::new();
        let entries = match std::fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(songs),
            Err(e) => return Err(e).with_context(|| format!("reading {}", self.dir.display())),
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || !entry.path().is_dir() {
                continue;
            }
            if let Some(song) = self.load(&name) {
                songs.push(song);
            }
        }
        songs.sort_by(|a, b| b.created.cmp(&a.created).then_with(|| a.id.cmp(&b.id)));
        Ok(songs)
    }

    pub fn get(&self, id: &str) -> Result<Song> {
        check_id(id)?;
        self.load(id).with_context(|| format!("no song named {id}"))
    }

    fn load(&self, id: &str) -> Option<Song> {
        let dir = self.dir.join(id);
        let artifact = artifact_dir(&dir);
        let meta = read_meta(&dir);
        let request = read_json(&artifact.join("request.json"));
        if request.is_none() && meta.is_none() {
            return None; // not a song directory
        }
        let meta = meta.unwrap_or_default();
        let request = request.unwrap_or(Value::Null);
        let result = read_json(&artifact.join("result.json"));
        let gen_config = read_json(&artifact.join("config.json"));

        let audio = Some(artifact.join("audio.flac")).filter(|p| p.is_file());
        let mp3 = Some(artifact.join("audio.mp3")).filter(|p| p.is_file());
        let result_complete = result
            .as_ref()
            .and_then(|r| r.get("status"))
            .and_then(Value::as_str)
            == Some("complete");
        let status = if result_complete && audio.is_some() {
            SongStatus::Complete
        } else {
            match meta.status {
                Some(s @ (SongStatus::Failed | SongStatus::Cancelled)) => s,
                _ => SongStatus::Incomplete,
            }
        };
        // YuE2's request.json is the truth; Canto's copy covers runs that died early.
        let asked = meta.request.clone().unwrap_or_default();
        let str_field = |key: &str, fallback: &str| {
            request.get(key).and_then(Value::as_str).unwrap_or(fallback).to_string()
        };
        let steps = meta
            .steps
            .or_else(|| {
                gen_config
                    .as_ref()
                    .and_then(|c| c.pointer("/generation/ode_steps"))
                    .and_then(Value::as_u64)
                    .map(|n| n as u32)
            })
            .or(meta.request.as_ref().map(|r| r.steps))
            .unwrap_or(FULL_STEPS);

        Some(Song {
            id: id.to_string(),
            title: if meta.title.is_empty() { humanize(id) } else { meta.title.clone() },
            style: str_field("style", &asked.style),
            lyrics: str_field("lyrics", &asked.lyrics),
            mode: Mode::parse(&str_field("cot", asked.mode.as_str())).unwrap_or_default(),
            seed: request.get("seed").and_then(Value::as_u64).unwrap_or(asked.seed),
            steps,
            max_duration_seconds: asked.max_duration_seconds,
            instrumental: asked.instrumental,
            abc: asked.abc.clone(),
            created: if meta.created > 0 { meta.created } else { mtime(&dir) },
            favorite: meta.favorite,
            status,
            error: meta.error.clone(),
            duration_secs: result.as_ref().and_then(|r| r.get("audio_seconds")).and_then(Value::as_f64),
            audio,
            mp3,
            seconds_to_generate: meta.seconds_to_generate,
            prompt_source: meta.prompt_source.clone(),
            dir,
        })
    }

    /// A fresh directory name derived from the title, never reusing one.
    pub fn new_id(&self, title: &str) -> String {
        let base = slug(title);
        let mut candidate = base.clone();
        let mut n = 2;
        while self.dir.join(&candidate).exists() || self.dir.join(TRASH_DIR).join(&candidate).exists() {
            candidate = format!("{base}-{n}");
            n += 1;
        }
        candidate
    }

    pub fn read_meta(&self, id: &str) -> SongMeta {
        read_meta(&self.dir.join(id)).unwrap_or_default()
    }

    pub fn write_meta(&self, id: &str, meta: &SongMeta) -> Result<()> {
        check_id(id)?;
        let dir = self.dir.join(id);
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join(META_FILE), serde_json::to_string_pretty(meta)? + "\n")?;
        Ok(())
    }

    pub fn update_meta(&self, id: &str, f: impl FnOnce(&mut SongMeta)) -> Result<Song> {
        let song = self.get(id)?;
        let mut meta = self.read_meta(id);
        if meta.title.is_empty() {
            meta.title = song.title.clone();
        }
        if meta.created == 0 {
            meta.created = song.created;
        }
        f(&mut meta);
        self.write_meta(id, &meta)?;
        self.get(id)
    }

    pub fn set_favorite(&self, id: &str, favorite: bool) -> Result<Song> {
        self.update_meta(id, |m| m.favorite = favorite)
    }

    pub fn rename(&self, id: &str, title: &str) -> Result<Song> {
        let title = title.trim();
        anyhow::ensure!(!title.is_empty(), "title cannot be empty");
        self.update_meta(id, |m| m.title = title.to_string())
    }

    /// Move a song into the library's `.trash/`, where it can be recovered by hand.
    pub fn trash(&self, id: &str) -> Result<PathBuf> {
        check_id(id)?;
        let src = self.dir.join(id);
        if !src.is_dir() {
            bail!("no song named {id}");
        }
        let trash = self.dir.join(TRASH_DIR);
        std::fs::create_dir_all(&trash)?;
        let mut dest = trash.join(id);
        let mut n = 2;
        while dest.exists() {
            dest = trash.join(format!("{id}-{n}"));
            n += 1;
        }
        std::fs::rename(&src, &dest)?;
        Ok(dest)
    }
}

/// Covers made by the bundled studio keep the song in `<dir>/song/`.
fn artifact_dir(dir: &Path) -> PathBuf {
    let nested = dir.join("song");
    if !dir.join("audio.flac").is_file() && nested.join("audio.flac").is_file() {
        nested
    } else {
        dir.to_path_buf()
    }
}

fn read_meta(dir: &Path) -> Option<SongMeta> {
    let text = std::fs::read_to_string(dir.join(META_FILE)).ok()?;
    serde_json::from_str(&text).ok()
}

fn read_json(path: &Path) -> Option<Value> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

fn mtime(path: &Path) -> u64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn check_id(id: &str) -> Result<()> {
    if id.is_empty() || id.starts_with('.') || id.contains('/') || id.contains("..") {
        bail!("invalid song id {id:?}");
    }
    Ok(())
}

pub fn slug(text: &str) -> String {
    let mut out = String::new();
    for c in text.trim().to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
        if out.len() >= 40 {
            break;
        }
    }
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() { "song".into() } else { out }
}

fn humanize(id: &str) -> String {
    id.split(['-', '_'])
        .filter(|w| !w.is_empty())
        .map(|w| {
            let mut c = w.chars();
            c.next().map(|f| f.to_uppercase().chain(c).collect::<String>()).unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_requests_survive_library_load_and_remix_without_touching_engine_files() {
        let dir = std::env::temp_dir().join(format!("canto-library-duration-{}", crate::random_seed()));
        let library = Library::new(&dir);
        let request = crate::song::SongRequest { title: "Bounded".into(), style: "instrumental".into(), lyrics: "[intro]".into(), steps: 8, max_duration_seconds: Some(90), ..Default::default() };
        library.write_meta("bounded", &SongMeta { request: Some(request.clone()), ..Default::default() }).unwrap();
        // An early failed/incomplete song still retains the request and its bound.
        let song = library.get("bounded").unwrap();
        assert_eq!(song.max_duration_seconds, Some(90));
        assert_eq!(song.request().max_duration_seconds, Some(90));
        let native = r#"{"style":"native style","lyrics":"[verse]","seed":11,"cot":"melody"}"#;
        std::fs::write(dir.join("bounded/request.json"), native).unwrap();
        let song = library.get("bounded").unwrap();
        assert_eq!(song.style, "native style");
        assert_eq!(song.seed, 11);
        assert_eq!(song.request().max_duration_seconds, Some(90));
        library.rename("bounded", "Renamed").unwrap();
        assert_eq!(std::fs::read_to_string(dir.join("bounded/request.json")).unwrap(), native);
        assert_eq!(library.get("bounded").unwrap().request().max_duration_seconds, Some(90));
        library.write_meta("legacy", &SongMeta::default()).unwrap();
        assert_eq!(library.get("legacy").unwrap().request().max_duration_seconds, None);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn instrumental_metadata_survives_loading_and_remix_and_is_never_inferred() {
        let dir = std::env::temp_dir().join(format!("canto-library-instrumental-{}", crate::random_seed()));
        let library = Library::new(&dir);
        let request = crate::song::SongRequest { style: "piano".into(), steps: 32, instrumental: true, ..Default::default() };
        library.write_meta("instrumental", &SongMeta { request: Some(request), ..Default::default() }).unwrap();
        let native = r#"{"style":"instrumental, piano","lyrics":"","seed":42,"cot":"full"}"#;
        std::fs::write(dir.join("instrumental/request.json"), native).unwrap();
        let song = library.get("instrumental").unwrap();
        assert!(song.instrumental);
        assert!(song.request().instrumental);
        song.request().validate().unwrap();
        let mut remix = song.request();
        remix.lyrics = "[Verse]\nActual sung text".into();
        assert!(remix.validate().is_err());
        remix.instrumental = false;
        remix.validate().unwrap();
        library.rename("instrumental", "Renamed").unwrap();
        assert!(library.get("instrumental").unwrap().request().instrumental);
        assert_eq!(std::fs::read_to_string(dir.join("instrumental/request.json")).unwrap(), native);
        library.write_meta("native-only", &SongMeta::default()).unwrap();
        std::fs::write(dir.join("native-only/request.json"), native).unwrap();
        let legacy = library.get("native-only").unwrap();
        assert!(!legacy.instrumental);
        assert!(serde_json::to_value(legacy).unwrap().get("instrumental").is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn supplied_score_metadata_and_remix_retain_exact_text_without_inference() {
        let dir = std::env::temp_dir().join(format!("canto-library-score-{}", crate::random_seed()));
        let library = Library::new(&dir);
        let score = include_str!("../tests/fixtures/instrumental.abc");
        let request = crate::song::SongRequest { style: "piano".into(), steps: 32, instrumental: true, abc: Some(score.into()), ..Default::default() };
        library.write_meta("supplied", &SongMeta { request: Some(request), ..Default::default() }).unwrap();
        let native = serde_json::json!({"style":"instrumental, piano", "lyrics":"", "abc":score, "cot":"full", "seed":42}).to_string();
        std::fs::write(dir.join("supplied/request.json"), &native).unwrap();
        let song = library.get("supplied").unwrap();
        assert_eq!(song.abc.as_deref(), Some(score));
        let mut remix = song.request();
        assert_eq!(remix.abc.as_deref(), Some(score));
        remix.validate().unwrap();
        remix.mode = Mode::Off;
        assert!(remix.validate().is_err());
        remix.abc = None;
        remix.validate().unwrap();
        library.rename("supplied", "Renamed").unwrap();
        assert_eq!(library.get("supplied").unwrap().request().abc.as_deref(), Some(score));
        assert_eq!(std::fs::read_to_string(dir.join("supplied/request.json")).unwrap(), native);
        library.write_meta("legacy", &SongMeta::default()).unwrap();
        std::fs::write(dir.join("legacy/request.json"), native).unwrap();
        let legacy = library.get("legacy").unwrap();
        assert_eq!(legacy.request().abc, None);
        assert!(serde_json::to_value(legacy).unwrap().get("abc").is_none());
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn slug_is_filesystem_safe() {
        assert_eq!(slug("  Drive On, Drive On!  "), "drive-on-drive-on");
        assert_eq!(slug("¿¿??"), "song");
    }

    #[test]
    fn humanize_title_cases_ids() {
        assert_eq!(humanize("test-song"), "Test Song");
    }

    #[test]
    fn rejects_path_escapes() {
        assert!(check_id("../x").is_err());
        assert!(check_id(".trash").is_err());
        assert!(check_id("ok-song").is_ok());
    }
}
