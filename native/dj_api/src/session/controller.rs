//! The session controller: glue between the auto-mix scheduler (domain), the
//! audio engine and the decoding/analysis threads. Runs on its own thread
//! (never the audio thread).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender};
use dj_domain::ports::{AnalysisRequest, LibraryScanner, RandomSource};
use dj_domain::{
    AutoMixAction, AutoMixInput, AutoMixScheduler, AutoMixSettings, AutoMixState, AutoMixStatus,
    DeckId, PlayOrder, RepeatMode, Seconds, Track, TrackId, TrackTiming, TransitionPlan,
};
use dj_engine::{EngineCommand, EngineEvent, EngineHandle, EngineTransition};
use dj_infra::{DeckLoader, FolderScanner, LoaderEvent};

use super::analysis_worker::{AnalysisJob, AnalysisOutcome, AnalysisWorker};
use super::protocol::{LoadMode, ScanSummary, SessionCommand, SessionEvent};

/// Initial settings of a session.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionSettings {
    pub automix: AutoMixSettings,
    pub order: PlayOrder,
    pub repeat: RepeatMode,
    pub silence_threshold_db: f32,
    /// Minimum delay between two position/level events sent to the UI.
    pub ui_interval: Duration,
    /// Where analyses are cached between sessions (`None`: no cache).
    pub analysis_cache: Option<PathBuf>,
}

impl Default for SessionSettings {
    fn default() -> Self {
        Self {
            automix: AutoMixSettings::default(),
            order: PlayOrder::Sequential,
            repeat: RepeatMode::Off,
            silence_threshold_db: AnalysisRequest::default().silence_threshold_db,
            ui_interval: Duration::from_millis(33),
            analysis_cache: None,
        }
    }
}

/// Where a deck stands from the controller's point of view.
#[derive(Debug, Clone, Copy, Default)]
struct DeckSlot {
    track: Option<TrackId>,
    /// Load/seek generation shared by the loader and the engine.
    generation: u32,
    timing: Option<TrackTiming>,
    /// The first `Ready` of the current track was forwarded to the scheduler.
    ready_reported: bool,
    /// The loader has queued the whole track (from the cue point).
    fully_buffered: bool,
    /// Position the engine is cued at, before playing.
    cued_at: Seconds,
    playing: bool,
    last_ui_update: Option<Instant>,
}

/// Wiring of one session. See the module documentation.
pub struct Controller<R: RandomSource> {
    scheduler: AutoMixScheduler<R>,
    engine: EngineHandle,
    loaders: [DeckLoader; 2],
    loader_events: Receiver<LoaderEvent>,
    analysis: AnalysisWorker,
    analysis_events: Receiver<AnalysisOutcome>,
    library: HashMap<TrackId, Track>,
    next_track_id: u64,
    decks: [DeckSlot; 2],
    analysis_request: AnalysisRequest,
    events: Sender<SessionEvent>,
    ui_interval: Duration,
    last_levels: Option<Instant>,
    published_state: Option<(AutoMixState, AutoMixStatus)>,
    published_queue: Vec<TrackId>,
    last_failure: HashMap<TrackId, String>,
}

impl<R: RandomSource> std::fmt::Debug for Controller<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Controller")
            .field("state", &self.scheduler.state())
            .field("library_len", &self.library.len())
            .finish_non_exhaustive()
    }
}

/// Everything a controller needs, already built.
#[derive(Debug)]
pub(crate) struct ControllerParts<R: RandomSource> {
    pub(crate) engine: EngineHandle,
    pub(crate) loaders: [DeckLoader; 2],
    pub(crate) loader_events: Receiver<LoaderEvent>,
    pub(crate) analysis: AnalysisWorker,
    pub(crate) analysis_events: Receiver<AnalysisOutcome>,
    pub(crate) rng: R,
    pub(crate) events: Sender<SessionEvent>,
}

