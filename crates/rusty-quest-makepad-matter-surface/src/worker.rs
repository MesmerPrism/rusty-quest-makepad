//! Nonblocking latest-wins worker for Quest Makepad Matter surface frames.
//!
//! The worker owns no simulation truth. It runs the existing native Matter
//! adapter runtime on a background thread and publishes the latest completed
//! adapter frame for a Makepad/OpenXR render loop to consume without waiting.

use std::{
    fmt,
    sync::{Arc, Condvar, Mutex},
    thread::{self, JoinHandle},
    time::Instant,
};

use rusty_quest_makepad_mesh_replay::MeshReplayRuntime;

use crate::{
    MatterSurfaceContactProbe, QuestMakepadMatterSurfaceConfig, QuestMakepadMatterSurfaceError,
    QuestMakepadMatterSurfaceFrame, QuestMakepadMatterSurfaceRuntime,
    QuestMakepadMatterSurfaceSourceFrame,
};

/// Quest Makepad Matter surface worker marker schema.
pub const QUEST_MAKEPAD_MATTER_SURFACE_WORKER_SCHEMA_ID: &str =
    "rusty.quest.makepad.matter_surface_worker.v1";
/// Quest Makepad Matter surface worker marker prefix.
pub const QUEST_MAKEPAD_MATTER_SURFACE_WORKER_MARKER_PREFIX: &str =
    "RUSTY_QUEST_MAKEPAD_MATTER_SURFACE_WORKER";

/// Snapshot of worker counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QuestMakepadMatterSurfaceWorkerStats {
    /// Number of requests accepted by the worker handle.
    pub submitted_count: u64,
    /// Number of requests that completed successfully.
    pub completed_count: u64,
    /// Number of requests that failed in replay, Matter, or Optics.
    pub failed_count: u64,
    /// Number of pending requests overwritten before the worker consumed them.
    pub dropped_pending_count: u64,
    /// Latest request sequence submitted by the app thread.
    pub latest_submitted_sequence: u64,
    /// Latest request sequence completed or failed by the worker thread.
    pub latest_finished_sequence: u64,
    /// Whether a request is currently waiting in the mailbox.
    pub pending_request: bool,
    /// Whether the worker thread is currently processing a request.
    pub in_flight: bool,
}

/// Completed Matter surface worker frame.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadMatterSurfaceWorkerFrame {
    /// Request sequence that produced this frame.
    pub sequence: u64,
    /// Worker counters captured after publishing the frame.
    pub stats: QuestMakepadMatterSurfaceWorkerStats,
    /// Wall-clock latency from submit to frame publication.
    pub latency_ms: f32,
    /// Adapter frame produced by the native Matter runtime.
    pub frame: QuestMakepadMatterSurfaceFrame,
    /// Matter runtime evidence marker produced on the worker thread.
    pub runtime_marker_line: String,
}

impl QuestMakepadMatterSurfaceWorkerFrame {
    /// Builds a worker evidence marker for this completed frame.
    #[must_use]
    pub fn marker_line(&self, phase: &str) -> String {
        worker_marker_line(
            phase,
            "ready",
            self.sequence,
            self.latency_ms,
            self.stats,
            Some(&self.frame.source_id),
            None,
        )
    }
}

/// Failed Matter surface worker request.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestMakepadMatterSurfaceWorkerError {
    /// Request sequence that failed.
    pub sequence: u64,
    /// Worker counters captured after publishing the failure.
    pub stats: QuestMakepadMatterSurfaceWorkerStats,
    /// Wall-clock latency from submit to failure publication.
    pub latency_ms: f32,
    /// Adapter error.
    pub error: QuestMakepadMatterSurfaceError,
}

impl QuestMakepadMatterSurfaceWorkerError {
    /// Builds a worker evidence marker for this failed request.
    #[must_use]
    pub fn marker_line(&self, phase: &str) -> String {
        worker_marker_line(
            phase,
            "error",
            self.sequence,
            self.latency_ms,
            self.stats,
            None,
            Some(&self.error.to_string()),
        )
    }
}

/// Latest worker output consumed by the app thread.
#[derive(Clone, Debug, PartialEq)]
pub enum QuestMakepadMatterSurfaceWorkerOutput {
    /// The worker produced a Matter-backed frame.
    Frame(QuestMakepadMatterSurfaceWorkerFrame),
    /// The worker failed a submitted request.
    Error(QuestMakepadMatterSurfaceWorkerError),
}

