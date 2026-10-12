//! `canto` — the agent-facing surface. Every command has `--json`.

use std::io::Write as _;
use std::path::PathBuf;

use anyhow::{Context as _, Result, bail};
use canto_core::{
    CancelToken, Config, EngineEvent, Mode, RefineTask, RefinerKind, Scope, Song, SongRequest, Studio, DRAFT_STEPS,
    FULL_STEPS,
};
use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "canto", version, about = "Make songs with YuE2 locally, with prompts refined by Claude or Codex")]
struct Cli {
    /// Machine-readable output (JSON, or JSONL for streamed progress)
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Generate a song. Prints progress, then the finished song.
    Generate(GenerateArgs),
    /// Rewrite a song brief with an agent CLI (no generation).
    Refine(RefineArgs),
    /// List songs, newest first.
    List {
        /// Only favorites
        #[arg(long)]
        favorites: bool,
    },
    /// Show one song.
    Show { id: String },
    /// Mark or unmark a favorite.
    Favorite {
        id: String,
        #[arg(long)]
        off: bool,
    },
    /// Change a song's display title.
    Rename { id: String, title: String },
    /// Move a song to the library's .trash/ folder.
    Delete { id: String },
    /// Write a 320 kbps MP3 next to the FLAC and print its path.
    ExportMp3 { id: String },
    /// Print the effective configuration and where it is stored.
    Config,
}

#[derive(Args)]
struct GenerateArgs {
    /// Display title (also names the song's folder)
    #[arg(long, default_value = "")]
    title: String,
    /// Style tags: genre, vocal, instruments, tempo, mood
    #[arg(long, default_value = "")]
    style: String,
    #[arg(long, conflicts_with = "lyrics_file")]
    lyrics: Option<String>,
    #[arg(long)]
    lyrics_file: Option<PathBuf>,
    /// Write title, style and lyrics from this idea first
    #[arg(long)]
    idea: Option<String>,
    /// Refine the given style and lyrics before generating
    #[arg(long)]
    refine: bool,
    /// Refiner to use with --idea or --refine
    #[arg(long, value_parser = parse_refiner)]
    with: Option<RefinerKind>,
    /// Seed (random when omitted). Same seed + prompt = same song.
    #[arg(long)]
    seed: Option<u64>,
    /// Quick 8-step audition instead of the full 32 steps
    #[arg(long)]
    draft: bool,
    #[arg(long, value_parser = parse_mode, default_value = "full")]
    mode: Mode,
}

#[derive(Args)]
struct RefineArgs {
    /// Write a whole brief from a one-line idea
    #[arg(long, conflicts_with_all = ["title", "style", "lyrics", "lyrics_file"])]
    idea: Option<String>,
    #[arg(long, default_value = "")]
    title: String,
    #[arg(long, default_value = "")]
    style: String,
    #[arg(long, conflicts_with = "lyrics_file")]
    lyrics: Option<String>,
    #[arg(long)]
    lyrics_file: Option<PathBuf>,
    /// Direction for the rewrite, e.g. "make the chorus punchier"
    #[arg(long)]
    instruction: Option<String>,
    /// Which fields to change: all, style or lyrics
    #[arg(long, value_parser = parse_scope, default_value = "all")]
    only: Scope,
    #[arg(long, value_parser = parse_refiner)]
    with: Option<RefinerKind>,
}

fn parse_refiner(s: &str) -> Result<RefinerKind, String> {
    RefinerKind::parse(s).ok_or_else(|| "expected claude or codex".into())
}
fn parse_mode(s: &str) -> Result<Mode, String> {
    Mode::parse(s).ok_or_else(|| "expected full, melody or off".into())
}
fn parse_scope(s: &str) -> Result<Scope, String> {
    match s {
        "all" => Ok(Scope::All),
        "style" => Ok(Scope::Style),
        "lyrics" => Ok(Scope::Lyrics),
        _ => Err("expected all, style or lyrics".into()),
    }
}

fn read_lyrics(inline: Option<String>, file: Option<PathBuf>) -> Result<String> {
    match (inline, file) {
        (Some(l), _) => Ok(l),
        (None, Some(path)) if path.as_os_str() == "-" => Ok(std::io::read_to_string(std::io::stdin())?),
        (None, Some(path)) => std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display())),
        (None, None) => Ok(String::new()),
    }
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(&cli) {
        if cli.json {
            println!("{}", serde_json::json!({ "error": format!("{e:#}") }));
        } else {
            eprintln!("canto: {e:#}");
        }
        std::process::exit(1);
    }
}