impl<R: RandomSource> Controller<R> {
    pub(crate) fn new(parts: ControllerParts<R>, settings: &SessionSettings) -> Self {
        Self {
            scheduler: AutoMixScheduler::new(
                settings.automix,
                settings.order,
                settings.repeat,
                parts.rng,
            ),
            engine: parts.engine,
            loaders: parts.loaders,
            loader_events: parts.loader_events,
            analysis: parts.analysis,
            analysis_events: parts.analysis_events,
            library: HashMap::new(),
            next_track_id: 1,
            decks: [DeckSlot::default(); 2],
            analysis_request: AnalysisRequest {
                silence_threshold_db: settings.silence_threshold_db,
                detect_bpm: true,
            },
            events: parts.events,
            ui_interval: settings.ui_interval,
            last_levels: None,
            published_state: None,
            published_queue: Vec::new(),
            last_failure: HashMap::new(),
        }
    }

    pub fn state(&self) -> AutoMixState {
        self.scheduler.state()
    }

    pub fn status(&self) -> AutoMixStatus {
        self.scheduler.status()
    }

    pub fn deck_track(&self, deck: DeckId) -> Option<TrackId> {
        self.decks[deck.index()].track
    }

    /// `true` when no deck is waiting for analysis or decoding.
    pub fn is_settled(&self) -> bool {
        self.decks
            .iter()
            .all(|slot| slot.track.is_none() || slot.ready_reported)
    }

    /// `true` when every loaded deck has its whole track buffered. Offline
    /// tests use it to render faster than real time without starving decks.
    pub fn is_fully_buffered(&self) -> bool {
        self.is_settled()
            && self
                .decks
                .iter()
                .all(|slot| slot.track.is_none() || slot.fully_buffered)
    }

    pub fn track(&self, id: TrackId) -> Option<&Track> {
        self.library.get(&id)
    }

    pub(crate) fn loader_events(&self) -> &Receiver<LoaderEvent> {
        &self.loader_events
    }

    pub(crate) fn analysis_events(&self) -> &Receiver<AnalysisOutcome> {
        &self.analysis_events
    }

    /// Processes every pending loader, analysis and engine event.
    pub fn poll(&mut self) {
        while let Ok(outcome) = self.analysis_events.try_recv() {
            self.on_analysis(outcome);
        }
        while let Ok(event) = self.loader_events.try_recv() {
            self.on_loader(event);
        }
        while let Some(event) = self.engine.try_recv() {
            self.on_engine(event);
        }
    }

    pub(crate) fn on_analysis_outcome(&mut self, outcome: AnalysisOutcome) {
        self.on_analysis(outcome);
    }

    pub(crate) fn on_loader_event(&mut self, event: LoaderEvent) {
        self.on_loader(event);
    }

    /// Executes a use case. Returns `false` for [`SessionCommand::Shutdown`].
    pub fn handle(&mut self, command: SessionCommand) -> bool {
        match command {
            SessionCommand::LoadFolder { path, mode, reply } => {
                let _ = reply.send(self.load_folder(&path, mode));
            }
            SessionCommand::SetAutoMix(enabled) => self.feed(AutoMixInput::SetEnabled(enabled)),
            SessionCommand::SetCrossfadeDuration(duration) => {
                self.feed(AutoMixInput::SetCrossfadeDuration(duration));
            }
            SessionCommand::SetCrossfadeCurve(curve) => {
                self.send_engine(EngineCommand::SetCrossfaderCurve(curve));
                self.feed(AutoMixInput::SetCurve(curve));
            }
            SessionCommand::SetSilenceThreshold(db) => {
                self.analysis_request.silence_threshold_db = db.clamp(-90.0, -20.0);
            }
            SessionCommand::SetShuffle(on) => {
                let order = if on {
                    PlayOrder::Shuffle
                } else {
                    PlayOrder::Sequential
                };
                self.scheduler.set_order(order);
                self.publish();
            }
            SessionCommand::SetRepeat(on) => {
                let repeat = if on { RepeatMode::All } else { RepeatMode::Off };
                let actions = self.scheduler.set_repeat(repeat);
                self.execute(&actions);
            }
            SessionCommand::ResumeAutoMix => self.feed(AutoMixInput::Resume),
            SessionCommand::TogglePlay(deck) => self.toggle_play(deck),
            SessionCommand::Seek { deck, position } => self.seek(deck, position),
            SessionCommand::SetCrossfader(position) => {
                self.feed(AutoMixInput::ManualIntervention);
                self.send_engine(EngineCommand::SetCrossfader(position));
            }
            SessionCommand::SetDeckGain { deck, gain } => {
                self.send_engine(EngineCommand::SetDeckGain { deck, gain });
            }
            SessionCommand::SetEq { deck, band, gain } => {
                self.send_engine(EngineCommand::SetEq { deck, band, gain });
            }
            SessionCommand::SetMasterGain(gain) => {
                self.send_engine(EngineCommand::SetMasterGain(gain));
            }
            SessionCommand::MoveInQueue { from, to } => {
                if self.scheduler.move_upcoming(from, to).is_ok() {
                    self.publish();
                }
            }
            SessionCommand::LoadTrack { deck, track } => {
                if self.library.contains_key(&track) {
                    self.feed(AutoMixInput::ManualLoad { deck, track });
                }
            }
            SessionCommand::Shutdown => return false,
        }
        true
    }

