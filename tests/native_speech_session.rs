use cunzhi::native_speech::session::*;

fn epoch(byte: u8) -> OwnerEpoch {
    OwnerEpoch([byte; 16])
}

fn owned(byte: u8) -> SpeechSessionReducer {
    let mut reducer = SpeechSessionReducer::default();
    reducer.apply(SpeechControlInput::OwnerAcquired { epoch: epoch(byte) }, 1);
    reducer
}

fn effect_identity(
    effects: &[SpeechEffect],
    pick: impl Fn(&SpeechEffect) -> Option<SpeechLayerIdentity>,
) -> SpeechLayerIdentity {
    effects.iter().find_map(pick).expect("matching effect")
}

fn start_session(
    reducer: &mut SpeechSessionReducer,
    now: u64,
) -> (SpeechLayerIdentity, Vec<SpeechEffect>) {
    let effects = reducer.apply(SpeechControlInput::FnPressed, now);
    let identity = effect_identity(&effects, |effect| match effect {
        SpeechEffect::PrepareStart { identity } => Some(*identity),
        _ => None,
    });
    (identity, effects)
}

fn enter_listening(reducer: &mut SpeechSessionReducer, now: u64) -> SpeechLayerIdentity {
    let (prepare, _) = start_session(reducer, now);
    let effects = reducer.apply(
        SpeechControlInput::StartPrepared { identity: prepare },
        now + 1,
    );
    let native = effect_identity(&effects, |effect| match effect {
        SpeechEffect::StartNative { identity } => Some(*identity),
        _ => None,
    });
    reducer.apply(
        SpeechControlInput::NativeStarted { identity: native },
        now + 2,
    );
    native
}

fn finish_identity(effects: &[SpeechEffect]) -> SpeechLayerIdentity {
    effect_identity(effects, |effect| match effect {
        SpeechEffect::FinishNative { identity } => Some(*identity),
        _ => None,
    })
}

fn processing_identity(effects: &[SpeechEffect]) -> SpeechLayerIdentity {
    effect_identity(effects, |effect| match effect {
        SpeechEffect::ProcessTranscript { identity, .. } => Some(*identity),
        _ => None,
    })
}

fn drive_to_dispatch(reducer: &mut SpeechSessionReducer, now: u64) -> SpeechLayerIdentity {
    let native = enter_listening(reducer, now);
    reducer.apply(
        SpeechControlInput::NativeFinal {
            identity: native,
            text: "final text".into(),
        },
        now + 3,
    );
    let stop = reducer.apply(SpeechControlInput::FnPressed, now + 4);
    let process = processing_identity(&stop);
    let dispatch = reducer.apply(
        SpeechControlInput::TranscriptProcessed {
            identity: process,
            text: "processed text".into(),
        },
        now + 5,
    );
    let identity = effect_identity(&dispatch, |effect| match effect {
        SpeechEffect::DispatchWriteback { identity, .. } => Some(*identity),
        _ => None,
    });
    reducer.apply(
        SpeechControlInput::WritebackDispatched { identity },
        now + 6,
    );
    identity
}

#[test]
fn owner_acquisition_enters_idle_off() {
    let reducer = owned(1);
    let snapshot = reducer.snapshot();
    assert_eq!(snapshot.phase, SpeechPhase::Idle);
    assert_eq!(snapshot.desired_state, DesiredState::Off);
    assert!(!snapshot.visible);
    assert_eq!(snapshot.owner_epoch, Some(epoch(1)));
}

#[test]
fn first_fn_creates_one_session_and_prepares_start() {
    let mut reducer = owned(2);
    let (_, effects) = start_session(&mut reducer, 10);
    assert_eq!(reducer.snapshot().phase, SpeechPhase::Arming);
    assert_eq!(reducer.snapshot().desired_state, DesiredState::On);
    assert!(reducer.snapshot().visible);
    assert_eq!(
        effects
            .iter()
            .filter(|effect| matches!(effect, SpeechEffect::ShowOverlay { .. }))
            .count(),
        1
    );
    assert_eq!(
        effects
            .iter()
            .filter(|effect| matches!(effect, SpeechEffect::PrepareStart { .. }))
            .count(),
        1
    );
}

