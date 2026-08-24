use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::UNIX_EPOCH;
use tauri::{AppHandle, Manager, Runtime};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveGoalSnapshot {
    pub id: String,
    pub title: String,
    pub status: String,
    pub phase: Option<String>,
    pub status_text: Option<String>,
    pub progress_percent: Option<f64>,
    pub progress_source: Option<String>,
    pub progress_label: Option<String>,
    pub plan_total: Option<u32>,
    pub plan_completed: Option<u32>,
    pub tokens_used: Option<u64>,
    pub token_budget: Option<u64>,
    pub time_used_seconds: Option<u64>,
    pub started_at_ms: i64,
    pub updated_at_ms: i64,
    pub completed_at_ms: Option<i64>,
    pub project_path: Option<String>,
    pub request_id: Option<String>,
    pub codex_thread_id: Option<String>,
    pub codex_deeplink: Option<String>,
    pub run_id: Option<String>,
    pub generation: Option<u64>,
    pub stale_of: Option<String>,
    pub superseded_by: Option<String>,
    pub last_codex_event_at_ms: Option<i64>,
    pub source: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct LiveGoalProgressUpdate {
    pub progress_percent: Option<f64>,
    pub progress_source: Option<String>,
    pub progress_label: Option<String>,
    pub phase: Option<String>,
    pub status: Option<String>,
    pub status_text: Option<String>,
    pub plan_total: Option<u32>,
    pub plan_completed: Option<u32>,
    pub tokens_used: Option<u64>,
    pub token_budget: Option<u64>,
    pub time_used_seconds: Option<u64>,
    pub project_path: Option<String>,
    pub request_id: Option<String>,
    pub codex_thread_id: Option<String>,
    pub codex_deeplink: Option<String>,
    pub run_id: Option<String>,
    pub generation: Option<u64>,
    pub stale_of: Option<String>,
    pub superseded_by: Option<String>,
    pub last_codex_event_at_ms: Option<i64>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct LiveGoalResponseMetadata {
    pub run_id: Option<String>,
    pub generation: Option<u64>,
    pub stale_of: Option<String>,
    pub superseded_by: Option<String>,
    pub is_stale: bool,
}

#[derive(Debug, Clone)]
pub struct CodexGoalObserverUpdate {
    pub goal_id: String,
    pub title: String,
    pub status: String,
    pub project_path: String,
    pub codex_thread_id: String,
    pub token_budget: Option<u64>,
    pub tokens_used: Option<u64>,
    pub time_used_seconds: Option<u64>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Default)]
pub struct LiveGoalTrayState {
    current: Mutex<Option<LiveGoalSnapshot>>,
}

const CODEX_GOAL_OBSERVER_SOURCE: &str = "codex_goal_observer";

#[derive(Debug)]
enum LiveGoalEventKind {
    ProgressUpdate(LiveGoalProgressUpdate),
    InteractionPhase { phase: String, status_text: String },
    QuotaStatus { status_text: String },
}

#[derive(Debug)]
struct LiveGoalEvent {
    kind: LiveGoalEventKind,
    project_path: Option<String>,
    request_id: Option<String>,
    source: String,
    now_ms: i64,
}

impl LiveGoalEvent {
    fn progress_update(update: LiveGoalProgressUpdate) -> Self {
        let project_path = update
            .project_path
            .as_deref()
            .and_then(|value| normalize_optional_str(Some(value)));
        let request_id = update
            .request_id
            .as_deref()
            .and_then(|value| normalize_optional_str(Some(value)));
        let source = update
            .source
            .as_deref()
            .and_then(|value| normalize_optional_str(Some(value)))
            .unwrap_or_else(|| "progress_update".to_string());

        Self {
            kind: LiveGoalEventKind::ProgressUpdate(update),
            project_path,
            request_id,
            source,
            now_ms: chrono::Utc::now().timestamp_millis(),
        }
    }

    fn interaction_phase(
        project_path: Option<&str>,
        request_id: Option<&str>,
        phase: &str,
        status_text: &str,
        source: &str,
    ) -> Self {
        Self {
            kind: LiveGoalEventKind::InteractionPhase {
                phase: phase.to_string(),
                status_text: status_text.to_string(),
            },
            project_path: normalize_optional_str(project_path),
            request_id: normalize_optional_str(request_id),
            source: normalize_optional_str(Some(source))
                .unwrap_or_else(|| "interaction".to_string()),
            now_ms: chrono::Utc::now().timestamp_millis(),
        }
    }

    fn quota_status(
        project_path: Option<&str>,
        request_id: Option<&str>,
        status_text: &str,
    ) -> Self {
        Self {
            kind: LiveGoalEventKind::QuotaStatus {
                status_text: status_text.to_string(),
            },
            project_path: normalize_optional_str(project_path),
            request_id: normalize_optional_str(request_id),
            source: "codex_quota".to_string(),
            now_ms: chrono::Utc::now().timestamp_millis(),
        }
    }
}

enum LiveGoalIntent {
    Start(String),
    Complete,
    Clear,
}

impl LiveGoalTrayState {
    fn set(&self, goal: Option<LiveGoalSnapshot>) {
        if let Ok(mut current) = self.current.lock() {
            *current = goal.clone();
        }
        if let Err(error) = persist_live_goal_snapshot(goal.as_ref()) {
            log::warn!("[LiveGoal] persist failed: {}", error);
        }
    }

    fn get(&self) -> Option<LiveGoalSnapshot> {
        self.current
            .lock()
            .ok()
            .and_then(|current| current.clone())
            .or_else(read_live_goal_snapshot)
    }
}

#[tauri::command]
pub fn get_live_goal(state: tauri::State<'_, LiveGoalTrayState>) -> Option<LiveGoalSnapshot> {
    state.get()
}

#[tauri::command]
pub fn resolve_live_goal_response_metadata(
    state: tauri::State<'_, LiveGoalTrayState>,
    project_path: Option<String>,
    run_id: Option<String>,
    generation: Option<u64>,
) -> LiveGoalResponseMetadata {
    resolve_live_goal_response_metadata_from_current(
        state.get(),
        project_path.as_deref(),
        run_id.as_deref(),
        generation,
    )
}

#[tauri::command]
pub fn start_live_goal(
    app: AppHandle,
    state: tauri::State<'_, LiveGoalTrayState>,
    title: String,
    project_path: Option<String>,
    request_id: Option<String>,
    codex_thread_id: Option<String>,
    codex_deeplink: Option<String>,
) -> Result<LiveGoalSnapshot, String> {
    start_live_goal_inner(
        &app,
        &state,
        &title,
        project_path.as_deref(),
        request_id.as_deref(),
        codex_thread_id.as_deref(),
        codex_deeplink.as_deref(),
        "manual",
    )
}

pub fn apply_live_goal_intent_from_response<R: Runtime>(
    app: Option<&AppHandle<R>>,
    response: &serde_json::Value,
    project_path: Option<&str>,
    request_id: Option<&str>,
) {
    let intent = resolve_live_goal_intent_from_response(response);
    crate::utils::append_timeline_debug_log(
        "rust/live_goal::apply_from_response",
        serde_json::json!({
            "matched": intent.is_some(),
            "user_input_preview": response
                .get("user_input")
                .and_then(|value| value.as_str())
                .map(|value| value.chars().take(80).collect::<String>()),
            "selected_options": response
                .get("selected_options")
                .and_then(|value| value.as_array())
                .map(|options| options
                    .iter()
                    .filter_map(|option| option.as_str())
                    .collect::<Vec<_>>()),
            "project_path": project_path,
            "request_id": request_id,
        }),
    );

    let Some(intent) = intent else {
        return;
    };

    let Some(app) = app else {
        let result = apply_live_goal_intent_persistent_only(intent, project_path, request_id);
        log_live_goal_apply_result(result);
        return;
    };

    let Some(state) = app.try_state::<LiveGoalTrayState>() else {
        crate::utils::append_timeline_debug_log(
            "rust/live_goal::apply_from_response:missing_state",
            serde_json::json!({}),
        );
        let result = apply_live_goal_intent_persistent_only(intent, project_path, request_id);
        log_live_goal_apply_result(result);
        return;
    };

    let result = match intent {
        LiveGoalIntent::Start(title) => start_live_goal_inner(
            app,
            &state,
            &title,
            project_path,
            request_id,
            None,
            None,
            "goal_intent",
        )
        .map(|_| ()),
        LiveGoalIntent::Complete => {
            complete_live_goal_inner(app, &state, project_path, request_id, "goal_intent")
                .map(|_| ())
        }
        LiveGoalIntent::Clear => clear_live_goal_inner(app, &state),
    };

    log_live_goal_apply_result(result);
}

fn start_live_goal_inner<R: Runtime>(
    _app: &AppHandle<R>,
    state: &LiveGoalTrayState,
    title: &str,
    project_path: Option<&str>,
    request_id: Option<&str>,
    codex_thread_id: Option<&str>,
    codex_deeplink: Option<&str>,
    source: &str,
) -> Result<LiveGoalSnapshot, String> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err("目标不能为空".to_string());
    }

    let goal = build_live_goal_snapshot(
        trimmed,
        project_path,
        request_id,
        codex_thread_id,
        codex_deeplink,
        source,
    );

    state.set(Some(goal.clone()));
    schedule_live_goal_live_activity_apns(&goal, "update");
    Ok(goal)
}