    // ----- use cases ---------------------------------------------------------

    fn load_folder(&mut self, path: &Path, mode: LoadMode) -> Result<ScanSummary, String> {
        let report = FolderScanner::starting_at(self.next_track_id)
            .scan(path)
            .map_err(|err| err.to_string())?;
        self.next_track_id += report.tracks.len() as u64;
        let ids: Vec<TrackId> = report.tracks.iter().map(|track| track.id).collect();
        for track in &report.tracks {
            self.library.insert(track.id, track.clone());
        }
        let mut tracks: Vec<Track> = self.library.values().cloned().collect();
        tracks.sort_by_key(|track| track.id);
        self.emit(SessionEvent::LibraryChanged { tracks });
        let actions = match mode {
            LoadMode::Replace => self.scheduler.load_library(ids),
            LoadMode::Append => self.scheduler.append_to_library(&ids),
        };
        self.execute(&actions);
        Ok(ScanSummary {
            tracks: report.tracks,
            skipped: report.skipped,
        })
    }

    fn toggle_play(&mut self, deck: DeckId) {
        let slot = self.decks[deck.index()];
        if slot.timing.is_none() {
            return;
        }
        // Pausing or resuming the deck on air leaves the automation in charge
        // (a paused deck simply delays the next transition). Starting the
        // other deck by hand means the user is mixing: suspend.
        let on_air = match self.scheduler.state() {
            AutoMixState::Playing { active } => Some(active),
            AutoMixState::Crossfading { to, .. } => Some(to),
            AutoMixState::Idle | AutoMixState::WaitingForNext { .. } | AutoMixState::Finished => {
                None
            }
        };
        if !slot.playing && on_air.is_some_and(|active| active != deck) {
            self.feed(AutoMixInput::ManualIntervention);
        }
        if slot.playing {
            self.send_engine(EngineCommand::Pause { deck });
            self.decks[deck.index()].playing = false;
        } else {
            self.send_engine(EngineCommand::Play { deck });
            self.decks[deck.index()].playing = true;
        }
    }

    fn seek(&mut self, deck: DeckId, position: Seconds) {
        if self.decks[deck.index()].timing.is_none() {
            return;
        }
        // Seeking keeps the plan valid: the engine starts the transition
        // when the outgoing deck reaches it, or at once if already past it.
        self.cue(deck, position);
    }

    // ----- scheduler actions ---------------------------------------------------

    fn feed(&mut self, input: AutoMixInput) {
        let actions = self.scheduler.handle(input);
        self.execute(&actions);
    }

