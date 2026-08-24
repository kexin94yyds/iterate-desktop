# VerificationReceipt v0

VerificationReceipt v0 is the required completion artifact for GoalRun, pairoom dispatch, release routines, and any loop that claims work is complete, fixed, verified, or ready for review.

## Receipt Template

```markdown
## Verification Receipt

- Goal:
- Scope:
- Changes:
- Commands:
  - command:
    result:
    evidence:
- Evidence:
- Risks:
- Next Gate:
```

## Field Rules

| Field | Required content |
|---|---|
| Goal | The normalized objective and the user's original wording when useful. |
| Scope | Files, modules, generated artifacts, and explicit exclusions. |
| Changes | What changed, grouped by responsibility rather than by commit. |
| Commands | Fresh verification commands run in this session, with exit status or exact failure. |
| Evidence | The concrete output that supports the claim, such as pass counts, diff checks, rendered artifact checks, or manual inspection scope. |
| Risks | Remaining uncertainty, skipped verification, dirty worktree caveats, and known user-owned changes. |
| Next Gate | The next user, test, review, release, or persistence decision. |

## Completion Rules

1. Do not write "done", "fixed", "verified", "ready", or "passes" without fresh evidence in `Commands` and `Evidence`.
2. If a command cannot be run, state the exact reason in `Commands` and keep the risk visible in `Risks`.
3. If the worktree contains unrelated changes, list them in `Risks`; do not revert them.
4. If the task found a new recurring or durable problem, ask before writing to `~/.cunzhi-knowledge/problems.md`.
5. Final handoff through zhi/call_zhi should include this receipt or point to the file that contains it.

## Compact zhi Form

Use this shorter form when the popup should stay concise:

```markdown
## 已完成

-

## 验证

-

## 风险

-

## 下一关口

-
```