#[tauri::command]
pub fn complete_live_goal(
    app: AppHandle,
    state: tauri::State<'_, LiveGoalTrayState>,
) -> Result<Option<LiveGoalSnapshot>, String> {
    complete_live_goal_inner(&app, &state, None, None, "manual")
}

fn complete_live_goal_inner<R: Runtime>(
    _app: &AppHandle<R>,
    state: &LiveGoalTrayState,
    project_path: Option<&str>,
    request_id: Option<&str>,
    source: &str,
) -> Result<Option<LiveGoalSnapshot>, String> {
    let Some(mut goal) = state.get() else {
        return Ok(None);
    };

    complete_live_goal_snapshot(&mut goal, project_path, request_id, source);
    state.set(Some(goal.clone()));
    schedule_live_goal_live_activity_apns(&goal, "end");
    Ok(Some(goal))
}

#[tauri::command]
pub fn update_live_goal_progress(
    app: AppHandle,
    state: tauri::State<'_, LiveGoalTrayState>,
    update: LiveGoalProgressUpdate,
) -> Result<Option<LiveGoalSnapshot>, String> {
    update_live_goal_progress_inner(&app, &state, update)
}

fn update_live_goal_progress_inner<R: Runtime>(
    _app: &AppHandle<R>,
    state: &LiveGoalTrayState,
    update: LiveGoalProgressUpdate,
) -> Result<Option<LiveGoalSnapshot>, String> {
    let Some(mut goal) = state.get() else {
        return Ok(None);
    };

    apply_live_goal_progress_update_to_snapshot(&mut goal, update);
    state.set(Some(goal.clone()));
    let event = if goal.status == "completed" {
        "end"
    } else {
        "update"
    };
    schedule_live_goal_live_activity_apns(&goal, event);
    Ok(Some(goal))
}

pub fn apply_codex_goal_observer_update<R: Runtime>(
    app: &AppHandle<R>,
    update: CodexGoalObserverUpdate,
) -> Result<Option<LiveGoalSnapshot>, String> {
    let Some(state) = app.try_state::<LiveGoalTrayState>() else {
        crate::utils::append_timeline_debug_log(
            "rust/live_goal::codex_goal_observer:missing_state",
            serde_json::json!({
                "goal_id": update.goal_id,
                "project_path": update.project_path,
            }),
        );
        return Ok(None);
    };

    let Some(goal) = apply_codex_goal_observer_update_to_current(state.get(), update)? else {
        return Ok(None);
    };

    state.set(Some(goal.clone()));
    let event = if goal.status == "completed" {
        "end"
    } else {
        "update"
    };
    schedule_live_goal_live_activity_apns(&goal, event);
    Ok(Some(goal))
}

#[tauri::command]
pub fn update_live_goal_quota_status(
    app: AppHandle,
    state: tauri::State<'_, LiveGoalTrayState>,
    status_text: String,
    project_path: Option<String>,
    request_id: Option<String>,
) -> Result<Option<LiveGoalSnapshot>, String> {
    update_live_goal_quota_status_inner(
        &app,
        &state,
        status_text,
        project_path.as_deref(),
        request_id.as_deref(),
    )
}

fn update_live_goal_quota_status_inner<R: Runtime>(
    _app: &AppHandle<R>,
    state: &LiveGoalTrayState,
    status_text: String,
    project_path: Option<&str>,
    request_id: Option<&str>,
) -> Result<Option<LiveGoalSnapshot>, String> {
    let Some(mut goal) = state.get() else {
        return Ok(None);
    };

    if !apply_live_goal_quota_status_to_snapshot(&mut goal, project_path, request_id, &status_text)
    {
        return Ok(None);
    }

    state.set(Some(goal.clone()));
    schedule_live_goal_live_activity_apns(&goal, "update");
    Ok(Some(goal))
}

pub fn update_live_goal_progress_persistent_only(
    goal_id: Option<&str>,
    update: LiveGoalProgressUpdate,
) -> Result<Option<LiveGoalSnapshot>, String> {
    let Some(mut goal) = read_live_goal_snapshot() else {
        return Ok(None);
    };

    if let Some(goal_id) = normalize_optional_str(goal_id) {
        if goal.id != goal_id {
            return Ok(None);
        }
    }

    apply_live_goal_progress_update_to_snapshot(&mut goal, update);
    persist_live_goal_snapshot(Some(&goal))?;
    Ok(Some(goal))
}

pub fn mark_live_goal_waiting_for_user(
    project_path: Option<&str>,
    request_id: Option<&str>,
) -> Result<Option<LiveGoalSnapshot>, String> {
    update_live_goal_interaction_phase(
        project_path,
        request_id,
        "waiting_for_user",
        "等待用户输入",
        "zhi_call",
    )
}

pub fn mark_live_goal_user_response_received(
    project_path: Option<&str>,
    request_id: Option<&str>,
) -> Result<Option<LiveGoalSnapshot>, String> {
    update_live_goal_interaction_phase(
        project_path,
        request_id,
        "running",
        "继续执行",
        "zhi_return",
    )
}

pub fn mark_live_goal_user_interaction_failed(
    project_path: Option<&str>,
    request_id: Option<&str>,
) -> Result<Option<LiveGoalSnapshot>, String> {
    update_live_goal_interaction_phase(
        project_path,
        request_id,
        "running",
        "等待输入失败",
        "zhi_error",
    )
}

pub fn should_auto_complete_live_goal_from_report(message: &str) -> bool {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return false;
    }

    let lower = trimmed.to_lowercase();
    let blocker_text = strip_resolved_completion_blocker_phrases(trimmed);
    let blockers = [
        "未完成",
        "没有完成",
        "尚未完成",
        "不算完成",
        "失败",
        "报错",
        "阻塞",
        "卡住",
        "无法",
        "不能",
        "没能",
        "需要你确认",
        "需要确认",
        "先停",
        "暂停",
    ];
    if blockers.iter().any(|needle| blocker_text.contains(needle)) {
        return false;
    }

    let lower_blocker_text = strip_resolved_completion_blocker_phrases(&lower);
    let lower_blockers = [
        "not completed",
        "not done",
        "failed",
        "blocked",
        "stuck",
        "needs confirmation",
        "need confirmation",
        "pause",
    ];
    if lower_blockers
        .iter()
        .any(|needle| lower_blocker_text.contains(needle))
    {
        return false;
    }

    let completion_markers = [
        "任务完成",
        "任务已完成",
        "已经完成",
        "已完成",
        "完成了",
        "我这边已经完成",
        "验证通过",
        "测试通过",
        "验收通过",
        "已生效",
        "已闭环",
        "闭环完成",
        "构建成功",
        "替换完成",
        "重启成功",
        "行为正常",
    ];
    completion_markers
        .iter()
        .any(|needle| trimmed.contains(needle))
        || lower.contains("task completed")
        || lower.contains("completed successfully")
        || lower.contains("verification passed")
        || lower.contains("tests passed")
        || lower.contains("build succeeded")
}

fn strip_resolved_completion_blocker_phrases(text: &str) -> String {
    let resolved_blocker_phrases = [
        "阻塞项：无",
        "阻塞项: 无",
        "阻塞项：没有",
        "阻塞项: 没有",
        "阻塞：无",
        "阻塞: 无",
        "无阻塞",
        "没有阻塞",
        "无 blocker",
        "无 blockers",
        "blocker: none",
        "blockers: none",
        "blocker：none",
        "blockers：none",
        "no blocker",
        "no blockers",
        "not blocked",
    ];

    let mut stripped = text.to_string();
    for phrase in resolved_blocker_phrases {
        stripped = stripped.replace(phrase, "");
    }
    stripped
}

