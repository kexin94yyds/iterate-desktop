//! 真实工作区双写冒烟（会创建 **iterate 主仓** 的一次自动检查点 commit，并追加一行对话 md）。
//!
//! `auto_create_checkpoint` 使用 **`git add -A`**：若工作区除探针外还有未提交文件，会把它们一并打进检查点。
//! - **默认**：`git status --porcelain` 非空则 **直接失败**（避免误打包）。
//! - 强行在脏工作区跑：`CUNZHI_DUALWRITE_FORCE=1`（后果自负）。
//!
//! ```text
//! CUNZHI_DUALWRITE_SMOKE=1 cargo test -p cunzhi --test dualwrite_real_repo_smoke -- --nocapture
//! ```
//!
//! 若工作区在写入探针**之前**已有未提交变更（例如临时仓库里故意改文件以触发检查点），需再加 **`CUNZHI_DUALWRITE_FORCE=1`**。
//!
//! 可选：改用其他根目录（例如临时克隆）  
//! `CUNZHI_DUALWRITE_SMOKE=1 CUNZHI_DUALWRITE_WORKSPACE=/path/to/repo cargo test ...`
//!
//! 撤回示例（确认 md / log 对上之后）：
//! ```text
//! git -C /path/to/cunzhi reset --hard HEAD~1
//! rm -f /path/to/cunzhi/.dualwrite-smoke-autogen.txt
//! ```
//! 若全局 `.cunzhi-knowledge` 因 `write_git_checkpoint` 产生额外 commit，在知识库内按需 `git reset --hard HEAD~1`。

use cunzhi::mcp::tools::checkpoint;
use cunzhi::mcp::tools::interaction::{append_conversation_log, ConversationEntry};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn conversation_md_contains(dir: &Path, needle: &str) -> Option<PathBuf> {
    let entries = fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            if let Some(hit) = conversation_md_contains(&p, needle) {
                return Some(hit);
            }
        } else if p.extension().is_some_and(|x| x == "md") {
            let text = fs::read_to_string(&p).unwrap_or_default();
            if text.contains(needle) {
                return Some(p);
            }
        }
    }
    None
}

#[test]
fn real_repo_dualwrite_smoke() {
    if std::env::var("CUNZHI_DUALWRITE_SMOKE").ok().as_deref() != Some("1") {
        eprintln!("skip: set CUNZHI_DUALWRITE_SMOKE=1 to run (mutates git + conversations)");
        return;
    }

    let root = std::env::var("CUNZHI_DUALWRITE_WORKSPACE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")));

    let force = std::env::var("CUNZHI_DUALWRITE_FORCE").ok().as_deref() == Some("1");
    let status_out = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&root)
        .output()
        .expect("git status");
    assert!(
        status_out.status.success(),
        "git status 失败: {}",
        String::from_utf8_lossy(&status_out.stderr)
    );
    let dirty = !String::from_utf8_lossy(&status_out.stdout)
        .trim()
        .is_empty();
    if dirty && !force {
        panic!(
            "工作区不干净，已拒绝执行（避免检查点误打包其它未提交文件）。\n\
             请先 `git stash` 或使用干净克隆；若确知风险，设置 CUNZHI_DUALWRITE_FORCE=1。\n\
             当前 porcelain:\n{}",
            String::from_utf8_lossy(&status_out.stdout)
        );
    }

    let marker = root.join(".dualwrite-smoke-autogen.txt");
    fs::write(
        &marker,
        format!(
            "smoke {}\n",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ),
    )
    .expect("write marker");

    let root_s = root.to_str().expect("utf8 path");
    let checkpoint = checkpoint::maybe_auto_checkpoint(root_s, Some("dualwrite-smoke"))
        .expect("checkpoint should return Some when dirty");

    let out = Command::new("git")
        .args(["log", "-1", "--pretty=%s"])
        .current_dir(&root)
        .output()
        .expect("git log");
    let subject = String::from_utf8_lossy(&out.stdout).trim().to_string();
    assert_eq!(
        checkpoint.commit_subject, subject,
        "对话将要写入的 subject 应与当前 HEAD commit 一致"
    );
    assert!(
        checkpoint.commit_subject.contains("iterate-checkpoint:"),
        "subject 应含 iterate-checkpoint 前缀"
    );

    append_conversation_log(&ConversationEntry {
        ai_message: "[dualwrite smoke] 自动化探针：校验 workspace checkpoint 与对话 md 对齐"
            .to_string(),
        user_response: "ok".to_string(),
        project_path: Some(root_s.to_string()),
        image_count: 0,
        file_paths: vec![],
        image_paths: vec![],
        selected_options: vec![],
        conversation_id: Some("dualwrite-tree".to_string()),
        current_node_id: None,
        timeline_route_id: None,
        run_id: None,
        generation: None,
        stale_of: None,
        superseded_by: None,
        request_id: Some("dualwrite-smoke".to_string()),
        checkpoint_id: Some(checkpoint.checkpoint_id.clone()),
        checkpoint_commit: Some(checkpoint.commit_hash.clone()),
        push_status: Some(checkpoint.push_status.clone()),
        response_source: None,
        workspace_checkpoint_message: Some(checkpoint.commit_subject.clone()),
    });

    let knowledge_dir = std::env::var("CUNZHI_KNOWLEDGE_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(std::env::var("HOME").expect("HOME should be set"))
                .join(".cunzhi-knowledge")
        });
    let conv_root = knowledge_dir.join("conversations");
    assert!(
        conv_root.exists(),
        "期望存在全局 .cunzhi-knowledge/conversations"
    );

    let hit =
        conversation_md_contains(&conv_root, &checkpoint.commit_subject).unwrap_or_else(|| {
            panic!(
            "应在全局 .cunzhi-knowledge/conversations 下某 .md 中找到与 git subject 完全相同的行：{}",
            checkpoint.commit_subject
        )
        });
    eprintln!("matched conversation file: {}", hit.display());

    eprintln!(
        "---- 对上。若要撤回本次检查点： git -C {:?} reset --hard HEAD~1 ; rm -f {:?}",
        root, marker
    );
}
