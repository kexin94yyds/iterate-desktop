//! Privacy-bounded, process-local tracing for desktop speech sessions.

use super::session::{SpeechLayerIdentity, SpeechPhase};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpeechTraceMilestone {
    OwnerLockAttempt,
    OwnerAcquired,
    OwnerLost,
    TapInstallRequested,
    TapReady,
    TapDisabled,
    TapRecovered,
    FnDown,
    IntentAccepted,
    TargetCaptureRequested,
    TargetCaptured,
    OverlayRevealRequested,
    OverlayVisibleAck,
    NativeStartRequested,
    NativeStarted,
    FirstPartial,
    StopIntent,
    OverlayHideRequested,
    OverlayHiddenAck,
    NativeFinishRequested,
    NativeFinalReceived,
    FallbackSelected,
    ProcessingRequested,
    ProcessingReceived,
    WritebackRequested,
    WritebackDispatched,
    WritebackUnverified,
    WritebackAck,
    WritebackUnknownAfterDispatch,
    SessionTerminal,
}

impl SpeechTraceMilestone {
    pub const REQUIRED: [Self; 30] = [
        Self::OwnerLockAttempt,
        Self::OwnerAcquired,
        Self::OwnerLost,
        Self::TapInstallRequested,
        Self::TapReady,
        Self::TapDisabled,
        Self::TapRecovered,
        Self::FnDown,
        Self::IntentAccepted,
        Self::TargetCaptureRequested,
        Self::TargetCaptured,
        Self::OverlayRevealRequested,
        Self::OverlayVisibleAck,
        Self::NativeStartRequested,
        Self::NativeStarted,
        Self::FirstPartial,
        Self::StopIntent,
        Self::OverlayHideRequested,
        Self::OverlayHiddenAck,
        Self::NativeFinishRequested,
        Self::NativeFinalReceived,
        Self::FallbackSelected,
        Self::ProcessingRequested,
        Self::ProcessingReceived,
        Self::WritebackRequested,
        Self::WritebackDispatched,
        Self::WritebackUnverified,
        Self::WritebackAck,
        Self::WritebackUnknownAfterDispatch,
        Self::SessionTerminal,
    ];
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpeechTraceOutcome {
    #[default]
    Pending,
    Success,
    Failed,
    Stale,
    Unknown,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpeechTraceTargetKind {
    #[default]
    None,
    Application,
    Popup,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SpeechTraceFinalSource {
    #[default]
    None,
    NativeFinal,
    PartialFallback,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpeechTraceFields {
    pub identity: Option<SpeechLayerIdentity>,
    pub phase: SpeechPhase,
    pub outcome: SpeechTraceOutcome,
    pub target_kind: SpeechTraceTargetKind,
    pub final_source: SpeechTraceFinalSource,
    pub text_len: usize,
    pub stale_count: u64,
    pub dispatch_count: u64,
    pub wall_clock_unix_ms: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpeechTraceRecord {
    pub milestone: SpeechTraceMilestone,
    pub elapsed_us: u64,
    pub identity: Option<SpeechLayerIdentity>,
    pub phase: SpeechPhase,
    pub outcome: SpeechTraceOutcome,
    pub target_kind: SpeechTraceTargetKind,
    pub final_source: SpeechTraceFinalSource,
    pub text_len: usize,
    pub stale_count: u64,
    pub dispatch_count: u64,
    pub wall_clock_unix_ms: Option<i64>,
}

#[derive(Debug)]
struct TraceState {
    records: VecDeque<SpeechTraceRecord>,
    capacity: usize,
}

#[derive(Clone, Debug)]
pub struct SpeechTrace {
    origin_us: u64,
    state: Arc<Mutex<TraceState>>,
}

impl SpeechTrace {
    pub fn new(origin_us: u64, capacity: usize) -> Self {
        Self {
            origin_us,
            state: Arc::new(Mutex::new(TraceState {
                records: VecDeque::with_capacity(capacity.max(1)),
                capacity: capacity.max(1),
            })),
        }
    }

    pub fn record(&self, now_us: u64, milestone: SpeechTraceMilestone, fields: SpeechTraceFields) {
        let mut state = self.state.lock().expect("speech trace poisoned");
        if state.records.len() == state.capacity {
            state.records.pop_front();
        }
        state.records.push_back(SpeechTraceRecord {
            milestone,
            elapsed_us: now_us.saturating_sub(self.origin_us),
            identity: fields.identity,
            phase: fields.phase,
            outcome: fields.outcome,
            target_kind: fields.target_kind,
            final_source: fields.final_source,
            text_len: fields.text_len,
            stale_count: fields.stale_count,
            dispatch_count: fields.dispatch_count,
            wall_clock_unix_ms: fields.wall_clock_unix_ms,
        });
    }

    pub fn records(&self) -> Vec<SpeechTraceRecord> {
        self.state
            .lock()
            .expect("speech trace poisoned")
            .records
            .iter()
            .copied()
            .collect()
    }
}