pub fn complete_live_goal_from_report(
    project_path: Option<&str>,
    request_id: Option<&str>,
) -> Result<Option<LiveGoalSnapshot>, String> {
    let Some(mut goal) = read_live_goal_snapshot() else {
        return Ok(None);
    };

    if is_terminal_live_goal_status(&goal.status) || !goal_matches_project(&goal, project_path) {
        return Ok(None);
    }

    complete_live_goal_snapshot(&mut goal, project_path, request_id, "zhi_completion_report");
    persist_live_goal_snapshot(Some(&goal))?;
    schedule_live_goal_live_activity_apns(&goal, "end");
    Ok(Some(goal))
}

fn update_live_goal_interaction_phase(
    project_path: Option<&str>,
    request_id: Option<&str>,
    phase: &str,
    status_text: &str,
    source: &str,
) -> Result<Option<LiveGoalSnapshot>, String> {
    let Some(mut goal) = read_live_goal_snapshot() else {
        return Ok(None);
    };

    if !apply_live_goal_interaction_phase_to_snapshot(
        &mut goal,
        project_path,
        request_id,
        phase,
        status_text,
        source,
    ) {
        return Ok(None);
    }

    persist_live_goal_snapshot(Some(&goal))?;
    schedule_live_goal_live_activity_apns(&goal, "update");
    Ok(Some(goal))
}

fn complete_live_goal_snapshot(
    goal: &mut LiveGoalSnapshot,
    project_path: Option<&str>,
    request_id: Option<&str>,
    source: &str,
) {
    let now = chrono::Utc::now().timestamp_millis();
    goal.status = "completed".to_string();
    goal.updated_at_ms = now;
    goal.completed_at_ms = Some(now);
    goal.phase = Some("completed".to_string());
    goal.status_text = Some("已完成".to_string());
    goal.progress_percent = Some(100.0);
    goal.progress_source = Some(source.to_string());
    goal.progress_label = Some("100%".to_string());
    if let Some(project_path) = normalize_optional_str(project_path) {
        goal.project_path = Some(project_path);
    }
    if let Some(request_id) = normalize_optional_str(request_id) {
        goal.request_id = Some(request_id);
    }
    goal.source = source.to_string();
}

fn apply_live_goal_interaction_phase_to_snapshot(
    goal: &mut LiveGoalSnapshot,
    project_path: Option<&str>,
    request_id: Option<&str>,
    phase: &str,
    status_text: &str,
    source: &str,
) -> bool {
    apply_live_goal_event_to_snapshot(
        goal,
        LiveGoalEvent::interaction_phase(project_path, request_id, phase, status_text, source),
    )
}

fn apply_live_goal_quota_status_to_snapshot(
    goal: &mut LiveGoalSnapshot,
    project_path: Option<&str>,
    request_id: Option<&str>,
    status_text: &str,
) -> bool {
    let Some(status_text) = normalize_optional_str(Some(status_text)) else {
        return false;
    };

    apply_live_goal_event_to_snapshot(
        goal,
        LiveGoalEvent::quota_status(project_path, request_id, &status_text),
    )
}

fn apply_live_goal_event_to_snapshot(goal: &mut LiveGoalSnapshot, event: LiveGoalEvent) -> bool {
    match event.kind {
        LiveGoalEventKind::ProgressUpdate(update) => {
            if let Some(stale) = live_goal_run_stale_context(
                goal,
                update.project_path.as_deref(),
                update.run_id.as_deref(),
                update.generation,
                true,
            ) {
                log_stale_live_goal_update_skip("progress_update", goal, &stale);
                return false;
            }

            apply_live_goal_progress_update_fields_to_snapshot(goal, update, event.now_ms);
            true
        }
        LiveGoalEventKind::InteractionPhase { phase, status_text } => {
            if is_terminal_live_goal_status(&goal.status)
                || !goal_matches_project(goal, event.project_path.as_deref())
            {
                return false;
            }

            apply_live_goal_progress_update_fields_to_snapshot(
                goal,
                LiveGoalProgressUpdate {
                    status: Some("running".to_string()),
                    phase: Some(phase),
                    status_text: Some(status_text),
                    project_path: event.project_path,
                    request_id: event.request_id,
                    last_codex_event_at_ms: Some(event.now_ms),
                    source: Some(event.source),
                    ..Default::default()
                },
                event.now_ms,
            );
            true
        }
        LiveGoalEventKind::QuotaStatus { status_text } => {
            if is_terminal_live_goal_status(&goal.status)
                || !goal_matches_project(goal, event.project_path.as_deref())
                || should_skip_live_goal_quota_status(goal)
            {
                return false;
            }

            goal.updated_at_ms = event.now_ms;
            goal.status_text = Some(status_text);
            goal.last_codex_event_at_ms = Some(event.now_ms);
            if let Some(project_path) = event.project_path {
                goal.project_path = Some(project_path);
            }
            if let Some(request_id) = event.request_id {
                goal.request_id = Some(request_id);
            }
            goal.source = event.source;
            true
        }
    }
}

fn should_skip_live_goal_quota_status(goal: &LiveGoalSnapshot) -> bool {
    if goal
        .phase
        .as_deref()
        .is_some_and(is_higher_priority_live_goal_phase)
    {
        return true;
    }

    is_higher_priority_live_goal_source(Some(goal.source.as_str()))
        || is_higher_priority_live_goal_source(goal.progress_source.as_deref())
}

fn is_higher_priority_live_goal_phase(phase: &str) -> bool {
    matches!(
        phase.trim().to_lowercase().as_str(),
        "waiting_for_user" | "waiting_for_approval" | "completed" | "cleared" | "failed"
    )
}

fn is_higher_priority_live_goal_source(source: Option<&str>) -> bool {
    let Some(source) = source else {
        return false;
    };
    let source = source.trim().to_lowercase();
    source == "apns_live_activity_update" || source == "plan_progress" || source.starts_with("zhi_")
}

fn is_terminal_live_goal_status(status: &str) -> bool {
    matches!(
        status.trim().to_lowercase().as_str(),
        "completed" | "cleared" | "cancelled" | "canceled" | "expired"
    )
}

fn apply_live_goal_progress_update_to_snapshot(
    goal: &mut LiveGoalSnapshot,
    update: LiveGoalProgressUpdate,
) {
    let _ = apply_live_goal_event_to_snapshot(goal, LiveGoalEvent::progress_update(update));
}

fn apply_live_goal_progress_update_fields_to_snapshot(
    goal: &mut LiveGoalSnapshot,
    update: LiveGoalProgressUpdate,
    now: i64,
) {
    goal.updated_at_ms = now;

    if let Some(status) = normalize_optional_owned(update.status) {
        goal.status = status;
    }
    if let Some(phase) = normalize_optional_owned(update.phase) {
        goal.phase = Some(phase);
    }
    if let Some(status_text) = normalize_optional_owned(update.status_text) {
        goal.status_text = Some(status_text);
    }
    if let Some(progress_percent) = update.progress_percent.and_then(normalize_progress_percent) {
        goal.progress_percent = Some(progress_percent);
    }
    if let Some(progress_source) = normalize_optional_owned(update.progress_source) {
        goal.progress_source = Some(progress_source);
    }
    if let Some(progress_label) = normalize_optional_owned(update.progress_label) {
        goal.progress_label = Some(progress_label);
    }
    if let Some(plan_total) = update.plan_total {
        goal.plan_total = Some(plan_total);
    }
    if let Some(plan_completed) = update.plan_completed {
        goal.plan_completed = Some(plan_completed);
    }
    if let Some(tokens_used) = update.tokens_used {
        goal.tokens_used = Some(tokens_used);
    }
    if let Some(token_budget) = update.token_budget {
        goal.token_budget = Some(token_budget);
    }
    if let Some(time_used_seconds) = update.time_used_seconds {
        goal.time_used_seconds = Some(time_used_seconds);
    }
    if let Some(project_path) = normalize_optional_owned(update.project_path) {
        goal.project_path = Some(project_path);
    }
    if let Some(request_id) = normalize_optional_owned(update.request_id) {
        goal.request_id = Some(request_id);
    }
    if let Some(codex_thread_id) = normalize_optional_owned(update.codex_thread_id) {
        goal.codex_thread_id = Some(codex_thread_id);
    }
    if let Some(codex_deeplink) = normalize_optional_owned(update.codex_deeplink) {
        goal.codex_deeplink = Some(codex_deeplink);
    }
    if let Some(run_id) = normalize_optional_owned(update.run_id) {
        goal.run_id = Some(run_id);
    }
    if let Some(generation) = update.generation {
        goal.generation = Some(generation);
    }
    if let Some(stale_of) = normalize_optional_owned(update.stale_of) {
        goal.stale_of = Some(stale_of);
    }
    if let Some(superseded_by) = normalize_optional_owned(update.superseded_by) {
        goal.superseded_by = Some(superseded_by);
    }
    if let Some(last_codex_event_at_ms) = update.last_codex_event_at_ms {
        goal.last_codex_event_at_ms = Some(last_codex_event_at_ms);
    }
    if let Some(source) = normalize_optional_owned(update.source) {
        goal.source = source;
    }

    if goal.status == "completed" {
        goal.completed_at_ms.get_or_insert(now);
        goal.phase.get_or_insert_with(|| "completed".to_string());
        goal.status_text.get_or_insert_with(|| "已完成".to_string());
        goal.progress_percent = Some(100.0);
    }
}

