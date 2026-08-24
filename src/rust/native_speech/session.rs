//! Deterministic control-plane reducer for desktop speech sessions.

use serde::{Deserialize, Serialize};

pub const SPEECH_IDENTITY_SCHEMA_VERSION: u64 = 1;
pub const MAX_FINISH_GRACE_US: u64 = 3_500_000;
/// Processing 依赖前端 webview 回调 `complete_speech_processing`；webview 卡死或事件丢失时
/// 没有任何输入能离开 Processing，Fn 键会永久失效。看门狗到期即按处理失败收尾。
pub const MAX_PROCESSING_GRACE_US: u64 = 10_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OwnerEpoch(pub [u8; 16]);

impl OwnerEpoch {
    pub fn to_canonical_string(self) -> String {
        uuid::Uuid::from_bytes(self.0).to_string()
    }

    pub fn parse_canonical(value: &str) -> Option<Self> {
        uuid::Uuid::parse_str(value)
            .ok()
            .map(|value| Self(*value.as_bytes()))
    }

    pub fn to_be_halves(self) -> (u64, u64) {
        let mut hi = [0; 8];
        let mut lo = [0; 8];
        hi.copy_from_slice(&self.0[..8]);
        lo.copy_from_slice(&self.0[8..]);
        (u64::from_be_bytes(hi), u64::from_be_bytes(lo))
    }

    pub fn from_be_halves(hi: u64, lo: u64) -> Self {
        let mut bytes = [0; 16];
        bytes[..8].copy_from_slice(&hi.to_be_bytes());
        bytes[8..].copy_from_slice(&lo.to_be_bytes());
        Self(bytes)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpeechLayerIdentity {
    #[serde(with = "u64_string")]
    pub schema_version: u64,
    #[serde(with = "u64_string")]
    pub owner_epoch_hi: u64,
    #[serde(with = "u64_string")]
    pub owner_epoch_lo: u64,
    #[serde(with = "u64_string")]
    pub control_seq: u64,
    #[serde(with = "u64_string")]
    pub session_sequence: u64,
    #[serde(with = "u64_string")]
    pub revision: u64,
}

mod u64_string {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum WireValue {
            String(String),
            Number(u64),
        }

        match WireValue::deserialize(deserializer)? {
            WireValue::String(value) => value.parse().map_err(serde::de::Error::custom),
            WireValue::Number(value) => Ok(value),
        }
    }
}

impl SpeechLayerIdentity {
    pub fn new(
        owner_epoch: OwnerEpoch,
        control_seq: u64,
        session_sequence: u64,
        revision: u64,
    ) -> Self {
        let (owner_epoch_hi, owner_epoch_lo) = owner_epoch.to_be_halves();
        Self {
            schema_version: SPEECH_IDENTITY_SCHEMA_VERSION,
            owner_epoch_hi,
            owner_epoch_lo,
            control_seq,
            session_sequence,
            revision,
        }
    }

