# iterate 排查指南

这份文档只保留当前主方案下仍然有用的排查路径。

如果你是普通用户，优先使用：
- `docs/INSTALLATION.md`
- 安装包里的 `INSTALLATION.md`

下面这些内容更适合开发者、接入者和排障场景。

## 1. `iterate-zhi` 不可用

先检查三件事：

1. 客户端配置文件里是否真的存在 `iterate-zhi`
2. `command` 是否指向真实存在的 `mcp-server`
3. 修改配置后是否已经重启客户端

常见配置文件位置：
- Windsurf: `~/.codeium/windsurf/mcp_config.json`
- Cursor: `~/.cursor/mcp.json`
- Codex CLI: `~/.codex/config.toml`

## 2. 出现重复入口或历史入口

如果客户端里同时出现多个 iterate 相关入口，优先检查是否还残留了历史配置，例如：
- `iterate-zhi`
- `iterate-xin`
- `cunzhi`
- 其他测试入口

除非你明确要保留多入口实验，否则建议只保留一套当前主入口：`iterate-zhi`。

## 3. 弹窗空白或前端资源缺失

开发场景下，常见原因是前端资源没有被正确打进产物。

推荐做法：

```bash
pnpm build
pnpm tauri build --no-bundle
```

不要把单独的 `cargo build --release` 当成完整的桌面端交付构建。

## 4. 发布物和文档对不上

当前对外交付的安装包应至少包含：
- `iterate`
- `mcp-server`
- `install.sh`（macOS / Linux）
- `INSTALLATION.md`

如果你下载到的包里没有这些内容，或文档仍在要求用户手工拼旧配置，这就是发布物问题，不是用户操作问题。

## 5. 手动配置的最小正确示例

### Windsurf / Cursor

```json
{
  "mcpServers": {
    "iterate-zhi": {
      "command": "/path/to/mcp-server",
      "args": [],
      "disabled": false,
      "enabled": true,
      "tool_timeout_sec": 315360000
    }
  }
}
```

### Codex CLI

```toml
[mcp_servers."iterate-zhi"]
command = "/path/to/mcp-server"
args = []
disabled = false
enabled = true
tool_timeout_sec = 315360000
```

对 `call_zhi` 这类需要等待用户回复的交互工具，建议显式保留 `tool_timeout_sec = 315360000`，用作 10 年级别的近似无限等待。

## 6. 如果你是开发者

你在本地改动前端或安装链路后，建议至少验证：

```bash
bash -n install.sh
git diff --check
pnpm exec eslint src/frontend/components/tabs/PromptsTab.vue
```

如果你还改了发布物名称或安装材料，再额外核对：
- `.github/workflows/release.yml`
- `.github/workflows/update-homebrew.yml`
- `release-package/README.md`
- `release-package/install.sh`

## 相关文档

- [docs/INSTALLATION.md](docs/INSTALLATION.md)
- [MCP_INSTALL.md](MCP_INSTALL.md)
- [MCP_SERVER.md](MCP_SERVER.md)
