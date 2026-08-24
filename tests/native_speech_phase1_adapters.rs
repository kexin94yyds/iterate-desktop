use cunzhi::native_speech::coordinator::validate_control_identity;
use cunzhi::native_speech::native_backend::map_bridge_callback;
use cunzhi::native_speech::overlay::overlay_creation_allowed;
use cunzhi::native_speech::owner::SpeechProcessRole;
use cunzhi::native_speech::session::{
    OwnerEpoch, SpeechControlInput, SpeechLayerIdentity, SpeechSessionReducer,
};
use std::fs;
use std::path::PathBuf;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn source(path: &str) -> String {
    fs::read_to_string(root().join(path)).expect("source file")
}

fn active_snapshot() -> cunzhi::native_speech::session::SpeechSnapshot {
    let mut reducer = SpeechSessionReducer::default();
    reducer.apply(
        SpeechControlInput::OwnerAcquired {
            epoch: OwnerEpoch([7; 16]),
        },
        1,
    );
    reducer.apply(SpeechControlInput::FnPressed, 2);
    reducer.snapshot()
}

#[test]
fn tauri_config_does_not_create_a_static_speech_overlay() {
    let config: serde_json::Value =
        serde_json::from_str(&source("tauri.conf.json")).expect("tauri config json");
    let windows = config["app"]["windows"].as_array().expect("windows array");
    assert!(windows
        .iter()
        .all(|window| window["label"] != "speech-overlay"));
}

#[test]
fn only_an_acquired_eligible_owner_can_create_the_dynamic_overlay() {
    let epoch = OwnerEpoch([3; 16]);
    assert!(overlay_creation_allowed(
        SpeechProcessRole::CanonicalGui,
        Some(epoch)
    ));
    for role in [
        SpeechProcessRole::StandalonePopup,
        SpeechProcessRole::Serve,
        SpeechProcessRole::McpRequest,
        SpeechProcessRole::Bridge,
    ] {
        assert!(!overlay_creation_allowed(role, Some(epoch)));
    }
    assert!(!overlay_creation_allowed(
        SpeechProcessRole::CanonicalGui,
        None
    ));
}

#[test]
fn frontmost_capture_and_activation_use_nsworkspace_not_applescript() {
    let target = source("src/rust/native_speech/target.rs");
    let bridge = source("src/rust/native_speech/macos_speech_bridge.m");
    assert!(target.contains("speech_bridge_copy_frontmost_application"));
    assert!(target.contains("speech_bridge_activate_application"));
    assert!(!target.contains("osascript"));
    assert!(bridge.contains("NSWorkspace"));
    assert!(!bridge.contains("tell application"));
}

#[test]
fn external_writeback_restores_the_exact_captured_focus_before_dispatch() {
    let speech = source("src/rust/native_speech/mod.rs");
    assert!(
        speech.contains("enum CapturedFocusEvidence")
            && speech.contains("Exact(RetainedAxElement)")
            && speech.contains("FrontmostPidFallback"),
        "the retained external target must distinguish exact AX focus from the bounded PID fallback"
    );
    assert!(
        speech.contains("capture_focused_element_for_pid"),
        "target capture must retain the focused element before the overlay changes focus"
    );
    assert!(
        speech.contains("restore_captured_focused_element"),
        "external writeback must restore the captured element before posting paste"
    );

    let paste = speech
        .find("fn paste_text_with_identity")
        .expect("external paste entry point");
    let restore = speech[paste..]
        .find("restore_captured_focused_element")
        .map(|offset| paste + offset)
        .expect("focus restore call inside paste_text");
    let dispatch = speech[restore..]
        .find("dispatch_with_launcher")
        .map(|offset| restore + offset)
        .expect("paste dispatch after focus restore");
    assert!(
        restore < dispatch,
        "focus must be restored before one-shot helper dispatch"
    );

    assert!(speech.contains("const AX_ERROR_NO_VALUE: AXError = -25212"));
    assert!(speech.contains("PasteDispatchRoute::AnnotatedSession"));
    assert!(!speech.contains("PasteDispatchRoute::Pid"));
    assert!(speech.contains("target-changed-before-dispatch"));
}

