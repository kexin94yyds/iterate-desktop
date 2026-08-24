# CunZhi / iterate Ubiquitous Language

This document defines shared engineering terms for CunZhi / iterate. It is meant to be read before planning, review, research, or gray-box AI delegation.

## Hard Principles

1. **User interaction is MCP-only.** If the agent must wait for the user, it must directly call the MCP `zhi` / `call_zhi` tool.
2. **Facts before views.** Define the source of truth before defining caches, UI state, or sync payloads.
3. **User intent before queue order.** `focused` must not be overwritten by `latest` / `head` unless a valid intent event allows it.
4. **Validation is layered.** Dev state, installed app, real device, public tunnel, and production artifact are not interchangeable.
5. **Workers do not take over user interaction.** A worker must not inherit the main session's infinite dialogue protocol.
6. **The knowledge triplet is engineering memory.** Check `problems`, `regressions`, and `patterns` before planning changes in known risk areas.

## Protocol And Interaction

| Term | Definition | Common failure | Constraint |
| --- | --- | --- | --- |
| `zhi` / `call_zhi` | MCP user interaction tool. | Replacing it with Python, shell, bridge, or an async background command. | Only direct MCP tool calls are valid for user interaction. |
| MCP blocking semantics | The agent waits until the GUI/user response returns from the tool call. | A background process is still waiting, but the current agent turn has already ended. | The loop is closed only when the MCP tool call returns. |
| `keep_going` | Whether the user allows the current loop to continue. | Treating a status report as permission to stop or continue. | Each round is decided by the returned user response. |
| `loop_active` | Automatic continuation session state. | Infinite continuation without user takeover. | Must have stop conditions, max iteration, and forced popup escape. |
| `response_source` | Source of a response, such as popup, loop, continue, or cancel. | Parsing only response text and ignoring the source. | State machines must branch on source. |

## Request, Conversation, And Intent

| Term | Definition | Common failure | Constraint |
| --- | --- | --- | --- |
| `request_id` | Identity of a single MCP/GUI request. | Routing only by `project_path`. | Popup, checkpoint, timeline, and active session state should trace to a request id. |
| `project_path` | Workspace path for a request. | Treating it as a unique conversation/thread id. | Same-project concurrent conversations need a finer route key. |
| Conversation tree | Node tree for conversation/timeline history. | Treating it as a flat log list. | Node, parent, and current node semantics must remain recoverable. |
| Timeline | Visible view over conversation state. | Treating the timeline as the source of truth. | Timeline is a view, not the authority. |
| Focused request | The request the user is currently reading or acting on. | Overwriting it with `latest` / `head`. | Switch only on explicit selection, notification intent, or invalid old focus. |
| `latest` / `head` | Most recent item in a queue/list. | Treating it as user intent. | It means ordering only, not what the user wants to see. |
| Draft input | User input that has not been submitted yet. | Background sync overwrites it. | Automatic switching should be frozen while the user is drafting. |

## State Sources

| Term | Definition | Common failure | Constraint |
| --- | --- | --- | --- |
| SSOT | Single source of truth. | Several files/caches each claim authority. | Every state must name its authority. |
| Live window registry | Truth source for currently existing windows bound to requests. | Exposing recent cache as an active list. | Active sessions should be derived from live registry. |
| Recent session cache | Historical/cache data for first paint or fallback. | Treating cached sessions as still active. | It may only be used as fallback. |
| Derived state | UI state computed from facts. | Writing derived state back as authority. | It should be disposable and recomputable. |
| Health signal | Evidence for system health. | Treating one `200` response as whole-chain health. | Split owner, child, metrics, public probe, and local origin. |
| Artifact source | The artifact actually running or delivered. | Treating source edits as user-visible updates. | Verify the installed app, running process path, or production bundle. |

## Runtime Entrypoints And Bridges

