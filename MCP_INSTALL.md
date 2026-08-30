# iterate MCP 集成指南

这份文档面向需要手动接入 MCP 的用户和开发者。

如果你只是要把 iterate 交付给普通用户，优先使用主安装方案：
- macOS / Linux：运行安装包中的 `install.sh`
- Windows：打开安装包中的 `INSTALLATION.md`

普通用户不需要手工拼配置；只有在自动安装失败，或你正在接入一个非标准客户端时，才需要参考下面的手动步骤。

## 推荐安装路径

1. 从 GitHub Releases 下载对应系统的 iterate 安装包
2. macOS / Linux：解压后运行 `./install.sh`
3. Windows：按 `INSTALLATION.md` 完成安装和客户端接入
4. 重启已配置的客户端

## 手动接入 MCP

先确认你已经拿到实际安装好的 `mcp-server` 路径，然后按客户端类型更新配置。

### Windsurf / Cursor

Windsurf 配置文件：
- `~/.codeium/windsurf/mcp_config.json`

Cursor 配置文件：
- `~/.cursor/mcp.json`

示例：

```json
{
  "mcpServers": {
    "iterate-zhi": {
      "command": "/path/to/mcp-server",
      "args": []
    }
  }
}
```

### Codex CLI

配置文件：
- `~/.codex/config.toml`

示例：

```toml
[mcp_servers."iterate-zhi"]
command = "/path/to/mcp-server"
args = []
disabled = false
enabled = true
tool_timeout_sec = 315360000
```

对 Codex CLI，建议显式保留 `tool_timeout_sec = 315360000`，作为 `call_zhi` 等待用户回复时的 10 年级别近似无限等待。

### 其他客户端

1. 先确认该客户端的 MCP 配置文件位置与格式
2. 只做最小改动，不要覆盖已有其他 MCP 条目
3. 配置完成后重启客户端并做一次最小调用验证

## 验证

完成配置后，至少确认下面四件事：

1. `iterate` 已安装
2. 客户端里能看到 `iterate-zhi`
3. `iterate-zhi` 可以正常启动
4. 能完成一次最小调用测试

## 如果安装卡住

优先回到主安装指南：
- [docs/INSTALLATION.md](docs/INSTALLATION.md)

如果你需要低层排查，再看：
- [MCP_SERVER.md](MCP_SERVER.md)
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
