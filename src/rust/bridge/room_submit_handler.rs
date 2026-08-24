use super::mcp_action_delivery::preflight_mcp_action_delivery;
use super::mcp_action_handler::{try_handle_mcp_action_directly, try_handle_mcp_action_headless};
use super::room_submit::{
    cached_room_submit_outcome, remember_room_submit_outcome, room_submit_outcome,
    room_submit_outcome_with_attempts, room_submit_request_from_payload,
    room_submit_target_matches_room_state, room_token_matches, RoomSubmitOutcome,
};
use super::ws::{bridge_debug_log, MCP_ACTION_CACHE_TTL_SECS};
use tauri::AppHandle;

pub(super) async fn handle_room_submit_action(
    app_handle: Option<&AppHandle>,
    project_path: &str,
    request_id: Option<&str>,
    timeline_route_id: Option<&str>,
    payload: &serde_json::Value,
) -> RoomSubmitOutcome {
    let action = payload
        .get("action")
        .and_then(|value| value.as_str())
        .unwrap_or("");
    let request = match room_submit_request_from_payload(project_path, request_id, payload) {
        Ok(request) => request,
        Err(reason) => {
            return room_submit_outcome(
                None,
                action,
                project_path,
                request_id,
                "rejected",
                Some(&reason),
                false,
            );
        }
    };

    let room_state = match room_token_matches(
        &request.project_path,
        request.room_storage.as_deref(),
        &request.room_id,
        &request.room_token,
    ) {
        Ok(room_state) => room_state,
        Err(reason) => {
            return room_submit_outcome(
                Some(&request),
                action,
                project_path,
                request_id,
                "rejected",
                Some(&reason),
                false,
            );
        }
    };

    if let Err(reason) = room_submit_target_matches_room_state(&request, &room_state) {
        return room_submit_outcome(
            Some(&request),
            action,
            project_path,
            request_id,
            "rejected",
            Some(&reason),
            false,
        );
    }

    if let Some(outcome) = cached_room_submit_outcome(&request) {
        return outcome;
    }

    let preflight = preflight_mcp_action_delivery(
        app_handle,
        &request.project_path,
        request.request_id.as_deref(),
        MCP_ACTION_CACHE_TTL_SECS,
        &bridge_debug_log,
    );
    if !preflight.delivered {
        let reason = preflight
            .rejection_reason()
            .unwrap_or("delivery_preflight_failed")
            .to_string();
        return room_submit_outcome_with_attempts(
            Some(&request),
            action,
            project_path,
            request_id,
            "rejected",
            Some(&reason),
            false,
            preflight.attempts,
        );
    }

    let delivery = if let Some(app_handle) = app_handle {
        try_handle_mcp_action_directly(
            app_handle,
            &request.project_path,
            request.request_id.as_deref(),
            timeline_route_id.or(request.request_id.as_deref()),
            payload,
        )
        .await
    } else {
        try_handle_mcp_action_headless(
            &request.project_path,
            request.request_id.as_deref(),
            timeline_route_id.or(request.request_id.as_deref()),
            payload,
        )
        .await
    };
    let delivered = delivery.delivered;
    let rejection_reason = delivery
        .rejection_reason()
        .unwrap_or("route_miss")
        .to_string();
    let mut delivery_attempts = preflight.attempts;
    delivery_attempts.extend(delivery.attempts);

    let outcome = if delivered {
        room_submit_outcome_with_attempts(
            Some(&request),
            action,
            project_path,
            request_id,
            "accepted",
            None,
            true,
            delivery_attempts,
        )
    } else {
        room_submit_outcome_with_attempts(
            Some(&request),
            action,
            project_path,
            request_id,
            "rejected",
            Some(&rejection_reason),
            false,
            delivery_attempts,
        )
    };

    remember_room_submit_outcome(&request, &outcome);
    outcome
}