    fn execute(&mut self, actions: &[AutoMixAction]) {
        for action in actions {
            match *action {
                AutoMixAction::LoadTrack { deck, track } => self.start_load(deck, track),
                AutoMixAction::Play { deck, from } => {
                    if let Some(from) = from
                        && from != self.decks[deck.index()].cued_at
                    {
                        self.cue(deck, from);
                    }
                    self.send_engine(EngineCommand::Play { deck });
                    self.decks[deck.index()].playing = true;
                }
                AutoMixAction::Stop { deck } => {
                    self.send_engine(EngineCommand::Eject { deck });
                    self.loaders[deck.index()].unload();
                    let generation = self.decks[deck.index()].generation;
                    self.decks[deck.index()] = DeckSlot {
                        generation,
                        ..DeckSlot::default()
                    };
                }
                AutoMixAction::SetCrossfader(position) => {
                    self.send_engine(EngineCommand::SetCrossfader(position));
                }
                AutoMixAction::ScheduleTransition(plan) => self.schedule(&plan),
                AutoMixAction::CancelTransition => {
                    self.send_engine(EngineCommand::CancelTransition);
                }
                AutoMixAction::SkippedUnplayable { track } => {
                    let reason = self
                        .last_failure
                        .remove(&track)
                        .unwrap_or_else(|| "unreadable file".into());
                    self.emit(SessionEvent::TrackSkipped { track, reason });
                }
                AutoMixAction::QueueExhausted => self.emit(SessionEvent::QueueExhausted),
            }
        }
        self.publish();
    }

    fn schedule(&mut self, plan: &TransitionPlan) {
        // Decks are cued at their effective start; re-cue if the plan differs.
        if plan.incoming_start != self.decks[plan.to.index()].cued_at {
            self.cue(plan.to, plan.incoming_start);
        }
        let transition = EngineTransition {
            from: plan.from,
            to: plan.to,
            start_at_frame: self.engine.frames(plan.start_at),
            fade_frames: self.engine.frames(plan.fade),
            curve: plan.curve,
        };
        self.send_engine(EngineCommand::ScheduleTransition(transition));
    }

    /// Requests analysis then decoding of `track` on `deck`.
    fn start_load(&mut self, deck: DeckId, track: TrackId) {
        let Some(path) = self.library.get(&track).map(|t| t.path.clone()) else {
            return;
        };
        let slot = &mut self.decks[deck.index()];
        slot.generation = slot.generation.wrapping_add(1);
        slot.track = Some(track);
        slot.timing = None;
        slot.ready_reported = false;
        slot.fully_buffered = false;
        slot.playing = false;
        let generation = slot.generation;
        self.send_engine(EngineCommand::Eject { deck });
        self.analysis.submit(AnalysisJob {
            deck,
            generation,
            track,
            path,
            request: self.analysis_request,
        });
        self.emit(SessionEvent::DeckLoading { deck, track });
    }

    /// Moves `deck` to `position` under a new generation.
    fn cue(&mut self, deck: DeckId, position: Seconds) {
        let slot = &mut self.decks[deck.index()];
        slot.generation = slot.generation.wrapping_add(1);
        slot.cued_at = position;
        slot.fully_buffered = false;
        let generation = slot.generation;
        self.loaders[deck.index()].seek(generation, position);
        let position_frame = self.engine.frames(position);
        self.send_engine(EngineCommand::Cue {
            deck,
            generation,
            position_frame,
        });
    }

    // ----- worker and engine reports ------------------------------------------

    fn on_analysis(&mut self, outcome: AnalysisOutcome) {
        let job = outcome.job;
        let slot = self.decks[job.deck.index()];
        if slot.generation != job.generation || slot.track != Some(job.track) {
            return; // superseded by another load
        }
        match outcome.result {
            Ok(analysis) => {
                self.emit(SessionEvent::Waveform {
                    deck: job.deck,
                    track: job.track,
                    waveform: analysis.waveform,
                });
                if let Some(track) = self.library.get_mut(&job.track) {
                    track.bpm = analysis.beat_grid.map(|grid| grid.bpm);
                }
                self.emit(SessionEvent::TrackAnalyzed {
                    track: job.track,
                    beat_grid: analysis.beat_grid,
                });
                let start = analysis.timing.effective_start();
                let slot = &mut self.decks[job.deck.index()];
                slot.timing = Some(analysis.timing);
                slot.cued_at = start;
                self.loaders[job.deck.index()].load(job.path, job.generation, start);
                let position_frame = self.engine.frames(start);
                self.send_engine(EngineCommand::Cue {
                    deck: job.deck,
                    generation: job.generation,
                    position_frame,
                });
            }
            Err(error) => self.fail_load(job.deck, job.track, error.to_string()),
        }
    }

