# iterate MCP 服务器

这份文档记录 `mcp-server` 的低层接入方式。

如果你是在给普通用户做安装交付，优先使用：
- `install.sh`
- `docs/iterate_安装指南.md`

只有在下面这些场景，才建议直接看本文件：
- 你在开发或调试 `mcp-server`
- 你要把 iterate 接到一个非标准 MCP 客户端
- 你要核对工具调用协议

## 架构

```text
MCP 客户端
  ↓ stdio
mcp-server
  ↓
iterate
  ↓
弹窗 / 用户输入
```

## 编译

```bash
cargo build --bin mcp-server --release
```

编译产物：

```text
target/release/mcp-server
```

## 手动配置示例

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

`tool_timeout_sec = 315360000` 表示 10 年级别的近似无限等待，适合 `call_zhi` 这类需要长期等待用户输入的交互型 MCP。

## 工具调用示例

`call_zhi` 的典型参数如下：

```json
{
  "name": "call_zhi",
  "arguments": {
    "message": "## 任务摘要\n\n这里是要显示给用户的内容",
    "project_path": "/Users/example/my-project",
    "predefined_options": ["继续", "取消", "修改"],
    "is_markdown": true
  }
}
```

## 返回信息

常见返回字段包括：
- 用户输入
- 选中的选项
- 附加文件或图片路径
- 是否继续对话

## 常见问题

### 客户端识别不到 `iterate-zhi`

1. 确认 `command` 指向真实存在的 `mcp-server`
2. 不要把旧的 `寸止`、`cunzhi`、测试入口和 `iterate-zhi` 混着配
3. 修改配置后重启客户端

### MCP 能启动，但对话弹不出来

优先检查：
1. `iterate` 是否已经安装
2. 当前客户端配置是否指向正确的 `mcp-server`
3. 是否需要先重启客户端或重载 MCP

## 相关文档

- [MCP_INSTALL.md](MCP_INSTALL.md)
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md)
- [docs/iterate_安装指南.md](docs/iterate_安装指南.md)
