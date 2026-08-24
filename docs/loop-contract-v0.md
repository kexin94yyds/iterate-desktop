# LoopContract v0

LoopContract v0 is the shared runtime contract for iterate, GoalRun, zhi, pairoom, built-in subagents, release routines, and future automation loops. It keeps every loop bounded by one goal, one run identity, explicit tool policy, and a verification handoff.

## Required Envelope

Every loop run should be describable by this envelope before work starts:

```yaml
loop_id: stable workflow family id, for example goalrun, pairoom, release, debug
run_id: unique execution id for this run
generation: integer generation for retries or superseding runs
timeline_route_id: current timeline or conversation route id when available
project_path: absolute project path
trigger: user | zhi | goal_submit | hook | automation | room
goal: concise objective in the user's words plus normalized execution target
target_files:
  - path: exact file or directory
    source: user | hui1 | xi | diff | search | plan
    reason: why this file is in scope
    confidence: high | medium | low
excluded_files:
  - path: exact file or directory
    reason: why it is out of scope
context_policy:
  hui1_required: true | false
  xi_required: true | false
  stable_artifacts_first: true
  stale_run_policy: ignore | mark_stale | supersede
tool_policy:
  can_read: true
  can_write: true | false
  can_network: true | false
  can_use_computer_use: false
  can_commit: false
  can_push: false
  requires_zhi:
    - scope_expansion
    - destructive_operation
    - credentials_or_login
    - computer_use
    - commit_push_publish
    - new_problem_recording
verification:
  required: true
  commands:
    - exact command or explicit reason when not runnable
  evidence_expected:
    - test output
    - diff check
    - artifact inspection
handoff:
  receipt_required: true
  worker_handoff_required: true | false
  user_visible_channel: zhi | call_zhi | room_post | final
stop_conditions:
  - success criteria met
  - blocked with evidence
  - scope expansion needed
  - high risk authorization needed
  - consecutive no-progress attempts
```

## Operating Rules

1. A loop without `goal`, `tool_policy`, `verification`, and `stop_conditions` is only a draft, not an executable GoalRun.
2. `run_id`, `generation`, and `timeline_route_id` must travel together when available so old runs can be marked stale instead of merged into the current handoff.
3. `context_policy` prefers stable artifacts over long conversation replay. Use `hui1` only when the target depends on previous state, compressed context, or an explicit user request.
4. `tool_policy.requires_zhi` is the hard boundary. If any listed event happens, stop automatic execution and ask through zhi/call_zhi.
5. Completion claims require a VerificationReceipt v0. A passing command without risks and next gate is not a complete handoff.
6. Worker output uses WorkerHandoff v0. Workers do not own final user interaction unless explicitly assigned by the main session.

## Minimal GoalRun Template

```markdown
## Goal Spec

- loop_id:
- run_id:
- timeline_route_id:
- goal:
- target_files:
- excluded_files:
- context_policy:
- tool_policy:
- success_criteria:
- verification:
- stop_conditions:
```

## Minimal Ledger Entry

This v0 contract does not require a persistent ledger yet. When a ledger exists, each event should still contain:

```yaml
run_id:
generation:
timeline_route_id:
step:
actor:
action:
target_files:
evidence:
stale_of:
superseded_by:
created_at:
```
