mod audio_lease;
mod continuity;

pub(super) fn last_continuity_project_path() -> Result<Option<String>, String> {
    continuity::last_project_path()
}

use axum::extract::ws::{Message, WebSocket};
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::Path;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Semaphore;
use tokio::time::{timeout, Duration};

const CODEX_APP_SERVER: &str = "/Applications/ChatGPT.app/Contents/Resources/codex";
const MAX_SDP_BYTES: usize = 64 * 1024;
const MAX_PROJECT_PATH_BYTES: usize = 4 * 1024;
const MAX_TRANSCRIPT_SEGMENTS: usize = 32;
const MAX_TRANSCRIPT_BYTES: usize = 16 * 1024;
const START_TIMEOUT: Duration = Duration::from_secs(20);
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(8);
const HUI_SKILL_NAME: &str = "hui";
const HUI_SKILL_RELATIVE_PATH: &str = ".cunzhi-knowledge/prompts/skills/hui/SKILL.md";
const XI_SKILL_NAME: &str = "xi";
const XI_SKILL_RELATIVE_PATH: &str = ".cunzhi-knowledge/prompts/skills/xi/SKILL.md";
static CODEX_LIVE_SESSION_SLOTS: Semaphore = Semaphore::const_new(1);

const CODEX_VOICE_DEVELOPER_INSTRUCTIONS: &str = r#"
You are the execution side of a realtime voice coordinator. The voice surface and this Codex
thread are one primary assistant. A direct actionable voice request arrives inside
<realtime_delegation> and contains <iterate_voice_actionable>true</iterate_voice_actionable>.

For actionable requests, carry the work through to a verified outcome in the supplied project.
Preserve unrelated user changes. Make reasonable assumptions instead of using an interactive
request-user-input tool; if an essential decision is missing, stop with one concise question in
the final response so the voice coordinator can ask it. Keep progress grounded and concise because
updates are streamed live to the user's iPhone.

This realtime_voice thread is the coordinating primary execution agent. Never invoke zhi, call_zhi,
or iterate, and never wait for a popup. The user has explicitly authorized smart built-in-subagent
delegation for GPT-Live tasks. Perform truly instantaneous work yourself when it is one
obvious direct action, such as opening an app, pausing media, or reading one known path. Delegate
bounded, time-consuming, or context-noisy work to built-in subagents, including research, repository
exploration, multi-file implementation, test or log analysis, and batch processing. Use at most three
concurrent children and parallelize only independent, non-overlapping work. Children must not spawn
more children or invoke zhi, call_zhi, or iterate. Do not use codex-room as an automatic fallback.

The primary agent owns scope, decisions, live user steering, real-diff inspection, verification, and
the consolidated final response. If the required built-in agent role or model is unavailable, report
that instead of silently substituting another role or taking over substantial delegated work. When
work finishes, report exactly what changed and how it was verified directly in the final response.
Ending the execution turn must not end the GPT-Live voice session; the host will return the same
connected session to discussion-only mode for the next request.

Voice shortcut routing is strict and must stay compact. Follow any skill injected by the host
immediately: hui is conversation-only recall, while xi restores unified conversation, project, and
experience context. Never substitute relearn for either one. Relearn is only for an explicit
relearn/relearn1/relearn0 request. Do not load every knowledge skill up front; use the injected
skill's progressive-disclosure rules. On macOS, if `rg` is not on PATH, call the bundled
`/Applications/ChatGPT.app/Contents/Resources/rg` directly instead of spending a turn probing and
falling back to another search command.
"#;

const REALTIME_VOICE_PROMPT: &str = r#"
You are Codex's realtime conversational surface on the user's active Iterate device (iPhone or
Mac). Speak naturally and briefly in the user's language. Help the user clarify the desired
outcome and constraints. Until you receive the developer control message
[ITERATE_EXECUTION_ACTIVE], do not send a handoff_request, do not claim that tool work has already
finished, and do not perform or delegate any action yourself. Never ask for execution confirmation
and never repeat “我再确认一下” or similar filler. When the user gives a direct actionable request,
acknowledge it once in one short sentence; the host automatically starts the Codex execution turn.
If the user is only asking a conversational question, answer it directly without opening a task.

In discussion-only mode you have no search, file, terminal, web, screen, or execution tools. Never
say or imply “我查一下”, “我在看”, “我再催一下”, “稍等”, “马上”, or any equivalent claim that
you are checking, searching, reading, writing, running, waiting for, or following up on tool work.
For an explicit request to search, research, inspect, read, organize, analyze, write, modify, run,
test, install, continue an already agreed task, delegate it to built-in subagents, or invoke hui/xi,
acknowledge once and wait for the automatic execution update. Do not promise a later result.

While [ITERATE_EXECUTION_ACTIVE] is in effect, backend Codex updates may arrive as conversation
context. Treat them as authoritative, summarize only useful progress, and allow the user to steer
the running task by speaking. When [ITERATE_EXECUTION_IDLE] arrives, return to discussion mode; the
next direct actionable voice request will automatically start a fresh Codex turn.
"#;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
    Start {
        session_id: String,
        sdp: String,
        project_path: String,
    },
    Confirm {
        session_id: String,
    },
    Interrupt {
        session_id: String,
    },
    Stop {
        session_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TranscriptSegment {
    role: String,
    text: String,
}

#[derive(Debug)]
struct ExecutionCompletion {
    speak_text: String,
    requires_interrupt: bool,
}

pub async fn serve(socket: WebSocket) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let start = match timeout(Duration::from_secs(15), ws_receiver.next()).await {
        Ok(Some(Ok(Message::Text(text)))) if text.len() <= 96 * 1024 => {
            serde_json::from_str::<ClientMessage>(&text).ok()
        }
        _ => None,
    };
    let Some(ClientMessage::Start {
        session_id,
        sdp,
        project_path,
    }) = start
    else {
        let _ = send_error(
            &mut ws_sender,
            None,
            "invalid_start",
            "需要有效的 start 消息",
        )
        .await;
        return;
    };
    if uuid::Uuid::parse_str(&session_id).is_err()
        || sdp.is_empty()
        || sdp.len() > MAX_SDP_BYTES
        || !sdp.lines().any(|line| line.starts_with("m=audio"))
    {
        let _ = send_error(
            &mut ws_sender,
            Some(&session_id),
            "invalid_offer",
            "WebRTC offer 缺少有效音频 SDP",
        )
        .await;
        return;
    }
    let project_path = match validated_project_path(&project_path) {
        Ok(project_path) => project_path,
        Err(error) => {
            let _ = send_error(
                &mut ws_sender,
                Some(&session_id),
                "invalid_project_path",
                &error,
            )
            .await;
            return;
        }
    };
    let _session_permit = match CODEX_LIVE_SESSION_SLOTS.try_acquire() {
        Ok(permit) => permit,
        Err(_) => {
            let _ = send_error(
                &mut ws_sender,
                Some(&session_id),
                "live_session_busy",
                "另一台设备正在使用 Codex GPT-Live",
            )
            .await;
            return;
        }
    };
    let _audio_lease = match audio_lease::try_acquire() {
        Ok(Some(lease)) => lease,
        Ok(None) => {
            let _ = send_error(
                &mut ws_sender,
                Some(&session_id),
                "live_session_busy",
                "另一台设备或另一个 Iterate 服务正在使用 Codex GPT-Live",
            )
            .await;
            return;
        }
        Err(error) => {
            let _ = send_error(
                &mut ws_sender,
                Some(&session_id),
                "live_session_lock_failed",
                &error,
            )
            .await;
            return;
        }
    };

    if send_json(
        &mut ws_sender,
        json!({"type":"status","session_id":session_id,"status":"starting"}),
    )
    .await
    .is_err()
    {
        return;
    }

    let (start_result, start_cancelled) = {
        let start_future =
            CodexLiveProcess::start(&session_id, &sdp, &project_path, &mut ws_sender);
        tokio::pin!(start_future);
        let mut cancelled = false;
        loop {
            if cancelled {
                break (start_future.as_mut().await, true);
            }
            tokio::select! {
                result = &mut start_future => break (result, false),
                client_message = ws_receiver.next() => {
                    match client_message {
                        Some(Ok(Message::Text(text))) if text.len() <= 8 * 1024 => {
                            match serde_json::from_str::<ClientMessage>(&text) {
                                Ok(ClientMessage::Stop { session_id: requested }) if requested == session_id => {
                                    cancelled = true;
                                }
                                Ok(ClientMessage::Confirm { session_id: requested }) if requested == session_id => {}
                                Ok(ClientMessage::Interrupt { session_id: requested }) if requested == session_id => {}
                                _ => {}
                            }
                        }
                        Some(Ok(Message::Close(_))) | None | Some(Err(_)) => cancelled = true,
                        _ => {}
                    }
                }
            }
        }
    };
    if start_cancelled {
        if let Ok(mut process) = start_result {
            process.stop().await;
        }
        return;
    }
    let mut process = match start_result {
        Ok(process) => process,
        Err(error) => {
            let compact_error = error.split_whitespace().collect::<Vec<_>>().join(" ");
            eprintln!(
                "[codex-live] start failed code=codex_live_start_failed error={}",
                truncate_chars(&compact_error, 800)
            );
            let _ = send_error(
                &mut ws_sender,
                Some(&session_id),
                "codex_live_start_failed",
                &error,
            )
            .await;
            return;
        }
    };

    loop {
        tokio::select! {
            client_message = ws_receiver.next() => {
                match client_message {
                    Some(Ok(Message::Text(text))) if text.len() <= 8 * 1024 => {
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(ClientMessage::Stop { session_id: requested }) if requested == session_id => break,
                            Ok(ClientMessage::Confirm { session_id: requested }) if requested == session_id => {
                                if let Err(error) = process.confirm_execution(&mut ws_sender, &session_id).await {
                                    let _ = send_error(
                                        &mut ws_sender,
                                        Some(&session_id),
                                        "execution_start_failed",
                                        &error,
                                    )
                                    .await;
                                    if process.execution_active {
                                        break;
                                    }
                                }
                            }
                            Ok(ClientMessage::Interrupt { session_id: requested }) if requested == session_id => {
                                if let Err(error) = process
                                    .cancel_current_interaction(&mut ws_sender, &session_id)
                                    .await
                                {
                                    let _ = send_error(
                                        &mut ws_sender,
                                        Some(&session_id),
                                        "interaction_interrupt_failed",
                                        &error,
                                    )
                                    .await;
                                }
                            }
                            Ok(ClientMessage::Start { .. }) => {
                                let _ = send_error(&mut ws_sender, Some(&session_id), "session_already_active", "当前连接已有 Live 会话").await;
                            }
                            _ => {
                                let _ = send_error(&mut ws_sender, Some(&session_id), "invalid_message", "不支持的 Live 控制消息").await;
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    _ => {}
                }
            }
            line = process.lines.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        if let Ok(message) = serde_json::from_str::<Value>(&line) {
                            if process.forward_notification(&mut ws_sender, &session_id, &message).await.is_err() {
                                break;
                            }
                            if let Some(turn_id) = process.take_unconfirmed_turn() {
                                let _ = process.interrupt_turn(&turn_id, &mut ws_sender, &session_id).await;
                            }
                            if let Some(steering) = process.take_pending_steering() {
                                if process
                                    .steer_execution(&steering, &mut ws_sender, &session_id)
                                    .await
                                    .is_err()
                                {
                                    let _ = send_json(
                                        &mut ws_sender,
                                        json!({
                                            "type":"task_progress",
                                            "session_id":session_id,
                                            "kind":"activity",
                                            "text":"这句语音调整未能送达，Codex 继续执行原任务"
                                        }),
                                    )
                                    .await;
                                }
                            }
                            if process.take_auto_execution_request() {
                                if let Err(error) = process.confirm_execution(&mut ws_sender, &session_id).await {
                                    let _ = send_error(
                                        &mut ws_sender,
                                        Some(&session_id),
                                        "execution_start_failed",
                                        &error,
                                    )
                                    .await;
                                    if process.execution_active {
                                        break;
                                    }
                                }
                            }
                            if let Some(completion) = process.take_completion() {
                                if let Err(error) = process
                                    .lock_execution_gate(completion, &mut ws_sender, &session_id)
                                    .await
                                {
                                    let _ = send_error(
                                        &mut ws_sender,
                                        Some(&session_id),
                                        "execution_gate_reset_failed",
                                        &error,
                                    )
                                    .await;
                                    break;
                                }
                            }
                            if message.get("method").and_then(Value::as_str) == Some("thread/realtime/closed") {
                                break;
                            }
                        }
                    }
                    _ => break,
                }
            }
        }
    }

    process.stop().await;
    let _ = send_json(
        &mut ws_sender,
        json!({"type":"closed","session_id":session_id}),
    )
    .await;
}

