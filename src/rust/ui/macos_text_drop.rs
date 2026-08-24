//! Native macOS text-drop capture for the main Wry webview.
//!
//! Wry forwards only file paths through Tauri's `DragDropEvent`. Plain text is
//! available solely from `NSDraggingInfo.draggingPasteboard()` while AppKit is
//! executing `performDragOperation:`. Capture it there, then let the normal Wry
//! implementation enqueue the Tauri window event that consumes this value.

#![allow(deprecated, unexpected_cfgs)]

use cocoa::appkit::NSPasteboardTypeString;
use cocoa::base::id;
use objc::runtime::{
    class_getInstanceMethod, method_setImplementation, object_getClass, Imp, Method, Object, Sel,
    BOOL, NO,
};
use objc::{msg_send, sel, sel_impl};
use std::ffi::{c_void, CStr};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

const MAX_NATIVE_DROP_TEXT_BYTES: usize = 200_000;

type PerformDragOperation = unsafe extern "C" fn(*mut Object, Sel, *mut Object) -> BOOL;

#[derive(Default)]
struct PendingDragText {
    value: Mutex<Option<String>>,
}

impl PendingDragText {
    fn replace(&self, value: Option<String>) {
        *self
            .value
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = value;
    }

    fn take(&self) -> Option<String> {
        self.value
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
    }
}

static PENDING_MAIN_WEBVIEW_TEXT: OnceLock<PendingDragText> = OnceLock::new();
static MAIN_WEBVIEW_PTR: AtomicUsize = AtomicUsize::new(0);
static ORIGINAL_PERFORM_DRAG_OPERATION: AtomicUsize = AtomicUsize::new(0);
static INSTALL_RESULT: OnceLock<Result<(), String>> = OnceLock::new();

fn pending_text() -> &'static PendingDragText {
    PENDING_MAIN_WEBVIEW_TEXT.get_or_init(PendingDragText::default)
}

fn normalize_drag_text(text: String) -> Option<String> {
    if text.is_empty() || text.len() > MAX_NATIVE_DROP_TEXT_BYTES {
        None
    } else {
        Some(text)
    }
}

unsafe fn read_dragging_pasteboard_text(dragging_info: *mut Object) -> Option<String> {
    if dragging_info.is_null() {
        return None;
    }

    let pasteboard: id = msg_send![dragging_info, draggingPasteboard];
    if pasteboard.is_null() {
        return None;
    }

    let value: id = msg_send![pasteboard, stringForType: NSPasteboardTypeString];
    if value.is_null() {
        return None;
    }

    let utf8: *const i8 = msg_send![value, UTF8String];
    if utf8.is_null() {
        return None;
    }

    normalize_drag_text(CStr::from_ptr(utf8).to_string_lossy().into_owned())
}

unsafe extern "C" fn capture_main_webview_text_drop(
    webview: *mut Object,
    selector: Sel,
    dragging_info: *mut Object,
) -> BOOL {
    if webview as usize == MAIN_WEBVIEW_PTR.load(Ordering::Acquire) {
        // Replace even with None so an unsupported drop can never reuse text
        // captured from an earlier session.
        pending_text().replace(read_dragging_pasteboard_text(dragging_info));
    }

    let original = ORIGINAL_PERFORM_DRAG_OPERATION.load(Ordering::Acquire);
    if original == 0 {
        return NO;
    }

    let original: PerformDragOperation = std::mem::transmute(original);
    original(webview, selector, dragging_info)
}

unsafe fn install_method_hook(webview: *mut Object) -> Result<(), String> {
    let class = object_getClass(webview);
    if class.is_null() {
        return Err("main webview Objective-C class is unavailable".to_string());
    }

    let selector = Sel::register("performDragOperation:");
    let method = class_getInstanceMethod(class, selector) as *mut Method;
    if method.is_null() {
        return Err("main webview does not implement performDragOperation:".to_string());
    }

    let replacement: PerformDragOperation = capture_main_webview_text_drop;
    let replacement: Imp = std::mem::transmute(replacement);
    let original = method_setImplementation(method, replacement);
    if original as usize == 0 {
        return Err(
            "failed to retain the original performDragOperation: implementation".to_string(),
        );
    }

    ORIGINAL_PERFORM_DRAG_OPERATION.store(original as usize, Ordering::Release);
    Ok(())
}

/// Install the one-process Wry hook and mark this exact WKWebView as the only
/// source whose plain-text drops may populate the pending value.
pub(crate) fn install_main_webview_text_drop_capture(webview: *mut c_void) -> Result<(), String> {
    if webview.is_null() {
        return Err("main webview pointer is null".to_string());
    }

    MAIN_WEBVIEW_PTR.store(webview as usize, Ordering::Release);
    INSTALL_RESULT
        .get_or_init(|| unsafe { install_method_hook(webview.cast()) })
        .clone()
}

/// Consume the text captured for the most recent main-webview drop.
pub(crate) fn take_main_webview_drop_text() -> Option<String> {
    pending_text().take()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_text_is_consumed_once() {
        let pending = PendingDragText::default();
        pending.replace(Some("hello".to_string()));

        assert_eq!(pending.take().as_deref(), Some("hello"));
        assert_eq!(pending.take(), None);
    }

    #[test]
    fn replacing_with_none_clears_stale_text() {
        let pending = PendingDragText::default();
        pending.replace(Some("stale".to_string()));
        pending.replace(None);

        assert_eq!(pending.take(), None);
    }

    #[test]
    fn native_drop_text_has_a_bounded_utf8_payload() {
        assert_eq!(normalize_drag_text(String::new()), None);
        assert_eq!(normalize_drag_text("ok".to_string()).as_deref(), Some("ok"));
        assert_eq!(
            normalize_drag_text("汉".repeat(MAX_NATIVE_DROP_TEXT_BYTES / 3))
                .as_deref()
                .map(str::len),
            Some((MAX_NATIVE_DROP_TEXT_BYTES / 3) * 3)
        );
        assert_eq!(
            normalize_drag_text("汉".repeat((MAX_NATIVE_DROP_TEXT_BYTES / 3) + 1)),
            None
        );
        assert_eq!(
            normalize_drag_text("x".repeat(MAX_NATIVE_DROP_TEXT_BYTES + 1)),
            None
        );
    }
}
