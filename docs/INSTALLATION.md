# iterate 安装与使用说明

这是一份同时适用于 macOS 和 Windows 的总说明。安装步骤按平台区分；MCP 接入、系统提示词和验证步骤共用。普通用户不需要手动长期运行 `mcp-server`，配置完成后由 Windsurf、Cursor、Codex 等 AI 客户端自动启动它。

## 下载

从 [iterate Releases](https://github.com/kexin94yyds/iterate-releases/releases/latest) 下载当前正式版本：

| 平台 | 安装包 | 适用范围 |
| --- | --- | --- |
| macOS | `iterate_*.dmg` 或 `iterate-macos.zip` | 按 Release 标注选择与 Mac 架构匹配的包 |
| Windows | `iterate-windows-x64.zip` | 64 位 Windows |

不要下载 `.sha256`、`.sig` 或 `provenance.json` 作为安装包；这些文件用于完整性验证。

## macOS

1. 打开下载的 DMG；如果下载的是 ZIP，先解压得到 `iterate.app`。
2. 把 `iterate.app` 拖入“应用程序”（`/Applications`）。
3. 从“应用程序”打开 iterate。
4. 如果 macOS 首次打开时拦截应用，使用 Finder 中的“右键 → 打开”，并确认系统提示。
5. 在 iterate 主界面确认本地服务已经运行，然后继续完成下方的 MCP 接入。

macOS 的 MCP command 是：

```text
/Applications/iterate.app/Contents/MacOS/mcp-server
```

这个文件由 AI 客户端按 MCP 配置自动启动，不需要用户另开终端常驻运行。

## Windows

1. 解压 `iterate-windows-x64.zip`，不要直接在压缩包预览窗口里运行。
2. 双击 `Install iterate.bat`。
3. 安装程序会检查 Microsoft WebView2 Runtime；缺少时会联网安装。
4. 安装完成后，从桌面快捷方式启动 iterate。
5. 在 iterate 主界面确认本地服务已经运行，然后继续完成下方的 MCP 接入。

Windows 的 MCP command 安装在：

```text
%LOCALAPPDATA%\iterate\bin\mcp-server.exe
```

写入 MCP 配置时应使用解析后的绝对路径，例如：

```text
C:\Users\username\AppData\Local\iterate\bin\mcp-server.exe
```

不要假设所有客户端都会展开 `%LOCALAPPDATA%`。

## 接入 AI 客户端

安装 iterate 只完成了桌面端准备；还要把 `mcp-server` 注册到正在使用的 AI 客户端。配置名建议使用 `iterate-zhi`。

| 客户端 | macOS 配置位置 | Windows 配置位置 |
| --- | --- | --- |
| Windsurf | `~/.codeium/windsurf/mcp_config.json` | `%USERPROFILE%\.codeium\windsurf\mcp_config.json` |
| Cursor | `~/.cursor/mcp.json` | `%USERPROFILE%\.cursor\mcp.json` |
| Codex | `~/.codex/config.toml` | `%USERPROFILE%\.codex\config.toml` |

### Windsurf / Cursor

按上表打开当前系统对应的配置文件。保留文件中已有的其他 MCP，仅添加 iterate：

```json
{
  "mcpServers": {
    "iterate-zhi": {
      "command": "这里填写当前系统的 mcp-server 绝对路径",
      "args": []
    }
  }
}
```

Windows JSON 中的反斜杠需要按 JSON 语法写成 `\\`。

### Codex

按上表打开当前系统对应的 `config.toml`。保留已有配置，增加：

```toml
[mcp_servers."iterate-zhi"]
command = "这里填写当前系统的 mcp-server 绝对路径"
args = []
enabled = true
tool_timeout_sec = 315360000

[mcp_servers."iterate-zhi".tools.call_zhi]
approval_mode = "prompt"
```

较长的超时用于等待用户在 iterate 弹窗中回复；不要把正常等待误判为 MCP 卡死。
全新安装默认使用 `approval_mode = "prompt"`。如果配置中已有显式 `approval_mode`，安装器必须保留已有值，不得覆盖用户选择。

### 其他客户端

先查该客户端的官方 MCP 配置格式和位置，再填入当前系统的 `mcp-server` 绝对路径。不要照抄其他客户端的文件位置，也不要覆盖用户已有配置。

## MCP 如何启动

1. 安装 iterate；
2. 将上面的 command 写入 AI 客户端；
3. 完全退出并重新打开 IDE，或结束并重启 CLI 会话；
4. 客户端自动启动 MCP server；
5. 发起一次真实调用验证。

iterate 界面中的“本地服务已启动”只表示桌面端已经运行，不能单独证明当前 AI 客户端已经接入。必须以客户端能看到工具并成功弹出一次 iterate 为准。

## 配置通用系统提示词

打开 iterate 的“使用说明书”，复制“通用系统提示词”，再按当前客户端支持的方式放入 system prompt、rules 或 always-on memory。

这份模板只包含 iterate 的通用调用协议，不依赖 `.cunzhi-knowledge`、个人目录、Relearn 或专属编排规则。仓库中的基准文本见 [SYSTEM_PROMPT.md](SYSTEM_PROMPT.md)。

如果安装过程需要 AI 协助，可把 [INSTALL_PROMPT.md](INSTALL_PROMPT.md) 整段发送给 AI。

## 最小验证

安装完成后必须逐项确认：

1. iterate App 能正常打开；
2. AI 客户端已经完全重启；
3. 客户端能看到 `iterate-zhi`、`zhi` 或 `call_zhi`；
4. 调用时发送一个非空 `message`，例如“iterate 安装验证：请确认是否看到此弹窗”；
5. 桌面出现 iterate 弹窗；
6. 用户提交后，客户端收到 `继续对话: true` 和用户输入；
7. 用户点击左上角红叉时，只结束当前调用并返回 `继续对话: false`，不出现 MCP error。

## 常见问题

- macOS 找不到 MCP：确认 App 位于 `/Applications`，并完全重启 AI 客户端。
- Windows 找不到 MCP：确认 `mcp-server.exe` 已安装到 `%LOCALAPPDATA%\iterate\bin`，配置中使用了当前用户的绝对路径。
- Windows 窗口无法启动：确认 WebView2 Runtime 已安装。
- 配置已写但没有工具：完全退出旧 IDE/CLI 进程后重新打开，而不是只新建一个聊天。
- 调用一直等待：先确认 iterate 弹窗是否正在等待用户；交互型 MCP 的等待本身不是失败。

更多排障见 [TROUBLESHOOTING.md](../TROUBLESHOOTING.md)。
