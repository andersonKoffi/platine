//! A playing session: engine + decoders + analysis + auto-mix, on a
//! dedicated controller thread. The FRB API and the CLI example drive it
//! through [`SessionHandle`].

mod analysis_worker;
mod controller;
mod protocol;

use std::thread::{self, JoinHandle};
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender, select, unbounded};
use dj_domain::DeckId;
use dj_domain::ports::{AudioOutput, OutputError, RandomSource, RenderCallback};
use dj_engine::{Engine, EngineConfig};
use dj_infra::{DeckLoader, SilenceAnalyzer, SymphoniaDecoder};

pub use controller::{Controller, SessionSettings};

use controller::ControllerParts;
pub use protocol::{LoadMode, ScanSummary, SessionCommand, SessionEvent};

use analysis_worker::AnalysisWorker;

/// How often the controller thread polls the engine when nothing else happens.
const ENGINE_POLL: Duration = Duration::from_millis(5);

/// A controller ready to run, plus the render callback for the audio output
/// and the stream of UI events.
pub struct SessionParts<R: RandomSource> {
    pub controller: Controller<R>,
    pub render: RenderCallback,
    pub events: Receiver<SessionEvent>,
}

impl<R: RandomSource> std::fmt::Debug for SessionParts<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionParts")
            .field("controller", &self.controller)
            .finish_non_exhaustive()
    }
}

/// Builds every part of a session for an output running at `sample_rate`
/// with `channels` channels.
///
/// # Errors
/// Returns an error if a worker thread cannot be spawned.
pub fn build<R: RandomSource>(
    sample_rate: u32,
    channels: u16,
    settings: SessionSettings,
    rng: R,
) -> std::io::Result<SessionParts<R>> {
    build_with_config(EngineConfig::new(sample_rate), channels, settings, rng)
}

/// Like [`build`] with an explicit engine configuration (tests use larger rings).
///
/// # Errors
/// Returns an error if a worker thread cannot be spawned.
pub fn build_with_config<R: RandomSource>(
    config: EngineConfig,
    channels: u16,
    settings: SessionSettings,
    rng: R,
) -> std::io::Result<SessionParts<R>> {
    let (engine, handle, [feed_a, feed_b]) = Engine::new(config);
    let (loader_tx, loader_events) = unbounded();
    let rate = config.sample_rate;
    let loaders = [
        DeckLoader::spawn(DeckId::A, SymphoniaDecoder, feed_a, rate, loader_tx.clone())?,
        DeckLoader::spawn(DeckId::B, SymphoniaDecoder, feed_b, rate, loader_tx)?,
    ];
    let (analysis, analysis_events) =
        AnalysisWorker::spawn(SilenceAnalyzer::new(SymphoniaDecoder))?;
    let (events_tx, events) = unbounded();
    let controller = Controller::new(
        ControllerParts {
            engine: handle,
            loaders,
            loader_events,
            analysis,
            analysis_events,
            rng,
            events: events_tx,
        },
        settings,
    );
    Ok(SessionParts {
        controller,
        render: engine.into_render_callback(usize::from(channels)),
        events,
    })
}

/// A session running on its own thread, playing through an [`AudioOutput`].
pub struct SessionHandle {
    commands: Sender<SessionCommand>,
    events: Receiver<SessionEvent>,
    thread: Option<JoinHandle<()>>,
    output: Box<dyn AudioOutput + Send>,
}

impl std::fmt::Debug for SessionHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionHandle").finish_non_exhaustive()
    }
}

/// Error starting a session.
#[derive(Debug)]
pub enum StartError {
    Output(OutputError),
    Thread(std::io::Error),
}

impl std::fmt::Display for StartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Output(err) => write!(f, "audio output: {err}"),
            Self::Thread(err) => write!(f, "cannot start session thread: {err}"),
        }
    }
}

impl std::error::Error for StartError {}

impl SessionHandle {
    /// Starts a session on `output`.
    ///
    /// # Errors
    /// Returns an error if the output cannot start or a thread cannot be spawned.
    pub fn start<R: RandomSource + Send + 'static>(
        mut output: Box<dyn AudioOutput + Send>,
        settings: SessionSettings,
        rng: R,
    ) -> Result<Self, StartError> {
        let spec = output.spec();
        let parts =
            build(spec.sample_rate, spec.channels, settings, rng).map_err(StartError::Thread)?;
        output.start(parts.render).map_err(StartError::Output)?;
        let (commands, inbox) = unbounded();
        let controller = parts.controller;
        let thread = thread::Builder::new()
            .name("platine-session".into())
            .spawn(move || run(controller, &inbox))
            .map_err(StartError::Thread)?;
        Ok(Self {
            commands,
            events: parts.events,
            thread: Some(thread),
            output,
        })
    }

    /// Sends a use case to the session.
    pub fn send(&self, command: SessionCommand) {
        let _ = self.commands.send(command);
    }

    /// Stream of UI events.
    pub fn events(&self) -> &Receiver<SessionEvent> {
        &self.events
    }
}

impl Drop for SessionHandle {
    fn drop(&mut self) {
        let _ = self.commands.send(SessionCommand::Shutdown);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
        self.output.stop();
    }
}

/// Controller thread: reacts to commands and worker reports, polls the engine.
fn run<R: RandomSource>(mut controller: Controller<R>, inbox: &Receiver<SessionCommand>) {
    loop {
        let loader_events = controller.loader_events().clone();
        let analysis_events = controller.analysis_events().clone();
        select! {
            recv(inbox) -> command => match command {
                Ok(command) => {
                    if !controller.handle(command) {
                        return;
                    }
                }
                Err(_) => return,
            },
            recv(loader_events) -> event => {
                if let Ok(event) = event {
                    controller.on_loader_event(event);
                }
            },
            recv(analysis_events) -> outcome => {
                if let Ok(outcome) = outcome {
                    controller.on_analysis_outcome(outcome);
                }
            },
            default(ENGINE_POLL) => {}
        }
        controller.poll();
    }
}
