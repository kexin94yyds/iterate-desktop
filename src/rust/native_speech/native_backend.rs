use super::session::{SpeechControlInput, SpeechLayerIdentity};
use std::ffi::{c_char, c_void};

pub type NativeSpeechCallback =
    extern "C" fn(SpeechLayerIdentity, *const c_char, *const c_char, *mut c_void);

pub fn map_bridge_callback(
    expected: SpeechLayerIdentity,
    received: SpeechLayerIdentity,
    event_type: &str,
    text: &str,
) -> Option<SpeechControlInput> {
    if received != expected {
        return None;
    }
    match event_type {
        "started" => Some(SpeechControlInput::NativeStarted { identity: received }),
        "partial" => Some(SpeechControlInput::NativePartial {
            identity: received,
            text: text.to_owned(),
        }),
        "final" => Some(SpeechControlInput::NativeFinal {
            identity: received,
            text: text.to_owned(),
        }),
        "error" => Some(SpeechControlInput::NativeError { identity: received }),
        _ => None,
    }
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn speech_bridge_start(
        identity: SpeechLayerIdentity,
        callback: NativeSpeechCallback,
        user_data: *mut c_void,
        contextual_strings: *const *const c_char,
        contextual_count: usize,
        force_on_device: bool,
    );
    fn speech_bridge_finish(identity: SpeechLayerIdentity);
    fn speech_bridge_cancel(identity: SpeechLayerIdentity);
}

#[cfg(target_os = "macos")]
pub unsafe fn start(
    identity: SpeechLayerIdentity,
    callback: NativeSpeechCallback,
    user_data: *mut c_void,
    contextual_strings: *const *const c_char,
    contextual_count: usize,
    force_on_device: bool,
) {
    unsafe {
        speech_bridge_start(
            identity,
            callback,
            user_data,
            contextual_strings,
            contextual_count,
            force_on_device,
        );
    }
}

#[cfg(target_os = "macos")]
pub unsafe fn finish(identity: SpeechLayerIdentity) {
    unsafe { speech_bridge_finish(identity) };
}

#[cfg(target_os = "macos")]
pub unsafe fn cancel(identity: SpeechLayerIdentity) {
    unsafe { speech_bridge_cancel(identity) };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_speech::session::OwnerEpoch;

    fn identity(epoch: u8) -> SpeechLayerIdentity {
        SpeechLayerIdentity::new(OwnerEpoch([epoch; 16]), 7, 11, 13)
    }

    #[test]
    fn rejects_old_epoch_even_when_all_numeric_sequences_are_reused() {
        let expected = identity(1);
        let received = identity(2);
        assert!(map_bridge_callback(expected, received, "final", "secret").is_none());
    }

    #[test]
    fn maps_supported_events_to_identity_tagged_reducer_inputs() {
        let identity = identity(3);
        assert_eq!(
            map_bridge_callback(identity, identity, "started", "quality"),
            Some(SpeechControlInput::NativeStarted { identity })
        );
        assert_eq!(
            map_bridge_callback(identity, identity, "partial", "one"),
            Some(SpeechControlInput::NativePartial {
                identity,
                text: "one".into(),
            })
        );
        assert_eq!(
            map_bridge_callback(identity, identity, "final", "two"),
            Some(SpeechControlInput::NativeFinal {
                identity,
                text: "two".into(),
            })
        );
        assert_eq!(
            map_bridge_callback(identity, identity, "error", "not retained"),
            Some(SpeechControlInput::NativeError { identity })
        );
        assert!(map_bridge_callback(identity, identity, "diagnostic", "ignored").is_none());
    }

    #[test]
    fn adapter_contains_no_transcript_logging_path() {
        let source = include_str!("native_backend.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("production source");
        assert!(!production.contains("debug_log"));
        assert!(!production.contains("println!"));
        assert!(!production.contains("eprintln!"));
    }
}