impl QuestMakepadMatterSurfaceWorkerOutput {
    /// Returns the output sequence.
    #[must_use]
    pub fn sequence(&self) -> u64 {
        match self {
            Self::Frame(frame) => frame.sequence,
            Self::Error(error) => error.sequence,
        }
    }

    /// Builds a worker marker for this output.
    #[must_use]
    pub fn marker_line(&self, phase: &str) -> String {
        match self {
            Self::Frame(frame) => frame.marker_line(phase),
            Self::Error(error) => error.marker_line(phase),
        }
    }
}

/// Nonblocking worker handle for native Matter surface adapter frames.
#[derive(Debug)]
pub struct QuestMakepadMatterSurfaceWorker {
    config: QuestMakepadMatterSurfaceConfig,
    shared: Arc<(Mutex<WorkerState>, Condvar)>,
    worker: Option<JoinHandle<()>>,
}

impl QuestMakepadMatterSurfaceWorker {
    /// Creates a worker from an adapter config.
    ///
    /// # Errors
    ///
    /// Returns [`QuestMakepadMatterSurfaceError`] when the native Matter
    /// runtime config is invalid.
    pub fn new(
        config: QuestMakepadMatterSurfaceConfig,
    ) -> Result<Self, QuestMakepadMatterSurfaceError> {
        let runtime = QuestMakepadMatterSurfaceRuntime::new(config)?;
        Ok(Self::from_runtime(runtime))
    }

    /// Creates a worker from an already validated adapter runtime.
    #[must_use]
    pub fn from_runtime(runtime: QuestMakepadMatterSurfaceRuntime) -> Self {
        let config = runtime.config().clone();
        let shared = Arc::new((Mutex::new(WorkerState::default()), Condvar::new()));
        let worker_shared = Arc::clone(&shared);
        let worker = thread::Builder::new()
            .name("quest-makepad-matter-surface".to_owned())
            .spawn(move || worker_loop(runtime, worker_shared))
            .expect("Quest Makepad Matter surface worker thread should spawn");
        Self {
            config,
            shared,
            worker: Some(worker),
        }
    }

    /// Returns the adapter config used by this worker.
    #[must_use]
    pub fn config(&self) -> &QuestMakepadMatterSurfaceConfig {
        &self.config
    }

    /// Submits a replay frame without blocking for Matter processing.
    ///
    /// If an older request is still pending and the worker has not consumed it,
    /// the older request is dropped and counted. The worker always processes
    /// the latest pending request.
    ///
    /// # Errors
    ///
    /// Returns [`QuestMakepadMatterSurfaceError`] when the replay frame cannot
    /// be converted into the native Matter source-frame boundary.
    pub fn submit_replay_frame(
        &self,
        phase: impl Into<String>,
        replay: &MeshReplayRuntime,
        delta_seconds: f32,
        probes: &[MatterSurfaceContactProbe],
    ) -> Result<u64, QuestMakepadMatterSurfaceError> {
        let source_frame = QuestMakepadMatterSurfaceSourceFrame::from_replay(replay)?;
        Ok(self.submit_source_frame(phase, source_frame, delta_seconds, probes))
    }

    /// Submits a source frame without blocking for Matter processing.
    ///
    /// Returns the request sequence assigned by the worker handle.
    pub fn submit_source_frame(
        &self,
        phase: impl Into<String>,
        source_frame: QuestMakepadMatterSurfaceSourceFrame,
        delta_seconds: f32,
        probes: &[MatterSurfaceContactProbe],
    ) -> u64 {
        let (lock, wake) = &*self.shared;
        let mut state = lock.lock().expect("Matter worker state lock poisoned");
        if state.pending.is_some() {
            state.stats.dropped_pending_count = state.stats.dropped_pending_count.saturating_add(1);
        }
        state.stats.submitted_count = state.stats.submitted_count.saturating_add(1);
        let sequence = state.stats.submitted_count;
        state.stats.latest_submitted_sequence = sequence;
        state.stats.pending_request = true;
        let request = WorkerRequest {
            sequence,
            phase: phase.into(),
            source_frame,
            delta_seconds,
            probes: probes.to_vec(),
            submitted_at: Instant::now(),
        };
        state.pending = Some(request);
        wake.notify_one();
        sequence
    }