#[tauri::command]
pub fn clear_live_goal(
    app: AppHandle,
    state: tauri::State<'_, LiveGoalTrayState>,
) -> Result<(), String> {
    clear_live_goal_inner(&app, &state)
}

fn clear_live_goal_inner<R: Runtime>(
    _app: &AppHandle<R>,
    state: &LiveGoalTrayState,
) -> Result<(), String> {
    if let Some(mut goal) = state.get() {
        let now = chrono::Utc::now().timestamp_millis();
        goal.status = "cleared".to_string();
        goal.phase = Some("cleared".to_string());
        goal.status_text = Some("已清除".to_string());
        goal.updated_at_ms = now;
        goal.completed_at_ms.get_or_insert(now);
        schedule_live_goal_live_activity_apns(&goal, "end");
    }
    state.set(None);
    Ok(())
}

pub fn ensure_live_goal_in_mcp_state<R: Runtime>(
    app: Option<&AppHandle<R>>,
    payload: &mut serde_json::Value,
    project_path: Option<&str>,
) {
    let Some(object) = payload.as_object_mut() else {
        return;
    };

    if object
        .get("live_goal")
        .map(|value| !value.is_null())
        .unwrap_or(false)
    {
        return;
    }

    if let Some(live_goal) = live_goal_payload_for_project(app, project_path) {
        object.insert("live_goal".to_string(), live_goal);
    }
}

pub fn live_goal_payload_for_project<R: Runtime>(
    app: Option<&AppHandle<R>>,
    project_path: Option<&str>,
) -> Option<serde_json::Value> {
    let goal = current_live_goal_snapshot(app)?;
    if !goal_matches_project(&goal, project_path) {
        return None;
    }

    Some(live_goal_payload(&goal))
}

pub fn live_goal_snapshot_for_project_strict(
    project_path: Option<&str>,
) -> Option<LiveGoalSnapshot> {
    let requested_project_path = normalize_optional_str(project_path)?;
    let goal = read_live_goal_snapshot()?;
    if is_terminal_live_goal_status(&goal.status) {
        return None;
    }

    let goal_project_path = goal
        .project_path
        .as_ref()
        .and_then(|value| normalize_optional_str(Some(value.as_str())))?;
    if goal_project_path != requested_project_path {
        return None;
    }

    Some(goal)
}

pub fn live_goal_codex_thread_id_for_project(project_path: Option<&str>) -> Option<String> {
    let goal = read_live_goal_snapshot()?;
    live_goal_codex_thread_id_from_snapshot(&goal, project_path)
}

pub fn live_goal_codex_thread_id_for_project_with_app<R: Runtime>(
    app: Option<&AppHandle<R>>,
    project_path: Option<&str>,
) -> Option<String> {
    let goal = current_live_goal_snapshot(app)?;
    live_goal_codex_thread_id_from_snapshot(&goal, project_path)
}

pub fn start_live_goal_tray_timer(_app: AppHandle) {
    // Live Goal is kept as a transport state; visible display moves to iOS ActivityKit.
}

fn resolve_live_goal_intent_from_response(response: &serde_json::Value) -> Option<LiveGoalIntent> {
    if let Some(user_input) = response.get("user_input").and_then(|value| value.as_str()) {
        if let Some(intent) = parse_live_goal_intent_value(user_input) {
            return Some(intent);
        }
    }

    let options = response
        .get("selected_options")
        .and_then(|value| value.as_array())?;

    for option in options {
        if let Some(value) = option.as_str() {
            if let Some(intent) = parse_live_goal_intent_value(value) {
                return Some(intent);
            }
        }
    }

    None
}

fn parse_live_goal_intent_value(value: &str) -> Option<LiveGoalIntent> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    let command_text = trimmed
        .strip_prefix("@/goal")
        .or_else(|| trimmed.strip_prefix("＠/goal"))
        .map(|suffix| format!("/goal{}", suffix))
        .unwrap_or_else(|| trimmed.to_string());

    let lower_command_text = command_text.to_lowercase();
    let payload = if lower_command_text == "/goal"
        || lower_command_text.starts_with("/goal ")
        || lower_command_text.starts_with("/goal:")
        || lower_command_text.starts_with("/goal：")
    {
        command_text["/goal".len()..]
            .trim()
            .trim_start_matches([':', '：'])
            .trim()
            .to_string()
    } else if lower_command_text.starts_with("goal:") || lower_command_text.starts_with("goal：") {
        command_text["goal".len()..]
            .trim()
            .trim_start_matches([':', '：'])
            .trim()
            .to_string()
    } else {
        return None;
    };

    let command = payload.to_lowercase();
    if ["done", "complete", "finish", "完成", "已完成"].contains(&command.as_str()) {
        return Some(LiveGoalIntent::Complete);
    }

    if ["clear", "cancel", "stop", "reset", "清除", "取消", "停止"].contains(&command.as_str())
    {
        return Some(LiveGoalIntent::Clear);
    }

    let title = strip_live_goal_start_keyword(&payload);
    if title.is_empty() {
        return None;
    }

    Some(LiveGoalIntent::Start(title.to_string()))
}

fn strip_live_goal_start_keyword(payload: &str) -> &str {
    let trimmed = payload.trim();
    let lower_trimmed = trimmed.to_lowercase();

    for keyword in ["start", "开始", "启动"] {
        let prefix = format!("{} ", keyword);
        if lower_trimmed.starts_with(&prefix) {
            return trimmed[prefix.len()..].trim();
        }
    }

    trimmed
}

fn apply_codex_goal_observer_update_to_current(
    current: Option<LiveGoalSnapshot>,
    update: CodexGoalObserverUpdate,
) -> Result<Option<LiveGoalSnapshot>, String> {
    let Some(update) = normalize_codex_goal_observer_update(update) else {
        return Ok(None);
    };
    let now = chrono::Utc::now().timestamp_millis();

    let Some(mut goal) = current else {
        return Ok(Some(build_codex_goal_observer_snapshot(&update, now)));
    };

    if !goal_matches_project(&goal, Some(update.project_path.as_str())) {
        return Ok(None);
    }

    if goal.id == update.goal_id {
        if should_skip_same_codex_goal_update(&goal, &update) {
            return Ok(None);
        }
        apply_codex_goal_observer_fields(&mut goal, &update, now);
        return Ok(Some(goal));
    }

    if let Some(stale) = live_goal_run_stale_context(
        &goal,
        Some(update.project_path.as_str()),
        Some(update.goal_id.as_str()),
        live_goal_generation(update.created_at_ms),
        true,
    ) {
        log_stale_live_goal_update_skip("codex_goal_observer", &goal, &stale);
        return Ok(None);
    }

    if can_replace_goal_with_codex_goal(&goal, &update) {
        return Ok(Some(build_codex_goal_observer_snapshot(&update, now)));
    }

    Ok(None)
}

fn normalize_codex_goal_observer_update(
    update: CodexGoalObserverUpdate,
) -> Option<CodexGoalObserverUpdate> {
    let goal_id = normalize_optional_str(Some(update.goal_id.as_str()))?;
    let project_path = normalize_optional_str(Some(update.project_path.as_str()))?;
    let codex_thread_id = normalize_optional_str(Some(update.codex_thread_id.as_str()))?;
    let status = normalize_codex_goal_live_status(update.status.as_str())?;
    let title = normalize_optional_str(Some(update.title.as_str()))
        .unwrap_or_else(|| "Codex Goal".to_string());

    Some(CodexGoalObserverUpdate {
        goal_id,
        title,
        status,
        project_path,
        codex_thread_id,
        token_budget: update.token_budget,
        tokens_used: update.tokens_used,
        time_used_seconds: update.time_used_seconds,
        created_at_ms: update.created_at_ms,
        updated_at_ms: update.updated_at_ms,
    })
}

fn normalize_codex_goal_live_status(status: &str) -> Option<String> {
    match status.trim().to_ascii_lowercase().as_str() {
        "running" | "active" => Some("running".to_string()),
        "paused" => Some("paused".to_string()),
        "blocked" | "usage_limited" | "budget_limited" => Some("blocked".to_string()),
        "completed" | "complete" => Some("completed".to_string()),
        _ => None,
    }
}

