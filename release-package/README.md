# iterate 安装包说明

这个目录保留一份和当前发布链路一致的安装资料，方便本地检查与手工打包时对照。

当前对外安装包的目标是：
- 安装 `iterate` 主程序
- 安装 `mcp-server`
- 接入用户正在使用的客户端
- 在卡住时提供一份可以直接交给 AI 的安装提示词

## 包内应包含的文件

- `iterate` / `iterate.exe`
- `mcp-server` / `mcp-server.exe`
- `WebView2Loader.dll`（Windows）
- `install.sh`（macOS / Linux）
- `Install iterate.bat`（Windows）
- `Start iterate.bat`（Windows）
- `INSTALLATION.md`

## 用户如何使用

1. 下载对应系统的 iterate 安装包
2. macOS / Linux：解压后运行 `./install.sh`
3. Windows：解压后先运行 `Install iterate.bat`
4. 安装完成后重启已配置的客户端

## 支持的客户端

- Windsurf
- Cursor
- Codex CLI
- 其他支持 MCP 的客户端

## 如果安装卡住

直接把 `INSTALLATION.md` 里的 AI 提示词发给 AI，让 AI 继续帮助完成：
- iterate 安装
- 客户端接入
- MCP 配置
- 最小调用验证

## 维护说明

GitHub Release 的安装包由仓库根目录的这些文件生成：
- `install.sh`
- `release-package/windows/Install iterate.bat`
- `release-package/windows/Start iterate.bat`
- `docs/iterate_安装指南.md`

如果这里的内容和根目录方案不一致，应该优先以根目录为准并同步更新这个目录。