#[test]
fn second_fn_while_arming_hides_and_cancels_without_commit() {
    let mut reducer = owned(3);
    start_session(&mut reducer, 10);
    let effects = reducer.apply(SpeechControlInput::FnPressed, 11);
    assert_eq!(reducer.snapshot().desired_state, DesiredState::Off);
    assert!(!reducer.snapshot().visible);
    assert!(effects
        .iter()
        .any(|effect| matches!(effect, SpeechEffect::HideOverlay { .. })));
    assert!(effects
        .iter()
        .any(|effect| matches!(effect, SpeechEffect::CancelStart { .. })));
    assert!(!effects.iter().any(|effect| matches!(
        effect,
        SpeechEffect::ProcessTranscript { .. } | SpeechEffect::DispatchWriteback { .. }
    )));
}

#[test]
fn ready_after_double_fn_cannot_resurrect_cancelled_session() {
    let mut reducer = owned(4);
    let (old, _) = start_session(&mut reducer, 10);
    reducer.apply(SpeechControlInput::FnPressed, 11);
    let before = reducer.snapshot();
    let effects = reducer.apply(SpeechControlInput::StartPrepared { identity: old }, 12);
    assert!(effects.is_empty());
    assert_eq!(reducer.snapshot(), before);
}

#[test]
fn on_off_on_creates_new_session_and_rejects_old_callbacks() {
    let mut reducer = owned(5);
    let (old, _) = start_session(&mut reducer, 10);
    reducer.apply(SpeechControlInput::FnPressed, 11);
    let (new, _) = start_session(&mut reducer, 12);
    assert_ne!(old.session_sequence, new.session_sequence);
    let before = reducer.snapshot();
    assert!(reducer
        .apply(SpeechControlInput::StartPrepared { identity: old }, 13)
        .is_empty());
    assert_eq!(reducer.snapshot(), before);
    assert!(reducer
        .apply(SpeechControlInput::StartPrepared { identity: new }, 14)
        .iter()
        .any(|effect| matches!(effect, SpeechEffect::StartNative { .. })));
}

#[test]
fn partial_then_stop_snapshots_partial_and_schedules_bounded_finish() {
    assert!(
        MAX_FINISH_GRACE_US >= 3_000_000,
        "the reducer must not expire before the native bridge's three-second final grace"
    );
    let mut reducer = owned(6);
    let native = enter_listening(&mut reducer, 100);
    reducer.apply(
        SpeechControlInput::NativePartial {
            identity: native,
            text: "partial".into(),
        },
        103,
    );
    let effects = reducer.apply(SpeechControlInput::FnPressed, 104);
    assert_eq!(reducer.snapshot().phase, SpeechPhase::Finishing);
    assert_eq!(reducer.snapshot().partial_len, 7);
    assert!(!reducer.snapshot().visible);
    let deadline = effects
        .iter()
        .find_map(|effect| match effect {
            SpeechEffect::ScheduleFinishDeadline { deadline_us, .. } => Some(*deadline_us),
            _ => None,
        })
        .expect("deadline");
    assert!(deadline > 104);
    assert!(deadline - 104 <= MAX_FINISH_GRACE_US);
}

#[test]
fn finish_keeps_the_started_native_layer_identity_while_overlay_revision_advances() {
    let mut reducer = owned(16);
    let native = enter_listening(&mut reducer, 100);
    let effects = reducer.apply(SpeechControlInput::FnPressed, 104);
    let finish = finish_identity(&effects);
    let hide = effect_identity(&effects, |effect| match effect {
        SpeechEffect::HideOverlay { identity } => Some(*identity),
        _ => None,
    });

    assert_eq!(finish, native);
    assert!(hide.revision > native.revision);
    assert!(reducer
        .apply(
            SpeechControlInput::NativeFinal {
                identity: native,
                text: "final".into(),
            },
            105,
        )
        .iter()
        .any(|effect| matches!(effect, SpeechEffect::ProcessTranscript { text, .. } if text == "final")));
}

#[test]
fn early_final_is_cached_until_stop_and_committed_once() {
    let mut reducer = owned(7);
    let native = enter_listening(&mut reducer, 100);
    assert!(reducer
        .apply(
            SpeechControlInput::NativeFinal {
                identity: native,
                text: "cached final".into(),
            },
            103,
        )
        .is_empty());
    assert_eq!(reducer.snapshot().phase, SpeechPhase::Listening);
    let effects = reducer.apply(SpeechControlInput::FnPressed, 104);
    assert_eq!(
        effects
            .iter()
            .filter(|effect| matches!(effect, SpeechEffect::ProcessTranscript { text, .. } if text == "cached final"))
            .count(),
        1
    );
}

