use cunzhi::native_speech::coordinator::{
    MonotonicClock, SpeechCompletionSender, SpeechCoordinatorHandle, SpeechEffectExecutor,
};
use cunzhi::native_speech::session::{
    DesiredState, OwnerEpoch, SpeechControlInput, SpeechEffect, SpeechPhase,
};
use cunzhi::native_speech::trace::{
    SpeechTrace, SpeechTraceFields, SpeechTraceFinalSource, SpeechTraceMilestone,
    SpeechTraceOutcome, SpeechTraceTargetKind,
};
use std::sync::{Arc, Mutex};
use std::{fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[derive(Default)]
struct FakeClock(Mutex<u64>);
impl MonotonicClock for FakeClock {
    fn now_us(&self) -> u64 {
        *self.0.lock().unwrap()
    }
}

#[derive(Default)]
struct FakeExecutor {
    effects: Mutex<Vec<SpeechEffect>>,
    completions: Mutex<Vec<SpeechCompletionSender>>,
}
impl SpeechEffectExecutor for FakeExecutor {
    fn execute(&self, effect: SpeechEffect, completion: SpeechCompletionSender) {
        self.effects.lock().unwrap().push(effect);
        self.completions.lock().unwrap().push(completion);
    }
}

fn coordinator() -> (SpeechCoordinatorHandle, Arc<FakeExecutor>) {
    let executor = Arc::new(FakeExecutor::default());
    let handle = SpeechCoordinatorHandle::new(executor.clone(), Arc::new(FakeClock::default()), 32);
    handle
        .send(SpeechControlInput::OwnerAcquired {
            epoch: OwnerEpoch([1; 16]),
        })
        .unwrap();
    (handle, executor)
}

#[test]
fn held_start_completion_cannot_block_or_resurrect_after_second_fn() {
    let (coordinator, executor) = coordinator();
    coordinator.send(SpeechControlInput::FnPressed).unwrap();
    let prepare = executor
        .effects
        .lock()
        .unwrap()
        .iter()
        .find_map(|effect| match effect {
            SpeechEffect::PrepareStart { identity } => Some(*identity),
            _ => None,
        })
        .unwrap();

    coordinator.send(SpeechControlInput::FnPressed).unwrap();
    assert_eq!(coordinator.snapshot().desired_state, DesiredState::Off);
    assert!(executor
        .effects
        .lock()
        .unwrap()
        .iter()
        .any(|effect| matches!(effect, SpeechEffect::HideOverlay { .. })));
    coordinator
        .send(SpeechControlInput::StartPrepared { identity: prepare })
        .unwrap();
    assert!(!executor
        .effects
        .lock()
        .unwrap()
        .iter()
        .any(|effect| matches!(effect, SpeechEffect::StartNative { .. })));
}

#[test]
fn effect_completion_reenters_the_same_serialized_queue() {
    let (coordinator, executor) = coordinator();
    coordinator.send(SpeechControlInput::FnPressed).unwrap();
    let prepare = executor
        .effects
        .lock()
        .unwrap()
        .iter()
        .find_map(|effect| match effect {
            SpeechEffect::PrepareStart { identity } => Some(*identity),
            _ => None,
        })
        .unwrap();
    executor.completions.lock().unwrap()[0]
        .send(SpeechControlInput::StartPrepared { identity: prepare })
        .unwrap();
    coordinator.flush().unwrap();
    assert!(executor
        .effects
        .lock()
        .unwrap()
        .iter()
        .any(|effect| matches!(effect, SpeechEffect::StartNative { .. })));
}

#[test]
fn hide_is_dispatched_before_finish_and_only_one_deadline_is_scheduled() {
    let (coordinator, executor) = coordinator();
    coordinator.send(SpeechControlInput::FnPressed).unwrap();
    let prepare = executor
        .effects
        .lock()
        .unwrap()
        .iter()
        .find_map(|effect| effect.identity())
        .unwrap();
    coordinator
        .send(SpeechControlInput::StartPrepared { identity: prepare })
        .unwrap();
    let native = executor
        .effects
        .lock()
        .unwrap()
        .iter()
        .rev()
        .find_map(|effect| match effect {
            SpeechEffect::StartNative { identity } => Some(*identity),
            _ => None,
        })
        .unwrap();
    coordinator
        .send(SpeechControlInput::NativeStarted { identity: native })
        .unwrap();
    coordinator.send(SpeechControlInput::FnPressed).unwrap();
    let effects = executor.effects.lock().unwrap();
    let hide = effects
        .iter()
        .position(|effect| matches!(effect, SpeechEffect::HideOverlay { .. }))
        .unwrap();
    let finish = effects
        .iter()
        .position(|effect| matches!(effect, SpeechEffect::FinishNative { .. }))
        .unwrap();
    assert!(hide < finish);
    assert_eq!(
        effects
            .iter()
            .filter(|effect| matches!(effect, SpeechEffect::ScheduleFinishDeadline { .. }))
            .count(),
        1
    );
    assert_eq!(coordinator.snapshot().phase, SpeechPhase::Finishing);
}

#[test]
fn bounded_input_overflow_is_explicit_not_silent() {
    let executor = Arc::new(FakeExecutor::default());
    let coordinator = SpeechCoordinatorHandle::new(executor, Arc::new(FakeClock::default()), 1);
    coordinator.pause_for_test();
    coordinator
        .send(SpeechControlInput::OwnerAcquired {
            epoch: OwnerEpoch([2; 16]),
        })
        .unwrap();
    assert!(coordinator.send(SpeechControlInput::FnPressed).is_err());
}

fn enter_listening(
    coordinator: &SpeechCoordinatorHandle,
    executor: &FakeExecutor,
) -> cunzhi::native_speech::session::SpeechLayerIdentity {
    coordinator.send(SpeechControlInput::FnPressed).unwrap();
    let prepare = executor
        .effects
        .lock()
        .unwrap()
        .iter()
        .rev()
        .find_map(|effect| match effect {
            SpeechEffect::PrepareStart { identity } => Some(*identity),
            _ => None,
        })
        .unwrap();
    coordinator
        .send(SpeechControlInput::StartPrepared { identity: prepare })
        .unwrap();
    let native = executor
        .effects
        .lock()
        .unwrap()
        .iter()
        .rev()
        .find_map(|effect| match effect {
            SpeechEffect::StartNative { identity } => Some(*identity),
            _ => None,
        })
        .unwrap();
    coordinator
        .send(SpeechControlInput::NativeStarted { identity: native })
        .unwrap();
    native
}

#[test]
fn stale_and_duplicate_deadlines_cannot_process_twice() {
    let (coordinator, executor) = coordinator();
    let native = enter_listening(&coordinator, &executor);
    coordinator
        .send(SpeechControlInput::NativePartial {
            identity: native,
            text: "fallback".into(),
        })
        .unwrap();
    coordinator.send(SpeechControlInput::FnPressed).unwrap();
    let finish = executor
        .effects
        .lock()
        .unwrap()
        .iter()
        .rev()
        .find_map(|effect| match effect {
            SpeechEffect::ScheduleFinishDeadline { identity, .. } => Some(*identity),
            _ => None,
        })
        .unwrap();
    let mut stale = finish;
    stale.revision = stale.revision.saturating_sub(1);

    coordinator
        .send(SpeechControlInput::FinishDeadline { identity: stale })
        .unwrap();
    coordinator
        .send(SpeechControlInput::FinishDeadline { identity: finish })
        .unwrap();
    coordinator
        .send(SpeechControlInput::FinishDeadline { identity: finish })
        .unwrap();

    assert_eq!(
        executor
            .effects
            .lock()
            .unwrap()
            .iter()
            .filter(|effect| matches!(effect, SpeechEffect::ProcessTranscript { .. }))
            .count(),
        1
    );
}

#[test]
fn reordered_overlay_acknowledgements_reconcile_to_authoritative_snapshot() {
    let (coordinator, executor) = coordinator();
    coordinator.send(SpeechControlInput::FnPressed).unwrap();
    let authoritative = coordinator.snapshot();
    let current = authoritative.identity.unwrap();
    let mut stale = current;
    stale.revision = stale.revision.saturating_sub(1);

    coordinator
        .send(SpeechControlInput::OverlayVisibilityAcknowledged {
            identity: stale,
            visible: false,
        })
        .unwrap();
    coordinator
        .send(SpeechControlInput::OverlayVisibilityAcknowledged {
            identity: current,
            visible: false,
        })
        .unwrap();

    assert_eq!(coordinator.snapshot(), authoritative);
    assert!(executor
        .effects
        .lock()
        .unwrap()
        .iter()
        .any(|effect| matches!(effect, SpeechEffect::ShowOverlay { .. })));
}

#[test]
fn trace_has_required_milestones_and_lengths_but_no_transcript_field() {
    assert_eq!(SpeechTraceMilestone::REQUIRED.len(), 30);
    assert!(SpeechTraceMilestone::REQUIRED.contains(&SpeechTraceMilestone::WritebackUnverified));
    let trace = SpeechTrace::new(1_000, 8);
    trace.record(
        1_125,
        SpeechTraceMilestone::NativeFinalReceived,
        SpeechTraceFields {
            phase: SpeechPhase::Finishing,
            outcome: SpeechTraceOutcome::Success,
            target_kind: SpeechTraceTargetKind::Application,
            final_source: SpeechTraceFinalSource::NativeFinal,
            text_len: 20,
            stale_count: 2,
            dispatch_count: 1,
            wall_clock_unix_ms: Some(1_720_000_000_000),
            ..SpeechTraceFields::default()
        },
    );

    let records = trace.records();
    assert_eq!(records[0].elapsed_us, 125);
    assert_eq!(records[0].text_len, 20);
    assert_eq!(records[0].stale_count, 2);
    assert_eq!(records[0].dispatch_count, 1);
    let debug = format!("{records:?}");
    assert!(!debug.contains("secret transcript"));
    assert!(!debug.contains("text:"));
}

#[test]
fn coordinator_trace_records_control_and_dispatch_without_transcript_content() {
    let (coordinator, _) = coordinator();
    coordinator.send(SpeechControlInput::FnPressed).unwrap();
    let records = coordinator.trace().records();
    let milestones: Vec<_> = records.iter().map(|record| record.milestone).collect();

    assert!(milestones.contains(&SpeechTraceMilestone::OwnerAcquired));
    assert!(milestones.contains(&SpeechTraceMilestone::FnDown));
    assert!(milestones.contains(&SpeechTraceMilestone::IntentAccepted));
    assert!(milestones.contains(&SpeechTraceMilestone::OverlayRevealRequested));
    assert!(milestones.contains(&SpeechTraceMilestone::TargetCaptureRequested));
    assert!(!format!("{records:?}").contains("transcript"));
}

#[test]
fn dropping_the_last_handle_joins_the_worker_and_releases_executor() {
    let (coordinator, executor) = coordinator();
    assert_eq!(Arc::strong_count(&executor), 2);

    drop(coordinator);

    assert_eq!(Arc::strong_count(&executor), 1);
}

#[test]
fn phase1_popup_dispatch_requires_typed_ack_and_never_blind_pastes() {
    let phase1 = fs::read_to_string(root().join("src/rust/native_speech/phase1.rs")).unwrap();
    let speech = fs::read_to_string(root().join("src/rust/native_speech/mod.rs")).unwrap();
    let target = fs::read_to_string(root().join("src/rust/native_speech/target.rs")).unwrap();

    assert!(!phase1.contains("Ok(()) => SpeechControlInput::WritebackAcknowledged"));
    assert!(phase1.contains("ack_popup_speech_insert"));
    assert!(!speech.contains("own-app-pid-paste-fallback"));
    assert!(speech.contains("insert_id"));
    assert!(target.contains("speech_bridge_application_matches_identity"));
}