fn should_skip_same_codex_goal_update(
    goal: &LiveGoalSnapshot,
    update: &CodexGoalObserverUpdate,
) -> bool {
    if is_terminal_live_goal_status(&goal.status) && update.status != "completed" {
        return true;
    }

    goal.last_codex_event_at_ms
        .is_some_and(|last_event_at_ms| update.updated_at_ms <= last_event_at_ms)
}

fn can_replace_goal_with_codex_goal(
    goal: &LiveGoalSnapshot,
    update: &CodexGoalObserverUpdate,
) -> bool {
    if goal.source == CODEX_GOAL_OBSERVER_SOURCE {
        return update.created_at_ms > goal.started_at_ms;
    }

    if !is_terminal_live_goal_status(&goal.status) {
        return false;
    }

    let terminal_at_ms = goal.completed_at_ms.unwrap_or(goal.updated_at_ms);
    update.created_at_ms > terminal_at_ms
}

fn build_codex_goal_observer_snapshot(
    update: &CodexGoalObserverUpdate,
    now: i64,
) -> LiveGoalSnapshot {
    let started_at_ms = if update.created_at_ms > 0 {
        update.created_at_ms
    } else {
        now
    };
    let mut goal = LiveGoalSnapshot {
        id: update.goal_id.clone(),
        title: live_goal_title(update.title.as_str()),
        status: update.status.clone(),
        phase: Some(update.status.clone()),
        status_text: Some(codex_goal_observer_status_text(update.status.as_str()).to_string()),
        progress_percent: Some(if update.status == "completed" {
            100.0
        } else {
            0.0
        }),
        progress_source: Some(CODEX_GOAL_OBSERVER_SOURCE.to_string()),
        progress_label: Some(if update.status == "completed" {
            "100%".to_string()
        } else {
            "0%".to_string()
        }),
        plan_total: None,
        plan_completed: None,
        tokens_used: update.tokens_used,
        token_budget: update.token_budget,
        time_used_seconds: update.time_used_seconds,
        started_at_ms,
        updated_at_ms: now,
        completed_at_ms: None,
        project_path: Some(update.project_path.clone()),
        request_id: None,
        codex_thread_id: Some(update.codex_thread_id.clone()),
        codex_deeplink: None,
        run_id: Some(update.goal_id.clone()),
        generation: live_goal_generation(started_at_ms),
        stale_of: None,
        superseded_by: None,
        last_codex_event_at_ms: Some(update.updated_at_ms),
        source: CODEX_GOAL_OBSERVER_SOURCE.to_string(),
    };

    if update.status == "completed" {
        goal.completed_at_ms = Some(now);
    }

    goal
}

fn apply_codex_goal_observer_fields(
    goal: &mut LiveGoalSnapshot,
    update: &CodexGoalObserverUpdate,
    now: i64,
) {
    goal.title = live_goal_title(update.title.as_str());
    goal.status = update.status.clone();
    goal.phase = Some(update.status.clone());
    goal.status_text = Some(codex_goal_observer_status_text(update.status.as_str()).to_string());
    goal.tokens_used = update.tokens_used;
    goal.token_budget = update.token_budget;
    goal.time_used_seconds = update.time_used_seconds;
    goal.project_path = Some(update.project_path.clone());
    goal.codex_thread_id = Some(update.codex_thread_id.clone());
    goal.run_id = Some(update.goal_id.clone());
    if let Some(generation) = live_goal_generation(update.created_at_ms) {
        goal.generation = Some(generation);
    }
    goal.last_codex_event_at_ms = Some(update.updated_at_ms);
    goal.updated_at_ms = now;
    goal.source = CODEX_GOAL_OBSERVER_SOURCE.to_string();

    if update.status == "completed" {
        goal.completed_at_ms.get_or_insert(now);
        goal.progress_percent = Some(100.0);
        goal.progress_source = Some(CODEX_GOAL_OBSERVER_SOURCE.to_string());
        goal.progress_label = Some("100%".to_string());
    } else if goal.progress_percent.is_none() {
        goal.progress_percent = Some(0.0);
    }
}

fn codex_goal_observer_status_text(status: &str) -> &'static str {
    match status {
        "running" => "Codex Goal 执行中",
        "paused" => "Codex Goal 已暂停",
        "blocked" => "Codex Goal 受限",
        "completed" => "已完成",
        _ => "Codex Goal 状态未知",
    }
}

fn live_goal_title(title: &str) -> String {
    title.chars().take(48).collect()
}

fn live_goal_generation(timestamp_ms: i64) -> Option<u64> {
    (timestamp_ms >= 0).then_some(timestamp_ms as u64)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LiveGoalStaleRun {
    incoming_run_id: Option<String>,
    incoming_generation: Option<u64>,
    stale_of: Option<String>,
    superseded_by: Option<String>,
}

fn live_goal_run_stale_context(
    current: &LiveGoalSnapshot,
    project_path: Option<&str>,
    incoming_run_id: Option<&str>,
    incoming_generation: Option<u64>,
    ignore_terminal_current: bool,
) -> Option<LiveGoalStaleRun> {
    if ignore_terminal_current && is_terminal_live_goal_status(&current.status) {
        return None;
    }

    let incoming_run_id = normalize_optional_str(incoming_run_id);
    if incoming_run_id.is_none() && incoming_generation.is_none() {
        return None;
    }

    if !goal_matches_project(current, project_path) {
        return None;
    }

    let current_run_id = normalize_optional_str(current.run_id.as_deref());
    let current_generation = current.generation;
    let generation_is_stale = incoming_generation
        .zip(current_generation)
        .is_some_and(|(incoming, current)| incoming < current);
    let run_mismatch = incoming_run_id
        .as_deref()
        .zip(current_run_id.as_deref())
        .is_some_and(|(incoming, current)| incoming != current);
    let mismatched_run_is_current_or_newer = incoming_generation
        .zip(current_generation)
        .is_none_or(|(incoming, current)| incoming <= current);

    if !generation_is_stale && !(run_mismatch && mismatched_run_is_current_or_newer) {
        return None;
    }

    let stale_of = incoming_run_id
        .clone()
        .or_else(|| incoming_generation.map(|generation| format!("generation:{}", generation)));

    Some(LiveGoalStaleRun {
        incoming_run_id,
        incoming_generation,
        stale_of,
        superseded_by: current_run_id,
    })
}

fn resolve_live_goal_response_metadata_from_current(
    current: Option<LiveGoalSnapshot>,
    project_path: Option<&str>,
    run_id: Option<&str>,
    generation: Option<u64>,
) -> LiveGoalResponseMetadata {
    let run_id = normalize_optional_str(run_id);
    let mut metadata = LiveGoalResponseMetadata {
        run_id: run_id.clone(),
        generation,
        ..Default::default()
    };

    let Some(current) = current else {
        return metadata;
    };

    if let Some(stale) =
        live_goal_run_stale_context(&current, project_path, run_id.as_deref(), generation, false)
    {
        metadata.stale_of = stale.stale_of;
        metadata.superseded_by = stale.superseded_by;
        metadata.is_stale = true;
    }

    metadata
}

fn log_stale_live_goal_update_skip(
    source: &str,
    current: &LiveGoalSnapshot,
    stale: &LiveGoalStaleRun,
) {
    crate::utils::append_timeline_debug_log(
        "rust/live_goal::stale_update_skipped",
        serde_json::json!({
            "source": source,
            "incoming_run_id": stale.incoming_run_id,
            "incoming_generation": stale.incoming_generation,
            "stale_of": stale.stale_of,
            "superseded_by": stale.superseded_by,
            "current_goal_id": current.id,
            "current_run_id": current.run_id,
            "current_generation": current.generation,
            "current_project_path": current.project_path,
        }),
    );
}

fn live_goal_run_id(now: i64, request_id: Option<&str>, codex_thread_id: Option<&str>) -> String {
    let suffix = normalize_optional_str(request_id)
        .or_else(|| normalize_optional_str(codex_thread_id))
        .map(|value| {
            value
                .chars()
                .filter(|ch| ch.is_ascii_alphanumeric())
                .take(12)
                .collect::<String>()
        })
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "manual".to_string());
    format!("goal_{}_{}", now.max(0), suffix)
}

