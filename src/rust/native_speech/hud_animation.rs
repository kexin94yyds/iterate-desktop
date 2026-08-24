use serde::Deserialize;
use tauri::{AppHandle, Manager};

use super::SPEECH_OVERLAY_WINDOW_LABEL;

const HUD_FRAME_ANIMATION_SECONDS: f64 = 0.14;
const HUD_EDGE_MARGIN_PHYSICAL_PX: f64 = 8.0;
const MAX_HUD_CONTENT_DIMENSION_POINTS: f64 = 4_096.0;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechOverlayFrameRequest {
    target_content_width_points: f64,
    target_content_height_points: f64,
    #[serde(default)]
    reduced_motion: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct FrameGeometry {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

fn validate_request(request: SpeechOverlayFrameRequest) -> Result<(), String> {
    for (label, value) in [
        (
            "target_content_width_points",
            request.target_content_width_points,
        ),
        (
            "target_content_height_points",
            request.target_content_height_points,
        ),
    ] {
        if !value.is_finite() || value <= 0.0 || value > MAX_HUD_CONTENT_DIMENSION_POINTS {
            return Err(format!("invalid {label}: {value}"));
        }
    }
    Ok(())
}

fn right_anchored_clamped_frame(
    current: FrameGeometry,
    target_width: f64,
    target_height: f64,
    visible: Option<FrameGeometry>,
    margin: f64,
) -> FrameGeometry {
    let center_y = current.y + current.height / 2.0;
    let mut target = FrameGeometry {
        x: current.x + current.width - target_width,
        y: center_y - target_height / 2.0,
        width: target_width,
        height: target_height,
    };

    if let Some(visible) = visible {
        let min_x = visible.x + margin;
        let min_y = visible.y + margin;
        let max_x = (visible.x + visible.width - target.width - margin).max(min_x);
        let max_y = (visible.y + visible.height - target.height - margin).max(min_y);
        target.x = target.x.clamp(min_x, max_x);
        target.y = target.y.clamp(min_y, max_y);
    }

    target
}

#[tauri::command]
pub async fn animate_speech_overlay_frame(
    app: AppHandle,
    request: SpeechOverlayFrameRequest,
) -> Result<(), String> {
    validate_request(request)?;

    #[cfg(target_os = "macos")]
    {
        animate_macos_speech_overlay_frame(app, request).await
    }

    #[cfg(not(target_os = "macos"))]
    {
        let _ = app;
        Err("native speech overlay frame animation is only available on macOS".to_string())
    }
}

#[cfg(target_os = "macos")]
async fn animate_macos_speech_overlay_frame(
    app: AppHandle,
    request: SpeechOverlayFrameRequest,
) -> Result<(), String> {
    let window = app
        .get_webview_window(SPEECH_OVERLAY_WINDOW_LABEL)
        .ok_or_else(|| "speech overlay window not found".to_string())?;
    let window_for_main_thread = window.clone();
    let (result_tx, result_rx) = tokio::sync::oneshot::channel();

    window
        .run_on_main_thread(move || {
            let result = apply_macos_speech_overlay_frame(&window_for_main_thread, request);
            let _ = result_tx.send(result);
        })
        .map_err(|error| format!("failed to schedule speech overlay frame animation: {error}"))?;

    result_rx
        .await
        .map_err(|_| "speech overlay frame animation result channel closed".to_string())?
}

#[cfg(target_os = "macos")]
fn apply_macos_speech_overlay_frame(
    window: &tauri::WebviewWindow,
    request: SpeechOverlayFrameRequest,
) -> Result<(), String> {
    use cocoa::appkit::{NSScreen, NSWindow};
    use cocoa::base::{id, nil, YES};
    use cocoa::foundation::{NSPoint, NSRect, NSSize};
    use objc::{class, msg_send, sel, sel_impl};

    let ptr = window
        .ns_window()
        .map_err(|error| format!("failed to access native speech overlay window: {error}"))?;
    if ptr.is_null() {
        return Err("native speech overlay window pointer is null".to_string());
    }

    unsafe {
        let native_window = ptr as id;
        let current = NSWindow::frame(native_window);
        let target_content = NSRect::new(
            NSPoint::new(0.0, 0.0),
            NSSize::new(
                request.target_content_width_points,
                request.target_content_height_points,
            ),
        );
        let target_outer = NSWindow::frameRectForContentRect_(native_window, target_content);
        let screen = NSWindow::screen(native_window);
        let visible = if screen == nil {
            None
        } else {
            let frame = NSScreen::visibleFrame(screen);
            Some(FrameGeometry {
                x: frame.origin.x,
                y: frame.origin.y,
                width: frame.size.width,
                height: frame.size.height,
            })
        };
        let scale_factor = NSWindow::backingScaleFactor(native_window).max(1.0);
        let margin = HUD_EDGE_MARGIN_PHYSICAL_PX / scale_factor;
        let target = right_anchored_clamped_frame(
            FrameGeometry {
                x: current.origin.x,
                y: current.origin.y,
                width: current.size.width,
                height: current.size.height,
            },
            target_outer.size.width,
            target_outer.size.height,
            visible,
            margin,
        );
        let target_frame = NSRect::new(
            NSPoint::new(target.x, target.y),
            NSSize::new(target.width, target.height),
        );

        if request.reduced_motion {
            NSWindow::setFrame_display_(native_window, target_frame, YES);
            return Ok(());
        }

        let animation_context_class = class!(NSAnimationContext);
        let _: () = msg_send![animation_context_class, beginGrouping];
        let animation_context: id = msg_send![animation_context_class, currentContext];
        let _: () = msg_send![animation_context, setDuration: HUD_FRAME_ANIMATION_SECONDS];
        let animator: id = msg_send![native_window, animator];
        let _: () = msg_send![animator, setFrame: target_frame display: YES];
        let _: () = msg_send![animation_context_class, endGrouping];
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f64, y: f64, width: f64, height: f64) -> FrameGeometry {
        FrameGeometry {
            x,
            y,
            width,
            height,
        }
    }

    #[test]
    fn preserves_right_edge_and_vertical_center_when_target_fits_visible_frame() {
        let target = right_anchored_clamped_frame(
            rect(430.0, 323.0, 420.0, 156.0),
            126.0,
            48.0,
            Some(rect(0.0, 0.0, 1_440.0, 900.0)),
            4.0,
        );

        assert_eq!(target, rect(724.0, 377.0, 126.0, 48.0));
    }

    #[test]
    fn clamps_expansion_inside_visible_frame_with_margin() {
        let target = right_anchored_clamped_frame(
            rect(1_300.0, 40.0, 126.0, 48.0),
            420.0,
            156.0,
            Some(rect(0.0, 0.0, 1_440.0, 900.0)),
            4.0,
        );

        assert_eq!(target, rect(1_006.0, 4.0, 420.0, 156.0));
    }

    #[test]
    fn pins_oversized_target_to_visible_origin_margin() {
        let target = right_anchored_clamped_frame(
            rect(-1_200.0, 100.0, 126.0, 48.0),
            2_000.0,
            1_000.0,
            Some(rect(-1_440.0, 0.0, 1_440.0, 900.0)),
            8.0,
        );

        assert_eq!(target.x, -1_432.0);
        assert_eq!(target.y, 8.0);
    }

    #[test]
    fn rejects_non_finite_or_out_of_range_dimensions() {
        for request in [
            SpeechOverlayFrameRequest {
                target_content_width_points: f64::NAN,
                target_content_height_points: 48.0,
                reduced_motion: false,
            },
            SpeechOverlayFrameRequest {
                target_content_width_points: 0.0,
                target_content_height_points: 48.0,
                reduced_motion: false,
            },
            SpeechOverlayFrameRequest {
                target_content_width_points: 4_097.0,
                target_content_height_points: 48.0,
                reduced_motion: false,
            },
        ] {
            assert!(validate_request(request).is_err());
        }
    }
}