#[test]
fn final_wins_deadline_uses_stop_partial_and_empty_commits_nothing() {
    let mut final_case = owned(8);
    let native = enter_listening(&mut final_case, 100);
    final_case.apply(
        SpeechControlInput::NativePartial {
            identity: native,
            text: "partial".into(),
        },
        103,
    );
    let stop = final_case.apply(SpeechControlInput::FnPressed, 104);
    let finish = finish_identity(&stop);
    let final_effects = final_case.apply(
        SpeechControlInput::NativeFinal {
            identity: finish,
            text: "final".into(),
        },
        105,
    );
    assert!(final_effects.iter().any(
        |effect| matches!(effect, SpeechEffect::ProcessTranscript { text, .. } if text == "final")
    ));
    assert!(final_case
        .apply(SpeechControlInput::FinishDeadline { identity: finish }, 200)
        .is_empty());

    let mut partial_case = owned(9);
    let native = enter_listening(&mut partial_case, 300);
    partial_case.apply(
        SpeechControlInput::NativePartial {
            identity: native,
            text: "fallback".into(),
        },
        303,
    );
    let stop = partial_case.apply(SpeechControlInput::FnPressed, 304);
    let finish = finish_identity(&stop);
    let deadline = partial_case.apply(SpeechControlInput::FinishDeadline { identity: finish }, 400);
    assert!(deadline.iter().any(|effect| matches!(effect, SpeechEffect::ProcessTranscript { text, .. } if text == "fallback")));
    assert!(partial_case
        .apply(SpeechControlInput::FinishDeadline { identity: finish }, 401)
        .is_empty());

    let mut empty_case = owned(10);
    enter_listening(&mut empty_case, 500);
    let stop = empty_case.apply(SpeechControlInput::FnPressed, 504);
    let finish = finish_identity(&stop);
    let deadline = empty_case.apply(SpeechControlInput::FinishDeadline { identity: finish }, 600);
    assert!(!deadline.iter().any(|effect| matches!(
        effect,
        SpeechEffect::ProcessTranscript { .. } | SpeechEffect::DispatchWriteback { .. }
    )));
    assert_eq!(empty_case.snapshot().phase, SpeechPhase::Terminal);
}

#[test]
fn duplicate_final_processing_ack_and_deadline_cannot_duplicate_dispatch() {
    let mut reducer = owned(11);
    let native = enter_listening(&mut reducer, 100);
    reducer.apply(
        SpeechControlInput::NativeFinal {
            identity: native,
            text: "one".into(),
        },
        103,
    );
    let stop = reducer.apply(SpeechControlInput::FnPressed, 104);
    let finish = finish_identity(&stop);
    let process = processing_identity(&stop);
    assert!(reducer
        .apply(
            SpeechControlInput::NativeFinal {
                identity: finish,
                text: "two".into()
            },
            105
        )
        .is_empty());
    let dispatch = reducer.apply(
        SpeechControlInput::TranscriptProcessed {
            identity: process,
            text: "processed".into(),
        },
        106,
    );
    let writeback = effect_identity(&dispatch, |effect| match effect {
        SpeechEffect::DispatchWriteback { identity, .. } => Some(*identity),
        _ => None,
    });
    reducer.apply(
        SpeechControlInput::WritebackDispatched {
            identity: writeback,
        },
        106,
    );
    assert_eq!(
        dispatch
            .iter()
            .filter(|effect| matches!(effect, SpeechEffect::DispatchWriteback { .. }))
            .count(),
        1
    );
    assert!(reducer
        .apply(
            SpeechControlInput::TranscriptProcessed {
                identity: process,
                text: "again".into()
            },
            107
        )
        .is_empty());
    assert!(reducer
        .apply(SpeechControlInput::FinishDeadline { identity: finish }, 108)
        .is_empty());
    reducer.apply(
        SpeechControlInput::WritebackAcknowledged {
            identity: writeback,
        },
        109,
    );
    assert!(reducer
        .apply(
            SpeechControlInput::WritebackAcknowledged {
                identity: writeback
            },
            110
        )
        .is_empty());
}

