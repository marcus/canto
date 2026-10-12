//! The one entry point every surface uses: generate, refine, and manage songs.

use std::fs::{File, TryLockError};
use std::time::{Duration, Instant};

use anyhow::{Result, bail};

use crate::config::Config;
use crate::engine::{CancelToken, Cancelled, Engine, EngineEvent, MlxYue, Progress, Stage};
use crate::library::{Library, now};
use crate::refine::{Brief, RefineTask, Refiner, RefinerKind};
use crate::song::{Song, SongMeta, SongRequest, SongStatus};

pub struct Studio {
    pub config: Config,
    pub library: Library,
    engine: Box<dyn Engine>,
}

impl Studio {
    pub fn new(config: Config) -> Self {
        let engine = Box::new(MlxYue::new(config.clone()));
        Self::with_engine(config, engine)
    }

    pub fn with_engine(config: Config, engine: Box<dyn Engine>) -> Self {
        Self { library: Library::new(config.library_dir.clone()), config, engine }
    }

    pub fn open() -> Result<Self> {
        Ok(Self::new(Config::load()?))
    }

    pub fn refiner(&self, kind: Option<RefinerKind>) -> Box<dyn Refiner> {
        kind.unwrap_or(self.config.refiner).build(self.config.refiner_model.clone())
    }

    pub fn refine(&self, task: &RefineTask, kind: Option<RefinerKind>) -> Result<Brief> {
        self.refiner(kind).refine(task)
    }

    /// Pick the song's directory name up front so callers can track it.
    pub fn reserve_id(&self, request: &SongRequest) -> String {
        let basis = if request.title.trim().is_empty() {
            request.style.split(',').next().unwrap_or("song")
        } else {
            &request.title
        };
        self.library.new_id(basis)
    }

    /// Generate one song. Blocks until it finishes, fails, or is cancelled.
    ///
    /// Runs one at a time across every Canto process: a second caller waits
    /// on a lock in the library directory, reporting [`Stage::Waiting`].
    pub fn generate(
        &self,
        request: &SongRequest,
        id: &str,
        prompt_source: Option<String>,
        on_event: &mut dyn FnMut(EngineEvent),
        cancel: &CancelToken,
    ) -> Result<Song> {
        request.validate()?;
        let created = now();
        let _gpu = self.acquire_gpu(on_event, cancel)?;
        let started = Instant::now();
        let out_dir = self.library.dir().join(id);
        let outcome = self.engine.generate(request, id, &out_dir, on_event, cancel);

        let (status, error) = match &outcome {
            Ok(()) => (SongStatus::Complete, None),
            Err(e) if e.is::<Cancelled>() => (SongStatus::Cancelled, None),
            Err(e) => (SongStatus::Failed, Some(format!("{e:#}"))),
        };
        let meta = SongMeta {
            title: if request.title.trim().is_empty() { String::new() } else { request.title.trim().to_string() },
            created,
            favorite: false,
            steps: Some(request.steps),
            prompt_source,
            status: Some(status),
            error,
            seconds_to_generate: Some(started.elapsed().as_secs_f64()),
        };
        self.library.write_meta(id, &meta)?;
        if status == SongStatus::Cancelled && !out_dir.join("audio.flac").exists() {
            let _ = self.library.trash(id);
        }
        outcome?;
        on_event(EngineEvent::Progress(Progress { stage: Stage::Done, fraction: 1.0, detail: String::new() }));
        self.library.get(id)
    }

    fn acquire_gpu(&self, on_event: &mut dyn FnMut(EngineEvent), cancel: &CancelToken) -> Result<File> {
        std::fs::create_dir_all(self.library.dir())?;
        let file = File::create(self.library.dir().join(".canto.lock"))?;
        let mut announced = false;
        loop {
            match file.try_lock() {
                Ok(()) => return Ok(file),
                Err(TryLockError::WouldBlock) => {
                    if !announced {
                        on_event(EngineEvent::Progress(Progress {
                            stage: Stage::Waiting,
                            fraction: 0.0,
                            detail: "another song is generating".into(),
                        }));
                        announced = true;
                    }
                    if cancel.is_cancelled() {
                        bail!(Cancelled);
                    }
                    std::thread::sleep(Duration::from_millis(500));
                }
                Err(TryLockError::Error(e)) => return Err(e.into()),
            }
        }
    }
}