struct CodexLiveProcess {
    child: Child,
    stdin: ChildStdin,
    lines: Lines<BufReader<ChildStdout>>,
    thread_id: String,
    project_path: String,
    transcript: Vec<TranscriptSegment>,
    execution_turn_id: Option<String>,
    unconfirmed_turn_id: Option<String>,
    pending_steering: Option<String>,
    completion: Option<ExecutionCompletion>,
    execution_active: bool,
    execution_confirmation_pending: bool,
    explicit_execution_request_pending: bool,
    auto_execution_requested: bool,
    using_daemon_proxy: bool,
    continuity_status: &'static str,
    next_id: u64,
}

impl CodexLiveProcess {
    async fn start(
        session_id: &str,
        offer_sdp: &str,
        project_path: &str,
        ws_sender: &mut SplitSink<WebSocket, Message>,
    ) -> Result<Self, String> {
        if !cfg!(target_os = "macos") {
            return Err("Codex GPT-Live 当前仅支持 Mac host".to_string());
        }
        if !std::path::Path::new(CODEX_APP_SERVER).is_file() {
            return Err("未找到已安装的 Codex 客户端".to_string());
        }

        let daemon_started = Self::try_start_daemon().await;
        let mut process = Self::spawn(project_path, daemon_started)?;
        if let Err(proxy_error) = process.initialize(ws_sender, session_id).await {
            if !daemon_started {
                return Err(proxy_error);
            }
            process.terminate_child().await;
            process = Self::spawn(project_path, false)?;
            process
                .initialize(ws_sender, session_id)
                .await
                .map_err(|fallback_error| {
                    format!(
                        "Codex daemon proxy 初始化失败（{proxy_error}），独立 app-server 回退也失败（{fallback_error}）"
                    )
                })?;
        }

        let continuity_project_path = project_path.to_string();
        let continuity_snapshot =
            tokio::task::spawn_blocking(move || continuity::load(&continuity_project_path))
                .await
                .map_err(|error| format!("读取 GPT-Live continuity 后台任务失败: {error}"))??;
        if continuity_snapshot.store_recovered {
            process.continuity_status = "store_recovered";
        }
        let initial_items = continuity::initial_items(&continuity_snapshot);
        let stored_thread_id = continuity_snapshot.thread_id.clone();
        let thread = if let Some(thread_id) = stored_thread_id.as_deref() {
            process.thread_id = thread_id.to_string();
            match process
                .request(
                    "thread/resume",
                    resume_thread_params(thread_id, project_path),
                    ws_sender,
                    session_id,
                )
                .await
            {
                Ok(thread) => {
                    process.continuity_status = "resumed";
                    thread
                }
                Err(error) if thread_resume_requires_replacement(&error) => {
                    process.thread_id.clear();
                    process.continuity_status = if thread_has_active_writer_conflict(&error) {
                        "forked_from_active_writer"
                    } else {
                        "recovered_with_new_thread"
                    };
                    eprintln!(
                        "[codex-live] replacing continuity thread reason={} old_thread={thread_id}",
                        process.continuity_status
                    );
                    process
                        .request(
                            "thread/start",
                            new_thread_params(project_path),
                            ws_sender,
                            session_id,
                        )
                        .await?
                }
                Err(error) => {
                    return Err(format!(
                        "无法安全恢复同项目 GPT-Live thread；未创建替代会话: {error}"
                    ));
                }
            }
        } else {
            process
                .request(
                    "thread/start",
                    new_thread_params(project_path),
                    ws_sender,
                    session_id,
                )
                .await?
        };
        let returned_thread_id = thread
            .pointer("/thread/id")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "Codex 未返回 Live thread id".to_string())?
            .to_string();
        if process.continuity_status == "resumed"
            && stored_thread_id.as_deref() != Some(returned_thread_id.as_str())
        {
            return Err("Codex 恢复了与项目 continuity 不一致的 thread".to_string());
        }
        let continuity_project_path = project_path.to_string();
        let expected_thread_id = stored_thread_id.clone();
        let continuity_thread_id = returned_thread_id.clone();
        tokio::task::spawn_blocking(move || {
            continuity::store_thread(
                &continuity_project_path,
                expected_thread_id.as_deref(),
                &continuity_thread_id,
            )
        })
        .await
        .map_err(|error| format!("保存 GPT-Live continuity 后台任务失败: {error}"))??;
        process.thread_id = returned_thread_id;

        if process.continuity_status == "resumed" {
            if let Some(turn_id) = active_turn_id(&thread) {
                process
                    .interrupt_or_confirm_terminal(&process.thread_id.clone(), &turn_id)
                    .await?;
            }
        }
        process
            .request(
                "thread/settings/update",
                json!({
                    "threadId": process.thread_id,
                    "approvalPolicy": "never",
                    "sandboxPolicy": {"type": "readOnly", "networkAccess": false}
                }),
                ws_sender,
                session_id,
            )
            .await?;

        let realtime_params = realtime_start_params(&process.thread_id, offer_sdp, initial_items);
        if let Err(error) = process
            .request(
                "thread/realtime/start",
                realtime_params,
                ws_sender,
                session_id,
            )
            .await
        {
            process.stop().await;
            return Err(error);
        }

        let answer = timeout(START_TIMEOUT, async {
            loop {
                let line = process
                    .lines
                    .next_line()
                    .await
                    .map_err(|error| format!("读取 Codex Live 事件失败: {error}"))?
                    .ok_or_else(|| "Codex Live 在返回 SDP 前退出".to_string())?;
                let message: Value = serde_json::from_str(&line)
                    .map_err(|error| format!("Codex Live 返回无效消息: {error}"))?;
                match message.get("method").and_then(Value::as_str) {
                    Some("thread/realtime/sdp") => {
                        let sdp = message
                            .pointer("/params/sdp")
                            .and_then(Value::as_str)
                            .filter(|value| !value.is_empty())
                            .ok_or_else(|| "Codex Live 返回空 SDP".to_string())?;
                        break Ok::<String, String>(sdp.to_string());
                    }
                    Some("thread/realtime/error") => {
                        let message = message
                            .pointer("/params/message")
                            .and_then(Value::as_str)
                            .unwrap_or("Codex GPT-Live 启动失败");
                        break Err(message.to_string());
                    }
                    _ => {
                        process
                            .forward_notification(ws_sender, session_id, &message)
                            .await
                            .map_err(|error| format!("回传 Codex Live 状态失败: {error}"))?;
                    }
                }
            }
        })
        .await
        .map_err(|_| "等待 Codex Live SDP 超时".to_string())
        .and_then(|result| result);
        let answer = match answer {
            Ok(answer) => answer,
            Err(error) => {
                process.stop().await;
                return Err(error);
            }
        };

