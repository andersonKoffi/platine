//! Background thread running track analysis in request order.

use std::path::PathBuf;
use std::thread::{self, JoinHandle};

use crossbeam_channel::{Receiver, Sender, unbounded};
use dj_domain::ports::{AnalysisRequest, DecodeError, TrackAnalysis, TrackAnalyzer};
use dj_domain::{DeckId, TrackId};

/// Work item: analyse `path` for the load `generation` of `deck`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AnalysisJob {
    pub(crate) deck: DeckId,
    pub(crate) generation: u32,
    pub(crate) track: TrackId,
    pub(crate) path: PathBuf,
    pub(crate) request: AnalysisRequest,
}

/// Outcome of a job.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AnalysisOutcome {
    pub(crate) job: AnalysisJob,
    pub(crate) result: Result<TrackAnalysis, DecodeError>,
}

/// Handle on the analysis thread; dropping it stops the thread.
#[derive(Debug)]
pub(crate) struct AnalysisWorker {
    jobs: Option<Sender<AnalysisJob>>,
    thread: Option<JoinHandle<()>>,
}

impl AnalysisWorker {
    pub(crate) fn spawn<A>(analyzer: A) -> std::io::Result<(Self, Receiver<AnalysisOutcome>)>
    where
        A: TrackAnalyzer + Send + 'static,
    {
        let (jobs, inbox) = unbounded::<AnalysisJob>();
        let (results, outcomes) = unbounded();
        let thread = thread::Builder::new()
            .name("platine-analysis".into())
            .spawn(move || {
                for job in inbox {
                    let result = analyzer.analyze(&job.path, &job.request);
                    if results.send(AnalysisOutcome { job, result }).is_err() {
                        return;
                    }
                }
            })?;
        Ok((
            Self {
                jobs: Some(jobs),
                thread: Some(thread),
            },
            outcomes,
        ))
    }

    pub(crate) fn submit(&self, job: AnalysisJob) {
        if let Some(jobs) = &self.jobs {
            let _ = jobs.send(job);
        }
    }
}

impl Drop for AnalysisWorker {
    fn drop(&mut self) {
        self.jobs = None; // closes the channel: the thread finishes its loop
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