#[test]
fn writeback_failure_before_dispatch_is_clean_and_never_replayed() {
    let mut reducer = owned(18);
    let native = enter_listening(&mut reducer, 100);
    reducer.apply(
        SpeechControlInput::NativeFinal {
            identity: native,
            text: "final".into(),
        },
        103,
    );
    let stop = reducer.apply(SpeechControlInput::FnPressed, 104);
    let process = processing_identity(&stop);
    let dispatch = reducer.apply(
        SpeechControlInput::TranscriptProcessed {
            identity: process,
            text: "processed".into(),
        },
        105,
    );
    let writeback = effect_identity(&dispatch, |effect| match effect {
        SpeechEffect::DispatchWriteback { identity, .. } => Some(*identity),
        _ => None,
    });
    reducer.apply(
        SpeechControlInput::WritebackDispatchFailed {
            identity: writeback,
        },
        106,
    );

    assert_eq!(
        reducer.snapshot().writeback_outcome,
        WritebackOutcome::FailedBeforeDispatch
    );
    assert!(reducer
        .apply(SpeechControlInput::RecoverAfterCrash, 107)
        .is_empty());
    assert_eq!(
        reducer.snapshot().writeback_outcome,
        WritebackOutcome::FailedBeforeDispatch
    );
}

#[test]
fn missing_ack_after_dispatch_is_unknown_and_cannot_be_replayed() {
    let mut reducer = owned(19);
    let writeback = drive_to_dispatch(&mut reducer, 100);
    reducer.apply(
        SpeechControlInput::WritebackFailed {
            identity: writeback,
        },
        107,
    );
    assert_eq!(
        reducer.snapshot().writeback_outcome,
        WritebackOutcome::UnknownAfterDispatch
    );
    assert!(reducer
        .apply(
            SpeechControlInput::WritebackAcknowledged {
                identity: writeback,
            },
            108,
        )
        .is_empty());
    assert!(reducer
        .apply(SpeechControlInput::RecoverAfterCrash, 109)
        .is_empty());
}

#[test]
fn unverified_external_dispatch_completes_without_ack_or_replay() {
    let mut reducer = owned(20);
    let writeback = drive_to_dispatch(&mut reducer, 100);
    let mut stale = writeback;
    stale.revision = stale.revision.saturating_add(1);
    let dispatched = reducer.snapshot();

    assert!(reducer
        .apply(
            SpeechControlInput::WritebackUnverified { identity: stale },
            107,
        )
        .is_empty());
    assert_eq!(reducer.snapshot(), dispatched);

    assert!(reducer
        .apply(
            SpeechControlInput::WritebackUnverified {
                identity: writeback,
            },
            108,
        )
        .is_empty());
    let terminal = reducer.snapshot();
    assert_eq!(terminal.phase, SpeechPhase::Terminal);
    assert_eq!(terminal.desired_state, DesiredState::Off);
    assert_eq!(
        terminal.writeback_outcome,
        WritebackOutcome::DispatchedUnverified
    );
    assert!(!terminal.visible);

    assert!(reducer
        .apply(
            SpeechControlInput::WritebackAcknowledged {
                identity: writeback,
            },
            109,
        )
        .is_empty());
    assert!(reducer
        .apply(
            SpeechControlInput::WritebackUnverified {
                identity: writeback,
            },
            110,
        )
        .is_empty());
    assert!(reducer
        .apply(SpeechControlInput::RecoverAfterCrash, 111)
        .is_empty());
    assert_eq!(reducer.snapshot(), terminal);
}

#[test]
fn fn_while_committing_retains_latest_intent_and_starts_fresh_after_terminal() {
    let mut reducer = owned(12);
    let old_session = reducer.snapshot().session;
    let writeback = drive_to_dispatch(&mut reducer, 100);
    let committing_session = reducer.snapshot().session.expect("committing session");
    let effects = reducer.apply(SpeechControlInput::FnPressed, 106);
    assert!(effects.is_empty());
    assert_eq!(reducer.snapshot().desired_state, DesiredState::On);
    assert_eq!(reducer.snapshot().session, Some(committing_session));
    let restart = reducer.apply(
        SpeechControlInput::WritebackAcknowledged {
            identity: writeback,
        },
        107,
    );
    let new_session = reducer.snapshot().session.expect("new session");
    assert_ne!(Some(new_session), old_session);
    assert_ne!(new_session, committing_session);
    assert!(restart
        .iter()
        .any(|effect| matches!(effect, SpeechEffect::ShowOverlay { .. })));
}

