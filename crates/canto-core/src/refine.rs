//! Prompt refinement through an agent CLI on the user's subscription.
//!
//! Both adapters get the same instructions and return the same [`Brief`]; the
//! only difference is how each CLI is asked for structured output.

use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};

use crate::engine::tool_path;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RefinerKind {
    #[default]
    Claude,
    Codex,
}

impl RefinerKind {
    pub const ALL: [RefinerKind; 2] = [RefinerKind::Claude, RefinerKind::Codex];

    pub fn label(self) -> &'static str {
        match self {
            RefinerKind::Claude => "Claude",
            RefinerKind::Codex => "Codex",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "claude" => Some(Self::Claude),
            "codex" => Some(Self::Codex),
            _ => None,
        }
    }

    pub fn build(self, model: Option<String>) -> Box<dyn Refiner> {
        match self {
            RefinerKind::Claude => Box::new(ClaudeCli { model }),
            RefinerKind::Codex => Box::new(CodexCli { model }),
        }
    }
}

/// What to refine.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RefineTask {
    /// Write a whole song from a one-line idea.
    Idea { description: String },
    /// Improve an existing draft. Empty fields get written from scratch.
    Polish {
        title: String,
        style: String,
        lyrics: String,
        /// Free-form direction, e.g. "make the chorus punchier".
        instruction: Option<String>,
        /// Which fields to change; the rest come back untouched.
        scope: Scope,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    #[default]
    All,
    Style,
    Lyrics,
}

/// A refined song brief.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Brief {
    pub title: String,
    pub style: String,
    pub lyrics: String,
    /// One or two sentences on what changed and why.
    pub notes: String,
}

pub trait Refiner: Send + Sync {
    fn name(&self) -> &'static str;
    fn refine(&self, task: &RefineTask) -> Result<Brief>;
}

const SYSTEM_PROMPT: &str = r#"You write song briefs for YuE2, a local text-to-song model that sings full songs (vocals plus backing) from a style prompt and section-tagged lyrics.

What YuE2 responds to:
- style: one line of English, comma-separated tags, 12 to 30 words. Cover genre and sub-genre, lead vocal (gender, timbre, language), 3 to 5 signature instruments, tempo as "NN BPM", mood, and production texture. Example: "Indie rock, English male lead vocal, jangly electric guitars, live bass and drums, Hammond organ, mid tempo 92 BPM, bittersweet verse and big anthemic chorus, warm analog production". No artist names, no song titles, no sentences, no storytelling.
- lyrics: English unless asked otherwise. Use only these section tags, each on its own line in square brackets: [Intro], [Verse], [Pre-Chorus], [Chorus], [Bridge], [Outro]. Four-line verses and a chorus repeated word for word give the most coherent songs. Lines should be singable: 5 to 10 syllables, concrete images, natural rhyme, no stage directions, no parentheses, no ad-lib notation, no emoji. Put a blank line between sections. Song length follows the lyrics: aim for about 2 to 3 minutes, which is roughly Intro, Verse, Pre-Chorus, Chorus, Verse, Pre-Chorus, Chorus, Bridge, Chorus, Outro. [Intro] and [Outro] may be empty or one short line.
- title: two to five words, evocative, no quotation marks.
- notes: one or two plain sentences saying what you did.

Keep what the user clearly wants. When polishing, preserve their voice and any lines that already work; fix meter, clichés, structure and tag format. Return only the structured fields."#;

fn user_prompt(task: &RefineTask) -> String {
    match task {
        RefineTask::Idea { description } => format!(
            "Write a complete song brief (title, style, lyrics, notes) from this idea:\n\n{}",
            description.trim()
        ),
        RefineTask::Polish { title, style, lyrics, instruction, scope } => {
            let mut out = String::new();
            out.push_str(match scope {
                Scope::All => "Refine this song brief. Improve the title, style and lyrics.",
                Scope::Style => "Refine only the style prompt. Return title and lyrics exactly as given.",
                Scope::Lyrics => "Refine only the lyrics. Return title and style exactly as given.",
            });
            out.push_str(" Write any empty field from scratch so it fits the others.");
            if let Some(i) = instruction.as_deref().map(str::trim).filter(|i| !i.is_empty()) {
                out.push_str(&format!("\n\nDirection from the user: {i}"));
            }
            out.push_str(&format!(
                "\n\nTitle:\n{}\n\nStyle:\n{}\n\nLyrics:\n{}",
                or_empty(title),
                or_empty(style),
                or_empty(lyrics)
            ));
            out
        }
    }
}

fn or_empty(s: &str) -> &str {
    if s.trim().is_empty() { "(empty)" } else { s.trim() }
}

