use super::session::{SpeechControlInput, SpeechEffect, SpeechSessionReducer, SpeechSnapshot};
use super::trace::{
    SpeechTrace, SpeechTraceFields, SpeechTraceFinalSource, SpeechTraceMilestone,
    SpeechTraceOutcome,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{sync_channel, SyncSender, TrySendError};
use std::sync::{Arc, Mutex};

pub trait MonotonicClock: Send + Sync + 'static {
    fn now_us(&self) -> u64;
}

pub trait SpeechEffectExecutor: Send + Sync + 'static {
    fn execute(&self, effect: SpeechEffect, completion: SpeechCompletionSender);
}

#[derive(Clone)]
pub struct SpeechCompletionSender {
    sender: SyncSender<CoordinatorMessage>,
}

impl SpeechCompletionSender {
    pub fn send(&self, input: SpeechControlInput) -> Result<(), SpeechCoordinatorError> {
        self.sender
            .try_send(CoordinatorMessage::Input { input, ack: None })
            .map_err(SpeechCoordinatorError::from_try_send)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpeechCoordinatorError {
    QueueFull,
    Closed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpeechIdentityValidationError {
    NoActiveSession,
    SchemaVersion,
    OwnerEpoch,
    ControlSequence,
    SessionSequence,
    Revision,
}

pub fn validate_control_identity(
    snapshot: &SpeechSnapshot,
    received: super::session::SpeechLayerIdentity,
) -> Result<(), SpeechIdentityValidationError> {
    let expected = snapshot
        .identity
        .ok_or(SpeechIdentityValidationError::NoActiveSession)?;
    if received.schema_version != super::session::SPEECH_IDENTITY_SCHEMA_VERSION {
        return Err(SpeechIdentityValidationError::SchemaVersion);
    }
    if received.owner_epoch() != expected.owner_epoch()
        || snapshot.owner_epoch != Some(received.owner_epoch())
    {
        return Err(SpeechIdentityValidationError::OwnerEpoch);
    }
    if received.control_seq != expected.control_seq {
        return Err(SpeechIdentityValidationError::ControlSequence);
    }
    if received.session_sequence != expected.session_sequence {
        return Err(SpeechIdentityValidationError::SessionSequence);
    }
    if received.revision != expected.revision {
        return Err(SpeechIdentityValidationError::Revision);
    }
    Ok(())
}

impl SpeechCoordinatorError {
    fn from_try_send<T>(error: TrySendError<T>) -> Self {
        match error {
            TrySendError::Full(_) => Self::QueueFull,
            TrySendError::Disconnected(_) => Self::Closed,
        }
    }
}

enum CoordinatorMessage {
    Input {
        input: SpeechControlInput,
        ack: Option<SyncSender<()>>,
    },
    Flush(SyncSender<()>),
    Pause(SyncSender<()>),
    Shutdown,
}

pub struct SpeechCoordinatorHandle {
    sender: SyncSender<CoordinatorMessage>,
    snapshot: Arc<Mutex<SpeechSnapshot>>,
    paused: Arc<AtomicBool>,
    trace: SpeechTrace,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl SpeechCoordinatorHandle {
    pub fn new(
        executor: Arc<dyn SpeechEffectExecutor>,
        clock: Arc<dyn MonotonicClock>,
        capacity: usize,
    ) -> Self {
        let capacity = capacity.max(1);
        let (sender, receiver) = sync_channel(capacity);
        let reducer = SpeechSessionReducer::default();
        let snapshot = Arc::new(Mutex::new(reducer.snapshot()));
        let worker_snapshot = snapshot.clone();
        let trace = SpeechTrace::new(clock.now_us(), 256);
        let worker_trace = trace.clone();
        let completion = SpeechCompletionSender {
            sender: sender.clone(),
        };
        let paused = Arc::new(AtomicBool::new(false));
        let worker_paused = paused.clone();

        let worker = std::thread::spawn(move || {
            let mut reducer = reducer;
            let mut dispatch_count = 0_u64;
            while let Ok(message) = receiver.recv() {
                match message {
                    CoordinatorMessage::Input { input, ack } => {
                        let now_us = clock.now_us();
                        let before = reducer.snapshot();
                        trace_input(&worker_trace, now_us, &input, &before, dispatch_count);
                        let effects = reducer.apply(input, now_us);
                        let after = reducer.snapshot();
                        trace_transition(
                            &worker_trace,
                            now_us,
                            &before,
                            &after,
                            &effects,
                            &mut dispatch_count,
                        );
                        if let Ok(mut snapshot) = worker_snapshot.lock() {
                            *snapshot = after;
                        }
                        for effect in effects {
                            executor.execute(effect, completion.clone());
                        }
                        if let Some(ack) = ack {
                            let _ = ack.send(());
                        }
                    }
                    CoordinatorMessage::Flush(ack) => {
                        let _ = ack.send(());
                    }
                    CoordinatorMessage::Pause(ack) => {
                        worker_paused.store(true, Ordering::Release);
                        let _ = ack.send(());
                        while worker_paused.load(Ordering::Acquire) {
                            std::thread::park_timeout(std::time::Duration::from_millis(10));
                        }
                    }
                    CoordinatorMessage::Shutdown => break,
                }
            }
        });
        Self {
            sender,
            snapshot,
            paused,
            trace,
            worker: Some(worker),
        }
    }

    pub fn send(&self, input: SpeechControlInput) -> Result<(), SpeechCoordinatorError> {
        let (ack_sender, ack_receiver) = sync_channel(1);
        self.sender
            .try_send(CoordinatorMessage::Input {
                input,
                ack: (!self.paused.load(Ordering::Acquire)).then_some(ack_sender),
            })
            .map_err(SpeechCoordinatorError::from_try_send)?;
        if !self.paused.load(Ordering::Acquire) {
            ack_receiver
                .recv()
                .map_err(|_| SpeechCoordinatorError::Closed)?;
        }
        Ok(())
    }

    pub fn flush(&self) -> Result<(), SpeechCoordinatorError> {
        let (sender, receiver) = sync_channel(1);
        self.sender
            .try_send(CoordinatorMessage::Flush(sender))
            .map_err(SpeechCoordinatorError::from_try_send)?;
        receiver.recv().map_err(|_| SpeechCoordinatorError::Closed)
    }

    pub fn snapshot(&self) -> SpeechSnapshot {
        self.snapshot
            .lock()
            .expect("speech snapshot poisoned")
            .clone()
    }

    pub fn trace(&self) -> SpeechTrace {
        self.trace.clone()
    }

    #[doc(hidden)]
    pub fn pause_for_test(&self) {
        let (sender, receiver) = sync_channel(1);
        self.sender
            .send(CoordinatorMessage::Pause(sender))
            .expect("coordinator closed");
        receiver.recv().expect("coordinator pause failed");
    }
}

impl Drop for SpeechCoordinatorHandle {
    fn drop(&mut self) {
        self.paused.store(false, Ordering::Release);
        let _ = self.sender.send(CoordinatorMessage::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn trace_input(
    trace: &SpeechTrace,
    now_us: u64,
    input: &SpeechControlInput,
    before: &SpeechSnapshot,
    dispatch_count: u64,
) {
    let (milestone, identity, text_len, outcome) = match input {
        SpeechControlInput::OwnerAcquired { .. } => (
            SpeechTraceMilestone::OwnerAcquired,
            None,
            0,
            SpeechTraceOutcome::Success,
        ),
        SpeechControlInput::OwnerLost => (
            SpeechTraceMilestone::OwnerLost,
            before.identity,
            0,
            SpeechTraceOutcome::Success,
        ),
        SpeechControlInput::FnPressed => (
            SpeechTraceMilestone::FnDown,
            before.identity,
            0,
            SpeechTraceOutcome::Pending,
        ),
        SpeechControlInput::NativeStarted { identity } => (
            SpeechTraceMilestone::NativeStarted,
            Some(*identity),
            0,
            SpeechTraceOutcome::Success,
        ),
        SpeechControlInput::NativePartial { identity, text } if before.partial_len == 0 => (
            SpeechTraceMilestone::FirstPartial,
            Some(*identity),
            text.len(),
            SpeechTraceOutcome::Success,
        ),
        SpeechControlInput::NativeFinal { identity, text } => (
            SpeechTraceMilestone::NativeFinalReceived,
            Some(*identity),
            text.len(),
            SpeechTraceOutcome::Success,
        ),
        SpeechControlInput::TranscriptProcessed { identity, text } => (
            SpeechTraceMilestone::ProcessingReceived,
            Some(*identity),
            text.len(),
            SpeechTraceOutcome::Success,
        ),
        SpeechControlInput::WritebackAcknowledged { identity } => (
            SpeechTraceMilestone::WritebackAck,
            Some(*identity),
            0,
            SpeechTraceOutcome::Success,
        ),
        SpeechControlInput::WritebackDispatched { identity } => (
            SpeechTraceMilestone::WritebackDispatched,
            Some(*identity),
            0,
            SpeechTraceOutcome::Success,
        ),
        SpeechControlInput::WritebackUnverified { identity } => (
            SpeechTraceMilestone::WritebackUnverified,
            Some(*identity),
            0,
            SpeechTraceOutcome::Unknown,
        ),
        SpeechControlInput::WritebackDispatchFailed { identity } => (
            SpeechTraceMilestone::WritebackRequested,
            Some(*identity),
            0,
            SpeechTraceOutcome::Failed,
        ),
        SpeechControlInput::WritebackFailed { identity } => (
            SpeechTraceMilestone::WritebackUnknownAfterDispatch,
            Some(*identity),
            0,
            SpeechTraceOutcome::Unknown,
        ),
        SpeechControlInput::OverlayVisibilityAcknowledged { identity, visible } => (
            if *visible {
                SpeechTraceMilestone::OverlayVisibleAck
            } else {
                SpeechTraceMilestone::OverlayHiddenAck
            },
            Some(*identity),
            0,
            SpeechTraceOutcome::Success,
        ),
        _ => return,
    };
    trace.record(
        now_us,
        milestone,
        SpeechTraceFields {
            identity,
            phase: before.phase,
            outcome,
            text_len,
            dispatch_count,
            ..SpeechTraceFields::default()
        },
    );
}

fn trace_transition(
    trace: &SpeechTrace,
    now_us: u64,
    before: &SpeechSnapshot,
    after: &SpeechSnapshot,
    effects: &[SpeechEffect],
    dispatch_count: &mut u64,
) {
    if before.control_seq != after.control_seq {
        record_snapshot(
            trace,
            now_us,
            SpeechTraceMilestone::IntentAccepted,
            after,
            0,
            SpeechTraceFinalSource::None,
            *dispatch_count,
        );
    }
    if before.desired_state != after.desired_state
        && matches!(after.desired_state, super::session::DesiredState::Off)
    {
        record_snapshot(
            trace,
            now_us,
            SpeechTraceMilestone::StopIntent,
            after,
            0,
            SpeechTraceFinalSource::None,
            *dispatch_count,
        );
    }
    for effect in effects {
        let (milestone, text_len) = match effect {
            SpeechEffect::ShowOverlay { .. } => (SpeechTraceMilestone::OverlayRevealRequested, 0),
            SpeechEffect::HideOverlay { .. } => (SpeechTraceMilestone::OverlayHideRequested, 0),
            SpeechEffect::PrepareStart { .. } => (SpeechTraceMilestone::TargetCaptureRequested, 0),
            SpeechEffect::StartNative { .. } => (SpeechTraceMilestone::NativeStartRequested, 0),
            SpeechEffect::FinishNative { .. } => (SpeechTraceMilestone::NativeFinishRequested, 0),
            SpeechEffect::ProcessTranscript { text, .. } => {
                (SpeechTraceMilestone::ProcessingRequested, text.len())
            }
            SpeechEffect::DispatchWriteback { text, .. } => {
                *dispatch_count = dispatch_count.saturating_add(1);
                record_snapshot(
                    trace,
                    now_us,
                    SpeechTraceMilestone::WritebackRequested,
                    after,
                    text.len(),
                    SpeechTraceFinalSource::None,
                    *dispatch_count,
                );
                (SpeechTraceMilestone::WritebackDispatched, text.len())
            }
            SpeechEffect::CancelStart { .. }
            | SpeechEffect::CancelNative { .. }
            | SpeechEffect::ScheduleFinishDeadline { .. }
            | SpeechEffect::ScheduleProcessingDeadline { .. } => continue,
        };
        record_snapshot(
            trace,
            now_us,
            milestone,
            after,
            text_len,
            SpeechTraceFinalSource::None,
            *dispatch_count,
        );
    }
    if before.phase != after.phase && matches!(after.phase, super::session::SpeechPhase::Terminal) {
        record_snapshot(
            trace,
            now_us,
            SpeechTraceMilestone::SessionTerminal,
            after,
            0,
            SpeechTraceFinalSource::None,
            *dispatch_count,
        );
    }
}

fn record_snapshot(
    trace: &SpeechTrace,
    now_us: u64,
    milestone: SpeechTraceMilestone,
    snapshot: &SpeechSnapshot,
    text_len: usize,
    final_source: SpeechTraceFinalSource,
    dispatch_count: u64,
) {
    trace.record(
        now_us,
        milestone,
        SpeechTraceFields {
            identity: snapshot.identity,
            phase: snapshot.phase,
            outcome: SpeechTraceOutcome::Success,
            final_source,
            text_len,
            dispatch_count,
            ..SpeechTraceFields::default()
        },
    );
}