    fn on_loader(&mut self, event: LoaderEvent) {
        match event {
            LoaderEvent::Ready {
                deck, generation, ..
            } => {
                let slot = self.decks[deck.index()];
                let (Some(track), Some(timing)) = (slot.track, slot.timing) else {
                    return;
                };
                if slot.generation != generation || slot.ready_reported {
                    return;
                }
                self.decks[deck.index()].ready_reported = true;
                self.emit(SessionEvent::DeckLoaded {
                    deck,
                    track,
                    timing,
                });
                self.feed(AutoMixInput::TrackLoaded {
                    deck,
                    track,
                    timing,
                });
            }
            LoaderEvent::Failed {
                deck,
                generation,
                error,
            } => {
                let slot = self.decks[deck.index()];
                if slot.generation == generation
                    && let Some(track) = slot.track
                {
                    self.fail_load(deck, track, error.to_string());
                }
            }
            LoaderEvent::Finished { deck, generation } => {
                let slot = &mut self.decks[deck.index()];
                if slot.generation == generation {
                    slot.fully_buffered = true;
                }
            }
        }
    }

    fn fail_load(&mut self, deck: DeckId, track: TrackId, reason: String) {
        self.last_failure.insert(track, reason.clone());
        let actions = self
            .scheduler
            .handle(AutoMixInput::TrackLoadFailed { deck, track });
        if actions.is_empty() {
            // Manual load: the scheduler was not waiting for it.
            self.last_failure.remove(&track);
            self.emit(SessionEvent::TrackSkipped { track, reason });
        }
        self.execute(&actions);
    }

    fn on_engine(&mut self, event: EngineEvent) {
        match event {
            EngineEvent::Position {
                deck,
                generation,
                frame,
                playing,
            } => {
                let interval = self.ui_interval;
                let position = self.engine.seconds(frame);
                let slot = &mut self.decks[deck.index()];
                if generation != slot.generation || slot.track.is_none() {
                    return;
                }
                slot.playing = playing;
                let due = slot
                    .last_ui_update
                    .is_none_or(|last| last.elapsed() >= interval);
                if due {
                    slot.last_ui_update = Some(Instant::now());
                    self.emit(SessionEvent::DeckPosition {
                        deck,
                        position,
                        playing,
                    });
                }
            }
            EngineEvent::Levels(levels) => {
                let due = self
                    .last_levels
                    .is_none_or(|last| last.elapsed() >= self.ui_interval);
                if due {
                    self.last_levels = Some(Instant::now());
                    self.emit(SessionEvent::Levels(levels));
                }
            }
            EngineEvent::TrackEnded { deck, generation } => {
                if generation == self.decks[deck.index()].generation {
                    self.decks[deck.index()].playing = false;
                    self.feed(AutoMixInput::TrackEnded { deck });
                }
            }
            EngineEvent::TransitionStarted { from, to, .. } => {
                self.decks[to.index()].playing = true;
                self.emit(SessionEvent::TransitionStarted { from, to });
                self.feed(AutoMixInput::TransitionStarted { from, to });
            }
            EngineEvent::TransitionCompleted { from, to, .. } => {
                self.emit(SessionEvent::TransitionCompleted { from, to });
                self.feed(AutoMixInput::TransitionCompleted { from, to });
            }
            EngineEvent::Underrun { deck } => self.emit(SessionEvent::Underrun { deck }),
        }
    }

    // ----- output ----------------------------------------------------------------

    fn send_engine(&mut self, command: EngineCommand) {
        // The queue only fills up if the audio thread is stalled; dropping a
        // command is then the least harmful option.
        let _ = self.engine.send(command);
    }

    /// Publishes the auto-mix state and the queue when they changed.
    fn publish(&mut self) {
        let state = (self.scheduler.state(), self.scheduler.status());
        if self.published_state != Some(state) {
            self.published_state = Some(state);
            self.emit(SessionEvent::AutoMixChanged {
                state: state.0,
                status: state.1,
            });
        }
        let upcoming: Vec<TrackId> = self.scheduler.queue().upcoming().collect();
        if upcoming != self.published_queue {
            self.published_queue.clone_from(&upcoming);
            self.emit(SessionEvent::QueueChanged { upcoming });
        }
    }

    fn emit(&self, event: SessionEvent) {
        let _ = self.events.send(event);
    }
}
