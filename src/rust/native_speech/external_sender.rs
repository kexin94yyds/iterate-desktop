use super::session::OwnerEpoch;
use serde::{Deserialize, Serialize};
#[cfg(target_os = "macos")]
use std::io::{Read, Write};
#[cfg(target_os = "macos")]
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::{Command, Stdio};
#[cfg(target_os = "macos")]
use std::time::{Duration, Instant};

pub const PASTE_HELPER_SCHEMA_VERSION: u32 = 1;
pub const MAX_HELPER_REQUEST_BYTES: usize = 4096;
pub const MAX_ATTEMPT_ID_BYTES: usize = 64;
pub const MAX_BUNDLE_ID_BYTES: usize = 255;
pub const MAX_EXECUTABLE_PATH_BYTES: usize = 1024;
pub const MAX_SIGNING_FIELD_BYTES: usize = 256;
pub const MAX_HELPER_RESPONSE_BYTES: usize = 16 * 1024;
pub const DEFAULT_HELPER_TIMEOUT_MS: u64 = 2_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HelperFocusMode {
    Exact,
    FrontmostPidFallback,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedExecutableIdentity {
    pub executable_path: String,
    pub team_id: String,
    pub cdhash: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PasteHelperRequest {
    pub schema_version: u32,
    pub attempt_id: String,
    pub owner_epoch: String,
    pub parent_pid: u32,
    pub target_pid: i32,
    pub target_bundle_id: String,
    pub focus_mode: HelperFocusMode,
    pub expected_executable_identity: ExpectedExecutableIdentity,
}

impl PasteHelperRequest {
    pub fn validate(&self) -> Result<(), ProtocolError> {
        if self.schema_version != PASTE_HELPER_SCHEMA_VERSION {
            return Err(ProtocolError::UnsupportedVersion(self.schema_version));
        }
        if self.attempt_id.is_empty()
            || self.attempt_id.len() > MAX_ATTEMPT_ID_BYTES
            || uuid::Uuid::parse_str(&self.attempt_id).is_err()
        {
            return Err(ProtocolError::InvalidField("attempt_id"));
        }
        if OwnerEpoch::parse_canonical(&self.owner_epoch).is_none() {
            return Err(ProtocolError::InvalidField("owner_epoch"));
        }
        if self.parent_pid == 0 {
            return Err(ProtocolError::InvalidField("parent_pid"));
        }
        if self.target_pid <= 0 {
            return Err(ProtocolError::InvalidField("target_pid"));
        }
        if !valid_bundle_id(&self.target_bundle_id) {
            return Err(ProtocolError::InvalidField("target_bundle_id"));
        }
        let identity = &self.expected_executable_identity;
        if identity.executable_path.is_empty()
            || identity.executable_path.len() > MAX_EXECUTABLE_PATH_BYTES
            || !identity.executable_path.starts_with('/')
        {
            return Err(ProtocolError::InvalidField("executable_path"));
        }
        if !valid_signing_field(&identity.team_id) {
            return Err(ProtocolError::InvalidField("team_id"));
        }
        if !valid_signing_field(&identity.cdhash) {
            return Err(ProtocolError::InvalidField("cdhash"));
        }
        Ok(())
    }
}

fn valid_bundle_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_BUNDLE_ID_BYTES
        && value.contains('.')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

fn valid_signing_field(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= MAX_SIGNING_FIELD_BYTES
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PasteHelperPhase {
    Booted,
    Validated,
    PostStarted,
    PostedUnverified,
    RejectedBeforePost,
}

impl PasteHelperPhase {
    fn sequence(self) -> u8 {
        match self {
            Self::Booted => 1,
            Self::Validated => 2,
            Self::PostStarted => 3,
            Self::PostedUnverified => 4,
            Self::RejectedBeforePost => 255,
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::PostedUnverified | Self::RejectedBeforePost)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PasteHelperFrame {
    pub schema_version: u32,
    pub attempt_id: String,
    pub owner_epoch: String,
    pub helper_pid: u32,
    pub phase: PasteHelperPhase,
    pub reason: Option<String>,
    pub elapsed_ms: u64,
}

impl PasteHelperFrame {
    pub fn new(request: &PasteHelperRequest, phase: PasteHelperPhase, elapsed_ms: u64) -> Self {
        Self {
            schema_version: PASTE_HELPER_SCHEMA_VERSION,
            attempt_id: request.attempt_id.clone(),
            owner_epoch: request.owner_epoch.clone(),
            helper_pid: std::process::id(),
            phase,
            reason: None,
            elapsed_ms,
        }
    }

    pub fn rejected(
        request: &PasteHelperRequest,
        reason: impl Into<String>,
        elapsed_ms: u64,
    ) -> Self {
        let mut frame = Self::new(request, PasteHelperPhase::RejectedBeforePost, elapsed_ms);
        frame.reason = Some(reason.into());
        frame
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProtocolError {
    TooLarge,
    InvalidJson,
    UnsupportedVersion(u32),
    InvalidField(&'static str),
    IdentityMismatch,
    DuplicateOrOutOfOrder,
    FrameAfterTerminal,
}

pub fn parse_request(bytes: &[u8]) -> Result<PasteHelperRequest, ProtocolError> {
    if bytes.is_empty() || bytes.len() > MAX_HELPER_REQUEST_BYTES {
        return Err(ProtocolError::TooLarge);
    }
    let request: PasteHelperRequest =
        serde_json::from_slice(bytes).map_err(|_| ProtocolError::InvalidJson)?;
    request.validate()?;
    Ok(request)
}

pub fn encode_request(request: &PasteHelperRequest) -> Result<Vec<u8>, ProtocolError> {
    request.validate()?;
    let encoded = serde_json::to_vec(request).map_err(|_| ProtocolError::InvalidJson)?;
    if encoded.len() > MAX_HELPER_REQUEST_BYTES {
        return Err(ProtocolError::TooLarge);
    }
    Ok(encoded)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PasteHelperAttemptOutcome {
    SpawnFailed {
        reason: String,
    },
    RejectedBeforePost {
        helper_pid: u32,
        reason: String,
    },
    DispatchedUnverified {
        helper_pid: u32,
    },
    UnknownAfterDispatch {
        helper_pid: Option<u32>,
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PasteHelperDispatchOutcome {
    RejectedBeforePost {
        helper_pid: Option<u32>,
        reason: String,
        attempts: u8,
    },
    DispatchedUnverified {
        helper_pid: u32,
        attempts: u8,
    },
    UnknownAfterDispatch {
        helper_pid: Option<u32>,
        reason: String,
        attempts: u8,
    },
}

pub trait PasteHelperLauncher {
    fn launch_once(&mut self, request: &PasteHelperRequest) -> PasteHelperAttemptOutcome;
}

pub fn rejection_is_safe_to_retry(reason: &str) -> bool {
    [
        "codesign_start_failed:",
        "owner_lock_path_unavailable:",
        "current_executable_unavailable:",
    ]
    .iter()
    .any(|prefix| reason.starts_with(prefix))
}

pub fn dispatch_with_launcher<L: PasteHelperLauncher>(
    launcher: &mut L,
    request: &PasteHelperRequest,
) -> PasteHelperDispatchOutcome {
    for attempt in 1..=2_u8 {
        match launcher.launch_once(request) {
            PasteHelperAttemptOutcome::SpawnFailed { reason: _ } if attempt == 1 => continue,
            PasteHelperAttemptOutcome::SpawnFailed { reason } => {
                return PasteHelperDispatchOutcome::RejectedBeforePost {
                    helper_pid: None,
                    reason,
                    attempts: attempt,
                };
            }
            PasteHelperAttemptOutcome::RejectedBeforePost {
                helper_pid: _,
                reason,
            } if attempt == 1 && rejection_is_safe_to_retry(&reason) => {
                continue;
            }
            PasteHelperAttemptOutcome::RejectedBeforePost { helper_pid, reason } => {
                return PasteHelperDispatchOutcome::RejectedBeforePost {
                    helper_pid: Some(helper_pid),
                    reason,
                    attempts: attempt,
                };
            }
            PasteHelperAttemptOutcome::DispatchedUnverified { helper_pid } => {
                return PasteHelperDispatchOutcome::DispatchedUnverified {
                    helper_pid,
                    attempts: attempt,
                };
            }
            PasteHelperAttemptOutcome::UnknownAfterDispatch { helper_pid, reason } => {
                return PasteHelperDispatchOutcome::UnknownAfterDispatch {
                    helper_pid,
                    reason,
                    attempts: attempt,
                };
            }
        }
    }
    unreachable!("paste helper dispatch loop is bounded")
}

#[derive(Clone, Debug)]
pub struct PhaseFrameValidator {
    attempt_id: String,
    owner_epoch: String,
    helper_pid: u32,
    last_sequence: u8,
    terminal: bool,
    post_started: bool,
}

#[cfg(target_os = "macos")]
pub struct SystemPasteHelperLauncher {
    timeout: Duration,
}

#[cfg(target_os = "macos")]
impl Default for SystemPasteHelperLauncher {
    fn default() -> Self {
        Self {
            timeout: Duration::from_millis(DEFAULT_HELPER_TIMEOUT_MS),
        }
    }
}

#[cfg(target_os = "macos")]
impl SystemPasteHelperLauncher {
    pub fn with_timeout(timeout: Duration) -> Self {
        Self { timeout }
    }

    fn collect_stdout(child: &mut std::process::Child) -> Result<Vec<u8>, String> {
        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| "helper_stdout_missing".to_string())?;
        let mut bytes = Vec::new();
        stdout
            .by_ref()
            .take((MAX_HELPER_RESPONSE_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| format!("helper_stdout_read_failed:{error}"))?;
        if bytes.len() > MAX_HELPER_RESPONSE_BYTES {
            return Err("helper_stdout_too_large".into());
        }
        Ok(bytes)
    }

    fn classify_frames(
        request: &PasteHelperRequest,
        helper_pid: u32,
        bytes: &[u8],
    ) -> PasteHelperAttemptOutcome {
        let Ok(text) = std::str::from_utf8(bytes) else {
            return PasteHelperAttemptOutcome::UnknownAfterDispatch {
                helper_pid: Some(helper_pid),
                reason: "helper_stdout_not_utf8".into(),
            };
        };
        let mut validator = PhaseFrameValidator::new(request, helper_pid);
        let mut terminal = None;
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            let frame: PasteHelperFrame = match serde_json::from_str(line) {
                Ok(frame) => frame,
                Err(_) => {
                    return PasteHelperAttemptOutcome::UnknownAfterDispatch {
                        helper_pid: Some(helper_pid),
                        reason: "invalid_helper_frame".into(),
                    };
                }
            };
            if validator.accept(&frame).is_err() {
                return PasteHelperAttemptOutcome::UnknownAfterDispatch {
                    helper_pid: Some(helper_pid),
                    reason: "invalid_helper_frame_sequence".into(),
                };
            }
            if frame.phase.is_terminal() {
                terminal = Some(frame);
            }
        }
        match terminal {
            Some(PasteHelperFrame {
                phase: PasteHelperPhase::PostedUnverified,
                ..
            }) => PasteHelperAttemptOutcome::DispatchedUnverified { helper_pid },
            Some(PasteHelperFrame {
                phase: PasteHelperPhase::RejectedBeforePost,
                reason: Some(reason),
                ..
            }) => PasteHelperAttemptOutcome::RejectedBeforePost { helper_pid, reason },
            _ => PasteHelperAttemptOutcome::UnknownAfterDispatch {
                helper_pid: Some(helper_pid),
                reason: if validator.post_started() {
                    "helper_ended_after_post_started_without_terminal"
                } else {
                    "helper_ended_without_explicit_pre_post_rejection"
                }
                .into(),
            },
        }
    }
}

#[cfg(target_os = "macos")]
impl PasteHelperLauncher for SystemPasteHelperLauncher {
    fn launch_once(&mut self, request: &PasteHelperRequest) -> PasteHelperAttemptOutcome {
        let encoded = match encode_request(request) {
            Ok(encoded) => encoded,
            Err(error) => {
                return PasteHelperAttemptOutcome::SpawnFailed {
                    reason: format!("invalid_parent_request:{error:?}"),
                };
            }
        };
        let executable = &request.expected_executable_identity.executable_path;
        let mut child = match Command::new(executable)
            .arg("--speech-paste-helper")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
        {
            Ok(child) => child,
            Err(error) => {
                return PasteHelperAttemptOutcome::SpawnFailed {
                    reason: format!("helper_spawn_failed:{error}"),
                };
            }
        };
        let helper_pid = child.id();
        let write_result = child
            .stdin
            .take()
            .ok_or_else(|| "helper_stdin_missing".to_string())
            .and_then(|mut stdin| {
                stdin
                    .write_all(&encoded)
                    .map_err(|error| format!("helper_stdin_write_failed:{error}"))?;
                stdin
                    .flush()
                    .map_err(|error| format!("helper_stdin_flush_failed:{error}"))
            });
        if let Err(reason) = write_result {
            let _ = child.kill();
            let _ = child.wait();
            return PasteHelperAttemptOutcome::UnknownAfterDispatch {
                helper_pid: Some(helper_pid),
                reason,
            };
        }

        let deadline = Instant::now() + self.timeout;
        let mut timed_out = false;
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                Ok(None) => {
                    timed_out = true;
                    let _ = child.kill();
                    let _ = child.wait();
                    break;
                }
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return PasteHelperAttemptOutcome::UnknownAfterDispatch {
                        helper_pid: Some(helper_pid),
                        reason: format!("helper_wait_failed:{error}"),
                    };
                }
            }
        }
        let bytes = match Self::collect_stdout(&mut child) {
            Ok(bytes) => bytes,
            Err(reason) => {
                return PasteHelperAttemptOutcome::UnknownAfterDispatch {
                    helper_pid: Some(helper_pid),
                    reason,
                };
            }
        };
        let classified = Self::classify_frames(request, helper_pid, &bytes);
        if timed_out
            && !matches!(
                classified,
                PasteHelperAttemptOutcome::RejectedBeforePost { .. }
            )
        {
            PasteHelperAttemptOutcome::UnknownAfterDispatch {
                helper_pid: Some(helper_pid),
                reason: "helper_timeout_without_explicit_pre_post_rejection".into(),
            }
        } else {
            classified
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HelperBootEnvironment {
    pub actual_parent_pid: u32,
    pub current_executable_path: String,
    pub current_team_id: String,
    pub current_cdhash: String,
    pub owner: super::owner::OwnerMetadata,
}

pub fn validate_helper_boot_environment(
    request: &PasteHelperRequest,
    environment: &HelperBootEnvironment,
) -> Result<(), &'static str> {
    if environment.actual_parent_pid != request.parent_pid {
        return Err("parent_pid_mismatch");
    }
    if environment.owner.role != super::owner::SpeechProcessRole::CanonicalGui {
        return Err("owner_role_mismatch");
    }
    if environment.owner.pid != request.parent_pid {
        return Err("owner_pid_mismatch");
    }
    if environment.owner.epoch.as_deref() != Some(request.owner_epoch.as_str()) {
        return Err("owner_epoch_mismatch");
    }
    let expected = &request.expected_executable_identity;
    if environment.current_executable_path != expected.executable_path
        || environment.owner.executable != expected.executable_path
    {
        return Err("executable_path_mismatch");
    }
    if environment.current_team_id != expected.team_id
        || environment.owner.signing_identity != expected.team_id
    {
        return Err("team_id_mismatch");
    }
    if !environment
        .current_cdhash
        .eq_ignore_ascii_case(&expected.cdhash)
        || !environment
            .owner
            .signing_hash
            .eq_ignore_ascii_case(&expected.cdhash)
    {
        return Err("cdhash_mismatch");
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HelperTargetEnvironment {
    pub target_identity_matches: bool,
    pub frontmost_pid: i32,
    pub frontmost_bundle_id: String,
    pub accessibility_trusted: bool,
}

pub fn validate_helper_target_environment(
    request: &PasteHelperRequest,
    environment: &HelperTargetEnvironment,
) -> Result<(), &'static str> {
    if !environment.target_identity_matches {
        return Err("target_identity_mismatch");
    }
    if environment.frontmost_pid != request.target_pid
        || environment.frontmost_bundle_id != request.target_bundle_id
    {
        return Err("frontmost_target_mismatch");
    }
    if !environment.accessibility_trusted {
        return Err("accessibility_not_trusted");
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn codesign_identity(path: &Path) -> Result<(String, String), String> {
    let output = Command::new("/usr/bin/codesign")
        .args(["-dv", "--verbose=4"])
        .arg(path)
        .stdout(Stdio::null())
        .output()
        .map_err(|error| format!("codesign_start_failed:{error}"))?;
    if !output.status.success() {
        return Err("codesign_identity_unavailable".into());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let mut team_id = None;
    let mut cdhash = None;
    for line in stderr.lines() {
        if let Some(value) = line.strip_prefix("TeamIdentifier=") {
            team_id = Some(value.trim().to_owned());
        } else if let Some(value) = line.strip_prefix("CDHash=") {
            cdhash = Some(value.trim().to_owned());
        }
    }
    match (team_id, cdhash) {
        (Some(team_id), Some(cdhash)) if !team_id.is_empty() && !cdhash.is_empty() => {
            Ok((team_id, cdhash))
        }
        _ => Err("codesign_identity_incomplete".into()),
    }
}

#[cfg(target_os = "macos")]
fn canonical_executable_path(path: PathBuf) -> Result<String, String> {
    path.canonicalize()
        .map_err(|error| format!("current_executable_unavailable:{error}"))
        .map(|path| path.display().to_string())
}

#[cfg(target_os = "macos")]
fn load_boot_environment() -> Result<HelperBootEnvironment, String> {
    let current_path = std::env::current_exe()
        .map_err(|error| format!("current_executable_unavailable:{error}"))?;
    let current_executable_path = canonical_executable_path(current_path.clone())?;
    let (current_team_id, current_cdhash) = codesign_identity(&current_path)?;
    let owner_path = super::runtime_paths::production_owner_lock_path()
        .map_err(|error| format!("owner_lock_path_unavailable:{error}"))?;
    let owner =
        super::owner::read_existing_owner_metadata(&owner_path, unsafe { libc::geteuid() })?;
    Ok(HelperBootEnvironment {
        actual_parent_pid: unsafe { libc::getppid() }.max(0) as u32,
        current_executable_path,
        current_team_id,
        current_cdhash,
        owner,
    })
}

#[cfg(target_os = "macos")]
fn load_target_environment(request: &PasteHelperRequest) -> HelperTargetEnvironment {
    let target = super::target::FrontmostApplication {
        bundle_id: request.target_bundle_id.clone(),
        pid: request.target_pid,
    };
    let frontmost = super::target::capture_frontmost_application().ok();
    HelperTargetEnvironment {
        target_identity_matches: super::target::application_matches_identity(&target),
        frontmost_pid: frontmost
            .as_ref()
            .map(|target| target.pid)
            .unwrap_or_default(),
        frontmost_bundle_id: frontmost.map(|target| target.bundle_id).unwrap_or_default(),
        accessibility_trusted: super::check_accessibility_permission(),
    }
}

#[cfg(target_os = "macos")]
fn write_frame(stdout: &mut impl Write, frame: &PasteHelperFrame) -> Result<(), String> {
    serde_json::to_writer(&mut *stdout, frame).map_err(|error| error.to_string())?;
    stdout.write_all(b"\n").map_err(|error| error.to_string())?;
    stdout.flush().map_err(|error| error.to_string())
}

#[cfg(target_os = "macos")]
fn post_annotated_session_paste() -> Result<(), String> {
    use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation, CGKeyCode};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    const KEY_CODE_V: CGKeyCode = 9;
    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState)
        .map_err(|_| "event_source_unavailable".to_string())?;
    let key_down = CGEvent::new_keyboard_event(source.clone(), KEY_CODE_V, true)
        .map_err(|_| "key_down_unavailable".to_string())?;
    key_down.set_flags(CGEventFlags::CGEventFlagCommand);
    let key_up = CGEvent::new_keyboard_event(source, KEY_CODE_V, false)
        .map_err(|_| "key_up_unavailable".to_string())?;
    key_up.set_flags(CGEventFlags::CGEventFlagCommand);
    key_down.post(CGEventTapLocation::AnnotatedSession);
    key_up.post(CGEventTapLocation::AnnotatedSession);
    std::thread::sleep(Duration::from_millis(60));
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn run_paste_helper_stdio(dry_run: bool) -> Result<(), String> {
    let started = Instant::now();
    let mut bytes = Vec::new();
    std::io::stdin()
        .take((MAX_HELPER_REQUEST_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("request_read_failed:{error}"))?;
    let request = parse_request(&bytes).map_err(|error| format!("invalid_request:{error:?}"))?;
    let mut stdout = std::io::stdout().lock();

    let boot_environment = match load_boot_environment() {
        Ok(environment) => environment,
        Err(reason) => {
            write_frame(
                &mut stdout,
                &PasteHelperFrame::rejected(
                    &request,
                    reason.clone(),
                    started.elapsed().as_millis() as u64,
                ),
            )?;
            return Err(reason);
        }
    };
    if let Err(reason) = validate_helper_boot_environment(&request, &boot_environment) {
        write_frame(
            &mut stdout,
            &PasteHelperFrame::rejected(&request, reason, started.elapsed().as_millis() as u64),
        )?;
        return Err(reason.into());
    }
    write_frame(
        &mut stdout,
        &PasteHelperFrame::new(
            &request,
            PasteHelperPhase::Booted,
            started.elapsed().as_millis() as u64,
        ),
    )?;

    if super::is_own_bundle_id(&request.target_bundle_id) {
        let reason = "own_bundle_rejected";
        write_frame(
            &mut stdout,
            &PasteHelperFrame::rejected(&request, reason, started.elapsed().as_millis() as u64),
        )?;
        return Err(reason.into());
    }
    let target_environment = load_target_environment(&request);
    if let Err(reason) = validate_helper_target_environment(&request, &target_environment) {
        write_frame(
            &mut stdout,
            &PasteHelperFrame::rejected(&request, reason, started.elapsed().as_millis() as u64),
        )?;
        return Err(reason.into());
    }
    write_frame(
        &mut stdout,
        &PasteHelperFrame::new(
            &request,
            PasteHelperPhase::Validated,
            started.elapsed().as_millis() as u64,
        ),
    )?;
    if dry_run {
        write_frame(
            &mut stdout,
            &PasteHelperFrame::rejected(
                &request,
                "dry_run_validated",
                started.elapsed().as_millis() as u64,
            ),
        )?;
        return Ok(());
    }

    write_frame(
        &mut stdout,
        &PasteHelperFrame::new(
            &request,
            PasteHelperPhase::PostStarted,
            started.elapsed().as_millis() as u64,
        ),
    )?;
    post_annotated_session_paste()?;
    write_frame(
        &mut stdout,
        &PasteHelperFrame::new(
            &request,
            PasteHelperPhase::PostedUnverified,
            started.elapsed().as_millis() as u64,
        ),
    )?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn run_paste_helper_stdio(_dry_run: bool) -> Result<(), String> {
    Err("speech paste helper is only available on macOS".into())
}

impl PhaseFrameValidator {
    pub fn new(request: &PasteHelperRequest, helper_pid: u32) -> Self {
        Self {
            attempt_id: request.attempt_id.clone(),
            owner_epoch: request.owner_epoch.clone(),
            helper_pid,
            last_sequence: 0,
            terminal: false,
            post_started: false,
        }
    }

    pub fn accept(&mut self, frame: &PasteHelperFrame) -> Result<(), ProtocolError> {
        if self.terminal {
            return Err(ProtocolError::FrameAfterTerminal);
        }
        if frame.schema_version != PASTE_HELPER_SCHEMA_VERSION
            || frame.attempt_id != self.attempt_id
            || frame.owner_epoch != self.owner_epoch
            || frame.helper_pid != self.helper_pid
        {
            return Err(ProtocolError::IdentityMismatch);
        }
        let sequence = frame.phase.sequence();
        if frame.phase == PasteHelperPhase::RejectedBeforePost {
            if self.post_started || frame.reason.as_deref().unwrap_or_default().is_empty() {
                return Err(ProtocolError::DuplicateOrOutOfOrder);
            }
        } else if sequence != self.last_sequence.saturating_add(1) {
            return Err(ProtocolError::DuplicateOrOutOfOrder);
        }
        self.post_started |= frame.phase == PasteHelperPhase::PostStarted;
        self.terminal = frame.phase.is_terminal();
        self.last_sequence = sequence;
        Ok(())
    }

    pub fn post_started(&self) -> bool {
        self.post_started
    }

    pub fn terminal(&self) -> bool {
        self.terminal
    }
}