    pub fn owner_epoch(self) -> OwnerEpoch {
        OwnerEpoch::from_be_halves(self.owner_epoch_hi, self.owner_epoch_lo)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DesiredState {
    #[default]
    Off,
    On,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpeechPhase {
    #[default]
    Idle,
    Arming,
    Listening,
    Finishing,
    Processing,
    Committing,
    Terminal,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum WritebackOutcome {
    #[default]
    NotDispatched,
    Dispatched,
    DispatchedUnverified,
    Acknowledged,
    FailedBeforeDispatch,
    UnknownAfterDispatch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpeechControlInput {
    OwnerAcquired {
        epoch: OwnerEpoch,
    },
    OwnerLost,
    FnPressed,
    StartPrepared {
        identity: SpeechLayerIdentity,
    },
    StartFailed {
        identity: SpeechLayerIdentity,
    },
    NativeStarted {
        identity: SpeechLayerIdentity,
    },
    NativePartial {
        identity: SpeechLayerIdentity,
        text: String,
    },
    NativeFinal {
        identity: SpeechLayerIdentity,
        text: String,
    },
    NativeError {
        identity: SpeechLayerIdentity,
    },
    FinishDeadline {
        identity: SpeechLayerIdentity,
    },
    ProcessingDeadline {
        identity: SpeechLayerIdentity,
    },
    TranscriptProcessed {
        identity: SpeechLayerIdentity,
        text: String,
    },
    WritebackDispatched {
        identity: SpeechLayerIdentity,
    },
    WritebackUnverified {
        identity: SpeechLayerIdentity,
    },
    WritebackDispatchFailed {
        identity: SpeechLayerIdentity,
    },
    ProcessingFailed {
        identity: SpeechLayerIdentity,
    },
    WritebackAcknowledged {
        identity: SpeechLayerIdentity,
    },
    WritebackFailed {
        identity: SpeechLayerIdentity,
    },
    RecoverAfterCrash,
    OverlayVisibilityAcknowledged {
        identity: SpeechLayerIdentity,
        visible: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpeechEffect {
    ShowOverlay {
        identity: SpeechLayerIdentity,
    },
    HideOverlay {
        identity: SpeechLayerIdentity,
    },
    PrepareStart {
        identity: SpeechLayerIdentity,
    },
    CancelStart {
        identity: SpeechLayerIdentity,
    },
    StartNative {
        identity: SpeechLayerIdentity,
    },
    FinishNative {
        identity: SpeechLayerIdentity,
    },
    CancelNative {
        identity: SpeechLayerIdentity,
    },
    ScheduleFinishDeadline {
        identity: SpeechLayerIdentity,
        deadline_us: u64,
    },
    ScheduleProcessingDeadline {
        identity: SpeechLayerIdentity,
        deadline_us: u64,
    },
    ProcessTranscript {
        identity: SpeechLayerIdentity,
        text: String,
    },
    DispatchWriteback {
        identity: SpeechLayerIdentity,
        text: String,
    },
}

impl SpeechEffect {
    pub fn identity(&self) -> Option<SpeechLayerIdentity> {
        Some(match self {
            Self::ShowOverlay { identity }
            | Self::HideOverlay { identity }
            | Self::PrepareStart { identity }
            | Self::CancelStart { identity }
            | Self::StartNative { identity }
            | Self::FinishNative { identity }
            | Self::CancelNative { identity }
            | Self::ScheduleFinishDeadline { identity, .. }
            | Self::ScheduleProcessingDeadline { identity, .. }
            | Self::ProcessTranscript { identity, .. }
            | Self::DispatchWriteback { identity, .. } => *identity,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpeechSnapshot {
    pub owner_epoch: Option<OwnerEpoch>,
    pub desired_state: DesiredState,
    pub phase: SpeechPhase,
    pub visible: bool,
    pub control_seq: u64,
    pub session: Option<u64>,
    pub identity: Option<SpeechLayerIdentity>,
    pub partial_len: usize,
    pub writeback_outcome: WritebackOutcome,
}

#[derive(Debug, Default)]
pub struct SpeechSessionReducer {
    owner_epoch: Option<OwnerEpoch>,
    desired_state: DesiredState,
    phase: SpeechPhase,
    visible: bool,
    control_seq: u64,
    session_sequence: u64,
    revision: u64,
    identity: Option<SpeechLayerIdentity>,
    expected_prepare: Option<SpeechLayerIdentity>,
    expected_native: Option<SpeechLayerIdentity>,
    expected_finish: Option<SpeechLayerIdentity>,
    expected_process: Option<SpeechLayerIdentity>,
    expected_writeback: Option<SpeechLayerIdentity>,
    partial: String,
    stop_partial: String,
    early_final: Option<String>,
    writeback_outcome: WritebackOutcome,
}

impl SpeechSessionReducer {
    pub fn snapshot(&self) -> SpeechSnapshot {
        SpeechSnapshot {
            owner_epoch: self.owner_epoch,
            desired_state: self.desired_state,
            phase: self.phase,
            visible: self.visible,
            control_seq: self.control_seq,
            session: (self.session_sequence != 0).then_some(self.session_sequence),
            identity: self.identity,
            partial_len: self.partial.len(),
            writeback_outcome: self.writeback_outcome,
        }
    }

    pub fn apply(&mut self, input: SpeechControlInput, now_us: u64) -> Vec<SpeechEffect> {
        match input {
            SpeechControlInput::OwnerAcquired { epoch } => {
                self.reset_for_owner(Some(epoch));
                Vec::new()
            }
            SpeechControlInput::OwnerLost => {
                let was_visible = self.visible;
                let identity = self.identity;
                self.reset_for_owner(None);
                match (was_visible, identity) {
                    (true, Some(identity)) => vec![SpeechEffect::HideOverlay { identity }],
                    _ => Vec::new(),
                }
            }
            SpeechControlInput::FnPressed => self.on_fn_pressed(now_us),
            SpeechControlInput::StartPrepared { identity } => self.on_start_prepared(identity),
            SpeechControlInput::StartFailed { identity } => {
                if self.phase == SpeechPhase::Arming
                    && self.desired_state == DesiredState::On
                    && self.expected_prepare == Some(identity)
                {
                    self.desired_state = DesiredState::Off;
                    self.phase = SpeechPhase::Terminal;
                    self.visible = false;
                    self.expected_prepare = None;
                    self.bump_identity();
                    vec![SpeechEffect::HideOverlay {
                        identity: self.identity.expect("owned identity"),
                    }]
                } else {
                    Vec::new()
                }
            }
            SpeechControlInput::NativeStarted { identity } => {
                if self.phase == SpeechPhase::Arming
                    && self.desired_state == DesiredState::On
                    && self.expected_native == Some(identity)
                {
                    self.phase = SpeechPhase::Listening;
                    self.bump_identity();
                }
                Vec::new()
            }
            SpeechControlInput::NativePartial { identity, text } => {
                if self.phase == SpeechPhase::Listening && self.expected_native == Some(identity) {
                    self.partial = text;
                }
                Vec::new()
            }
            SpeechControlInput::NativeFinal { identity, text } => {
                self.on_native_final(identity, text, now_us)
            }
            SpeechControlInput::NativeError { identity } => self.on_native_error(identity, now_us),
            SpeechControlInput::FinishDeadline { identity } => {
                if self.phase == SpeechPhase::Finishing && self.expected_finish == Some(identity) {
                    self.expected_finish = None;
                    let text = std::mem::take(&mut self.stop_partial);
                    if text.is_empty() {
                        self.complete_terminal()
                    } else {
                        self.begin_processing(text, now_us)
                    }
                } else {
                    Vec::new()
                }
            }
            SpeechControlInput::ProcessingDeadline { identity } => {
                if self.phase == SpeechPhase::Processing && self.expected_process == Some(identity)
                {
                    self.expected_process = None;
                    self.desired_state = DesiredState::Off;
                    self.complete_terminal()
                } else {
                    Vec::new()
                }
            }
            SpeechControlInput::TranscriptProcessed { identity, text } => {
                if self.phase == SpeechPhase::Processing && self.expected_process == Some(identity)
                {
                    self.expected_process = None;
                    if text.is_empty() {
                        self.complete_terminal()
                    } else {
                        self.phase = SpeechPhase::Committing;
                        self.writeback_outcome = WritebackOutcome::NotDispatched;
                        let identity = self.bump_identity();
                        self.expected_writeback = Some(identity);
                        vec![SpeechEffect::DispatchWriteback { identity, text }]
                    }
                } else {
                    Vec::new()
                }
            }
            SpeechControlInput::ProcessingFailed { identity } => {
                if self.phase == SpeechPhase::Processing && self.expected_process == Some(identity)
                {
                    self.expected_process = None;
                    self.complete_terminal()
                } else {
                    Vec::new()
                }
            }
            SpeechControlInput::WritebackDispatched { identity } => {
                if self.phase == SpeechPhase::Committing
                    && self.expected_writeback == Some(identity)
                    && self.writeback_outcome == WritebackOutcome::NotDispatched
                {
                    self.writeback_outcome = WritebackOutcome::Dispatched;
                }
                Vec::new()
            }
            SpeechControlInput::WritebackUnverified { identity } => {
                if self.phase == SpeechPhase::Committing
                    && self.expected_writeback == Some(identity)
                    && self.writeback_outcome == WritebackOutcome::Dispatched
                {
                    self.expected_writeback = None;
                    self.writeback_outcome = WritebackOutcome::DispatchedUnverified;
                    self.complete_terminal()
                } else {
                    Vec::new()
                }
            }
            SpeechControlInput::WritebackDispatchFailed { identity } => {
                if self.phase == SpeechPhase::Committing
                    && self.expected_writeback == Some(identity)
                    && self.writeback_outcome == WritebackOutcome::NotDispatched
                {
                    self.expected_writeback = None;
                    self.writeback_outcome = WritebackOutcome::FailedBeforeDispatch;
                    self.desired_state = DesiredState::Off;
                    self.complete_terminal()
                } else {
                    Vec::new()
                }
            }
            SpeechControlInput::WritebackAcknowledged { identity } => {
                if self.phase == SpeechPhase::Committing
                    && self.expected_writeback == Some(identity)
                    && self.writeback_outcome == WritebackOutcome::Dispatched
                {
                    self.expected_writeback = None;
                    self.writeback_outcome = WritebackOutcome::Acknowledged;
                    self.complete_terminal()
                } else {
                    Vec::new()
                }
            }
            SpeechControlInput::WritebackFailed { identity } => {
                if self.phase == SpeechPhase::Committing
                    && self.expected_writeback == Some(identity)
                    && self.writeback_outcome == WritebackOutcome::Dispatched
                {
                    self.expected_writeback = None;
                    self.writeback_outcome = WritebackOutcome::UnknownAfterDispatch;
                    self.desired_state = DesiredState::Off;
                    self.complete_terminal()
                } else {
                    Vec::new()
                }
            }
            SpeechControlInput::RecoverAfterCrash => {
                if self.writeback_outcome == WritebackOutcome::Dispatched {
                    self.writeback_outcome = WritebackOutcome::UnknownAfterDispatch;
                    self.desired_state = DesiredState::Off;
                    self.visible = false;
                    self.phase = SpeechPhase::Terminal;
                    self.clear_expected();
                    self.bump_identity();
                }
                Vec::new()
            }
            SpeechControlInput::OverlayVisibilityAcknowledged { identity, visible } => {
                if self.identity == Some(identity) && self.visible == visible {
                    // Acknowledgements confirm an effect; desired state remains authoritative.
                }
                Vec::new()
            }
        }
    }

    fn on_fn_pressed(&mut self, now_us: u64) -> Vec<SpeechEffect> {
        if self.owner_epoch.is_none() {
            return Vec::new();
        }
        self.control_seq = self.control_seq.saturating_add(1);
        self.desired_state = match self.desired_state {
            DesiredState::Off => DesiredState::On,
            DesiredState::On => DesiredState::Off,
        };

        match self.desired_state {
            DesiredState::On => match self.phase {
                SpeechPhase::Idle | SpeechPhase::Terminal => self.begin_session(),
                SpeechPhase::Finishing
                | SpeechPhase::Processing
                | SpeechPhase::Committing
                | SpeechPhase::Arming
                | SpeechPhase::Listening => Vec::new(),
            },
            DesiredState::Off => match self.phase {
                SpeechPhase::Arming => self.cancel_arming(),
                SpeechPhase::Listening => self.stop_listening(now_us),
                _ => Vec::new(),
            },
        }
    }

    fn begin_session(&mut self) -> Vec<SpeechEffect> {
        self.session_sequence = self.session_sequence.saturating_add(1);
        self.phase = SpeechPhase::Arming;
        self.visible = true;
        self.partial.clear();
        self.stop_partial.clear();
        self.early_final = None;
        self.writeback_outcome = WritebackOutcome::NotDispatched;
        self.clear_expected();
        let identity = self.bump_identity();
        self.expected_prepare = Some(identity);
        vec![
            SpeechEffect::ShowOverlay { identity },
            SpeechEffect::PrepareStart { identity },
        ]
    }

    fn cancel_arming(&mut self) -> Vec<SpeechEffect> {
        let cancelled = self.expected_prepare.take().or(self.identity);
        self.phase = SpeechPhase::Idle;
        self.visible = false;
        let identity = self.bump_identity();
        let mut effects = vec![SpeechEffect::HideOverlay { identity }];
        if let Some(identity) = cancelled {
            effects.push(SpeechEffect::CancelStart { identity });
        }
        effects
    }

    fn on_start_prepared(&mut self, identity: SpeechLayerIdentity) -> Vec<SpeechEffect> {
        if self.phase != SpeechPhase::Arming
            || self.desired_state != DesiredState::On
            || self.expected_prepare != Some(identity)
        {
            return Vec::new();
        }
        self.expected_prepare = None;
        let identity = self.bump_identity();
        self.expected_native = Some(identity);
        vec![SpeechEffect::StartNative { identity }]
    }

    fn stop_listening(&mut self, now_us: u64) -> Vec<SpeechEffect> {
        self.visible = false;
        self.stop_partial = self.partial.clone();
        let native_identity = self
            .expected_native
            .take()
            .or(self.identity)
            .expect("listening requires a native identity");
        let hide_identity = self.bump_identity();
        let mut effects = vec![
            SpeechEffect::HideOverlay {
                identity: hide_identity,
            },
            SpeechEffect::FinishNative {
                identity: native_identity,
            },
        ];

        if let Some(text) = self.early_final.take().filter(|text| !text.is_empty()) {
            effects.extend(self.begin_processing(text, now_us));
        } else {
            self.phase = SpeechPhase::Finishing;
            self.expected_finish = Some(native_identity);
            effects.push(SpeechEffect::ScheduleFinishDeadline {
                identity: native_identity,
                deadline_us: now_us.saturating_add(MAX_FINISH_GRACE_US),
            });
        }
        effects
    }

    fn on_native_final(
        &mut self,
        identity: SpeechLayerIdentity,
        text: String,
        now_us: u64,
    ) -> Vec<SpeechEffect> {
        match self.phase {
            SpeechPhase::Listening if self.expected_native == Some(identity) => {
                if !text.is_empty() && self.early_final.is_none() {
                    self.early_final = Some(text);
                }
                Vec::new()
            }
            SpeechPhase::Finishing
                if self.expected_finish == Some(identity) && !text.is_empty() =>
            {
                self.expected_finish = None;
                self.begin_processing(text, now_us)
            }
            _ => Vec::new(),
        }
    }

    fn on_native_error(&mut self, identity: SpeechLayerIdentity, now_us: u64) -> Vec<SpeechEffect> {
        match self.phase {
            SpeechPhase::Arming if self.expected_native == Some(identity) => {
                self.desired_state = DesiredState::Off;
                self.visible = false;
                self.expected_native = None;
                self.phase = SpeechPhase::Terminal;
                let identity = self.bump_identity();
                vec![SpeechEffect::HideOverlay { identity }]
            }
            SpeechPhase::Listening if self.expected_native == Some(identity) => {
                self.desired_state = DesiredState::Off;
                self.visible = false;
                self.expected_native = None;
                self.phase = SpeechPhase::Terminal;
                let identity = self.bump_identity();
                vec![SpeechEffect::HideOverlay { identity }]
            }
            SpeechPhase::Finishing if self.expected_finish == Some(identity) => {
                self.expected_finish = None;
                let text = std::mem::take(&mut self.stop_partial);
                if text.is_empty() {
                    self.complete_terminal()
                } else {
                    self.begin_processing(text, now_us)
                }
            }
            _ => Vec::new(),
        }
    }

    fn begin_processing(&mut self, text: String, now_us: u64) -> Vec<SpeechEffect> {
        self.phase = SpeechPhase::Processing;
        let identity = self.bump_identity();
        self.expected_process = Some(identity);
        vec![
            SpeechEffect::ProcessTranscript { identity, text },
            SpeechEffect::ScheduleProcessingDeadline {
                identity,
                deadline_us: now_us.saturating_add(MAX_PROCESSING_GRACE_US),
            },
        ]
    }

    fn complete_terminal(&mut self) -> Vec<SpeechEffect> {
        self.phase = SpeechPhase::Terminal;
        self.visible = false;
        self.partial.clear();
        self.stop_partial.clear();
        self.early_final = None;
        self.clear_expected();
        self.bump_identity();
        if self.desired_state == DesiredState::On {
            self.begin_session()
        } else {
            Vec::new()
        }
    }

    fn bump_identity(&mut self) -> SpeechLayerIdentity {
        self.revision = self.revision.saturating_add(1);
        let identity = SpeechLayerIdentity::new(
            self.owner_epoch.expect("identity requires an owner"),
            self.control_seq,
            self.session_sequence,
            self.revision,
        );
        self.identity = Some(identity);
        identity
    }

    fn reset_for_owner(&mut self, owner_epoch: Option<OwnerEpoch>) {
        *self = Self {
            owner_epoch,
            ..Self::default()
        };
    }

    fn clear_expected(&mut self) {
        self.expected_prepare = None;
        self.expected_native = None;
        self.expected_finish = None;
        self.expected_process = None;
        self.expected_writeback = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enter_processing(reducer: &mut SpeechSessionReducer) -> SpeechLayerIdentity {
        reducer.apply(
            SpeechControlInput::OwnerAcquired {
                epoch: OwnerEpoch([7; 16]),
            },
            0,
        );
        let arming = reducer.apply(SpeechControlInput::FnPressed, 10);
        let prepared = arming
            .into_iter()
            .find_map(|effect| match effect {
                SpeechEffect::PrepareStart { identity } => Some(identity),
                _ => None,
            })
            .expect("Fn start must prepare native recognition");
        let native = reducer
            .apply(SpeechControlInput::StartPrepared { identity: prepared }, 20)
            .into_iter()
            .find_map(|effect| match effect {
                SpeechEffect::StartNative { identity } => Some(identity),
                _ => None,
            })
            .expect("prepared session must start native recognition");
        reducer.apply(SpeechControlInput::NativeStarted { identity: native }, 30);
        reducer.apply(SpeechControlInput::FnPressed, 40);
        reducer
            .apply(
                SpeechControlInput::NativeFinal {
                    identity: native,
                    text: "不会被吞掉".to_string(),
                },
                50,
            )
            .into_iter()
            .find_map(|effect| match effect {
                SpeechEffect::ScheduleProcessingDeadline {
                    identity,
                    deadline_us,
                } => {
                    assert_eq!(deadline_us, 50 + MAX_PROCESSING_GRACE_US);
                    Some(identity)
                }
                _ => None,
            })
            .expect("processing must schedule a watchdog")
    }

    #[test]
    fn processing_deadline_returns_to_terminal_when_frontend_never_completes() {
        let mut reducer = SpeechSessionReducer::default();
        let processing = enter_processing(&mut reducer);

        let effects = reducer.apply(
            SpeechControlInput::ProcessingDeadline {
                identity: processing,
            },
            60,
        );

        assert!(effects.is_empty());
        let snapshot = reducer.snapshot();
        assert_eq!(snapshot.phase, SpeechPhase::Terminal);
        assert_eq!(snapshot.desired_state, DesiredState::Off);
        assert!(!snapshot.visible);
    }

    #[test]
    fn stale_processing_deadline_cannot_cancel_the_current_session() {
        let mut reducer = SpeechSessionReducer::default();
        let processing = enter_processing(&mut reducer);
        let stale = SpeechLayerIdentity {
            revision: processing.revision.saturating_add(1),
            ..processing
        };

        let effects = reducer.apply(
            SpeechControlInput::ProcessingDeadline { identity: stale },
            60,
        );

        assert!(effects.is_empty());
        let snapshot = reducer.snapshot();
        assert_eq!(snapshot.phase, SpeechPhase::Processing);
        assert_eq!(snapshot.desired_state, DesiredState::Off);
        assert!(snapshot.visible == false);
    }
}