| Term | Definition | Common failure | Constraint |
| --- | --- | --- | --- |
| `mcp-server` | MCP stdio tool entrypoint. | Treating it as the full GUI app. | It handles protocol entry and tool routing. |
| `serve` | HTTP dialogue service and port listener. | Treating it as the only runtime instance. | It must carry workspace, port, and busy-state semantics. |
| Tauri popup | Desktop GUI surface for user interaction. | Window visible equals protocol ready. | Separate visible state from ready handshake. |
| Bridge | Sync layer for iOS/browser/external clients. | Creating authoritative state inside the bridge. | Bridge syncs state; it does not replace the source of truth. |
| APNs | iOS background notification fallback. | Treating it as the foreground realtime channel. | Foreground visible channel and background fallback stay separate. |
| Pro Bridge | File-based bridge to a high-depth model session. | Treating synced full text as a clean result. | Require capability gate, request marker, and structured extraction. |

## Validation Layers

| Term | Definition | Common failure | Constraint |
| --- | --- | --- | --- |
| Dev state | Source tree or development server state. | Treating it as the user's runtime. | It proves only development state. |
| Installed artifact | `/Applications/*.app` or equivalent installed runtime. | Workspace bundle passes, but user runs an old installed app. | User acceptance must verify process path and installed artifact. |
| Real device | Real iPhone/device, lock screen, tunnel, APNs, or platform runtime. | Replacing it with simulator-only checks. | Mobile/notification work needs real device closure. |
| Contract test | Test of protocol or state semantics. | Testing only internal implementation details. | Response, route, focus, and registry semantics should be tested first. |
| Regression check | Historical anti-regression gate. | Treating it as a generic TODO. | Run it before changing the related boundary. |
| Public artifact | Website, release asset, or production bundle. | Treating the source repo as production. | Check with URL, hash, asset name, and response behavior. |

## AI Collaboration Boundaries

| Term | Definition | Common failure | Constraint |
| --- | --- | --- | --- |
| Main agent | Agent responsible for user interaction, synthesis, and final decision. | Treating a worker as the main conversation owner. | Main agent owns `zhi` and user confirmation. |
| Worker | Limited agent for read-only or bounded subtasks. | Calling `zhi`, modifying unapproved files, or inheriting infinite dialogue rules. | Default is read-only or explicitly bounded writes; no user takeover. |
| Gray-box delegation | Human defines interface/tests; AI implements inside the box. | AI defines the interface itself. | Delegate only after interface, test, and acceptance are clear. |
| Grill me | Pre-implementation interrogation to align design concepts. | Generating a quick plan and coding immediately. | Cross-module work must ask about facts, intent, boundaries, and acceptance first. |
| TDD | Small-step feedback loop. | AI writes a large patch and tests afterward. | Contract first, implementation second, refactor third. |

## Knowledge Triplet

| Term | Definition | AI workflow use |
| --- | --- | --- |
| Problem | Observed issue fact. | Identify historical risk before planning. |
| Regression | Check that prevents recurrence. | Build the acceptance checklist before editing. |
| Pattern | Reusable method. | Prefer existing engineering moves over inventing new ones. |

## Pre-Task Grill Me Checklist

Ask these before `yan`, `qiu`, `cha`, `plan`, or any cross-module work:

1. What is the source of truth? What is cache? What is view state?
2. Is `latest` / `head` being confused with `focused` / user intent?
3. Is a single health signal being treated as whole-chain health?
4. Which protocol boundaries are touched? Does any part require MCP blocking semantics?
5. Is there any risk of replacing MCP user interaction with Python, shell, bridge, or a background command?
6. Can a worker accidentally call `zhi` or modify unapproved files?
7. Which validation layers are required: dev, installed app, real device, public tunnel, or production artifact?
8. Which existing regression checks apply?
9. Which module is the deep module? Is its interface simple enough?
10. Which parts are safe for gray-box delegation, and which interfaces must be designed first?
