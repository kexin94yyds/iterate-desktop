#[cfg(target_os = "macos")]
use super::runtime_paths::{open_existing_private_lock_file, open_private_lock_file};
use super::session::OwnerEpoch;
use serde::{Deserialize, Serialize};
#[cfg(target_os = "macos")]
use std::fs::File;
#[cfg(target_os = "macos")]
use std::io::{Read, Seek, SeekFrom, Write};
#[cfg(target_os = "macos")]
use std::os::fd::AsRawFd;
#[cfg(target_os = "macos")]
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpeechProcessRole {
    CanonicalGui,
    StandalonePopup,
    Serve,
    McpRequest,
    Bridge,
    Agent,
    PasteHelper,
}

impl SpeechProcessRole {
    pub fn from_args<'a>(args: impl IntoIterator<Item = &'a str>) -> Self {
        Self::from_runtime(args, false)
    }

    pub fn from_runtime<'a>(
        args: impl IntoIterator<Item = &'a str>,
        standalone_launch: bool,
    ) -> Self {
        let args: Vec<&str> = args.into_iter().collect();
        let executable = args.first().copied().unwrap_or_default();
        if standalone_launch {
            Self::StandalonePopup
        } else if args.iter().any(|arg| {
            matches!(
                *arg,
                "--speech-paste-helper" | "--speech-paste-helper-dry-run"
            )
        }) {
            Self::PasteHelper
        } else if executable.contains("speech-agent") {
            Self::Agent
        } else if args.iter().any(|arg| *arg == "--standalone") {
            Self::StandalonePopup
        } else if args.iter().any(|arg| *arg == "--bridge-only") {
            Self::Bridge
        } else if args
            .iter()
            .any(|arg| matches!(*arg, "mcp-request" | "--mcp-request"))
        {
            Self::McpRequest
        } else if args.iter().any(|arg| matches!(*arg, "serve" | "--serve")) {
            Self::Serve
        } else {
            Self::CanonicalGui
        }
    }

    pub fn is_owner_eligible(self) -> bool {
        matches!(self, Self::CanonicalGui | Self::Agent)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerMetadata {
    pub epoch: Option<String>,
    pub pid: u32,
    pub executable: String,
    pub signing_identity: String,
    pub signing_hash: String,
    pub acquired_unix_ms: u64,
    pub role: SpeechProcessRole,
}

impl OwnerMetadata {
    pub fn new(
        role: SpeechProcessRole,
        pid: u32,
        executable: impl Into<String>,
        signing_identity: impl Into<String>,
        signing_hash: impl Into<String>,
    ) -> Self {
        Self {
            epoch: None,
            pid,
            executable: executable.into(),
            signing_identity: signing_identity.into(),
            signing_hash: signing_hash.into(),
            acquired_unix_ms: 0,
            role,
        }
    }

    pub fn for_test(
        role: SpeechProcessRole,
        pid: u32,
        executable: &str,
        signing_identity: &str,
        signing_hash: &str,
    ) -> Self {
        Self::new(role, pid, executable, signing_identity, signing_hash)
    }

    fn unknown() -> Self {
        Self::for_test(
            SpeechProcessRole::CanonicalGui,
            0,
            "unknown",
            "unknown",
            "unknown",
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OwnerLeaseAttempt {
    Acquired,
    Busy(OwnerMetadata),
    Failed(String),
}

pub trait OwnerLeaseBackend {
    fn try_acquire(&mut self, metadata: &OwnerMetadata) -> OwnerLeaseAttempt;
    fn release(&mut self);
}

#[cfg(target_os = "macos")]
pub struct FileOwnerLease {
    path: PathBuf,
    uid: u32,
    file: Option<File>,
}

#[cfg(target_os = "macos")]
impl FileOwnerLease {
    pub fn new(path: PathBuf, uid: u32) -> Self {
        Self {
            path,
            uid,
            file: None,
        }
    }
}

#[cfg(target_os = "macos")]
pub fn read_existing_owner_metadata(
    path: &std::path::Path,
    uid: u32,
) -> Result<OwnerMetadata, String> {
    let mut file = open_existing_private_lock_file(path, uid).map_err(|error| error.to_string())?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|error| error.to_string())?;
    serde_json::from_str(&contents).map_err(|error| format!("invalid owner metadata: {error}"))
}

#[cfg(target_os = "macos")]
impl OwnerLeaseBackend for FileOwnerLease {
    fn try_acquire(&mut self, metadata: &OwnerMetadata) -> OwnerLeaseAttempt {
        if self.file.is_some() {
            return OwnerLeaseAttempt::Acquired;
        }
        let mut file = match open_private_lock_file(&self.path, self.uid) {
            Ok(file) => file,
            Err(error) => return OwnerLeaseAttempt::Failed(error.to_string()),
        };
        let result = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if result != 0 {
            let mut contents = String::new();
            let _ = file.seek(SeekFrom::Start(0));
            let _ = file.read_to_string(&mut contents);
            let owner =
                serde_json::from_str(&contents).unwrap_or_else(|_| OwnerMetadata::unknown());
            return OwnerLeaseAttempt::Busy(owner);
        }
        let mut stored = metadata.clone();
        if stored.epoch.is_none() {
            let bytes = uuid::Uuid::new_v4().into_bytes();
            stored.epoch = Some(OwnerEpoch(bytes).to_canonical_string());
        }
        stored.acquired_unix_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .min(u64::MAX as u128) as u64;
        let encoded = serde_json::to_vec(&stored).unwrap_or_default();
        let _ = file.set_len(0);
        let _ = file.seek(SeekFrom::Start(0));
        if file.write_all(&encoded).is_err() || file.sync_data().is_err() {
            let _ = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) };
            return OwnerLeaseAttempt::Failed("failed to write owner metadata".into());
        }
        self.file = Some(file);
        OwnerLeaseAttempt::Acquired
    }

    fn release(&mut self) {
        if let Some(file) = self.file.take() {
            let _ = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_UN) };
        }
    }
}