fn schema() -> serde_json::Value {
    let text = serde_json::json!({ "type": "string" });
    serde_json::json!({
        "type": "object",
        "properties": { "title": text, "style": text, "lyrics": text, "notes": text },
        "required": ["title", "style", "lyrics", "notes"],
        "additionalProperties": false,
    })
}

/// A scratch directory so the agent CLI picks up no project instructions.
fn scratch_dir() -> Result<PathBuf> {
    let dir = std::env::temp_dir().join("canto-refine");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Keep the user's text where the scope says it must not change.
fn apply_scope(task: &RefineTask, mut brief: Brief) -> Brief {
    if let RefineTask::Polish { title, style, lyrics, scope, .. } = task {
        match scope {
            Scope::All => {}
            Scope::Style => {
                brief.title = title.clone();
                brief.lyrics = lyrics.clone();
            }
            Scope::Lyrics => {
                brief.title = title.clone();
                brief.style = style.clone();
            }
        }
        if brief.title.trim().is_empty() {
            brief.title = title.clone();
        }
    }
    brief.lyrics = brief.lyrics.trim().to_string() + "\n";
    brief
}

fn run(mut cmd: Command, what: &str) -> Result<String> {
    let out = cmd
        .current_dir(scratch_dir()?)
        .env("PATH", tool_path())
        .stdin(Stdio::null())
        .output()
        .with_context(|| format!("starting {what}; is it installed and on PATH?"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let tail: String = err.lines().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("\n");
        bail!("{what} failed ({}): {tail}", out.status);
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

pub struct ClaudeCli {
    pub model: Option<String>,
}

impl Refiner for ClaudeCli {
    fn name(&self) -> &'static str {
        "claude"
    }

    fn refine(&self, task: &RefineTask) -> Result<Brief> {
        let mut cmd = Command::new("claude");
        cmd.args(["-p", "--output-format", "json", "--tools", "", "--no-session-persistence"])
            .args(["--system-prompt", SYSTEM_PROMPT])
            .args(["--json-schema", &schema().to_string()]);
        if let Some(m) = &self.model {
            cmd.args(["--model", m]);
        }
        cmd.arg(user_prompt(task));
        let stdout = run(cmd, "claude")?;
        let reply: serde_json::Value = serde_json::from_str(&stdout).context("claude returned non-JSON output")?;
        if reply.get("is_error").and_then(|v| v.as_bool()) == Some(true) {
            bail!("claude: {}", reply.get("result").and_then(|v| v.as_str()).unwrap_or("unknown error"));
        }
        let structured = reply
            .get("structured_output")
            .cloned()
            .or_else(|| reply.get("result").and_then(|r| r.as_str()).and_then(|s| serde_json::from_str(s).ok()))
            .context("claude reply had no structured output")?;
        Ok(apply_scope(task, serde_json::from_value(structured)?))
    }
}

pub struct CodexCli {
    pub model: Option<String>,
}

impl Refiner for CodexCli {
    fn name(&self) -> &'static str {
        "codex"
    }

    fn refine(&self, task: &RefineTask) -> Result<Brief> {
        let dir = scratch_dir()?;
        let nonce = format!("{}-{}", std::process::id(), crate::library::now());
        let schema_path = dir.join(format!("schema-{nonce}.json"));
        let out_path = dir.join(format!("reply-{nonce}.json"));
        std::fs::write(&schema_path, schema().to_string())?;
        let mut cmd = Command::new("codex");
        cmd.args(["exec", "--skip-git-repo-check", "--ephemeral", "-s", "read-only", "--color", "never"])
            .arg("--output-schema")
            .arg(&schema_path)
            .arg("-o")
            .arg(&out_path);
        if let Some(m) = &self.model {
            cmd.args(["-m", m]);
        }
        cmd.arg(format!("{SYSTEM_PROMPT}\n\n---\n\n{}", user_prompt(task)));
        let result = run(cmd, "codex").and_then(|_| {
            let text = std::fs::read_to_string(&out_path).context("codex wrote no reply")?;
            Ok(serde_json::from_str::<Brief>(text.trim())?)
        });
        let _ = std::fs::remove_file(&schema_path);
        let _ = std::fs::remove_file(&out_path);
        Ok(apply_scope(task, result?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_protects_untouched_fields() {
        let task = RefineTask::Polish {
            title: "Mine".into(),
            style: "rock".into(),
            lyrics: "[Verse]\nhello".into(),
            instruction: None,
            scope: Scope::Style,
        };
        let brief = Brief { title: "Theirs".into(), style: "better rock".into(), lyrics: "changed".into(), notes: String::new() };
        let out = apply_scope(&task, brief);
        assert_eq!(out.title, "Mine");
        assert_eq!(out.style, "better rock");
        assert_eq!(out.lyrics, "[Verse]\nhello\n");
    }
}
