# WorkerHandoff Schema v0

WorkerHandoff Schema v0 standardizes worker output for pairoom workers, Codex built-in subagents, and future verifier roles. It separates worker findings from final user-facing control transfer.

## Worker Kinds

| Kind | Identifier | Final user interaction |
|---|---|---|
| pairoom worker | `pairoom` | Post to room, then call zhi/call_zhi only to return to standby. The main session performs the final user handoff. |
| Codex built-in subagent | `built_in_subagent` | Return result to the main session. It must not call zhi/call_zhi. |
| verifier | `verifier` | Return verification findings to the main session or room hub. It does not expand scope by itself. |

## Markdown Handoff Template

```markdown
## WorkerHandoff v0

- Owner:
- Kind: pairoom | built_in_subagent | verifier
- Task:
- Scope:
- Inputs Reviewed:
- Output:
- Evidence:
- Assumptions:
- Risks:
- Verification:
- Open Questions:
- Next Handoff:
```

## JSON Shape

```json
{
  "schema": "worker_handoff_v0",
  "owner": "agent-or-role-id",
  "kind": "pairoom",
  "task": "short task statement",
  "scope": ["file-or-module"],
  "inputs_reviewed": ["path-or-source"],
  "output": ["finding-or-change"],
  "evidence": ["command output, file reference, or inspection note"],
  "assumptions": ["explicit assumption"],
  "risks": ["remaining risk"],
  "verification": ["command or not-run reason"],
  "open_questions": ["question or none"],
  "next_handoff": "main session action"
}
```

## Room Post Compact Form

The existing `worker_done | ...` room post stays supported, but v0 requires the same concepts:

```text
worker_done | from=<agent> | schema=worker_handoff_v0 | kind=pairoom | status=<success|partial|failed> | scope=<scope> | output=<output> | evidence=<evidence> | assumptions=<assumptions> | risks=<risks> | verification=<verification> | open_questions=<open_questions> | next_handoff=<main-session-action>
```

## Rules

1. Workers report facts and evidence. The main session decides whether the overall GoalRun is complete.
2. `Inputs Reviewed` must name the files, docs, commands, or prompts actually inspected.
3. `Evidence` must be enough for the main session to review without replaying the whole worker context.
4. `Open Questions` should be `none` when there is no blocker; do not omit the field.
5. `Next Handoff` must say whether the main session should merge, verify, ask the user, record a problem, or stop.
6. `pairoom` workers may use `post -> zhi/call_zhi` only for room standby. `built_in_subagent` workers never call zhi/call_zhi.