fn build_live_goal_snapshot(
    title: &str,
    project_path: Option<&str>,
    request_id: Option<&str>,
    codex_thread_id: Option<&str>,
    codex_deeplink: Option<&str>,
    source: &str,
) -> LiveGoalSnapshot {
    let now = chrono::Utc::now().timestamp_millis();
    let run_id = live_goal_run_id(now, request_id, codex_thread_id);
    LiveGoalSnapshot {
        id: format!("goal_{}", now),
        title: live_goal_title(title),
        status: "running".to_string(),
        phase: Some("running".to_string()),
        status_text: Some("执行中".to_string()),
        progress_percent: Some(0.0),
        progress_source: Some(source.to_string()),
        progress_label: Some("0%".to_string()),
        plan_total: None,
        plan_completed: None,
        tokens_used: None,
        token_budget: None,
        time_used_seconds: None,
        started_at_ms: now,
        updated_at_ms: now,
        completed_at_ms: None,
        project_path: normalize_optional_str(project_path),
        request_id: normalize_optional_str(request_id),
        codex_thread_id: normalize_optional_str(codex_thread_id),
        codex_deeplink: normalize_optional_str(codex_deeplink),
        run_id: Some(run_id),
        generation: live_goal_generation(now),
        stale_of: None,
        superseded_by: None,
        last_codex_event_at_ms: None,
        source: source.to_string(),
    }
}

fn apply_live_goal_intent_persistent_only(
    intent: LiveGoalIntent,
    project_path: Option<&str>,
    request_id: Option<&str>,
) -> Result<(), String> {
    match intent {
        LiveGoalIntent::Start(title) => {
            let goal = build_live_goal_snapshot(
                &title,
                project_path,
                request_id,
                None,
                None,
                "goal_intent",
            );
            persist_live_goal_snapshot(Some(&goal))
        }
        LiveGoalIntent::Complete => {
            let Some(mut goal) = read_live_goal_snapshot() else {
                return Ok(());
            };
            complete_live_goal_snapshot(&mut goal, project_path, request_id, "goal_intent");
            persist_live_goal_snapshot(Some(&goal))
        }
        LiveGoalIntent::Clear => persist_live_goal_snapshot(None),
    }
}

fn log_live_goal_apply_result(result: Result<(), String>) {
    match result {
        Ok(()) => crate::utils::append_timeline_debug_log(
            "rust/live_goal::apply_from_response:applied",
            serde_json::json!({}),
        ),
        Err(error) => crate::utils::append_timeline_debug_log(
            "rust/live_goal::apply_from_response:failed",
            serde_json::json!({ "error": error }),
        ),
    }
}

fn current_live_goal_snapshot<R: Runtime>(app: Option<&AppHandle<R>>) -> Option<LiveGoalSnapshot> {
    if let Some(app) = app {
        if let Some(state) = app.try_state::<LiveGoalTrayState>() {
            if let Some(goal) = state.get() {
                return Some(goal);
            }
        }
    }

    read_live_goal_snapshot()
}

fn live_goal_payload(goal: &LiveGoalSnapshot) -> serde_json::Value {
    let elapsed_ms = match goal.completed_at_ms {
        Some(completed_at_ms) => completed_at_ms.saturating_sub(goal.started_at_ms),
        None => chrono::Utc::now()
            .timestamp_millis()
            .saturating_sub(goal.started_at_ms),
    };

    serde_json::json!({
        "id": goal.id,
        "goal_id": goal.id,
        "title": goal.title,
        "status": goal.status,
        "phase": goal.phase,
        "status_text": goal.status_text,
        "progress_percent": goal.progress_percent,
        "progress_source": goal.progress_source,
        "progress_label": goal.progress_label,
        "plan_total": goal.plan_total,
        "plan_completed": goal.plan_completed,
        "tokens_used": goal.tokens_used,
        "token_budget": goal.token_budget,
        "time_used_seconds": goal.time_used_seconds,
        "started_at_ms": goal.started_at_ms,
        "updated_at_ms": goal.updated_at_ms,
        "completed_at_ms": goal.completed_at_ms,
        "elapsed_ms": elapsed_ms.max(0),
        "project_path": goal.project_path,
        "request_id": goal.request_id,
        "codex_thread_id": goal.codex_thread_id,
        "codex_deeplink": goal.codex_deeplink,
        "run_id": goal.run_id,
        "generation": goal.generation,
        "stale_of": goal.stale_of,
        "superseded_by": goal.superseded_by,
        "last_codex_event_at_ms": goal.last_codex_event_at_ms,
        "source": goal.source,
    })
}

fn schedule_live_goal_live_activity_apns(goal: &LiveGoalSnapshot, event: &'static str) {
    let payload = live_goal_payload(goal);
    tauri::async_runtime::spawn(async move {
        crate::bridge::ws::send_live_goal_live_activity_apns(payload, event).await;
    });
}

fn goal_matches_project(goal: &LiveGoalSnapshot, project_path: Option<&str>) -> bool {
    let Some(requested_project_path) = normalize_optional_str(project_path) else {
        return true;
    };
    let Some(goal_project_path) = goal
        .project_path
        .as_ref()
        .and_then(|value| normalize_optional_str(Some(value.as_str())))
    else {
        return true;
    };

    goal_project_path == requested_project_path
}

fn live_goal_codex_thread_id_from_snapshot(
    goal: &LiveGoalSnapshot,
    project_path: Option<&str>,
) -> Option<String> {
    if is_terminal_live_goal_status(&goal.status) {
        return None;
    }

    let requested_project_path = normalize_optional_str(project_path)?;
    let goal_project_path = goal
        .project_path
        .as_ref()
        .and_then(|value| normalize_optional_str(Some(value.as_str())))?;
    if goal_project_path != requested_project_path {
        return None;
    }

    normalize_optional_str(goal.codex_thread_id.as_deref())
}

fn normalize_optional_str(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != ".")
        .map(ToOwned::to_owned)
}

fn normalize_optional_owned(value: Option<String>) -> Option<String> {
    value.and_then(|value| normalize_optional_str(Some(value.as_str())))
}

fn normalize_progress_percent(value: f64) -> Option<f64> {
    if !value.is_finite() {
        return None;
    }
    Some(value.clamp(0.0, 100.0))
}

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

fn live_goal_store_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(data_dir) = dirs::data_local_dir().or_else(dirs::data_dir) {
        push_unique_path(&mut paths, data_dir.join("cunzhi").join("live_goal.json"));
    }

    if let Some(home) = dirs::home_dir() {
        push_unique_path(&mut paths, home.join(".cunzhi").join("live_goal.json"));
    }

    if let Some(config_dir) =
        dirs::config_dir().or_else(|| dirs::home_dir().map(|home| home.join(".config")))
    {
        push_unique_path(&mut paths, config_dir.join("cunzhi").join("live_goal.json"));
    }

    paths
}

fn persist_live_goal_snapshot_to_path(
    path: &Path,
    goal: Option<&LiveGoalSnapshot>,
) -> Result<(), String> {
    if let Some(goal) = goal {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let content = serde_json::to_string_pretty(goal).map_err(|error| error.to_string())?;
        fs::write(path, content).map_err(|error| error.to_string())?;
        if let Ok(file) = fs::OpenOptions::new().write(true).open(path) {
            let _ = file.sync_all();
        }
    } else if let Err(error) = fs::remove_file(path) {
        if error.kind() != std::io::ErrorKind::NotFound {
            return Err(error.to_string());
        }
    }

    Ok(())
}

fn persist_live_goal_snapshot(goal: Option<&LiveGoalSnapshot>) -> Result<(), String> {
    let paths = live_goal_store_paths();
    let Some(primary_path) = paths.first() else {
        return Err("无法解析 Live Goal 状态目录".to_string());
    };

    persist_live_goal_snapshot_to_path(primary_path, goal)?;

    for mirror_path in paths.iter().skip(1) {
        if let Err(error) = persist_live_goal_snapshot_to_path(mirror_path, goal) {
            log::warn!(
                "[LiveGoal] mirror persist failed: path={}, error={}",
                mirror_path.display(),
                error
            );
        }
    }

    Ok(())
}

