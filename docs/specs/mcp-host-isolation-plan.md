# MCP Host Isolation Plan

## 背景

当前 `Codex`、`Windsurf`、`CUI/CLI` 会共享：

- 同一个 `mcp-server` 二进制
- 同一份 `~/.cunzhi_ports` 全局登记表
- 相同或相近的默认入口端口

这会带来两个问题：

1. 一个宿主制造的历史死口会污染所有宿主的路由判断。
2. 多个宿主如果都把 `5311` 当首选入口，会在端口刚变成 idle 时互相“捡走”。

这类共享污染会在 IDE 侧表现成黄灯、重连、误以为“端口冲突”。

## 目标

- 不同宿主默认不再互相抢同一个入口端口。
- 路由和登记先按宿主隔离，再按 workspace 调度。
- dead port 自动清理，避免历史残留跨宿主传播。
- 保留现有 same-workspace busy spillover 能力，但限制在宿主自己的池子内。

## 非目标

- 不改变 `call_zhi` / `sync` / `checkpoint` 的工具协议。
- 不把 `mcp-server` 合并回 GUI 主进程。
- 不要求不同宿主完全禁止同时访问同一个 workspace。

## 推荐方案

### 1. 宿主独立端口池

为每个宿主分配独立的默认端口段：

- `Windsurf`: `5311~5399`
- `Codex`: `5411~5499`
- `CUI/CLI`: `5511~5599`

默认情况下：

1. 宿主只在自己的端口段里选 preferred port。
2. busy spillover 也只在自己的端口段里扩容。
3. 不跨宿主复用对方的 idle 端口。

### 2. 端口登记引入 `host_id`

当前 `~/.cunzhi_ports/<port>` 只记录 workspace，不足以区分宿主。

建议升级成可扩展元数据，至少包含：

- `host_id`
- `workspace`
- `port`
- `state`
- `updated_at`

最小兼容方案有两个：

1. 保持文件名仍为端口号，文件内容改成 JSON。
2. 改成目录结构：`~/.cunzhi_ports/<host_id>/<port>.json`

推荐第 2 种，因为天然避免不同宿主写到同一层目录。

### 3. 路由顺序

每次请求按以下顺序：

1. 识别宿主：`windsurf` / `codex` / `cui`
2. 先清理当前宿主目录里的 dead port
3. 查找当前宿主下、同 workspace 的 live port
4. 有 idle 则复用
5. 全 busy 且池未满，则在当前宿主端口段内启动新端口
6. 池满才排队或提示

不建议默认跨宿主 fallback，否则隔离会重新被打穿。

### 4. 自动回收

每个宿主独立执行：

- dead port probe 后立即删登记
- idle TTL 到期后回收端口
- reload/restart 时做一次启动清扫

## 兼容迁移

### Phase 1: 配置隔离

- Windsurf 保持 `5311`
- Codex 改到 `5411`
- CUI/CLI 预留 `5511`

这一步不改协议，收益最快。

### Phase 2: 代码隔离

`mcp-server` 增加 `host_id` 参数，例如：

```bash
mcp-server --host-id windsurf 5311
mcp-server --host-id codex 5411
mcp-server --host-id cui 5511
```

内部路由和登记全部改成先按 `host_id` 过滤。

### Phase 3: 数据迁移

启动时：

1. 读取旧的平铺 `~/.cunzhi_ports/*`
2. 对 live 条目按默认规则归类到对应宿主目录
3. dead 条目直接丢弃

## 观测与排障

路由日志至少打印：

- `host_id`
- `workspace`
- `preferred_port`
- `candidate_ports`
- `selected_port`
- `reason`

这样以后碰到黄灯，可以直接判断：

- 是宿主自己的池满了
- 还是 dead port 清理没生效
- 还是某个宿主仍在错误地跨池抢端口

## 当前状态

本轮已完成的止血措施：

- 清理 `~/.cunzhi_ports` 中 87 个 dead 条目
- `mcp-server` 增加 dead port 自动清理
- Windsurf 暂时只保留一套主 `iterate-zhi`
- Codex 默认入口从 `5311` 调整到 `5411`

## 下一步建议

优先级从高到低：

1. 给 `mcp-server` 正式加入 `host_id`
2. 把 `~/.cunzhi_ports` 升级成按宿主分目录的索引
3. 为宿主内端口池增加 `max_pool_size` 和 `idle_ttl`
4. 补一组 `Codex + Windsurf + CUI` 并发回归验证