        if let Err(error) = send_json(
            ws_sender,
            json!({"type":"answer","session_id":session_id,"sdp":answer}),
        )
        .await
        {
            process.stop().await;
            return Err(format!("回传 Codex Live SDP 失败: {error}"));
        }
        Ok(process)
    }

    async fn try_start_daemon() -> bool {
        let mut command = Command::new(CODEX_APP_SERVER);
        configure_codex_command(&mut command);
        command
            .args(["app-server", "daemon", "start"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let Ok(mut child) = command.spawn() else {
            return false;
        };
        match timeout(Duration::from_secs(12), child.wait()).await {
            Ok(Ok(status)) => status.success(),
            _ => {
                let _ = child.kill().await;
                let _ = child.wait().await;
                false
            }
        }
    }

    fn spawn(project_path: &str, use_proxy: bool) -> Result<Self, String> {
        let mut command = Command::new(CODEX_APP_SERVER);
        configure_codex_command(&mut command);
        command
            .args(if use_proxy {
                vec!["app-server", "proxy"]
            } else {
                vec!["app-server", "--listen", "stdio://"]
            })
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true);
        let mut child = command
            .spawn()
            .map_err(|error| format!("无法启动 Codex app-server: {error}"))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| "Codex app-server stdin 不可用".to_string())?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Codex app-server stdout 不可用".to_string())?;
        Ok(Self {
            child,
            stdin,
            lines: BufReader::new(stdout).lines(),
            thread_id: String::new(),
            project_path: project_path.to_string(),
            transcript: Vec::new(),
            execution_turn_id: None,
            unconfirmed_turn_id: None,
            pending_steering: None,
            completion: None,
            execution_active: false,
            execution_confirmation_pending: false,
            explicit_execution_request_pending: false,
            auto_execution_requested: false,
            using_daemon_proxy: use_proxy,
            continuity_status: "new",
            next_id: 1,
        })
    }

    async fn initialize(
        &mut self,
        ws_sender: &mut SplitSink<WebSocket, Message>,
        session_id: &str,
    ) -> Result<(), String> {
        self.request(
            "initialize",
            json!({
                "clientInfo": {
                    "name": "iterate-ios-codex-live",
                    "title": "iterate iOS Codex GPT-Live",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": {"experimentalApi": true}
            }),
            ws_sender,
            session_id,
        )
        .await
        .map(|_| ())
    }

    async fn initialize_silent(&mut self) -> Result<(), String> {
        self.request_silent(
            "initialize",
            json!({
                "clientInfo": {
                    "name": "iterate-ios-codex-live-cleanup",
                    "title": "iterate iOS Codex GPT-Live Cleanup",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": {"experimentalApi": true}
            }),
        )
        .await
        .map(|_| ())
    }

    async fn request(
        &mut self,
        method: &str,
        params: Value,
        ws_sender: &mut SplitSink<WebSocket, Message>,
        session_id: &str,
    ) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        let request = json!({"id": id, "method": method, "params": params});
        self.stdin
            .write_all(
                serde_json::to_string(&request)
                    .map_err(|error| error.to_string())?
                    .as_bytes(),
            )
            .await
            .map_err(|error| format!("写入 Codex app-server 失败: {error}"))?;
        self.stdin
            .write_all(b"\n")
            .await
            .map_err(|error| format!("写入 Codex app-server 失败: {error}"))?;
        self.stdin
            .flush()
            .await
            .map_err(|error| format!("刷新 Codex app-server 失败: {error}"))?;

        timeout(START_TIMEOUT, async {
            loop {
                let line = self
                    .lines
                    .next_line()
                    .await
                    .map_err(|error| format!("读取 Codex app-server 失败: {error}"))?
                    .ok_or_else(|| "Codex app-server 已退出".to_string())?;
                let message: Value = serde_json::from_str(&line)
                    .map_err(|error| format!("Codex app-server 返回无效消息: {error}"))?;
                if message.get("method").is_none()
                    && message.get("id").and_then(Value::as_u64) == Some(id)
                {
                    if let Some(error) = message.get("error") {
                        return Err(format!("Codex {method} 失败: {error}"));
                    }
                    return Ok(message.get("result").cloned().unwrap_or(Value::Null));
                }
                self.forward_notification(ws_sender, session_id, &message)
                    .await
                    .map_err(|error| format!("回传 Codex app-server 状态失败: {error}"))?;
            }
        })
        .await
        .map_err(|_| format!("Codex {method} 超时"))?
    }

    async fn request_silent(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        let mut encoded =
            serde_json::to_vec(&json!({"id": id, "method": method, "params": params}))
                .map_err(|error| error.to_string())?;
        encoded.push(b'\n');
        self.stdin
            .write_all(&encoded)
            .await
            .map_err(|error| format!("写入 Codex app-server 失败: {error}"))?;
        self.stdin
            .flush()
            .await
            .map_err(|error| format!("刷新 Codex app-server 失败: {error}"))?;

        timeout(CLEANUP_TIMEOUT, async {
            loop {
                let line = self
                    .lines
                    .next_line()
                    .await
                    .map_err(|error| format!("读取 Codex app-server 失败: {error}"))?
                    .ok_or_else(|| "Codex app-server 已退出".to_string())?;
                let message: Value = serde_json::from_str(&line)
                    .map_err(|error| format!("Codex app-server 返回无效消息: {error}"))?;
                if message.get("method").is_none()
                    && message.get("id").and_then(Value::as_u64) == Some(id)
                {
                    if let Some(error) = message.get("error") {
                        return Err(format!("Codex {method} 失败: {error}"));
                    }
                    return Ok(message.get("result").cloned().unwrap_or(Value::Null));
                }
            }
        })
        .await
        .map_err(|_| format!("Codex {method} 清理超时"))?
    }

    async fn confirm_execution(
        &mut self,
        ws_sender: &mut SplitSink<WebSocket, Message>,
        session_id: &str,
    ) -> Result<(), String> {
        if self.execution_active || self.execution_turn_id.is_some() {
            return Ok(());
        }
        if !self.execution_confirmation_pending && !self.explicit_execution_request_pending {
            return Err("本次连接尚未收到明确执行任务，请先说明要搜索、读取或修改什么".to_string());
        }
        let mut input = confirmed_delegation_input(&self.transcript, &self.project_path)?;
        self.execution_confirmation_pending = false;
        self.explicit_execution_request_pending = false;
        self.auto_execution_requested = false;
        let injected_skill = requested_voice_skill_input(&self.transcript);
        if let Some((skill_name, _)) = &injected_skill {
            input = format!("${skill_name}\n\n{input}");
        }
        let mut turn_input = vec![json!({
            "type": "text",
            "text": input,
            "text_elements": []
        })];
        if let Some((_, skill)) = injected_skill {
            turn_input.push(skill);
        }

        self.request(
            "thread/realtime/appendText",
            json!({
                "threadId": self.thread_id,
                "role": "developer",
                            "text": "[ITERATE_EXECUTION_ACTIVE] The user gave a direct actionable voice request. A Codex execution turn is starting now without a second confirmation step."
            }),
            ws_sender,
            session_id,
        )
        .await?;

        self.execution_active = true;
        let turn = self
            .request(
                "turn/start",
                json!({
                    "threadId": self.thread_id,
                    "input": turn_input,
                    "cwd": self.project_path,
                    "runtimeWorkspaceRoots": [self.project_path],
                    "approvalPolicy": "never",
                    "sandboxPolicy": {"type": "dangerFullAccess"}
                }),
                ws_sender,
                session_id,
            )
            .await;
        let turn = match turn {
            Ok(turn) => turn,
            Err(error) => {
                let cleanup = self.interrupt_active_execution_or_confirm_idle().await;
                if cleanup.is_ok() {
                    self.execution_active = false;
                    self.execution_turn_id = None;
                }
                let _ = self
                    .request(
                        "thread/realtime/appendText",
                        json!({
                            "threadId": self.thread_id,
                            "role": "developer",
                            "text": "[ITERATE_EXECUTION_IDLE] The Codex turn failed to start. Return to live discussion mode and wait for the next direct request."
                        }),
                        ws_sender,
                        session_id,
                    )
                    .await;
                return match cleanup {
                    Ok(()) => Err(error),
                    Err(cleanup_error) => Err(format!(
                        "{error}；并且无法确认潜在执行 turn 已停止: {cleanup_error}"
                    )),
                };
            }
        };
        let response_turn_id = turn
            .pointer("/turn/id")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let turn_id = match response_turn_id.or_else(|| self.execution_turn_id.clone()) {
            Some(turn_id) => turn_id,
            None => match self.read_active_turn_id().await {
                Ok(Some(turn_id)) => turn_id,
                Ok(None) => {
                    self.execution_active = false;
                    let _ = self
                        .request(
                            "thread/realtime/appendText",
                            json!({
                                "threadId": self.thread_id,
                                "role": "developer",
                                "text": "[ITERATE_EXECUTION_IDLE] The Codex turn did not start. Return to live discussion mode and wait for the next direct request."
                            }),
                            ws_sender,
                            session_id,
                        )
                        .await;
                    return Err("Codex 未返回执行 turn id，且 thread 中没有活动 turn".to_string());
                }
                Err(error) => {
                    return Err(format!(
                        "Codex 未返回执行 turn id，也无法确认 thread 是否存在活动 turn: {error}"
                    ));
                }
            },
        };
        self.execution_turn_id = Some(turn_id.clone());
        send_json(
            ws_sender,
            json!({
                "type": "task_started",
                "session_id": session_id,
                "thread_id": self.thread_id,
                "turn_id": turn_id,
                "project_path": self.project_path
            }),
        )
        .await
        .map_err(|error| format!("回传 Codex 执行状态失败: {error}"))?;
        Ok(())
    }

    async fn interrupt_turn(
        &mut self,
        turn_id: &str,
        ws_sender: &mut SplitSink<WebSocket, Message>,
        session_id: &str,
    ) -> Result<(), String> {
        self.request(
            "turn/interrupt",
            json!({"threadId": self.thread_id, "turnId": turn_id}),
            ws_sender,
            session_id,
        )
        .await?;
        send_error(
            ws_sender,
            Some(session_id),
            "execution_confirmation_required",
            "已拦截未经过语音或底部备用确认的提前执行",
        )
        .await
        .map_err(|error| format!("回传执行拦截状态失败: {error}"))
    }

    fn take_unconfirmed_turn(&mut self) -> Option<String> {
        self.unconfirmed_turn_id.take()
    }

    fn take_pending_steering(&mut self) -> Option<String> {
        self.pending_steering.take()
    }

    fn take_auto_execution_request(&mut self) -> bool {
        std::mem::take(&mut self.auto_execution_requested)
    }

    async fn cancel_current_interaction(
        &mut self,
        ws_sender: &mut SplitSink<WebSocket, Message>,
        session_id: &str,
    ) -> Result<(), String> {
        self.interrupt_active_execution_or_confirm_idle().await?;
        self.execution_active = false;
        self.execution_turn_id = None;
        self.execution_confirmation_pending = false;
        self.explicit_execution_request_pending = false;
        self.auto_execution_requested = false;
        self.pending_steering = None;
        self.completion = None;
        self.transcript.clear();
        self.request(
            "thread/settings/update",
            json!({
                "threadId": self.thread_id,
                "approvalPolicy": "never",
                "sandboxPolicy": {"type": "readOnly", "networkAccess": false}
            }),
            ws_sender,
            session_id,
        )
        .await?;
        self.request(
            "thread/realtime/appendText",
            json!({
                "threadId": self.thread_id,
                "role": "developer",
                "text": "[ITERATE_EXECUTION_IDLE] The user cancelled the current interaction. Stop the current reply, discard the cancelled task context, remain connected, and wait silently for the next live request."
            }),
            ws_sender,
            session_id,
        )
        .await?;
        send_json(
            ws_sender,
            json!({
                "type":"interaction_interrupted",
                "session_id":session_id,
                "text":"已取消当前对话，GPT-Live 继续聆听"
            }),
        )
        .await
        .map_err(|error| format!("回传当前对话取消状态失败: {error}"))
    }

    async fn steer_execution(
        &mut self,
        text: &str,
        ws_sender: &mut SplitSink<WebSocket, Message>,
        session_id: &str,
    ) -> Result<(), String> {
        let turn_id = self
            .execution_turn_id
            .clone()
            .ok_or_else(|| "Codex 当前没有可调整的执行任务".to_string())?;
        self.request(
            "turn/steer",
            json!({
                "threadId": self.thread_id,
                "expectedTurnId": turn_id,
                "input": [{
                    "type": "text",
                    "text": format!("The user is steering the active task by voice: {text}"),
                    "text_elements": []
                }]
            }),
            ws_sender,
            session_id,
        )
        .await?;
        send_json(
            ws_sender,
            json!({
                "type":"task_progress",
                "session_id":session_id,
                "kind":"activity",
                "text":"Codex 已收到语音调整"
            }),
        )
        .await
        .map_err(|error| format!("回传语音调整状态失败: {error}"))
    }

    fn take_completion(&mut self) -> Option<ExecutionCompletion> {
        self.completion.take()
    }

    async fn lock_execution_gate(
        &mut self,
        completion: ExecutionCompletion,
        ws_sender: &mut SplitSink<WebSocket, Message>,
        session_id: &str,
    ) -> Result<(), String> {
        if completion.requires_interrupt {
            self.interrupt_active_execution_or_confirm_idle().await?;
        }
        self.execution_active = false;
        self.execution_turn_id = None;
        self.execution_confirmation_pending = false;
        self.explicit_execution_request_pending = false;
        self.auto_execution_requested = false;
        self.transcript.clear();
        self.request(
            "thread/settings/update",
            json!({
                "threadId": self.thread_id,
                "approvalPolicy": "never",
                "sandboxPolicy": {"type": "readOnly", "networkAccess": false}
            }),
            ws_sender,
            session_id,
        )
        .await?;
        self.request(
            "thread/realtime/appendText",
            json!({
                "threadId": self.thread_id,
                "role": "developer",
                "text": "[ITERATE_EXECUTION_IDLE] The Codex turn is complete. Return to live discussion mode; the next direct actionable voice request starts automatically without a confirmation loop."
            }),
            ws_sender,
            session_id,
        )
        .await?;
        let _ = self
            .request(
                "thread/realtime/appendSpeech",
                json!({"threadId": self.thread_id, "text": completion.speak_text}),
                ws_sender,
                session_id,
            )
            .await;
        Ok(())
    }

    async fn forward_notification(
        &mut self,
        ws_sender: &mut SplitSink<WebSocket, Message>,
        session_id: &str,
        message: &Value,
    ) -> Result<(), axum::Error> {
        let method = message.get("method").and_then(Value::as_str);
        let params = message.get("params").cloned().unwrap_or(Value::Null);
        let notification_thread_id = params.get("threadId").and_then(Value::as_str);
        let belongs_to_thread = notification_thread_id
            .map(|thread_id| thread_id == self.thread_id)
            .unwrap_or(true);

        if method == Some("thread/realtime/transcript/done") && belongs_to_thread {
            if let (Some(role), Some(text)) = (
                params.get("role").and_then(Value::as_str),
                params.get("text").and_then(Value::as_str),
            ) {
                let added = push_transcript_segment(&mut self.transcript, role, text);
                if added {
                    if role == "assistant" {
                        self.execution_confirmation_pending = false;
                    } else if role == "user" {
                        if is_negative_execution_decision(text) {
                            self.execution_confirmation_pending = false;
                            self.explicit_execution_request_pending = false;
                            self.auto_execution_requested = false;
                        } else {
                            self.explicit_execution_request_pending =
                                next_explicit_execution_request_pending(
                                    self.explicit_execution_request_pending,
                                    text,
                                );
                            if !self.execution_active && is_explicit_execution_request(text) {
                                self.auto_execution_requested = true;
                            }
                        }
                    }
                    let continuity_project_path = self.project_path.clone();
                    let continuity_thread_id = self.thread_id.clone();
                    let persisted_role = role.to_string();
                    let persisted_text = text.to_string();
                    match tokio::task::spawn_blocking(move || {
                        let continuity_error = continuity::append_transcript(
                            &continuity_project_path,
                            &continuity_thread_id,
                            &persisted_role,
                            &persisted_text,
                        )
                        .err();
                        let markdown_error =
                            crate::speech_memory::append_gpt_live_transcript_markdown(
                                &persisted_role,
                                &persisted_text,
                            )
                            .err()
                            .map(|error| error.to_string());
                        (continuity_error, markdown_error)
                    })
                    .await
                    {
                        Ok((continuity_error, markdown_error)) => {
                            if let Some(error) = continuity_error {
                                eprintln!("[codex-live] failed to persist continuity: {error}");
                            }
                            if let Some(error) = markdown_error {
                                eprintln!(
                                    "[codex-live] failed to append realtime transcript: {error}"
                                );
                            }
                        }
                        Err(error) => {
                            eprintln!("[codex-live] transcript persistence task failed: {error}");
                        }
                    }
                    if role == "user" && self.execution_active {
                        self.pending_steering = Some(text.trim().to_string());
                    }
                }
            }
        }

        let outbound = match method {
            Some("thread/realtime/started") => Some(json!({
                "type":"status",
                "session_id":session_id,
                "status":"ready",
                "thread_id":self.thread_id,
                "project_path":self.project_path,
                "continuity_status":self.continuity_status
            })),
            Some("thread/realtime/transcript/delta") => Some(json!({
                "type":"transcript_delta",
                "session_id":session_id,
                "role":params.get("role"),
                "text":params.get("delta")
            })),
            Some("thread/realtime/transcript/done") => Some(json!({
                "type":"transcript_done",
                "session_id":session_id,
                "role":params.get("role"),
                "text":params.get("text")
            })),
            Some("turn/started") if belongs_to_thread => {
                let turn_id = params.pointer("/turn/id").and_then(Value::as_str);
                if let Some(turn_id) = turn_id {
                    if self.execution_active && self.execution_turn_id.is_none() {
                        self.execution_turn_id = Some(turn_id.to_string());
                    } else if !self.execution_active {
                        self.unconfirmed_turn_id = Some(turn_id.to_string());
                    }
                }
                None
            }
            Some("item/agentMessage/delta") if belongs_to_thread && self.execution_active => {
                Some(json!({
                    "type":"task_progress",
                    "session_id":session_id,
                    "kind":"agent",
                    "text":params.get("delta")
                }))
            }
            Some("turn/plan/updated") if belongs_to_thread && self.execution_active => {
                notification_plan_text(&params).map(|text| {
                    json!({
                        "type":"task_progress",
                        "session_id":session_id,
                        "kind":"plan",
                        "text":text
                    })
                })
            }
            Some("item/started") if belongs_to_thread && self.execution_active => {
                notification_item_text(&params).map(|text| {
                    json!({
                        "type":"task_progress",
                        "session_id":session_id,
                        "kind":"activity",
                        "text":text
                    })
                })
            }
            Some("turn/completed") if belongs_to_thread && self.execution_active => {
                let turn_id = params.pointer("/turn/id").and_then(Value::as_str);
                if turn_id == self.execution_turn_id.as_deref() {
                    let status = params
                        .pointer("/turn/status")
                        .and_then(Value::as_str)
                        .unwrap_or("failed");
                    let final_text = completed_turn_final_text(&params);
                    let error_message = params
                        .pointer("/turn/error/message")
                        .and_then(Value::as_str)
                        .unwrap_or("Codex 执行失败");
                    let succeeded = status == "completed";
                    let event_type = execution_completion_event_type(succeeded, error_message);
                    let event_status = if event_type == "task_interrupted" {
                        "interrupted"
                    } else {
                        status
                    };
                    let display_text = if succeeded {
                        final_text
                            .clone()
                            .unwrap_or_else(|| "Codex 已完成任务".to_string())
                    } else {
                        execution_failure_display_text(error_message)
                    };
                    self.completion = Some(ExecutionCompletion {
                        speak_text: if succeeded {
                            spoken_execution_result(&display_text)
                        } else if is_transient_codex_stream_error(error_message) {
                            "Codex 网络响应临时中断，任务可能已经完成一部分。请让我先检查进度，再继续。".to_string()
                        } else {
                            "任务执行遇到问题，详情已显示在 iPhone 上。".to_string()
                        },
                        requires_interrupt: false,
                    });
                    Some(json!({
                        "type":event_type,
                        "session_id":session_id,
                        "thread_id":self.thread_id,
                        "turn_id":turn_id,
                        "status":event_status,
                        "text":display_text
                    }))
                } else {
                    None
                }
            }
            Some("thread/realtime/error") => {
                let error_message = params
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("Codex GPT-Live 发生错误");
                if is_transient_codex_stream_error(error_message) {
                    let display_text = execution_failure_display_text(error_message);
                    if self.execution_active {
                        self.completion = Some(ExecutionCompletion {
                            speak_text: "Codex 网络响应临时中断，任务可能已经完成一部分。请让我先检查进度，再继续。".to_string(),
                            requires_interrupt: true,
                        });
                    }
                    Some(json!({
                        "type":"task_interrupted",
                        "session_id":session_id,
                        "thread_id":self.thread_id,
                        "turn_id":self.execution_turn_id,
                        "status":"interrupted",
                        "text":display_text
                    }))
                } else {
                    Some(json!({
                        "type":"error",
                        "session_id":session_id,
                        "code":"codex_live_error",
                        "message":error_message
                    }))
                }
            }
            Some("thread/realtime/closed") => {
                Some(json!({"type":"closed","session_id":session_id}))
            }
            _ => None,
        };
        if let Some(outbound) = outbound {
            send_json(ws_sender, outbound).await?;
        }
        Ok(())
    }

    async fn terminate_child(&mut self) {
        let _ = self.child.kill().await;
        let _ = timeout(Duration::from_secs(2), self.child.wait()).await;
    }

    async fn interrupt_or_confirm_terminal(
        &mut self,
        thread_id: &str,
        turn_id: &str,
    ) -> Result<(), String> {
        let mut last_error = "Codex turn 中止未确认".to_string();
        for _ in 0..3 {
            if let Err(error) = self
                .request_silent(
                    "turn/interrupt",
                    json!({"threadId": thread_id, "turnId": turn_id}),
                )
                .await
            {
                last_error = error;
            }
            match self
                .request_silent(
                    "thread/read",
                    json!({"threadId": thread_id, "includeTurns": true}),
                )
                .await
            {
                Ok(thread) if turn_is_terminal(&thread, turn_id) => return Ok(()),
                Ok(_) => {}
                Err(error) => last_error = error,
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        Err(format!("无法确认 Codex turn 已停止: {last_error}"))
    }

    async fn read_active_turn_id(&mut self) -> Result<Option<String>, String> {
        let thread = self
            .request_silent(
                "thread/read",
                json!({"threadId": self.thread_id, "includeTurns": true}),
            )
            .await?;
        Ok(active_turn_id(&thread))
    }

    async fn interrupt_active_execution_or_confirm_idle(&mut self) -> Result<(), String> {
        let turn_id = match self.execution_turn_id.clone() {
            Some(turn_id) => Some(turn_id),
            None => self.read_active_turn_id().await?,
        };
        if let Some(turn_id) = turn_id {
            self.execution_turn_id = Some(turn_id.clone());
            self.interrupt_or_confirm_terminal(&self.thread_id.clone(), &turn_id)
                .await?;
        }
        Ok(())
    }

    async fn recover_daemon_cleanup(
        project_path: &str,
        thread_id: &str,
        turn_id: Option<&str>,
    ) -> Result<(), String> {
        if !Self::try_start_daemon().await {
            return Err("无法重连 Codex app-server daemon 完成安全清理".to_string());
        }
        let mut recovery = Self::spawn(project_path, true)?;
        let result = async {
            recovery.initialize_silent().await?;
            recovery.thread_id = thread_id.to_string();
            recovery
                .request_silent(
                    "thread/resume",
                    json!({
                        "threadId": thread_id,
                        "cwd": project_path,
                        "runtimeWorkspaceRoots": [project_path],
                        "approvalPolicy": "never",
                        "sandbox": "read-only",
                        "excludeTurns": true
                    }),
                )
                .await?;
            let active_turn_id = match turn_id {
                Some(turn_id) => Some(turn_id.to_string()),
                None => recovery.read_active_turn_id().await?,
            };
            if let Some(turn_id) = active_turn_id.as_deref() {
                recovery
                    .interrupt_or_confirm_terminal(thread_id, turn_id)
                    .await?;
            }
            recovery
                .request_silent(
                    "thread/settings/update",
                    json!({
                        "threadId": thread_id,
                        "approvalPolicy": "never",
                        "sandboxPolicy": {"type": "readOnly", "networkAccess": false}
                    }),
                )
                .await?;
            let _ = recovery
                .request_silent("thread/realtime/stop", json!({"threadId": thread_id}))
                .await;
            Ok(())
        }
        .await;
        recovery.terminate_child().await;
        result
    }

    async fn stop(&mut self) {
        let mut cleanup_failed = false;
        if (self.execution_active || self.execution_turn_id.is_some())
            && self
                .interrupt_active_execution_or_confirm_idle()
                .await
                .is_err()
        {
            cleanup_failed = true;
        }
        let turn_id = self.execution_turn_id.take();
        self.execution_active = false;
        self.execution_confirmation_pending = false;
        self.explicit_execution_request_pending = false;
        self.auto_execution_requested = false;
        self.pending_steering = None;
        self.completion = None;
        if !self.thread_id.is_empty() {
            if self
                .request_silent(
                    "thread/settings/update",
                    json!({
                        "threadId": self.thread_id,
                        "approvalPolicy": "never",
                        "sandboxPolicy": {"type": "readOnly", "networkAccess": false}
                    }),
                )
                .await
                .is_err()
            {
                cleanup_failed = true;
            }
            let _ = self
                .request_silent("thread/realtime/stop", json!({"threadId": self.thread_id}))
                .await;
        }
        if cleanup_failed && self.using_daemon_proxy && !self.thread_id.is_empty() {
            let _ = Self::recover_daemon_cleanup(
                &self.project_path,
                &self.thread_id,
                turn_id.as_deref(),
            )
            .await;
        }
        self.terminate_child().await;
    }
}

fn validated_project_path(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.len() > MAX_PROJECT_PATH_BYTES {
        return Err("目标项目路径为空或过长".to_string());
    }
    let path = Path::new(trimmed);
    if !path.is_absolute() {
        return Err("目标项目必须是 Mac 上的绝对路径".to_string());
    }
    let canonical = std::fs::canonicalize(path)
        .map_err(|error| format!("目标项目不存在或无法访问: {error}"))?;
    if canonical == Path::new("/") || !canonical.is_dir() {
        return Err("目标项目必须是具体的可访问目录".to_string());
    }
    canonical
        .to_str()
        .map(str::to_string)
        .ok_or_else(|| "目标项目路径不是有效 UTF-8".to_string())
}

fn new_thread_params(project_path: &str) -> Value {
    json!({
        "ephemeral": false,
        "cwd": project_path,
        "sandbox": "read-only",
        "approvalPolicy": "never",
        "config": realtime_thread_config(),
        "developerInstructions": CODEX_VOICE_DEVELOPER_INSTRUCTIONS,
        "threadSource": "realtime_voice",
        "dynamicTools": [],
        "environments": [],
        "runtimeWorkspaceRoots": [project_path]
    })
}

fn resume_thread_params(thread_id: &str, project_path: &str) -> Value {
    json!({
        "threadId": thread_id,
        "cwd": project_path,
        "runtimeWorkspaceRoots": [project_path],
        "approvalPolicy": "never",
        "sandbox": "read-only",
        "config": realtime_thread_config(),
        "developerInstructions": CODEX_VOICE_DEVELOPER_INSTRUCTIONS,
        "excludeTurns": false
    })
}

fn realtime_thread_config() -> Value {
    json!({
        "mcp_servers.iterate-zhi.enabled": false,
        "agents.enabled": true,
        "agents.max_concurrent_threads_per_session": 3
    })
}

fn realtime_start_params(thread_id: &str, offer_sdp: &str, initial_items: Vec<Value>) -> Value {
    let mut params = json!({
        "threadId": thread_id,
        "outputModality": "audio",
        "transport": {"type":"webrtc", "sdp": offer_sdp},
        "version": "v3",
        "voice": "cove",
        "model": "gpt-live-1-boulder-alpha",
        "includeStartupContext": false,
        "flushTranscriptTailOnSessionEnd": true,
        "codexResponsesAsItems": false,
        "clientManagedHandoffs": false,
        "prompt": REALTIME_VOICE_PROMPT
    });
    if !initial_items.is_empty() {
        params["initialItems"] = Value::Array(initial_items);
    }
    params
}

fn thread_is_definitively_missing(error: &str) -> bool {
    let normalized = error.to_ascii_lowercase();
    [
        "thread_not_found",
        "thread not found",
        "unknown thread",
        "no thread found",
        "no rollout found for thread id",
    ]
    .iter()
    .any(|fragment| normalized.contains(fragment))
}

fn thread_has_active_writer_conflict(error: &str) -> bool {
    let normalized = error.to_ascii_lowercase();
    normalized.contains("already has an active writer")
        && (normalized.contains("thread-store conflict") || normalized.contains("thread/resume"))
}

fn thread_resume_requires_replacement(error: &str) -> bool {
    thread_is_definitively_missing(error) || thread_has_active_writer_conflict(error)
}

fn active_turn_id(thread_read: &Value) -> Option<String> {
    thread_read
        .pointer("/thread/turns")?
        .as_array()?
        .iter()
        .rev()
        .find(|turn| {
            !matches!(
                turn.get("status").and_then(Value::as_str),
                Some("completed" | "interrupted" | "failed")
            )
        })
        .and_then(|turn| turn.get("id"))
        .and_then(Value::as_str)
        .filter(|turn_id| !turn_id.is_empty())
        .map(str::to_string)
}

fn turn_is_terminal(thread_read: &Value, turn_id: &str) -> bool {
    thread_read
        .pointer("/thread/turns")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|turn| {
            turn.get("id").and_then(Value::as_str) == Some(turn_id)
                && matches!(
                    turn.get("status").and_then(Value::as_str),
                    Some("completed" | "interrupted" | "failed")
                )
        })
}

fn push_transcript_segment(segments: &mut Vec<TranscriptSegment>, role: &str, text: &str) -> bool {
    let role = match role {
        "user" => "user",
        "assistant" => "assistant",
        _ => return false,
    };
    let text = text.trim();
    if text.is_empty() {
        return false;
    }
    if segments
        .last()
        .is_some_and(|segment| segment.role == role && segment.text == text)
    {
        return false;
    }
    segments.push(TranscriptSegment {
        role: role.to_string(),
        text: text.to_string(),
    });
    while segments.len() > MAX_TRANSCRIPT_SEGMENTS
        || segments
            .iter()
            .map(|segment| segment.text.len())
            .sum::<usize>()
            > MAX_TRANSCRIPT_BYTES
    {
        segments.remove(0);
    }
    true
}

fn confirmed_delegation_input(
    transcript: &[TranscriptSegment],
    project_path: &str,
) -> Result<String, String> {
    if !transcript
        .iter()
        .any(|segment| segment.role == "user" && !is_execution_control_utterance(&segment.text))
    {
        return Err("请先通过 GPT-Live 说清需要执行的任务".to_string());
    }
    let transcript = transcript
        .iter()
        .filter(|segment| segment.role != "user" || !is_execution_control_utterance(&segment.text))
        .map(|segment| format!("{}: {}", segment.role.to_ascii_uppercase(), segment.text))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(format!(
        "<realtime_delegation>\n  <iterate_voice_actionable>true</iterate_voice_actionable>\n  <input>\nThe user gave a direct actionable voice request. Work in the target project {}. Infer and execute the latest actionable request from this voice discussion without asking for a second confirmation. Prefer the most recent clarified decision over earlier alternatives. Complete and verify the work; do not merely explain how it could be done.\n\n{}\n  </input>\n</realtime_delegation>",
        xml_escape(project_path),
        xml_escape(&transcript)
    ))
}

fn normalized_voice_decision(text: &str) -> String {
    text.chars()
        .flat_map(char::to_lowercase)
        .filter(|character| character.is_alphanumeric())
        .collect()
}

fn is_explicit_execution_request(text: &str) -> bool {
    let normalized = normalized_voice_decision(text);
    if normalized.is_empty()
        || is_negative_execution_decision(&normalized)
        || is_execution_control_utterance(&normalized)
        || normalized.starts_with("不要")
        || normalized.starts_with("不用")
        || normalized.starts_with('别')
    {
        return false;
    }

    let capability_questions = [
        "能不能",
        "可不可以",
        "可以吗",
        "能否",
        "是否可以",
        "会不会",
        "有没有权限",
        "有权限吗",
        "canyou",
        "areyouable",
        "doyouhavepermission",
    ];
    if capability_questions
        .iter()
        .any(|marker| normalized.contains(marker))
    {
        return false;
    }

    if is_hui_voice_request(text) || is_xi_voice_request(text) {
        return true;
    }

    let explicit_markers = [
        "调研",
        "研究",
        "整理",
        "分析",
        "汇总",
        "帮我查",
        "帮我搜",
        "查一下",
        "搜一下",
        "搜索",
        "查找",
        "联网查",
        "帮我读取",
        "读取文件",
        "帮我写",
        "写入文件",
        "创建文件",
        "新建文件",
        "修改文件",
        "改一下",
        "帮我改",
        "帮我修",
        "修复一下",
        "帮我安装",
        "安装一下",
        "帮我运行",
        "运行一下",
        "帮我执行",
        "执行一下",
        "帮我验证",
        "验证一下",
        "帮我测试",
        "测试一下",
        "帮我打开",
        "帮我截图",
        "调用hui",
        "使用hui",
        "按照hui",
        "调用xi",
        "使用xi",
        "按照xi",
        "searchfor",
        "lookfor",
        "writefile",
        "modifyfile",
        "pleasefix",
        "fixthe",
        "pleaseinstall",
        "installthe",
        "pleaserun",
        "runthe",
        "pleasetest",
        "testthe",
        "pleaseverify",
        "verifythe",
        "research",
        "analyze",
        "organize",
        "summarize",
    ];
    if explicit_markers
        .iter()
        .any(|marker| normalized.contains(marker))
    {
        return true;
    }

    if [
        "为什么",
        "怎么回事",
        "什么问题",
        "是什么",
        "什么意思",
        "what",
        "why",
        "how",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
    {
        return false;
    }

    [
        "继续",
        "接着",
        "看看",
        "看一下",
        "检查",
        "处理",
        "解决",
        "修复",
        "修改",
        "改成",
        "改为",
        "删除",
        "移除",
        "新增",
        "加上",
        "创建",
        "生成",
        "导出",
        "安装",
        "重启",
        "构建",
        "提交",
        "推送",
        "运行",
        "执行",
        "测试",
        "验证",
        "打开",
        "关闭",
        "截图",
    ]
    .iter()
    .any(|marker| normalized.contains(marker))
}

fn is_delegation_clarification(text: &str) -> bool {
    let normalized = normalized_voice_decision(text);
    if normalized.is_empty() || is_negative_execution_decision(&normalized) {
        return false;
    }
    let capability_questions = [
        "能不能",
        "可不可以",
        "可以吗",
        "能否",
        "是否可以",
        "会不会",
        "有没有权限",
        "有权限吗",
        "canyou",
        "areyouable",
        "doyouhavepermission",
    ];
    if capability_questions
        .iter()
        .any(|marker| normalized.contains(marker))
    {
        return false;
    }
    ["子代理", "subagent", "worker"]
        .iter()
        .any(|marker| normalized.contains(marker))
}

fn next_explicit_execution_request_pending(current: bool, text: &str) -> bool {
    if is_negative_execution_decision(text) {
        return false;
    }
    if is_execution_control_utterance(text) {
        return current;
    }
    if is_explicit_execution_request(text) {
        return true;
    }
    current && is_delegation_clarification(text)
}

fn is_execution_confirmation_question(text: &str) -> bool {
    normalized_voice_decision(text) == "需求已经确认是否现在开始执行"
}

fn is_negative_execution_decision(text: &str) -> bool {
    matches!(
        normalized_voice_decision(text).as_str(),
        "不" | "不要"
            | "不用"
            | "不可以"
            | "取消"
            | "等等"
            | "等一下"
            | "先等等"
            | "不要执行"
            | "先不要执行"
            | "先不执行"
            | "暂不执行"
            | "再商量一下"
            | "我再想想"
    )
}

fn is_execution_control_utterance(text: &str) -> bool {
    if is_negative_execution_decision(text) {
        return true;
    }
    matches!(
        normalized_voice_decision(text).as_str(),
        "可以"
            | "可以的"
            | "好"
            | "好的"
            | "行"
            | "确认"
            | "确认执行"
            | "我确认执行"
            | "开始"
            | "开始吧"
            | "开始执行"
            | "开始执行吧"
            | "你开始吧"
            | "那就开始吧"
            | "现在执行"
            | "直接执行"
            | "直接做吧"
            | "你直接做吧"
            | "执行吧"
            | "动手吧"
            | "就这么做"
            | "就这么做吧"
            | "就这样做"
            | "就这样做吧"
            | "那就这么做"
            | "那就这样做"
            | "去做吧"
    )
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn configure_codex_command(command: &mut Command) {
    command
        .env_remove("OPENAI_API_KEY")
        .env_remove("CODEX_API_KEY");

    let Some(resources_dir) = Path::new(CODEX_APP_SERVER).parent() else {
        return;
    };
    let inherited_path = std::env::var_os("PATH")
        .unwrap_or_else(|| std::ffi::OsString::from("/usr/bin:/bin:/usr/sbin:/sbin"));
    let mut search_paths = vec![resources_dir.to_path_buf()];
    search_paths.extend(std::env::split_paths(&inherited_path));
    if let Ok(path) = std::env::join_paths(search_paths) {
        command.env("PATH", path);
    }
}

fn normalized_voice_intent(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|character| character.is_alphanumeric())
        .collect()
}

fn is_hui_voice_request(text: &str) -> bool {
    let normalized = normalized_voice_intent(text);
    if ["不要回溯", "不用回溯", "别回溯"]
        .iter()
        .any(|negative| normalized.contains(negative))
    {
        return false;
    }
    if matches!(normalized.as_str(), "hui" | "hui1" | "hui0" | "回") {
        return true;
    }
    [
        "回溯",
        "调用hui",
        "使用hui",
        "调用回这个skill",
        "调用回就行了",
        "刚讲了什么",
        "刚刚讲了什么",
        "刚才说了什么",
        "上次讨论了什么",
        "之前做了什么",
        "项目进度",
        "昨天进行到哪",
        "昨天做到哪",
        "进行到哪",
        "做到哪",
    ]
    .iter()
    .any(|trigger| normalized.contains(trigger))
}

fn is_xi_voice_request(text: &str) -> bool {
    let normalized = normalized_voice_intent(text);
    if ["不要经验库", "不用经验库", "别恢复上下文"]
        .iter()
        .any(|negative| normalized.contains(negative))
    {
        return false;
    }
    if matches!(normalized.as_str(), "xi" | "习") {
        return true;
    }
    [
        "调用xi",
        "使用xi",
        "经验库",
        "全局知识库",
        "上下文记忆",
        "知道我们在干嘛",
        "恢复上下文",
    ]
    .iter()
    .any(|trigger| normalized.contains(trigger))
}

fn requested_voice_skill_name(transcript: &[TranscriptSegment]) -> Option<&'static str> {
    let latest_request = transcript
        .iter()
        .rev()
        .filter(|segment| segment.role == "user")
        .find(|segment| !is_execution_control_utterance(&segment.text))?;
    if is_xi_voice_request(&latest_request.text) {
        Some(XI_SKILL_NAME)
    } else {
        is_hui_voice_request(&latest_request.text).then_some(HUI_SKILL_NAME)
    }
}

fn requested_voice_skill_input(transcript: &[TranscriptSegment]) -> Option<(&'static str, Value)> {
    let skill_name = requested_voice_skill_name(transcript)?;
    let relative_path = match skill_name {
        HUI_SKILL_NAME => HUI_SKILL_RELATIVE_PATH,
        XI_SKILL_NAME => XI_SKILL_RELATIVE_PATH,
        _ => return None,
    };
    let skill_path = dirs::home_dir()?.join(relative_path);
    if !skill_path.is_file() {
        return None;
    }
    Some((
        skill_name,
        json!({
            "type": "skill",
            "name": skill_name,
            "path": skill_path.to_string_lossy()
        }),
    ))
}

fn notification_plan_text(params: &Value) -> Option<String> {
    let steps = params.get("plan")?.as_array()?;
    let text = steps
        .iter()
        .filter_map(|step| {
            let label = step.get("step")?.as_str()?.trim();
            let status = step
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("pending");
            (!label.is_empty()).then(|| format!("{status}: {label}"))
        })
        .take(4)
        .collect::<Vec<_>>()
        .join(" · ");
    (!text.is_empty()).then(|| truncate_chars(&text, 500))
}

fn notification_item_text(params: &Value) -> Option<String> {
    let item = params.get("item")?;
    match item.get("type")?.as_str()? {
        "commandExecution" => item
            .get("command")
            .and_then(Value::as_str)
            .map(|command| format!("正在运行：{}", truncate_chars(command, 220))),
        "fileChange" => Some("正在修改项目文件".to_string()),
        "collabAgentToolCall" | "subAgentActivity" => Some("正在调度 Codex worker".to_string()),
        "mcpToolCall" => {
            let tool = item.get("tool").and_then(Value::as_str).unwrap_or("工具");
            Some(format!("正在调用：{}", truncate_chars(tool, 120)))
        }
        _ => None,
    }
}

fn completed_turn_final_text(params: &Value) -> Option<String> {
    params
        .pointer("/turn/items")?
        .as_array()?
        .iter()
        .rev()
        .find(|item| item.get("type").and_then(Value::as_str) == Some("agentMessage"))
        .and_then(|item| item.get("text"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(|text| truncate_chars(text, 2_000))
}

fn is_transient_codex_stream_error(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    [
        "stream disconnected before completion",
        "unexpected eof",
        "unexpected-eof",
        "close_notify",
        "connection reset by peer",
    ]
    .iter()
    .any(|fragment| normalized.contains(fragment))
}

fn execution_completion_event_type(succeeded: bool, error_message: &str) -> &'static str {
    if succeeded {
        "task_completed"
    } else if is_transient_codex_stream_error(error_message) {
        "task_interrupted"
    } else {
        "task_failed"
    }
}

fn execution_failure_display_text(message: &str) -> String {
    if is_transient_codex_stream_error(message) {
        return "Codex 网络响应流临时中断；任务可能已完成一部分。GPT-Live 仍可继续，请说“检查刚才的进度，再继续完成”，不要直接重复整个任务。".to_string();
    }
    truncate_chars(message, 2_000)
}

fn spoken_execution_result(display_text: &str) -> String {
    let compact = display_text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.trim_start_matches(['#', '*', '-', '>', '`', ' ']))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if compact.is_empty() {
        "任务已经完成。".to_string()
    } else {
        format!("任务完成。{}", truncate_chars(&compact, 600))
    }
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    let mut chars = value.chars();
    let truncated = chars.by_ref().take(max_chars).collect::<String>();
    if chars.next().is_some() {
        format!("{truncated}…")
    } else {
        truncated
    }
}

async fn send_error(
    ws_sender: &mut SplitSink<WebSocket, Message>,
    session_id: Option<&str>,
    code: &str,
    message: &str,
) -> Result<(), axum::Error> {
    send_json(
        ws_sender,
        json!({"type":"error","session_id":session_id,"code":code,"message":message}),
    )
    .await
}

async fn send_json(
    ws_sender: &mut SplitSink<WebSocket, Message>,
    value: Value,
) -> Result<(), axum::Error> {
    ws_sender.send(Message::Text(value.to_string())).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_path_requires_existing_absolute_directory() {
        assert!(validated_project_path("relative/path").is_err());
        assert!(validated_project_path("/").is_err());
        assert_eq!(
            validated_project_path(std::env::temp_dir().to_str().unwrap()).unwrap(),
            std::fs::canonicalize(std::env::temp_dir())
                .unwrap()
                .to_str()
                .unwrap()
        );
    }

    #[test]
    fn actionable_voice_request_wraps_the_consensus_and_escapes_xml() {
        let transcript = vec![
            TranscriptSegment {
                role: "user".to_string(),
                text: "修改 <按钮> & 测试".to_string(),
            },
            TranscriptSegment {
                role: "assistant".to_string(),
                text: "只改当前页面".to_string(),
            },
        ];
        let input = confirmed_delegation_input(&transcript, "/tmp/demo").unwrap();
        assert!(input.contains("<iterate_voice_actionable>true"));
        assert!(input.contains("USER: 修改 &lt;按钮&gt; &amp; 测试"));
        assert!(input.contains("ASSISTANT: 只改当前页面"));
    }

    #[test]
    fn transcript_is_deduplicated_and_requires_a_user_request() {
        let mut transcript = Vec::new();
        push_transcript_segment(&mut transcript, "assistant", "你好");
        push_transcript_segment(&mut transcript, "assistant", "你好");
        assert_eq!(transcript.len(), 1);
        assert!(confirmed_delegation_input(&transcript, "/tmp/demo").is_err());
        push_transcript_segment(&mut transcript, "user", "开始做");
        assert!(confirmed_delegation_input(&transcript, "/tmp/demo").is_ok());
    }

    #[test]
    fn confirmation_words_alone_cannot_become_a_delegated_task() {
        for confirmation in ["可以", "开始执行", "直接执行", "先不要执行"] {
            let transcript = vec![TranscriptSegment {
                role: "user".to_string(),
                text: confirmation.to_string(),
            }];
            assert!(confirmed_delegation_input(&transcript, "/tmp/demo").is_err());
        }

        let transcript = vec![
            TranscriptSegment {
                role: "user".to_string(),
                text: "把首页按钮改成蓝色并运行测试".to_string(),
            },
            TranscriptSegment {
                role: "assistant".to_string(),
                text: "需求已经确认，是否现在开始执行？".to_string(),
            },
            TranscriptSegment {
                role: "user".to_string(),
                text: "可以".to_string(),
            },
        ];
        let input = confirmed_delegation_input(&transcript, "/tmp/demo").unwrap();
        assert!(input.contains("把首页按钮改成蓝色并运行测试"));
        assert!(!input.contains("USER: 可以"));
        assert!(is_execution_confirmation_question(
            "需求已经确认，是否现在开始执行？"
        ));
        assert!(!is_execution_confirmation_question(
            "需求还不清楚，现在不能确认是否开始执行。"
        ));
    }

    #[test]
    fn explicit_tool_request_can_open_the_direct_confirmation_fallback() {
        for request in [
            "帮我搜索一下当前项目的最近提交",
            "按照 xi 查一下以前怎么修的",
            "你这样回溯这个技能，你回溯一下",
            "我们昨天进行到哪了",
            "读取文件并运行测试",
            "你继续刚刚的调研吧，做完了汇报给我",
            "把结果整理成表格",
            "please search for the failing test",
        ] {
            assert!(is_explicit_execution_request(request), "request={request}");
        }
        for discussion in [
            "你能不能读写文件",
            "有没有权限搜索",
            "不要修改任何文件",
            "确认执行",
            "这个执行为什么这么慢",
            "今天天气怎么样",
        ] {
            assert!(
                !is_explicit_execution_request(discussion),
                "discussion={discussion}"
            );
        }
    }

    #[test]
    fn delegation_clarification_keeps_a_natural_research_task_pending() {
        let mut pending = false;
        for utterance in [
            "你继续刚刚的调研吧，做完了汇报给我",
            "子代理",
            "我说用子代理",
        ] {
            pending = next_explicit_execution_request_pending(pending, utterance);
        }
        assert!(pending);
        assert!(!next_explicit_execution_request_pending(false, "子代理"));
        assert!(!next_explicit_execution_request_pending(
            true,
            "今天天气怎么样"
        ));
    }

    #[test]
    fn realtime_prompt_auto_starts_direct_tasks_without_claiming_tools() {
        assert!(REALTIME_VOICE_PROMPT.contains("iPhone or\nMac"));
        assert!(REALTIME_VOICE_PROMPT.contains("Never ask for execution confirmation"));
        assert!(REALTIME_VOICE_PROMPT.contains("automatically starts the Codex execution turn"));
        assert!(REALTIME_VOICE_PROMPT.contains("you have no search, file, terminal, web, screen"));
        assert!(REALTIME_VOICE_PROMPT.contains("我查一下"));
        assert!(REALTIME_VOICE_PROMPT.contains("Do not promise a later result"));
    }

    #[test]
    fn realtime_start_uses_bounded_initial_items_without_startup_context() {
        let params = realtime_start_params(
            "thread-1",
            "v=0\nm=audio 9 UDP/TLS/RTP/SAVPF 111",
            vec![json!({"role":"user", "text":"历史上下文"})],
        );
        assert_eq!(params["threadId"], "thread-1");
        assert_eq!(params["includeStartupContext"], false);
        assert_eq!(params["initialItems"][0]["role"], "user");
        assert!(params.get("realtimeSessionId").is_none());
    }

    #[test]
    fn realtime_threads_disable_iterate_zhi_and_enable_bounded_workers() {
        let started = new_thread_params("/tmp/demo");
        let resumed = resume_thread_params("thread-1", "/tmp/demo");

        for params in [&started, &resumed] {
            assert_eq!(
                params.pointer("/config/mcp_servers.iterate-zhi.enabled"),
                Some(&Value::Bool(false))
            );
            assert_eq!(
                params.pointer("/config/agents.enabled"),
                Some(&Value::Bool(true))
            );
            assert_eq!(
                params.pointer("/config/agents.max_concurrent_threads_per_session"),
                Some(&json!(3))
            );
            assert_eq!(
                params.get("developerInstructions").and_then(Value::as_str),
                Some(CODEX_VOICE_DEVELOPER_INSTRUCTIONS)
            );
        }
        assert!(CODEX_VOICE_DEVELOPER_INSTRUCTIONS
            .contains("explicitly authorized smart built-in-subagent"));
        assert!(CODEX_VOICE_DEVELOPER_INSTRUCTIONS
            .contains("Perform truly instantaneous work yourself"));
        assert!(CODEX_VOICE_DEVELOPER_INSTRUCTIONS
            .contains("Delegate\nbounded, time-consuming, or context-noisy work"));
        assert!(CODEX_VOICE_DEVELOPER_INSTRUCTIONS
            .contains("The primary agent owns scope, decisions, live user steering"));
        assert!(CODEX_VOICE_DEVELOPER_INSTRUCTIONS
            .contains("Children must not spawn\nmore children or invoke zhi"));
    }

    #[test]
    fn confirmed_hui_recall_is_selected_without_confusing_relearn() {
        let transcript = vec![
            TranscriptSegment {
                role: "user".to_string(),
                text: "你回溯一下，调用回这个 skill 看我们刚讲了什么".to_string(),
            },
            TranscriptSegment {
                role: "assistant".to_string(),
                text: "需求已经确认，是否现在开始执行？".to_string(),
            },
            TranscriptSegment {
                role: "user".to_string(),
                text: "我确认执行".to_string(),
            },
        ];
        assert_eq!(requested_voice_skill_name(&transcript), Some("hui"));

        let relearn = vec![TranscriptSegment {
            role: "user".to_string(),
            text: "relearn".to_string(),
        }];
        assert_eq!(requested_voice_skill_name(&relearn), None);

        let changed_request = vec![
            TranscriptSegment {
                role: "user".to_string(),
                text: "先用 hui 看上次进度".to_string(),
            },
            TranscriptSegment {
                role: "user".to_string(),
                text: "现在改成只检查首页链接".to_string(),
            },
            TranscriptSegment {
                role: "user".to_string(),
                text: "确认执行".to_string(),
            },
        ];
        assert_eq!(requested_voice_skill_name(&changed_request), None);
    }

    #[test]
    fn natural_hui_recall_then_direct_start_keeps_the_execution_gate_armed() {
        let request = "你这样回溯这个技能，你回溯一下";
        assert!(is_hui_voice_request(request));
        assert!(is_explicit_execution_request(request));
        assert!(next_explicit_execution_request_pending(false, request));
        assert!(next_explicit_execution_request_pending(true, "开始执行"));

        let transcript = vec![
            TranscriptSegment {
                role: "user".to_string(),
                text: request.to_string(),
            },
            TranscriptSegment {
                role: "assistant".to_string(),
                text: "我再确认一下。".to_string(),
            },
            TranscriptSegment {
                role: "user".to_string(),
                text: "开始执行".to_string(),
            },
        ];
        assert_eq!(
            requested_voice_skill_name(&transcript),
            Some(HUI_SKILL_NAME)
        );
        assert!(confirmed_delegation_input(&transcript, "/tmp/demo").is_ok());
    }

    #[test]
    fn confirmed_xi_recall_uses_the_unified_knowledge_skill() {
        let transcript = vec![
            TranscriptSegment {
                role: "user".to_string(),
                text: "用 xi 恢复上下文，再看全局知识库里的经验".to_string(),
            },
            TranscriptSegment {
                role: "assistant".to_string(),
                text: "需求已经确认，是否现在开始执行？".to_string(),
            },
            TranscriptSegment {
                role: "user".to_string(),
                text: "可以".to_string(),
            },
        ];
        assert_eq!(requested_voice_skill_name(&transcript), Some("xi"));

        let negative = vec![TranscriptSegment {
            role: "user".to_string(),
            text: "这次不用经验库，只看当前文件".to_string(),
        }];
        assert_eq!(requested_voice_skill_name(&negative), None);
    }

    #[test]
    fn completed_work_is_reported_directly_to_the_live_session() {
        let spoken = spoken_execution_result("## 完成\n- 修复语音确认\n- 验证通过");
        assert_eq!(spoken, "任务完成。完成 修复语音确认 验证通过");
        assert!(!spoken.contains("zhi"));
    }

    #[test]
    fn progress_helpers_keep_phone_updates_compact() {
        let command = json!({
            "item": {"type": "commandExecution", "command": "cargo test"}
        });
        assert_eq!(
            notification_item_text(&command).as_deref(),
            Some("正在运行：cargo test")
        );
        let completed = json!({
            "turn": {"items": [
                {"type":"agentMessage", "text":"第一条"},
                {"type":"agentMessage", "text":"最终结果"}
            ]}
        });
        assert_eq!(
            completed_turn_final_text(&completed).as_deref(),
            Some("最终结果")
        );
    }

    #[test]
    fn transient_stream_failures_are_explained_without_claiming_a_retry() {
        let tls_error = "stream disconnected before completion: peer closed connection without sending TLS close_notify: unexpected-eof";
        assert!(is_transient_codex_stream_error(tls_error));
        assert_eq!(
            execution_completion_event_type(false, tls_error),
            "task_interrupted"
        );
        let display = execution_failure_display_text(tls_error);
        assert!(display.contains("任务可能已完成一部分"));
        assert!(display.contains("检查刚才的进度"));
        assert!(!display.contains("自动重试"));

        let ordinary = "permission denied";
        assert!(!is_transient_codex_stream_error(ordinary));
        assert_eq!(
            execution_completion_event_type(false, ordinary),
            "task_failed"
        );
        assert_eq!(
            execution_completion_event_type(true, ordinary),
            "task_completed"
        );
        assert_eq!(execution_failure_display_text(ordinary), ordinary);
    }

    #[test]
    fn cleanup_only_accepts_a_terminal_matching_turn() {
        let thread = json!({
            "thread": {"turns": [
                {"id":"active", "status":"inProgress"},
                {"id":"done", "status":"interrupted"}
            ]}
        });
        assert!(!turn_is_terminal(&thread, "active"));
        assert!(turn_is_terminal(&thread, "done"));
        assert!(!turn_is_terminal(&thread, "missing"));
    }

    #[test]
    fn resume_replaces_a_missing_or_exclusively_owned_thread() {
        assert!(thread_resume_requires_replacement(
            "Codex thread/resume 失败: {\"code\":\"thread_not_found\"}"
        ));
        assert!(thread_resume_requires_replacement(
            "thread-store conflict: thread 019fd705-18f3-7971-9c52-e6b0a2326098 already has an active writer"
        ));
        assert!(thread_resume_requires_replacement(
            "Codex thread/resume 失败: {\"code\":-32600,\"message\":\"thread 019fd705-18f3-7971-9c52-e6b0a2326098 already has an active writer\"}"
        ));
        assert!(thread_is_definitively_missing(
            "Codex thread/resume 失败: {\"code\":\"thread_not_found\"}"
        ));
        assert!(thread_is_definitively_missing("Unknown thread 123"));
        assert!(thread_is_definitively_missing(
            "Codex thread/resume 失败: {\"code\":-32600,\"message\":\"no rollout found for thread id 019fd4ce-2d32-7451-b367-3f2c6f8ca349\"}"
        ));
        assert!(!thread_is_definitively_missing(
            "stream disconnected before completion"
        ));
        assert!(!thread_is_definitively_missing("permission denied"));
        assert!(!thread_resume_requires_replacement(
            "stream disconnected before completion"
        ));
        assert!(!thread_resume_requires_replacement("permission denied"));
        assert!(!thread_resume_requires_replacement(
            "another component already has an active writer"
        ));
        assert!(!thread_resume_requires_replacement(
            "Codex thread/start 失败: {\"code\":-32600,\"message\":\"thread 019fd705-18f3-7971-9c52-e6b0a2326098 already has an active writer\"}"
        ));
    }

    #[test]
    fn resumed_thread_detects_only_the_latest_non_terminal_turn() {
        let thread = json!({
            "thread": {"turns": [
                {"id":"done", "status":"completed"},
                {"id":"active", "status":"inProgress"}
            ]}
        });
        assert_eq!(active_turn_id(&thread).as_deref(), Some("active"));

        let terminal = json!({
            "thread": {"turns": [
                {"id":"done", "status":"completed"},
                {"id":"stopped", "status":"interrupted"}
            ]}
        });
        assert!(active_turn_id(&terminal).is_none());
    }
}
