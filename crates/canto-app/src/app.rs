//! The root view: state, and every action the UI can take.
//!
//! All real work goes through `canto_core::Studio`, the same core the CLI uses.

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use canto_core::{
    Brief, CancelToken, Config, DRAFT_STEPS, EngineEvent, FULL_STEPS, Mode, Progress, RefineTask, RefinerKind, Scope,
    Song, SongRequest, Studio,
};
use gpui_kit::component::input::{InputEvent, InputState, TextareaState};
use gpui_kit::*;

use crate::player::Player;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Simple,
    Custom,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    All,
    Favorites,
}

/// What the refiner is busy with, so the matching button can spin.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Refining {
    Idea { then_create: bool },
    All,
    Style,
    Lyrics,
}

#[derive(Default)]
pub struct JobShared {
    pub progress: Option<Progress>,
    pub result: Option<anyhow::Result<Song>>,
}

pub struct Job {
    pub id: String,
    pub request: SongRequest,
    pub shared: Arc<Mutex<JobShared>>,
    pub cancel: CancelToken,
    pub started: Instant,
}

pub struct Toast {
    pub message: SharedString,
    pub is_error: bool,
    pub shown: Instant,
}

pub struct CantoApp {
    pub studio: Arc<Studio>,
    pub songs: Vec<Song>,
    pub jobs: Vec<Job>,

    pub tab: Tab,
    pub idea: Entity<TextareaState>,
    pub title: Entity<InputState>,
    pub style: Entity<TextareaState>,
    pub lyrics: Entity<TextareaState>,
    pub direction: Entity<InputState>,
    pub seed: Entity<InputState>,
    pub search: Entity<InputState>,
    pub draft: bool,
    pub mode: Mode,
    pub refiner: RefinerKind,
    pub refining: Option<Refining>,
    pub refine_notes: Option<SharedString>,
    pub prompt_source: Option<String>,
    pub form_error: Option<SharedString>,

    pub filter: Filter,
    pub selected: Option<String>,
    pub confirm_delete: Option<String>,

    pub player: Player,
    pub waveforms: HashMap<String, Rc<Vec<f32>>>,
    pub wave_bounds: Rc<std::cell::Cell<Bounds<Pixels>>>,
    pub volume_bounds: Rc<std::cell::Cell<Bounds<Pixels>>>,
    pub toast: Option<Toast>,
    _subscriptions: Vec<Subscription>,
}