fn read_live_goal_snapshot() -> Option<LiveGoalSnapshot> {
    live_goal_store_paths()
        .into_iter()
        .filter_map(|path| {
            let content = fs::read_to_string(&path).ok()?;
            let goal = serde_json::from_str::<LiveGoalSnapshot>(&content).ok()?;
            let modified_ms = fs::metadata(&path)
                .ok()
                .and_then(|metadata| metadata.modified().ok())
                .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
                .map(|duration| duration.as_millis())
                .unwrap_or(0);
            Some((modified_ms, goal))
        })
        .max_by_key(|(modified_ms, _)| *modified_ms)
        .map(|(_, goal)| goal)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn codex_observer_update(
        goal_id: &str,
        status: &str,
        created_at_ms: i64,
        updated_at_ms: i64,
    ) -> CodexGoalObserverUpdate {
        CodexGoalObserverUpdate {
            goal_id: goal_id.to_string(),
            title: "Official Codex goal".to_string(),
            status: status.to_string(),
            project_path: "/tmp/project".to_string(),
            codex_thread_id: "thread-1".to_string(),
            token_budget: Some(1_000),
            tokens_used: Some(120),
            time_used_seconds: Some(30),
            created_at_ms,
            updated_at_ms,
        }
    }

    #[test]
    fn progress_update_mutates_snapshot_without_persistence() {
        let mut goal = build_live_goal_snapshot(
            "progress smoke",
            Some("/tmp/project"),
            Some("request-1"),
            None,
            None,
            "test",
        );
        let started_at_ms = goal.started_at_ms;

        apply_live_goal_progress_update_to_snapshot(
            &mut goal,
            LiveGoalProgressUpdate {
                progress_percent: Some(57.0),
                progress_source: Some("apns_live_activity_update".to_string()),
                progress_label: Some("57%".to_string()),
                phase: Some("verification".to_string()),
                status_text: Some("APNs 57%".to_string()),
                ..Default::default()
            },
        );

        assert_eq!(goal.progress_percent, Some(57.0));
        assert_eq!(
            goal.progress_source.as_deref(),
            Some("apns_live_activity_update")
        );
        assert_eq!(goal.progress_label.as_deref(), Some("57%"));
        assert_eq!(goal.phase.as_deref(), Some("verification"));
        assert_eq!(goal.status_text.as_deref(), Some("APNs 57%"));
        assert!(goal.updated_at_ms >= started_at_ms);
    }

    #[test]
    fn progress_update_ignores_stale_run_identity() {
        let mut goal = build_live_goal_snapshot(
            "progress stale",
            Some("/tmp/project"),
            Some("request-2"),
            None,
            None,
            "test",
        );
        goal.run_id = Some("run-new".to_string());
        goal.generation = Some(200);
        goal.progress_percent = Some(12.0);
        goal.progress_label = Some("12%".to_string());

        let applied = apply_live_goal_event_to_snapshot(
            &mut goal,
            LiveGoalEvent::progress_update(LiveGoalProgressUpdate {
                project_path: Some("/tmp/project".to_string()),
                run_id: Some("run-old".to_string()),
                generation: Some(100),
                progress_percent: Some(99.0),
                progress_label: Some("99%".to_string()),
                ..Default::default()
            }),
        );

        assert!(!applied);
        assert_eq!(goal.run_id.as_deref(), Some("run-new"));
        assert_eq!(goal.generation, Some(200));
        assert_eq!(goal.progress_percent, Some(12.0));
        assert_eq!(goal.progress_label.as_deref(), Some("12%"));
    }

    #[test]
    fn response_metadata_marks_superseded_run_stale() {
        let mut current = build_live_goal_snapshot(
            "current run",
            Some("/tmp/project"),
            Some("request-2"),
            None,
            None,
            "test",
        );
        current.run_id = Some("run-new".to_string());
        current.generation = Some(200);

        let metadata = resolve_live_goal_response_metadata_from_current(
            Some(current),
            Some("/tmp/project"),
            Some("run-old"),
            Some(100),
        );

        assert!(metadata.is_stale);
        assert_eq!(metadata.run_id.as_deref(), Some("run-old"));
        assert_eq!(metadata.generation, Some(100));
        assert_eq!(metadata.stale_of.as_deref(), Some("run-old"));
        assert_eq!(metadata.superseded_by.as_deref(), Some("run-new"));
    }

    #[test]
    fn response_metadata_keeps_current_run_fresh() {
        let mut current = build_live_goal_snapshot(
            "current run",
            Some("/tmp/project"),
            Some("request-2"),
            None,
            None,
            "test",
        );
        current.run_id = Some("run-new".to_string());
        current.generation = Some(200);

        let metadata = resolve_live_goal_response_metadata_from_current(
            Some(current),
            Some("/tmp/project"),
            Some("run-new"),
            Some(200),
        );

        assert!(!metadata.is_stale);
        assert_eq!(metadata.run_id.as_deref(), Some("run-new"));
        assert_eq!(metadata.generation, Some(200));
        assert_eq!(metadata.stale_of, None);
        assert_eq!(metadata.superseded_by, None);
    }

    #[test]
    fn live_goal_thread_fallback_requires_running_matching_project() {
        let goal = build_live_goal_snapshot(
            "route fallback",
            Some("/tmp/project"),
            Some("request-1"),
            Some("thread-1"),
            None,
            "test",
        );

        assert!(goal
            .run_id
            .as_deref()
            .is_some_and(|run_id| { run_id.starts_with("goal_") && run_id.ends_with("request1") }));
        assert_eq!(goal.generation, live_goal_generation(goal.started_at_ms));
        assert_eq!(
            live_goal_codex_thread_id_from_snapshot(&goal, Some("/tmp/project")).as_deref(),
            Some("thread-1")
        );
        assert_eq!(
            live_goal_codex_thread_id_from_snapshot(&goal, Some("/tmp/other")),
            None
        );
        assert_eq!(live_goal_codex_thread_id_from_snapshot(&goal, None), None);
    }

    #[test]
    fn live_goal_thread_fallback_ignores_terminal_goal() {
        let mut goal = build_live_goal_snapshot(
            "route fallback",
            Some("/tmp/project"),
            Some("request-1"),
            Some("thread-1"),
            None,
            "test",
        );
        goal.status = "completed".to_string();

        assert_eq!(
            live_goal_codex_thread_id_from_snapshot(&goal, Some("/tmp/project")),
            None
        );
    }

    #[test]
    fn completed_progress_update_forces_full_progress() {
        let mut goal = build_live_goal_snapshot("complete smoke", None, None, None, None, "test");

        apply_live_goal_progress_update_to_snapshot(
            &mut goal,
            LiveGoalProgressUpdate {
                status: Some("completed".to_string()),
                phase: Some("completed".to_string()),
                status_text: Some("已完成".to_string()),
                progress_percent: Some(12.0),
                ..Default::default()
            },
        );

        assert_eq!(goal.status, "completed");
        assert_eq!(goal.phase.as_deref(), Some("completed"));
        assert_eq!(goal.status_text.as_deref(), Some("已完成"));
        assert_eq!(goal.progress_percent, Some(100.0));
        assert!(goal.completed_at_ms.is_some());
    }

    #[test]
    fn completion_report_detector_accepts_verified_completion() {
        assert!(should_auto_complete_live_goal_from_report(
            "目标按钮测试通过。\n\n结论：新安装版已生效，目标按钮行为正常。"
        ));
        assert!(should_auto_complete_live_goal_from_report(
            "这轮任务我这边已经完成。验证通过，阻塞项：无。"
        ));
        assert!(should_auto_complete_live_goal_from_report(
            "已完成，测试通过，无阻塞。"
        ));
        assert!(should_auto_complete_live_goal_from_report(
            "Task completed successfully. Verification passed. No blockers."
        ));
        assert!(should_auto_complete_live_goal_from_report(
            "Task completed successfully. Verification passed."
        ));
    }

    #[test]
    fn completion_report_detector_rejects_incomplete_or_blocked_reports() {
        assert!(!should_auto_complete_live_goal_from_report(
            "构建成功，但签名失败，需要你确认是否继续。"
        ));
        assert!(!should_auto_complete_live_goal_from_report(
            "构建成功，阻塞项：无，但签名失败。"
        ));
        assert!(!should_auto_complete_live_goal_from_report(
            "目前尚未完成，仍然阻塞在 APNs 验证。"
        ));
        assert!(!should_auto_complete_live_goal_from_report(
            "Not completed yet; needs confirmation before continuing."
        ));
    }

    #[test]
    fn complete_live_goal_snapshot_sets_terminal_progress() {
        let mut goal = build_live_goal_snapshot(
            "complete from report",
            Some("/tmp/project"),
            Some("request-1"),
            None,
            None,
            "manual",
        );

        complete_live_goal_snapshot(
            &mut goal,
            Some("/tmp/project"),
            Some("request-2"),
            "zhi_completion_report",
        );

        assert_eq!(goal.status, "completed");
        assert_eq!(goal.phase.as_deref(), Some("completed"));
        assert_eq!(goal.status_text.as_deref(), Some("已完成"));
        assert_eq!(goal.progress_percent, Some(100.0));
        assert_eq!(goal.progress_label.as_deref(), Some("100%"));
        assert_eq!(
            goal.progress_source.as_deref(),
            Some("zhi_completion_report")
        );
        assert_eq!(goal.source, "zhi_completion_report");
        assert_eq!(goal.request_id.as_deref(), Some("request-2"));
        assert!(goal.completed_at_ms.is_some());
    }

    #[test]
    fn interaction_phase_marks_waiting_without_rewriting_progress_source() {
        let mut goal = build_live_goal_snapshot(
            "interaction smoke",
            Some("/tmp/project"),
            Some("request-1"),
            None,
            None,
            "goal_intent",
        );
        goal.progress_percent = Some(37.0);
        goal.progress_source = Some("apns_live_activity_update".to_string());
        goal.progress_label = Some("37%".to_string());

        let applied = apply_live_goal_interaction_phase_to_snapshot(
            &mut goal,
            Some("/tmp/project"),
            Some("request-2"),
            "waiting_for_user",
            "等待用户输入",
            "zhi_call",
        );

        assert!(applied);
        assert_eq!(goal.status, "running");
        assert_eq!(goal.phase.as_deref(), Some("waiting_for_user"));
        assert_eq!(goal.status_text.as_deref(), Some("等待用户输入"));
        assert_eq!(goal.request_id.as_deref(), Some("request-2"));
        assert_eq!(goal.progress_percent, Some(37.0));
        assert_eq!(
            goal.progress_source.as_deref(),
            Some("apns_live_activity_update")
        );
        assert_eq!(goal.progress_label.as_deref(), Some("37%"));
        assert_eq!(goal.source, "zhi_call");
        assert!(goal.last_codex_event_at_ms.is_some());
    }

    #[test]
    fn interaction_phase_ignores_terminal_goal() {
        let mut goal = build_live_goal_snapshot("terminal smoke", None, None, None, None, "test");
        goal.status = "completed".to_string();
        goal.phase = Some("completed".to_string());
        goal.progress_percent = Some(100.0);

        let applied = apply_live_goal_interaction_phase_to_snapshot(
            &mut goal,
            None,
            Some("request-2"),
            "running",
            "继续执行",
            "zhi_return",
        );

        assert!(!applied);
        assert_eq!(goal.status, "completed");
        assert_eq!(goal.phase.as_deref(), Some("completed"));
        assert_eq!(goal.progress_percent, Some(100.0));
        assert!(goal.last_codex_event_at_ms.is_none());
    }

    #[test]
    fn quota_status_updates_text_without_rewriting_progress() {
        let mut goal = build_live_goal_snapshot(
            "quota smoke",
            Some("/tmp/project"),
            Some("request-1"),
            None,
            None,
            "manual",
        );
        goal.progress_percent = Some(41.0);
        goal.progress_source = Some("tool_running".to_string());
        goal.progress_label = Some("41%".to_string());
        goal.phase = Some("running".to_string());

        let applied = apply_live_goal_quota_status_to_snapshot(
            &mut goal,
            Some("/tmp/project"),
            Some("request-2"),
            "Codex 5h 88%",
        );

        assert!(applied);
        assert_eq!(goal.status_text.as_deref(), Some("Codex 5h 88%"));
        assert_eq!(goal.request_id.as_deref(), Some("request-2"));
        assert_eq!(goal.phase.as_deref(), Some("running"));
        assert_eq!(goal.progress_percent, Some(41.0));
        assert_eq!(goal.progress_source.as_deref(), Some("tool_running"));
        assert_eq!(goal.progress_label.as_deref(), Some("41%"));
        assert_eq!(goal.source, "codex_quota");
        assert!(goal.last_codex_event_at_ms.is_some());
    }

    #[test]
    fn quota_status_does_not_override_waiting_for_user() {
        let mut goal =
            build_live_goal_snapshot("quota waiting", None, None, None, None, "zhi_call");
        goal.phase = Some("waiting_for_user".to_string());
        goal.status_text = Some("等待用户输入".to_string());
        goal.progress_percent = Some(37.0);

        let applied =
            apply_live_goal_quota_status_to_snapshot(&mut goal, None, None, "Codex 5h 88%");

        assert!(!applied);
        assert_eq!(goal.phase.as_deref(), Some("waiting_for_user"));
        assert_eq!(goal.status_text.as_deref(), Some("等待用户输入"));
        assert_eq!(goal.progress_percent, Some(37.0));
        assert_eq!(goal.source, "zhi_call");
        assert!(goal.last_codex_event_at_ms.is_none());
    }

    #[test]
    fn quota_status_does_not_override_apns_progress_state() {
        let mut goal = build_live_goal_snapshot(
            "quota apns",
            Some("/tmp/project"),
            None,
            None,
            None,
            "goal_intent",
        );
        goal.status_text = Some("APNs 73%".to_string());
        goal.progress_percent = Some(73.0);
        goal.progress_source = Some("apns_live_activity_update".to_string());
        goal.progress_label = Some("73%".to_string());

        let applied = apply_live_goal_quota_status_to_snapshot(
            &mut goal,
            Some("/tmp/project"),
            None,
            "Codex 5h 88%",
        );

        assert!(!applied);
        assert_eq!(goal.status_text.as_deref(), Some("APNs 73%"));
        assert_eq!(goal.progress_percent, Some(73.0));
        assert_eq!(
            goal.progress_source.as_deref(),
            Some("apns_live_activity_update")
        );
        assert_eq!(goal.source, "goal_intent");
        assert!(goal.last_codex_event_at_ms.is_none());
    }

    #[test]
    fn codex_goal_observer_creates_snapshot_without_deriving_token_progress() {
        let goal = apply_codex_goal_observer_update_to_current(
            None,
            codex_observer_update("official-1", "running", 100, 200),
        )
        .unwrap()
        .unwrap();

        assert_eq!(goal.id, "official-1");
        assert_eq!(goal.title, "Official Codex goal");
        assert_eq!(goal.status, "running");
        assert_eq!(goal.progress_percent, Some(0.0));
        assert_eq!(goal.progress_label.as_deref(), Some("0%"));
        assert_eq!(goal.tokens_used, Some(120));
        assert_eq!(goal.token_budget, Some(1_000));
        assert_eq!(goal.last_codex_event_at_ms, Some(200));
        assert_eq!(goal.source, CODEX_GOAL_OBSERVER_SOURCE);
    }

    #[test]
    fn codex_goal_observer_skips_stale_same_goal_update() {
        let current = apply_codex_goal_observer_update_to_current(
            None,
            codex_observer_update("official-1", "running", 100, 200),
        )
        .unwrap()
        .unwrap();

        let skipped = apply_codex_goal_observer_update_to_current(
            Some(current),
            codex_observer_update("official-1", "running", 100, 200),
        )
        .unwrap();

        assert!(skipped.is_none());
    }

    #[test]
    fn codex_goal_observer_completes_same_goal() {
        let current = apply_codex_goal_observer_update_to_current(
            None,
            codex_observer_update("official-1", "running", 100, 200),
        )
        .unwrap()
        .unwrap();

        let completed = apply_codex_goal_observer_update_to_current(
            Some(current),
            codex_observer_update("official-1", "completed", 100, 300),
        )
        .unwrap()
        .unwrap();

        assert_eq!(completed.status, "completed");
        assert_eq!(completed.phase.as_deref(), Some("completed"));
        assert_eq!(completed.status_text.as_deref(), Some("已完成"));
        assert_eq!(completed.progress_percent, Some(100.0));
        assert_eq!(completed.progress_label.as_deref(), Some("100%"));
        assert_eq!(
            completed.progress_source.as_deref(),
            Some(CODEX_GOAL_OBSERVER_SOURCE)
        );
        assert!(completed.completed_at_ms.is_some());
        assert_eq!(completed.last_codex_event_at_ms, Some(300));
    }

    #[test]
    fn codex_goal_observer_does_not_replace_non_terminal_manual_goal() {
        let current = build_live_goal_snapshot(
            "manual goal",
            Some("/tmp/project"),
            Some("request-1"),
            None,
            None,
            "goal_intent",
        );

        let skipped = apply_codex_goal_observer_update_to_current(
            Some(current),
            codex_observer_update("official-1", "running", 200, 300),
        )
        .unwrap();

        assert!(skipped.is_none());
    }

    #[test]
    fn codex_goal_observer_replaces_terminal_goal_when_official_goal_is_newer() {
        let mut current = build_live_goal_snapshot(
            "old complete",
            Some("/tmp/project"),
            Some("request-1"),
            None,
            None,
            "goal_intent",
        );
        current.status = "completed".to_string();
        current.phase = Some("completed".to_string());
        current.completed_at_ms = Some(200);
        current.updated_at_ms = 200;

        let replaced = apply_codex_goal_observer_update_to_current(
            Some(current),
            codex_observer_update("official-1", "running", 300, 400),
        )
        .unwrap()
        .unwrap();

        assert_eq!(replaced.id, "official-1");
        assert_eq!(replaced.status, "running");
        assert_eq!(replaced.source, CODEX_GOAL_OBSERVER_SOURCE);
    }
}