pub trait OwnerEpochSource {
    fn next_epoch(&mut self) -> [u8; 16];
}

pub struct RandomEpochSource;
impl OwnerEpochSource for RandomEpochSource {
    fn next_epoch(&mut self) -> [u8; 16] {
        uuid::Uuid::new_v4().into_bytes()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OwnerSupervisorCommand {
    Reconcile,
    ListenerEnded,
    RequestIdleHandoff,
    Shutdown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OwnerSupervisorEvent {
    Acquired(OwnerEpoch),
    Busy(OwnerMetadata),
    Released,
    Failed(String),
    ListenerStartRequested,
}

pub struct OwnerSupervisor<B, E> {
    metadata: OwnerMetadata,
    backend: B,
    epochs: E,
    epoch: Option<OwnerEpoch>,
    listener_starts: u64,
    shutdown: bool,
}

impl<B: OwnerLeaseBackend, E: OwnerEpochSource> OwnerSupervisor<B, E> {
    pub fn new(metadata: OwnerMetadata, backend: B, epochs: E) -> Self {
        Self {
            metadata,
            backend,
            epochs,
            epoch: None,
            listener_starts: 0,
            shutdown: false,
        }
    }

    pub fn apply(
        &mut self,
        command: OwnerSupervisorCommand,
        idle: bool,
    ) -> Vec<OwnerSupervisorEvent> {
        if self.shutdown {
            return Vec::new();
        }
        match command {
            OwnerSupervisorCommand::Reconcile
                if self.epoch.is_none() && self.metadata.role.is_owner_eligible() =>
            {
                let epoch = OwnerEpoch(self.epochs.next_epoch());
                let mut metadata = self.metadata.clone();
                metadata.epoch = Some(epoch.to_canonical_string());
                match self.backend.try_acquire(&metadata) {
                    OwnerLeaseAttempt::Acquired => {
                        self.epoch = Some(epoch);
                        self.listener_starts += 1;
                        vec![
                            OwnerSupervisorEvent::Acquired(epoch),
                            OwnerSupervisorEvent::ListenerStartRequested,
                        ]
                    }
                    OwnerLeaseAttempt::Busy(owner) => vec![OwnerSupervisorEvent::Busy(owner)],
                    OwnerLeaseAttempt::Failed(error) => vec![OwnerSupervisorEvent::Failed(error)],
                }
            }
            OwnerSupervisorCommand::ListenerEnded if self.epoch.is_some() => {
                self.listener_starts += 1;
                vec![OwnerSupervisorEvent::ListenerStartRequested]
            }
            OwnerSupervisorCommand::RequestIdleHandoff if idle && self.epoch.take().is_some() => {
                self.backend.release();
                vec![OwnerSupervisorEvent::Released]
            }
            OwnerSupervisorCommand::Shutdown => {
                if self.epoch.take().is_some() {
                    self.backend.release();
                }
                self.shutdown = true;
                Vec::new()
            }
            _ => Vec::new(),
        }
    }

    pub fn owner_epoch(&self) -> Option<OwnerEpoch> {
        self.epoch
    }
    pub fn listener_start_count(&self) -> u64 {
        self.listener_starts
    }
}