#[test]
fn generic_external_writeback_is_unverified_while_popup_ack_remains_typed() {
    let speech = source("src/rust/native_speech/mod.rs");
    let phase1 = source("src/rust/native_speech/phase1.rs");
    let dispatch = speech
        .find("pub(crate) fn dispatch_speech_writeback")
        .expect("tagged writeback dispatcher");
    let external = speech[dispatch..]
        .find("SpeechTarget::ExternalApp")
        .map(|offset| dispatch + offset)
        .expect("external writeback branch");
    let popup = speech[external..]
        .find("SpeechTarget::IteratePopupInput")
        .map(|offset| external + offset)
        .expect("popup writeback branch");
    let external_branch = &speech[external..popup];

    assert!(external_branch.contains("paste_text_with_identity(trimmed, identity)?"));
    assert!(external_branch.contains("ExternalDispatchedUnverified"));
    assert!(external_branch.contains("ExternalUnknownAfterDispatch"));
    assert!(!external_branch.contains("ExternalAcknowledged"));
    assert!(speech[popup..].contains("Ok(()) => Ok(SpeechWritebackDispatch::ExternalAcknowledged)"));
    assert!(phase1.contains("SpeechWritebackDispatch::ExternalDispatchedUnverified"));
    assert!(phase1.contains("SpeechControlInput::WritebackUnverified"));
}

#[test]
fn dynamic_overlay_and_new_commands_are_wired_without_legacy_toggle_atomics() {
    let overlay = source("src/rust/native_speech/overlay.rs");
    let speech = source("src/rust/native_speech/mod.rs");
    let builder = source("src/rust/app/builder.rs");
    let setup = source("src/rust/app/setup.rs");
    assert!(overlay.contains("WebviewWindowBuilder"));
    assert!(overlay.contains("index.html?view=speech-overlay"));
    assert!(setup.contains("start_phase1_runtime"));
    for command in [
        "get_speech_control_snapshot",
        "ack_speech_overlay_visibility",
        "configure_speech_recognition",
        "complete_speech_processing",
    ] {
        assert!(builder.contains(command), "missing command {command}");
    }
    for legacy in ["SPEECH_ACTIVE", "PENDING_TOGGLE", "OVERLAY_LISTENER_READY"] {
        assert!(!speech.contains(legacy), "legacy state remains: {legacy}");
    }
}

#[test]
fn copied_native_callback_identity_rejects_every_stale_layer() {
    let snapshot = active_snapshot();
    let current = snapshot.identity.expect("active identity");
    for stale in stale_identities(current) {
        assert!(map_bridge_callback(current, stale, "final", "secret").is_none());
    }
    assert_eq!(
        map_bridge_callback(current, current, "final", "accepted"),
        Some(SpeechControlInput::NativeFinal {
            identity: current,
            text: "accepted".into(),
        })
    );
}

#[test]
fn command_validation_rejects_stale_schema_epoch_control_session_and_revision() {
    let snapshot = active_snapshot();
    let current = snapshot.identity.expect("active identity");
    assert!(validate_control_identity(&snapshot, current).is_ok());
    for stale in stale_identities(current) {
        assert!(validate_control_identity(&snapshot, stale).is_err());
    }
}

fn stale_identities(current: SpeechLayerIdentity) -> [SpeechLayerIdentity; 5] {
    let mut schema = current;
    schema.schema_version = schema.schema_version.saturating_add(1);
    let mut epoch = current;
    epoch.owner_epoch_lo ^= 1;
    let mut control = current;
    control.control_seq = control.control_seq.saturating_add(1);
    let mut session = current;
    session.session_sequence = session.session_sequence.saturating_add(1);
    let mut revision = current;
    revision.revision = revision.revision.saturating_add(1);
    [schema, epoch, control, session, revision]
}