#[test]
fn stale_epoch_session_control_revision_and_overlay_replies_are_noops() {
    let mut reducer = owned(13);
    let (valid, _) = start_session(&mut reducer, 10);
    let before = reducer.snapshot();
    let stale_values = [
        SpeechLayerIdentity {
            owner_epoch_hi: valid.owner_epoch_hi ^ 1,
            ..valid
        },
        SpeechLayerIdentity {
            session_sequence: valid.session_sequence + 1,
            ..valid
        },
        SpeechLayerIdentity {
            control_seq: valid.control_seq.saturating_sub(1),
            ..valid
        },
        SpeechLayerIdentity {
            revision: valid.revision.saturating_sub(1),
            ..valid
        },
    ];
    for stale in stale_values {
        assert!(reducer
            .apply(SpeechControlInput::StartPrepared { identity: stale }, 11)
            .is_empty());
        assert!(reducer
            .apply(
                SpeechControlInput::OverlayVisibilityAcknowledged {
                    identity: stale,
                    visible: true
                },
                11
            )
            .is_empty());
        assert_eq!(reducer.snapshot(), before);
    }
}

#[test]
fn shuffled_stale_events_never_mutate_and_latest_fn_parity_wins() {
    let mut reducer = owned(14);
    let mut seed = 0x1234_5678_u64;
    for index in 0..1_000_u64 {
        let effects = reducer.apply(SpeechControlInput::FnPressed, index + 10);
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut stale = effects
            .iter()
            .find_map(|effect| effect.identity())
            .unwrap_or_else(|| reducer.snapshot().identity.expect("identity"));
        stale.owner_epoch_lo ^= seed | 1;
        let before = reducer.snapshot();
        assert!(reducer
            .apply(
                SpeechControlInput::StartPrepared { identity: stale },
                index + 20
            )
            .is_empty());
        assert_eq!(reducer.snapshot(), before);
        let expected = if index % 2 == 0 {
            DesiredState::On
        } else {
            DesiredState::Off
        };
        assert_eq!(reducer.snapshot().desired_state, expected);
    }
}

#[test]
fn two_epochs_can_reuse_numbers_without_accepting_each_others_events() {
    let mut reducer = owned(15);
    let (old, _) = start_session(&mut reducer, 10);
    reducer.apply(SpeechControlInput::OwnerAcquired { epoch: epoch(16) }, 20);
    let (new, _) = start_session(&mut reducer, 21);
    assert_eq!(old.session_sequence, new.session_sequence);
    assert_eq!(old.control_seq, new.control_seq);
    assert_ne!(old.owner_epoch_hi, new.owner_epoch_hi);
    let before = reducer.snapshot();
    assert!(reducer
        .apply(SpeechControlInput::StartPrepared { identity: old }, 22)
        .is_empty());
    assert_eq!(reducer.snapshot(), before);
}

#[test]
fn ambiguous_post_dispatch_recovery_is_unknown_and_never_retries() {
    let mut reducer = owned(17);
    let writeback = drive_to_dispatch(&mut reducer, 100);
    assert_eq!(
        reducer.snapshot().writeback_outcome,
        WritebackOutcome::Dispatched
    );
    let recovery = reducer.apply(SpeechControlInput::RecoverAfterCrash, 106);
    assert!(recovery.is_empty());
    assert_eq!(
        reducer.snapshot().writeback_outcome,
        WritebackOutcome::UnknownAfterDispatch
    );
    assert!(!reducer.snapshot().visible);
    assert!(reducer
        .apply(
            SpeechControlInput::WritebackAcknowledged {
                identity: writeback
            },
            107
        )
        .is_empty());
    assert!(reducer
        .apply(SpeechControlInput::RecoverAfterCrash, 108)
        .is_empty());
}

#[test]
fn epoch_json_ffi_and_repr_c_identity_round_trip() {
    let value = OwnerEpoch([
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee,
        0xff,
    ]);
    let canonical = value.to_canonical_string();
    assert_eq!(canonical, "00112233-4455-6677-8899-aabbccddeeff");
    assert_eq!(OwnerEpoch::parse_canonical(&canonical), Some(value));
    let (hi, lo) = value.to_be_halves();
    assert_eq!(OwnerEpoch::from_be_halves(hi, lo), value);
    let identity = SpeechLayerIdentity::new(value, 3, 4, 5);
    assert_eq!(identity.owner_epoch(), value);
    let wire = serde_json::to_value(identity).expect("identity JSON");
    assert_eq!(wire["owner_epoch_hi"], hi.to_string());
    assert_eq!(wire["owner_epoch_lo"], lo.to_string());
    assert_eq!(wire["control_seq"], "3");
    assert_eq!(
        serde_json::from_value::<SpeechLayerIdentity>(wire).expect("identity JSON round trip"),
        identity
    );
    assert_eq!(std::mem::size_of::<SpeechLayerIdentity>(), 48);
    assert_eq!(std::mem::align_of::<SpeechLayerIdentity>(), 8);
}
