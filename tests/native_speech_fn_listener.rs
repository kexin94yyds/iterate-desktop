use cunzhi::native_speech::fn_listener::{
    deliver_terminal_event, FnEdgeDecoder, FnListenerEvent, FnListenerHandle, FnRawEvent,
    PressDelivery, TapDisabledReason, CONTROL_TAP_MAX_US,
};
use std::sync::mpsc::{channel, TryRecvError};

#[test]
fn one_fn_down_edge_emits_once_until_release() {
    let mut decoder = FnEdgeDecoder::default();
    assert_eq!(
        decoder.decode(FnRawEvent::flags_changed(63, true, 10)),
        Some(FnListenerEvent::Pressed {
            observed_monotonic_us: 10,
        })
    );
    assert_eq!(
        decoder.decode(FnRawEvent::flags_changed(63, true, 11)),
        None
    );
    assert_eq!(
        decoder.decode(FnRawEvent::flags_changed(63, false, 12)),
        Some(FnListenerEvent::Released {
            observed_monotonic_us: 12,
        })
    );
    assert_eq!(
        decoder.decode(FnRawEvent::flags_changed(63, true, 13)),
        Some(FnListenerEvent::Pressed {
            observed_monotonic_us: 13,
        })
    );
}

#[test]
fn unrelated_flags_changed_events_do_nothing() {
    let mut decoder = FnEdgeDecoder::default();
    assert_eq!(
        decoder.decode(FnRawEvent::flags_changed(55, true, 10)),
        None
    );
    assert_eq!(decoder.decode(FnRawEvent::other(11)), None);
}

#[test]
fn bare_left_or_right_control_tap_emits_only_on_release() {
    for keycode in [59, 62] {
        let mut decoder = FnEdgeDecoder::default();
        assert_eq!(
            decoder.decode(FnRawEvent::flags_changed_with_modifiers(
                keycode, false, true, false, 100,
            )),
            None
        );
        assert_eq!(
            decoder.decode(FnRawEvent::flags_changed_with_modifiers(
                keycode, false, false, false, 200,
            )),
            Some(FnListenerEvent::ControlTapped)
        );
    }
}

#[test]
fn control_shortcut_or_mouse_gesture_never_emits_a_bare_control_tap() {
    for interference in [
        FnRawEvent::gesture_interference(150),
        FnRawEvent::flags_changed_with_modifiers(56, false, true, true, 150),
    ] {
        let mut decoder = FnEdgeDecoder::default();
        assert_eq!(
            decoder.decode(FnRawEvent::flags_changed_with_modifiers(
                59, false, true, false, 100,
            )),
            None
        );
        assert_eq!(decoder.decode(interference), None);
        assert_eq!(
            decoder.decode(FnRawEvent::flags_changed_with_modifiers(
                59, false, false, false, 200,
            )),
            None
        );
    }
}

#[test]
fn held_control_and_control_with_another_modifier_do_not_toggle_mute() {
    let mut held = FnEdgeDecoder::default();
    assert_eq!(
        held.decode(FnRawEvent::flags_changed_with_modifiers(
            59, false, true, false, 100,
        )),
        None
    );
    assert_eq!(
        held.decode(FnRawEvent::flags_changed_with_modifiers(
            59,
            false,
            false,
            false,
            100 + CONTROL_TAP_MAX_US + 1,
        )),
        None
    );

    let mut modified = FnEdgeDecoder::default();
    assert_eq!(
        modified.decode(FnRawEvent::flags_changed_with_modifiers(
            59, false, true, true, 100,
        )),
        None
    );
    assert_eq!(
        modified.decode(FnRawEvent::flags_changed_with_modifiers(
            59, false, false, true, 200,
        )),
        None
    );
}

#[test]
fn queued_press_edges_are_delivered_without_coalescing() {
    let delivery = PressDelivery::default();
    let (sender, receiver) = channel();

    delivery.observe_press(&sender, 10);
    delivery.observe_press(&sender, 11);
    delivery.observe_press(&sender, 12);

    for _ in 0..3 {
        assert!(matches!(
            receiver.recv().unwrap(),
            FnListenerEvent::Pressed { .. }
        ));
    }
    assert_eq!(receiver.try_recv(), Err(TryRecvError::Empty));
    let drain = delivery.drain_after(0);
    assert_eq!(drain.press_count, 3);
    assert_eq!(drain.latest_sequence, 3);
    assert_eq!(delivery.coalesced_wake_count(), 0);
    assert_eq!(drain.press_count % 2, 1);
}

#[test]
fn timeout_and_user_input_disable_request_recreation() {
    let mut decoder = FnEdgeDecoder::default();
    assert_eq!(
        decoder.decode(FnRawEvent::tap_disabled(TapDisabledReason::Timeout, 20,)),
        Some(FnListenerEvent::TapDisabled {
            reason: TapDisabledReason::Timeout,
        })
    );
    assert!(decoder.recreation_requested());

    decoder.clear_recreation_request();
    assert_eq!(
        decoder.decode(FnRawEvent::tap_disabled(TapDisabledReason::UserInput, 21,)),
        Some(FnListenerEvent::TapDisabled {
            reason: TapDisabledReason::UserInput,
        })
    );
    assert!(decoder.recreation_requested());
}

#[test]
fn stop_handle_is_idempotent_and_marks_run_loop_for_exit() {
    let handle = FnListenerHandle::detached_for_test();
    assert!(!handle.is_stop_requested());
    assert!(handle.stop());
    assert!(!handle.stop());
    assert!(handle.is_stop_requested());
}

#[test]
fn terminal_health_event_follows_queued_press_edges() {
    let delivery = PressDelivery::default();
    let (sender, receiver) = channel();
    delivery.observe_press(&sender, 10);

    let delivery_thread =
        std::thread::spawn(move || deliver_terminal_event(&sender, FnListenerEvent::TapEnded));
    assert!(matches!(
        receiver.recv().unwrap(),
        FnListenerEvent::Pressed { .. }
    ));
    assert_eq!(receiver.recv().unwrap(), FnListenerEvent::TapEnded);
    assert!(delivery_thread.join().unwrap());
}