impl CantoApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let studio = Arc::new(Studio::open().unwrap_or_else(|_| Studio::new(Config::default())));
        let refiner = studio.config.refiner;
        let idea = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(5, 10)
                .placeholder("Describe the song: genre, mood, what it's about")
        });
        let title = cx.new(|cx| InputState::new(window, cx).placeholder("Untitled"));
        let style = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(3, 6)
                .placeholder("Indie pop, airy female vocal, shimmering guitars, 104 BPM, nostalgic")
        });
        let lyrics = cx.new(|cx| {
            TextareaState::new(window, cx).placeholder("[Verse]\nWrite your lyrics, or describe the song\nand let AI write them\n\n[Chorus]\n…")
        });
        let direction = cx.new(|cx| InputState::new(window, cx).placeholder("Direction for AI, e.g. punchier chorus"));
        let seed = cx.new(|cx| InputState::new(window, cx).placeholder("Random"));
        let search = cx.new(|cx| InputState::new(window, cx).placeholder("Search songs"));
        let subscriptions = vec![cx.subscribe_in(&search, window, |_, _, event: &InputEvent, _, cx| {
            if matches!(event, InputEvent::Change) {
                cx.notify();
            }
        })];

        let mut app = Self {
            studio,
            songs: Vec::new(),
            jobs: Vec::new(),
            tab: Tab::Simple,
            idea,
            title,
            style,
            lyrics,
            direction,
            seed,
            search,
            draft: false,
            mode: Mode::Full,
            refiner,
            refining: None,
            refine_notes: None,
            prompt_source: None,
            form_error: None,
            filter: Filter::All,
            selected: None,
            confirm_delete: None,
            player: Player::new(),
            waveforms: HashMap::new(),
            wave_bounds: Default::default(),
            volume_bounds: Default::default(),
            toast: None,
            _subscriptions: subscriptions,
        };
        app.reload(cx);
        app.selected = app.songs.first().map(|s| s.id.clone());
        app.start_ticker(cx);
        app
    }

    // ------------------------------------------------------------ library

    pub fn reload(&mut self, cx: &mut Context<Self>) {
        match self.studio.library.list() {
            Ok(songs) => self.songs = songs,
            Err(e) => self.notify_error(format!("Couldn't read the library: {e:#}"), cx),
        }
        cx.notify();
    }

    pub fn visible_songs(&self, cx: &App) -> Vec<&Song> {
        let query = self.search.read(cx).value().to_lowercase();
        let in_flight: Vec<&str> = self.jobs.iter().map(|j| j.id.as_str()).collect();
        self.songs
            .iter()
            .filter(|s| !in_flight.contains(&s.id.as_str()))
            .filter(|s| self.filter == Filter::All || s.favorite)
            .filter(|s| {
                query.is_empty()
                    || s.title.to_lowercase().contains(&query)
                    || s.style.to_lowercase().contains(&query)
                    || s.lyrics.to_lowercase().contains(&query)
            })
            .collect()
    }

    pub fn song(&self, id: &str) -> Option<&Song> {
        self.songs.iter().find(|s| s.id == id)
    }

    pub fn select(&mut self, id: &str, cx: &mut Context<Self>) {
        self.selected = Some(id.to_string());
        self.confirm_delete = None;
        cx.notify();
    }

    pub fn toggle_favorite(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(song) = self.song(id) else { return };
        match self.studio.library.set_favorite(id, !song.favorite) {
            Ok(updated) => {
                if let Some(s) = self.songs.iter_mut().find(|s| s.id == id) {
                    *s = updated;
                }
            }
            Err(e) => self.notify_error(format!("{e:#}"), cx),
        }
        cx.notify();
    }

    pub fn delete(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.confirm_delete.as_deref() != Some(id) {
            self.confirm_delete = Some(id.to_string());
            cx.notify();
            return;
        }
        if self.player.song_id.as_deref() == Some(id) {
            self.player.stop();
        }
        match self.studio.library.trash(id) {
            Ok(_) => {
                self.notify("Moved to the library's .trash folder", cx);
                if self.selected.as_deref() == Some(id) {
                    self.selected = None;
                }
            }
            Err(e) => self.notify_error(format!("{e:#}"), cx),
        }
        self.confirm_delete = None;
        self.reload(cx);
    }

    pub fn reveal(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(song) = self.song(id) {
            let target = song.audio.clone().unwrap_or_else(|| song.dir.clone());
            if let Err(e) = std::process::Command::new("open").arg("-R").arg(target).spawn() {
                self.notify_error(format!("{e}"), cx);
            }
        }
    }

    pub fn export_mp3(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(song) = self.song(id).cloned() else { return };
        self.notify("Encoding MP3…", cx);
        cx.spawn(async move |this, cx| {
            let result = cx.background_spawn(async move { canto_core::audio::export_mp3(&song) }).await;
            this.update(cx, |this, cx| {
                match result {
                    Ok(path) => {
                        let _ = std::process::Command::new("open").arg("-R").arg(&path).spawn();
                        this.notify("MP3 ready", cx);
                    }
                    Err(e) => this.notify_error(format!("MP3 export failed: {e:#}"), cx),
                }
                this.reload(cx);
            })
            .ok();
        })
        .detach();
    }

    pub fn copy_lyrics(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(song) = self.song(id) {
            cx.write_to_clipboard(ClipboardItem::new_string(song.lyrics.clone()));
            self.notify("Lyrics copied", cx);
        }
    }

    // ------------------------------------------------------------ form

    pub fn set_tab(&mut self, tab: Tab, cx: &mut Context<Self>) {
        self.tab = tab;
        self.form_error = None;
        cx.notify();
    }

    pub fn set_refiner(&mut self, kind: RefinerKind, cx: &mut Context<Self>) {
        self.refiner = kind;
        if let Ok(mut config) = Config::load() {
            config.refiner = kind;
            let _ = config.save();
        }
        cx.notify();
    }

    pub fn randomize_seed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let seed = canto_core::random_seed().to_string();
        self.seed.update(cx, |s, cx| s.set_value(seed, window, cx));
    }

    pub fn append_style_tag(&mut self, tag: &str, window: &mut Window, cx: &mut Context<Self>) {
        let current = self.style.read(cx).value().trim().trim_end_matches(',').to_string();
        let next = if current.is_empty() { tag.to_string() } else { format!("{current}, {tag}") };
        self.style.update(cx, |s, cx| s.set_value(next, window, cx));
    }

    /// Fill the Custom form from a song, ready to tweak and regenerate.
    pub fn reuse(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        let Some(song) = self.song(id).cloned() else { return };
        self.fill_form(&song.title, &song.style, &song.lyrics, window, cx);
        self.mode = song.mode;
        self.draft = song.is_draft();
        let seed = song.seed.to_string();
        self.seed.update(cx, |s, cx| s.set_value(seed, window, cx));
        self.prompt_source = song.prompt_source.clone();
        self.refine_notes = None;
        self.tab = Tab::Custom;
        cx.notify();
    }

    fn fill_form(&mut self, title: &str, style: &str, lyrics: &str, window: &mut Window, cx: &mut Context<Self>) {
        let (title, style, lyrics) = (title.to_string(), style.to_string(), lyrics.trim_end().to_string());
        self.title.update(cx, |s, cx| s.set_value(title, window, cx));
        self.style.update(cx, |s, cx| s.set_value(style, window, cx));
        self.lyrics.update(cx, |s, cx| s.set_value(lyrics, window, cx));
    }

    fn form_request(&self, cx: &App) -> Result<SongRequest, SharedString> {
        let seed_text = self.seed.read(cx).value().trim().to_string();
        let seed = if seed_text.is_empty() {
            canto_core::random_seed()
        } else {
            seed_text.parse().map_err(|_| SharedString::from("Seed must be a whole number"))?
        };
        let request = SongRequest {
            title: self.title.read(cx).value().trim().to_string(),
            style: self.style.read(cx).value().trim().to_string(),
            lyrics: self.lyrics.read(cx).value().trim().to_string(),
            mode: self.mode,
            seed,
            steps: if self.draft { DRAFT_STEPS } else { FULL_STEPS },
            max_duration_seconds: None,
        };
        if request.style.is_empty() {
            return Err("Add a style, or use ✦ to write one".into());
        }
        if request.lyrics.is_empty() {
            return Err("Add lyrics, or use ✦ to write them".into());
        }
        Ok(request)
    }

    // ------------------------------------------------------------ refine

    pub fn refine(&mut self, what: Refining, window: &mut Window, cx: &mut Context<Self>) {
        if self.refining.is_some() {
            return;
        }
        let read = |e: &Entity<TextareaState>, cx: &App| e.read(cx).value().to_string();
        let task = match what {
            Refining::Idea { .. } => {
                let idea = read(&self.idea, cx).trim().to_string();
                if idea.is_empty() {
                    self.form_error = Some("Describe the song you want first".into());
                    cx.notify();
                    return;
                }
                self.prompt_source = Some(idea.clone());
                RefineTask::Idea { description: idea }
            }
            Refining::All | Refining::Style | Refining::Lyrics => RefineTask::Polish {
                title: self.title.read(cx).value().to_string(),
                style: read(&self.style, cx),
                lyrics: read(&self.lyrics, cx),
                instruction: Some(self.direction.read(cx).value().to_string()).filter(|s| !s.trim().is_empty()),
                scope: match what {
                    Refining::Style => Scope::Style,
                    Refining::Lyrics => Scope::Lyrics,
                    _ => Scope::All,
                },
            },
        };
        self.refining = Some(what);
        self.form_error = None;
        let studio = self.studio.clone();
        let kind = self.refiner;
        cx.spawn_in(window, async move |this, cx| {
            let result = cx.background_spawn(async move { studio.refine(&task, Some(kind)) }).await;
            this.update_in(cx, |this, window, cx| {
                this.refining = None;
                match result {
                    Ok(brief) => this.apply_brief(brief, what, window, cx),
                    Err(e) => this.form_error = Some(format!("{} couldn't refine: {e:#}", kind.label()).into()),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn apply_brief(&mut self, brief: Brief, what: Refining, window: &mut Window, cx: &mut Context<Self>) {
        self.fill_form(&brief.title, &brief.style, &brief.lyrics, window, cx);
        self.refine_notes = Some(brief.notes.into());
        self.tab = Tab::Custom;
        if let Refining::Idea { then_create: true } = what {
            self.create(window, cx);
        }
    }

    // ------------------------------------------------------------ generate

    /// The big Create button.
    pub fn create(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.tab == Tab::Simple {
            self.refine(Refining::Idea { then_create: true }, window, cx);
            return;
        }
        match self.form_request(cx) {
            Ok(request) => {
                self.form_error = None;
                let source = self.prompt_source.clone();
                self.enqueue(request, source, cx);
            }
            Err(message) => {
                self.form_error = Some(message);
                cx.notify();
            }
        }
    }

    /// Same prompt, new seed.
    pub fn remix(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(song) = self.song(id) else { return };
        let mut request = song.request();
        request.seed = canto_core::random_seed();
        let source = song.prompt_source.clone();
        self.enqueue(request, source, cx);
    }

    pub fn enqueue(&mut self, request: SongRequest, prompt_source: Option<String>, cx: &mut Context<Self>) {
        let id = match self.studio.reserve_id(&request) {
            Ok(id) => id,
            Err(e) => return self.notify_error(format!("{e:#}"), cx),
        };
        let shared = Arc::new(Mutex::new(JobShared::default()));
        let cancel = CancelToken::default();
        let (studio, job_id, job_request, job_shared, job_cancel) =
            (self.studio.clone(), id.clone(), request.clone(), shared.clone(), cancel.clone());
        // A dedicated thread: generation blocks for minutes, and queued jobs
        // wait on the GPU lock rather than occupying the async pool.
        std::thread::spawn(move || {
            let progress_sink = job_shared.clone();
            let mut on_event = move |event: EngineEvent| {
                if let EngineEvent::Progress(p) = event {
                    progress_sink.lock().unwrap().progress = Some(p);
                }
            };
            let result = studio.generate(&job_request, &job_id, prompt_source, &mut on_event, &job_cancel);
            job_shared.lock().unwrap().result = Some(result);
        });
        self.jobs.insert(0, Job { id, request, shared, cancel, started: Instant::now() });
        cx.notify();
    }

    pub fn cancel_job(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(job) = self.jobs.iter().find(|j| j.id == id) {
            job.cancel.cancel();
        }
        cx.notify();
    }

    fn poll_jobs(&mut self, cx: &mut Context<Self>) {
        let mut finished = Vec::new();
        self.jobs.retain(|job| {
            let result = job.shared.lock().unwrap().result.take();
            match result {
                Some(r) => {
                    finished.push((job.request.title.clone(), r));
                    false
                }
                None => true,
            }
        });
        if finished.is_empty() {
            return;
        }
        self.reload(cx);
        for (title, result) in finished {
            match result {
                Ok(song) => {
                    self.notify(format!("“{}” is ready", song.title), cx);
                    if !self.player.is_playing() {
                        self.selected = Some(song.id.clone());
                    }
                }
                Err(e) if e.is::<canto_core::Cancelled>() => self.notify("Generation cancelled", cx),
                Err(e) => {
                    let name = if title.is_empty() { "Song".into() } else { format!("“{title}”") };
                    self.notify_error(format!("{name} failed: {e:#}"), cx)
                }
            }
        }
    }

    // ------------------------------------------------------------ playback

    pub fn play(&mut self, id: &str, cx: &mut Context<Self>) {
        if self.player.song_id.as_deref() == Some(id) && !self.player.finished() {
            self.player.toggle();
            cx.notify();
            return;
        }
        let Some(song) = self.song(id).cloned() else { return };
        let Some(audio) = song.audio.clone() else { return };
        match self.player.load(&song.id, &audio, song.duration_secs.unwrap_or(0.0) as f32) {
            Ok(()) => self.ensure_waveform(&song, cx),
            Err(e) => self.notify_error(format!("Couldn't play: {e:#}"), cx),
        }
        cx.notify();
    }

    pub fn toggle_playback(&mut self, cx: &mut Context<Self>) {
        match self.player.song_id.clone() {
            Some(id) => self.play(&id, cx),
            None => {
                if let Some(id) = self.selected.clone() {
                    self.play(&id, cx);
                }
            }
        }
    }

    pub fn seek(&mut self, fraction: f32, cx: &mut Context<Self>) {
        self.player.seek(fraction);
        cx.notify();
    }

    pub fn set_volume(&mut self, volume: f32, cx: &mut Context<Self>) {
        self.player.set_volume(volume);
        cx.notify();
    }

    pub fn ensure_waveform(&mut self, song: &Song, cx: &mut Context<Self>) {
        if self.waveforms.contains_key(&song.id) || song.audio.is_none() {
            return;
        }
        self.waveforms.insert(song.id.clone(), Rc::new(Vec::new())); // mark in flight
        let song = song.clone();
        cx.spawn(async move |this, cx| {
            let id = song.id.clone();
            let peaks = cx.background_spawn(async move { canto_core::audio::waveform(&song, 180) }).await;
            this.update(cx, |this, cx| {
                if let Ok(peaks) = peaks {
                    this.waveforms.insert(id, Rc::new(peaks));
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    // ------------------------------------------------------------ toasts & ticking

    pub fn notify(&mut self, message: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.toast = Some(Toast { message: message.into(), is_error: false, shown: Instant::now() });
        cx.notify();
    }

    pub fn notify_error(&mut self, message: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.toast = Some(Toast { message: message.into(), is_error: true, shown: Instant::now() });
        cx.notify();
    }

    pub fn dismiss_toast(&mut self, cx: &mut Context<Self>) {
        self.toast = None;
        cx.notify();
    }

    /// One timer drives progress bars, the playhead, and toast expiry.
    fn start_ticker(&self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_millis(120)).await;
                let alive = this
                    .update(cx, |this, cx| {
                        this.poll_jobs(cx);
                        if let Some(t) = &this.toast {
                            let ttl = if t.is_error { 12 } else { 4 };
                            if t.shown.elapsed() > Duration::from_secs(ttl) {
                                this.toast = None;
                                cx.notify();
                            }
                        }
                        if !this.jobs.is_empty() || this.player.is_playing() || this.refining.is_some() {
                            cx.notify();
                        }
                    })
                    .is_ok();
                if !alive {
                    break;
                }
            }
        })
        .detach();
    }
}