    /// Takes the latest completed worker output, if one is available.
    pub fn take_latest_output(&self) -> Option<QuestMakepadMatterSurfaceWorkerOutput> {
        let (lock, _) = &*self.shared;
        lock.lock()
            .expect("Matter worker state lock poisoned")
            .latest
            .take()
    }

    /// Returns a snapshot of the current worker counters.
    #[must_use]
    pub fn stats(&self) -> QuestMakepadMatterSurfaceWorkerStats {
        let (lock, _) = &*self.shared;
        lock.lock()
            .expect("Matter worker state lock poisoned")
            .stats
    }

    /// Builds a worker marker from the current counters.
    #[must_use]
    pub fn stats_marker_line(&self, phase: &str) -> String {
        let stats = self.stats();
        worker_marker_line(phase, "running", 0, 0.0, stats, None, None)
    }
}

impl Drop for QuestMakepadMatterSurfaceWorker {
    fn drop(&mut self) {
        let (lock, wake) = &*self.shared;
        if let Ok(mut state) = lock.lock() {
            state.shutdown = true;
            state.pending = None;
            state.stats.pending_request = false;
            wake.notify_all();
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[derive(Debug)]
struct WorkerRequest {
    sequence: u64,
    phase: String,
    source_frame: QuestMakepadMatterSurfaceSourceFrame,
    delta_seconds: f32,
    probes: Vec<MatterSurfaceContactProbe>,
    submitted_at: Instant,
}

#[derive(Debug, Default)]
struct WorkerState {
    pending: Option<WorkerRequest>,
    latest: Option<QuestMakepadMatterSurfaceWorkerOutput>,
    shutdown: bool,
    stats: QuestMakepadMatterSurfaceWorkerStats,
}

fn worker_loop(
    mut runtime: QuestMakepadMatterSurfaceRuntime,
    shared: Arc<(Mutex<WorkerState>, Condvar)>,
) {
    loop {
        let request = {
            let (lock, wake) = &*shared;
            let mut state = lock.lock().expect("Matter worker state lock poisoned");
            while state.pending.is_none() && !state.shutdown {
                state = wake
                    .wait(state)
                    .expect("Matter worker state lock poisoned while waiting");
            }
            if state.shutdown {
                return;
            }
            let request = state
                .pending
                .take()
                .expect("pending request should be present");
            state.stats.pending_request = false;
            state.stats.in_flight = true;
            request
        };

        let result = runtime.step_from_source_frame(
            request.source_frame,
            request.delta_seconds,
            &request.probes,
        );
        let latency_ms = request.submitted_at.elapsed().as_secs_f32() * 1000.0;

        let (lock, _) = &*shared;
        let mut state = lock.lock().expect("Matter worker state lock poisoned");
        state.stats.in_flight = false;
        state.stats.latest_finished_sequence = request.sequence;

        match result {
            Ok(frame) => {
                state.stats.completed_count = state.stats.completed_count.saturating_add(1);
                let stats = state.stats;
                let runtime_marker_line = runtime.marker_line(&request.phase, &frame);
                state.latest = Some(QuestMakepadMatterSurfaceWorkerOutput::Frame(
                    QuestMakepadMatterSurfaceWorkerFrame {
                        sequence: request.sequence,
                        stats,
                        latency_ms,
                        frame,
                        runtime_marker_line,
                    },
                ));
            }
            Err(error) => {
                state.stats.failed_count = state.stats.failed_count.saturating_add(1);
                let stats = state.stats;
                state.latest = Some(QuestMakepadMatterSurfaceWorkerOutput::Error(
                    QuestMakepadMatterSurfaceWorkerError {
                        sequence: request.sequence,
                        stats,
                        latency_ms,
                        error,
                    },
                ));
            }
        }
    }
}

fn worker_marker_line(
    phase: &str,
    status: &str,
    sequence: u64,
    latency_ms: f32,
    stats: QuestMakepadMatterSurfaceWorkerStats,
    source_id: Option<&str>,
    issue: Option<&str>,
) -> String {
    format!(
        "{} schema={} phase={} status={} mode=latest-wins workerThread=true renderThreadBlocking=false sequence={} submittedCount={} completedCount={} failedCount={} droppedPendingCount={} pendingRequest={} inFlight={} latestSubmittedSequence={} latestFinishedSequence={} latencyMs={:.3} sourceId={} issue={}",
        QUEST_MAKEPAD_MATTER_SURFACE_WORKER_MARKER_PREFIX,
        QUEST_MAKEPAD_MATTER_SURFACE_WORKER_SCHEMA_ID,
        sanitize_marker_value(phase),
        sanitize_marker_value(status),
        sequence,
        stats.submitted_count,
        stats.completed_count,
        stats.failed_count,
        stats.dropped_pending_count,
        stats.pending_request,
        stats.in_flight,
        stats.latest_submitted_sequence,
        stats.latest_finished_sequence,
        latency_ms.max(0.0),
        source_id.map(sanitize_marker_value).unwrap_or_else(|| "none".to_owned()),
        issue.map(sanitize_marker_value).unwrap_or_else(|| "none".to_owned()),
    )
}

fn sanitize_marker_value(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

impl fmt::Display for QuestMakepadMatterSurfaceWorkerStats {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "submitted={} completed={} failed={} droppedPending={} pending={} inFlight={}",
            self.submitted_count,
            self.completed_count,
            self.failed_count,
            self.dropped_pending_count,
            self.pending_request,
            self.in_flight,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusty_quest_makepad_mesh_replay::{MeshReplayConfig, MeshReplayRuntime};
    use std::time::Duration;

    fn enabled_replay() -> MeshReplayRuntime {
        let mut replay = MeshReplayRuntime::default();
        replay.configure(MeshReplayConfig::normalized(
            true,
            "public-synthetic-hand-sequence".to_owned(),
            1.0,
            0.75,
        ));
        replay.step(0.0);
        replay
    }

    #[test]
    fn worker_publishes_latest_completed_frame_without_blocking_submitter() {
        let replay = enabled_replay();
        let worker = QuestMakepadMatterSurfaceWorker::new(QuestMakepadMatterSurfaceConfig {
            enabled: true,
            particles_enabled: true,
            particle_count: 8,
            ..QuestMakepadMatterSurfaceConfig::default()
        })
        .expect("worker builds");

        let sequence = worker
            .submit_replay_frame("unit-test", &replay, 1.0 / 60.0, &[])
            .expect("replay frame submits");

        assert_eq!(sequence, 1);
        assert_eq!(worker.stats().submitted_count, 1);

        let output = wait_for_output(&worker);
        let QuestMakepadMatterSurfaceWorkerOutput::Frame(frame) = output else {
            panic!("expected completed worker frame");
        };
        assert_eq!(frame.sequence, 1);
        assert_eq!(frame.frame.source_id, "public-synthetic-hand-sequence");
        assert_eq!(frame.frame.particle_snapshot.samples.len(), 8);
        assert!(frame
            .runtime_marker_line
            .contains("nativeMatterRuntime=true"));
        assert!(frame.marker_line("unit-test").contains("mode=latest-wins"));
        assert!(worker.take_latest_output().is_none());
    }

    #[test]
    fn worker_counts_pending_drops_when_submitter_overwrites_mailbox() {
        let replay = enabled_replay();
        let worker = QuestMakepadMatterSurfaceWorker::new(QuestMakepadMatterSurfaceConfig {
            enabled: true,
            particles_enabled: false,
            ..QuestMakepadMatterSurfaceConfig::default()
        })
        .expect("worker builds");

        let first = QuestMakepadMatterSurfaceSourceFrame::from_replay(&replay)
            .expect("source frame builds");
        let second = first.clone();
        worker.submit_source_frame("unit-test", first, 1.0 / 60.0, &[]);
        worker.submit_source_frame("unit-test", second, 1.0 / 60.0, &[]);

        let output = wait_for_output(&worker);
        assert!(matches!(
            output,
            QuestMakepadMatterSurfaceWorkerOutput::Frame(_)
        ));
        let stats = worker.stats();
        assert_eq!(stats.submitted_count, 2);
        assert!(stats.completed_count >= 1);
        assert!(stats.dropped_pending_count <= 1);
    }

    fn wait_for_output(
        worker: &QuestMakepadMatterSurfaceWorker,
    ) -> QuestMakepadMatterSurfaceWorkerOutput {
        for _ in 0..100 {
            if let Some(output) = worker.take_latest_output() {
                return output;
            }
            thread::sleep(Duration::from_millis(5));
        }
        panic!(
            "timed out waiting for Matter worker output; stats={}",
            worker.stats()
        );
    }
}
