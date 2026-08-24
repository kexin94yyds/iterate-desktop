use std::fs;
use std::path::PathBuf;

fn source(path: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

#[test]
fn bridge_uses_shared_six_word_identity_abi() {
    let bridge = source("src/rust/native_speech/macos_speech_bridge.m");
    assert!(bridge.contains("#include \"macos_speech_abi.h\""));
    assert!(bridge.contains("void speech_bridge_start(SpeechLayerIdentity identity"));

    let header = source("src/rust/native_speech/macos_speech_abi.h");
    assert!(header.contains("SpeechBridgeCallback)(SpeechLayerIdentity identity"));
    let fields = [
        "control_schema_version",
        "owner_epoch_hi",
        "owner_epoch_lo",
        "control_seq",
        "session_sequence",
        "revision",
    ];
    let mut previous = 0;
    for field in fields {
        let position = header
            .find(field)
            .unwrap_or_else(|| panic!("missing ABI field {field}"));
        assert!(position >= previous, "ABI field {field} is out of order");
        previous = position;
    }
    assert!(header.contains("_Static_assert(sizeof(SpeechLayerIdentity) == 48"));
}

#[test]
fn finish_and_cancel_are_distinct_and_finish_ends_audio_first() {
    let bridge = source("src/rust/native_speech/macos_speech_bridge.m");
    assert!(bridge.contains("void speech_bridge_finish(SpeechLayerIdentity identity)"));
    assert!(bridge.contains("void speech_bridge_cancel(SpeechLayerIdentity identity)"));
    assert!(bridge.contains("[bridge finishAudioInputAndWaitForFinal]"));
    assert!(bridge.contains("[bridge resetRecognition:YES]"));

    let finish = bridge
        .split("- (void)finishAudioInputAndWaitForFinal")
        .nth(1)
        .expect("finish implementation");
    let end_audio = finish.find("endAudio").expect("finish must end audio");
    let finish_task = finish
        .find("[self.recognitionTask finish]")
        .expect("finish must ask the recognition task to produce its final result");
    let grace = finish
        .find("dispatch_after")
        .expect("finish must retain a hard cleanup grace");
    assert!(
        end_audio < finish_task && finish_task < grace,
        "endAudio and recognition-task finish must happen before the final grace period"
    );
}

#[test]
fn every_async_callback_checks_generation_and_complete_identity() {
    let bridge = source("src/rust/native_speech/macos_speech_bridge.m");
    assert!(bridge.contains("SpeechLayerIdentityEqual(strongSelf.identity, identity)"));
    assert!(bridge.contains("SpeechLayerIdentityEqual(innerSelf.identity, identity)"));
    assert!(bridge.contains("[strongSelf acceptsGeneration:generation identity:capturedIdentity]"));
    assert!(bridge.contains("strongSelf.generation != generation"));
    assert!(bridge.contains("atomic_load(&g_speech_generation) != generation"));
    assert!(bridge.contains("strongSelf.stopGeneration == stopGeneration"));
    assert!(bridge.contains("SpeechLayerIdentityEqual(strongSelf.identity, finishIdentity)"));
}

#[test]
fn stale_final_or_error_cleanup_cannot_clear_a_newer_generation() {
    let bridge = source("src/rust/native_speech/macos_speech_bridge.m");
    assert!(bridge
        .contains("if (![strongSelf acceptsGeneration:generation identity:capturedIdentity])"));
    assert!(bridge.contains("[strongSelf resetRecognitionForGeneration:generation"));
    assert!(!bridge.contains("self.callback(eventCString, textCString, self.userData)"));
}