fn run(cli: &Cli) -> Result<()> {
    let studio = Studio::open()?;
    let json = cli.json;
    match &cli.command {
        Cmd::Generate(args) => generate(&studio, args, json),
        Cmd::Refine(args) => {
            let task = match &args.idea {
                Some(idea) => RefineTask::Idea { description: idea.clone() },
                None => RefineTask::Polish {
                    title: args.title.clone(),
                    style: args.style.clone(),
                    lyrics: read_lyrics(args.lyrics.clone(), args.lyrics_file.clone())?,
                    instruction: args.instruction.clone(),
                    scope: args.only,
                },
            };
            let brief = studio.refine(&task, args.with)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&brief)?);
            } else {
                println!("Title: {}\nStyle: {}\n\n{}\n{}", brief.title, brief.style, brief.lyrics.trim(), brief.notes);
            }
            Ok(())
        }
        Cmd::List { favorites } => {
            let songs: Vec<Song> = studio.library.list()?.into_iter().filter(|s| !favorites || s.favorite).collect();
            if json {
                println!("{}", serde_json::to_string_pretty(&songs)?);
            } else {
                for s in &songs {
                    println!(
                        "{:<28} {:>6}  {:<10} {}{}",
                        s.id,
                        s.duration_secs.map(fmt_secs).unwrap_or_else(|| "—".into()),
                        format!("{:?}", s.status).to_lowercase(),
                        if s.favorite { "♥ " } else { "" },
                        s.title
                    );
                }
            }
            Ok(())
        }
        Cmd::Show { id } => print_song(&studio.library.get(id)?, json),
        Cmd::Favorite { id, off } => print_song(&studio.library.set_favorite(id, !off)?, json),
        Cmd::Rename { id, title } => print_song(&studio.library.rename(id, title)?, json),
        Cmd::Delete { id } => {
            let dest = studio.library.trash(id)?;
            if json {
                println!("{}", serde_json::json!({ "id": id, "trashed_to": dest }));
            } else {
                println!("moved {id} to {}", dest.display());
            }
            Ok(())
        }
        Cmd::ExportMp3 { id } => {
            let path = canto_core::audio::export_mp3(&studio.library.get(id)?)?;
            if json {
                println!("{}", serde_json::json!({ "id": id, "mp3": path }));
            } else {
                println!("{}", path.display());
            }
            Ok(())
        }
        Cmd::Config => {
            let path = Config::path();
            if json {
                println!("{}", serde_json::json!({ "path": path, "config": studio.config }));
            } else {
                println!("# {}\n{}", path.display(), serde_json::to_string_pretty(&studio.config)?);
            }
            Ok(())
        }
    }
}

fn generate(studio: &Studio, args: &GenerateArgs, json: bool) -> Result<()> {
    let mut request = SongRequest {
        title: args.title.clone(),
        style: args.style.clone(),
        lyrics: read_lyrics(args.lyrics.clone(), args.lyrics_file.clone())?,
        mode: args.mode,
        seed: args.seed.unwrap_or_else(canto_core::random_seed),
        steps: if args.draft { DRAFT_STEPS } else { FULL_STEPS },
    };
    let mut prompt_source = None;
    let task = if let Some(idea) = &args.idea {
        prompt_source = Some(idea.clone());
        Some(RefineTask::Idea { description: idea.clone() })
    } else if args.refine {
        Some(RefineTask::Polish {
            title: request.title.clone(),
            style: request.style.clone(),
            lyrics: request.lyrics.clone(),
            instruction: None,
            scope: Scope::All,
        })
    } else {
        None
    };
    if let Some(task) = task {
        emit(json, serde_json::json!({ "event": "refining" }), "Refining the brief…");
        let brief = studio.refine(&task, args.with)?;
        emit(json, serde_json::json!({ "event": "refined", "brief": brief }), &format!("Title: {}\nStyle: {}", brief.title, brief.style));
        request.title = brief.title;
        request.style = brief.style;
        request.lyrics = brief.lyrics;
    }
    if request.style.trim().is_empty() || request.lyrics.trim().is_empty() {
        bail!("need --style and --lyrics (or --lyrics-file), or an --idea to write them");
    }
    let id = studio.reserve_id(&request);
    emit(json, serde_json::json!({ "event": "started", "id": id, "request": request }), &format!("Generating {id} (seed {})", request.seed));

    let cancel = CancelToken::default();
    let mut last_stage = None;
    let mut on_event = |event: EngineEvent| {
        if let EngineEvent::Progress(p) = event {
            if json {
                println!("{}", serde_json::json!({ "event": "progress", "progress": p }));
            } else if last_stage != Some(p.stage) || p.fraction > 0.0 {
                last_stage = Some(p.stage);
                eprint!("\r\x1b[2K{:>3.0}%  {}  {}", p.fraction * 100.0, p.stage.label(), p.detail);
                let _ = std::io::stderr().flush();
            }
        }
    };
    let song = studio.generate(&request, &id, prompt_source, &mut on_event, &cancel);
    if !json {
        eprintln!();
    }
    let song = song?;
    if json {
        println!("{}", serde_json::json!({ "event": "done", "song": song }));
    } else {
        print_song(&song, false)?;
    }
    Ok(())
}

fn emit(json: bool, value: serde_json::Value, human: &str) {
    if json {
        println!("{value}");
    } else {
        eprintln!("{human}");
    }
}

fn print_song(song: &Song, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(song)?);
        return Ok(());
    }
    println!("{}{}", song.title, if song.favorite { "  ♥" } else { "" });
    println!("  id:       {}", song.id);
    println!("  status:   {:?}{}", song.status, if song.is_draft() { " (draft)" } else { "" });
    if let Some(d) = song.duration_secs {
        println!("  length:   {}", fmt_secs(d));
    }
    println!("  style:    {}", song.style);
    println!("  seed:     {}", song.seed);
    if let Some(a) = &song.audio {
        println!("  audio:    {}", a.display());
    }
    if let Some(e) = &song.error {
        println!("  error:    {e}");
    }
    Ok(())
}

fn fmt_secs(s: f64) -> String {
    let s = s.round() as u64;
    format!("{}:{:02}", s / 60, s % 60)
}
