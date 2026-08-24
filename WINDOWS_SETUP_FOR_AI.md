# Windows 配置指南 (给 AI 看)

> 历史说明：本文件保留了大量 Windows 手动接入、旧路径和旧命名示例，适合作为排障参考。
>
> 当前对外交付的主路径请优先以 [docs/iterate_安装指南.md](docs/iterate_安装指南.md) 和安装包中的 `INSTALLATION.md` 为准。
> 如果本文与当前安装包说法冲突，以安装包和主安装指南为准。

> 本文档用于指导 AI 帮助 Windows 用户配置 iterate
> 
> **目标读者**：AI 助手（Cursor、Claude Desktop、Windsurf 等）
> 
> **核心功能**：让 AI 在 Windows 上能够弹出 GUI 与用户交互，实现无限对话循环

---

## 目录

1. [快速开始（5分钟上手）](#快速开始5分钟上手)
2. [问题诊断流程](#问题诊断流程)
3. [安装方式](#安装方式)
4. [环境变量配置](#环境变量配置)
5. [VSCode 扩展完整配置](#vscode-扩展完整配置)
6. [复制开头语功能](#复制开头语功能)
7. [Windows Global Rules 配置](#windows-global-rules-配置)
8. [AI 调用脚本格式](#ai-调用脚本格式)
9. [完整端到端示例](#完整端到端示例)
10. [故障排除检查清单](#故障排除检查清单)
11. [防火墙与杀毒软件](#防火墙与杀毒软件)
12. [常见错误代码](#常见错误代码)
13. [常见问题](#常见问题)
14. [文件结构](#文件结构windows)

---

## 快速开始（5分钟上手）

> 最快的配置路径，适合想立即使用的用户

### 前置条件检查

```powershell
# 1. （可选）检查 Python（用于调试脚本，推荐 3.8+）
python --version
# 如果需要运行 Python 工具但未安装，访问: https://www.python.org/downloads/
# 安装时务必勾选 "Add Python to PATH"

# 2. 检查 Git
git --version
# 如果未安装，访问: https://git-scm.com/download/win
# 或使用: winget install Git.Git

# 3. 检查 Node.js（仅源码构建需要）
node --version
# 如果未安装，访问: https://nodejs.org/

# 4. 检查 Rust（仅源码构建需要）
rustc --version
# 如果未安装，访问: https://rustup.rs/
```

### 环境配置建议

| 工具 | 用途 | 必需性 | 安装方式 |
|------|------|--------|----------|
| **Python 3.8+** | 可选：运行自定义 Python 工具 | ⚠️ 可选 | [python.org](https://www.python.org/downloads/) |
| **Git** | 克隆仓库 | ✅ 必需 | [git-scm.com](https://git-scm.com/download/win) |
| **Node.js** | 构建前端（仅源码构建） | ⚠️ 源码构建需要 | [nodejs.org](https://nodejs.org/) |
| **pnpm** | 包管理器（仅源码构建） | ⚠️ 源码构建需要 | `npm install -g pnpm` |
| **Rust** | 编译 Tauri（仅源码构建） | ⚠️ 源码构建需要 | [rustup.rs](https://rustup.rs/) |
| **VS Build Tools** | C++ 编译（仅源码构建） | ⚠️ 源码构建需要 | [visualstudio.com](https://visualstudio.microsoft.com/downloads/) |
| **WebView2** | GUI 渲染 | ⚠️ Win10/11 预装 | [microsoft.com](https://developer.microsoft.com/microsoft-edge/webview2/) |

### 一键安装脚本

```powershell
# 在 PowerShell 中运行（管理员权限）
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser

# 克隆仓库
git clone https://github.com/kexin94yyds/iterate-desktop.git
cd iterate-desktop

# 方式 A: 如果有预编译版本
# 从 https://github.com/kexin94yyds/iterate-releases/releases 下载 iterate-windows-x64.zip
# 解压后运行:
.\install-windows.ps1 -BuildOnly:$false

# 方式 B: 从源码构建（需要 Rust + pnpm）
.\install-windows.ps1
```

### 验证安装

```powershell
# 检查安装是否成功
& "$env:LOCALAPPDATA\寸止\bin\cunzhi.exe" --version

# 启动服务测试
& "$env:LOCALAPPDATA\寸止\bin\cunzhi.exe" --serve --port 5310
```

### 可选：安装官方 VSCode 扩展（单独分发）

```powershell
# 本 desktop-source 仓库不提供 VSIX。取得官方扩展文件后可执行：
code --install-extension "C:\path\to\iterate-official.vsix"

# 重启 VSCode，扩展会自动启动服务
```

### 开始使用

1. 打开 VSCode 侧边栏的 iterate 面板
2. 点击 "📋 复制开头语"
3. 粘贴到 AI 对话框
4. 开始对话！

---

## 问题诊断流程

当 Windows 用户报告 "端口无法启动" 或 "GUI 无法弹出" 时，按以下顺序排查：

### 1. 检查 iterate.exe 是否存在

```powershell
# 检查默认安装路径
Test-Path "$env:LOCALAPPDATA\寸止\bin\cunzhi.exe"

# 或检查 PATH 中是否有 iterate
where iterate
where cunzhi
```

**如果不存在** → 需要安装（见下方安装步骤）

### 2. 检查 Python 环境

```powershell
python --version
# 需要 Python 3.8+
```

### 3. 检查端口是否被占用

```powershell
netstat -ano | findstr :5310
```

---

## 安装方式

### 方式 A: 下载预编译版本（推荐）

```powershell
# 1. 下载 Windows 版本
# 从 GitHub Releases 下载: https://github.com/kexin94yyds/iterate-releases/releases
# 文件: iterate-windows-x64.zip

# 2. 解压到安装目录
Expand-Archive iterate-windows-x64.zip -DestinationPath "$env:LOCALAPPDATA\寸止"

# 3. 添加到 PATH
$env:PATH += ";$env:LOCALAPPDATA\寸止\bin"
```

### 方式 B: 从源码构建

**前置条件**：
- Rust (https://rustup.rs/)
- Node.js + pnpm
- Visual Studio Build Tools
- Git

#### 详细环境配置步骤

##### 1. 安装 Git

```powershell
# 下载并安装 Git for Windows
# 访问: https://git-scm.com/download/win
# 或使用 winget:
winget install Git.Git

# 验证安装
git --version
# 应输出: git version 2.x.x
```

##### 2. 安装 Node.js

```powershell
# 方式 A: 使用 Node.js 官方安装包（推荐）
# 访问: https://nodejs.org/
# 下载 LTS 版本（如 20.x）

# 方式 B: 使用 winget
winget install OpenJS.NodeJS.LTS

# 验证安装
node --version
# 应输出: v20.x.x

npm --version
# 应输出: 10.x.x
```

##### 3. 安装 pnpm

```powershell
# 使用 npm 安装 pnpm
npm install -g pnpm

# 验证安装
pnpm --version
# 应输出: 8.x.x 或更高
```

##### 4. 安装 Rust

```powershell
# 下载并运行 rustup-init.exe
# 访问: https://rustup.rs/
# 或直接下载: https://win.rustup.rs/x86_64

# 运行安装程序后，选择默认安装（选项 1）
# 安装完成后，重启终端

# 验证安装
rustc --version
# 应输出: rustc 1.x.x

cargo --version
# 应输出: cargo 1.x.x
```

##### 5. 安装 Visual Studio Build Tools

```powershell
# 方式 A: 使用 Visual Studio Installer
# 访问: https://visualstudio.microsoft.com/downloads/
# 下载 "Build Tools for Visual Studio 2022"

# 安装时选择以下组件：
# - C++ build tools
# - Windows 10/11 SDK
# - MSVC v143 - VS 2022 C++ x64/x86 build tools

# 方式 B: 使用 winget
winget install Microsoft.VisualStudio.2022.BuildTools

# 验证安装（检查 cl.exe 是否可用）
# 打开 "x64 Native Tools Command Prompt for VS 2022"
cl
# 应显示 Microsoft C/C++ 编译器版本信息
```

##### 6. 安装 Tauri 依赖（WebView2）

```powershell
# WebView2 Runtime 通常已预装在 Windows 10/11
# 如果没有，下载安装：
# 访问: https://developer.microsoft.com/microsoft-edge/webview2/

# 验证 WebView2 是否已安装
Get-AppxPackage -Name Microsoft.WebView2Runtime
# 应显示包信息
```

##### 7. 克隆仓库并构建

```powershell
# 1. 克隆仓库
git clone https://github.com/kexin94yyds/iterate-desktop.git
cd iterate-desktop

# 2. 运行安装脚本
.\install-windows.ps1
```

---

## 环境变量配置

### PATH 环境变量详解

PATH 是 Windows 系统用于查找可执行文件的搜索路径列表。iterate 需要将其安装目录添加到 PATH 中。

#### 查看当前 PATH

```powershell
# 方式 1: 查看用户 PATH
[Environment]::GetEnvironmentVariable("PATH", "User")

# 方式 2: 查看系统 PATH
[Environment]::GetEnvironmentVariable("PATH", "Machine")

# 方式 3: 查看当前会话 PATH（用户 + 系统）
$env:PATH -split ';'
```

#### 添加 iterate 到 PATH

```powershell
# 方式 1: 永久添加到用户 PATH（推荐）
$iteratePath = "$env:LOCALAPPDATA\寸止\bin"
$currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($currentPath -notlike "*$iteratePath*") {
    [Environment]::SetEnvironmentVariable("PATH", "$currentPath;$iteratePath", "User")
    Write-Host "✅ 已添加到 PATH: $iteratePath" -ForegroundColor Green
    Write-Host "⚠️ 请重启终端使 PATH 生效" -ForegroundColor Yellow
}

# 方式 2: 临时添加到当前会话（仅本次有效）
$env:PATH += ";$env:LOCALAPPDATA\寸止\bin"

# 方式 3: 通过 GUI 添加
# 1. 右键"此电脑" → 属性 → 高级系统设置
# 2. 环境变量 → 用户变量 → PATH → 编辑
# 3. 新建 → 输入: C:\Users\username\AppData\Local\寸止\bin
# 4. 确定 → 重启终端
```

#### 验证 PATH 配置

```powershell
# 检查 iterate 是否在 PATH 中
where iterate
where cunzhi.exe
where 等一下.exe

# 如果找到，应显示完整路径：
# C:\Users\username\AppData\Local\寸止\bin\cunzhi.exe
```

### 重要环境变量

| 变量名 | 用途 | 示例值 | 作用域 |
|--------|------|--------|--------|
| `PATH` | 命令搜索路径 | `...;C:\Users\username\AppData\Local\寸止\bin` | 用户/系统 |
| `PYTHONIOENCODING` | Python 编码 | `utf-8` | 用户 |
| `USERPROFILE` | 用户主目录 | `C:\Users\username` | 系统 |
| `LOCALAPPDATA` | 本地应用数据 | `C:\Users\username\AppData\Local` | 系统 |
| `CARGO_HOME` | Rust Cargo 目录 | `C:\Users\username\.cargo` | 用户 |
| `RUSTUP_HOME` | Rust 工具链目录 | `C:\Users\username\.rustup` | 用户 |

### 设置 Python 编码环境变量

```powershell
# 永久设置（推荐）
[Environment]::SetEnvironmentVariable("PYTHONIOENCODING", "utf-8", "User")

# 临时设置（仅当前会话）
$env:PYTHONIOENCODING = "utf-8"

# 验证
python -c "import sys; print(sys.stdout.encoding)"
# 应输出: utf-8
```

### 验证所有环境变量

```powershell
# 一键检查脚本
Write-Host "=== 环境变量检查 ===" -ForegroundColor Cyan

# 检查 Python
$pythonPath = (Get-Command python -ErrorAction SilentlyContinue).Path
if ($pythonPath) {
    Write-Host "✅ Python: $pythonPath" -ForegroundColor Green
    python --version
} else {
    Write-Host "❌ Python 未找到" -ForegroundColor Red
}

# 检查 Git
$gitPath = (Get-Command git -ErrorAction SilentlyContinue).Path
if ($gitPath) {
    Write-Host "✅ Git: $gitPath" -ForegroundColor Green
} else {
    Write-Host "❌ Git 未找到" -ForegroundColor Red
}

# 检查 Node.js
$nodePath = (Get-Command node -ErrorAction SilentlyContinue).Path
if ($nodePath) {
    Write-Host "✅ Node.js: $nodePath" -ForegroundColor Green
    node --version
} else {
    Write-Host "⚠️ Node.js 未找到（仅源码构建需要）" -ForegroundColor Yellow
}

# 检查 Rust
$cargoPath = (Get-Command cargo -ErrorAction SilentlyContinue).Path
if ($cargoPath) {
    Write-Host "✅ Rust: $cargoPath" -ForegroundColor Green
    cargo --version
} else {
    Write-Host "⚠️ Rust 未找到（仅源码构建需要）" -ForegroundColor Yellow
}

# 检查 iterate
$iteratePath = (Get-Command cunzhi.exe -ErrorAction SilentlyContinue).Path
if ($iteratePath) {
    Write-Host "✅ iterate: $iteratePath" -ForegroundColor Green
} else {
    Write-Host "❌ iterate 未找到" -ForegroundColor Red
}

Write-Host "=== 检查完成 ===" -ForegroundColor Cyan
```

### 常见 PATH 问题

#### 问题 1: PATH 过长

Windows PATH 有长度限制（约 2048 字符）。如果超出，可能导致部分路径失效。

**解决方案**：
```powershell
# 查看 PATH 长度
$pathLength = [Environment]::GetEnvironmentVariable("PATH", "User").Length
Write-Host "当前 PATH 长度: $pathLength 字符"

# 如果超过 1900，建议清理无用路径
```

#### 问题 2: 重复路径

**解决方案**：
```powershell
# 去重 PATH
$paths = [Environment]::GetEnvironmentVariable("PATH", "User") -split ';' | Select-Object -Unique
$newPath = $paths -join ';'
[Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
```

#### 问题 3: 中文路径问题

Windows 支持中文路径（如 `寸止`），但某些工具可能不兼容。

**解决方案**：
- iterate 已支持中文路径
- 如果遇到问题，可以使用英文路径：`$env:LOCALAPPDATA\iterate\bin`

---

## 官方 VSCode 扩展配置（可选，单独分发）

### 1. 安装扩展

本 desktop-source 仓库不包含 VS Code / Windsurf 扩展。取得官方独立分发的 VSIX 后：

```powershell
# 方式 1: 命令行安装
code --install-extension "C:\path\to\iterate-official.vsix"

# 方式 2: VSCode 中安装
# 1. 打开 VSCode
# 2. Ctrl+Shift+P → "Extensions: Install from VSIX..."
# 3. 选择你取得的官方 VSIX 文件
```

### 2. 配置扩展设置

在 VSCode 设置 (`Ctrl+,`) 中搜索 "iterate"，配置以下选项：

```json
{
  "iterate.binaryPath": "C:\\Users\\username\\AppData\\Local\\寸止\\bin\\cunzhi.exe",
  "iterate.bridgeCommand": "iterate --bridge --port {PORT} --workspace \"{WORKSPACE}\"",
  "iterate.startPort": 5310,
  "iterate.autoStart": true,
  "iterate.port": 0
}
```

| 配置项 | 说明 | 默认值 |
|--------|------|--------|
| `iterate.bridgeCommand` | VSCode 调用 bridge 的命令模板，可用 `{PORT}` / `{WORKSPACE}` 占位符 | `iterate --bridge --port {PORT} --workspace "{WORKSPACE}"` |
| `iterate.binaryPath` | 主程序路径 | `%LOCALAPPDATA%\寸止\bin\cunzhi.exe` |
| `iterate.startPort` | 起始端口 | `5310` |
| `iterate.autoStart` | 自动启动服务 | `true` |
| `iterate.port` | 固定端口（0=自动分配） | `0` |

**注意**：`username` 是占位符，请替换为实际的 Windows 用户名

### 3. 复制必要文件

```powershell
# 确保安装目录存在
New-Item -ItemType Directory -Path "$env:LOCALAPPDATA\寸止\bin" -Force

# 复制 Python 脚本
Copy-Item "bin\cunzhi.py" "$env:LOCALAPPDATA\寸止\bin\cunzhi.py"

# 如果有 cunzhi.exe，也复制
Copy-Item "target\release\cunzhi.exe" "$env:LOCALAPPDATA\寸止\bin\cunzhi.exe"
```

### 4. 扩展功能说明

#### 控制面板（最新版）

扩展会在 VSCode 侧边栏显示 **iterate 控制面板**，包含：

**状态显示**：
- 🟢 **运行中** / 🔴 **未启动**
- ∞ **端口号**（如 5312）

**功能按钮**：
1. **📋 复制开头语** - 复制 AI 对话开头语到剪贴板
2. **▶️ 启动服务** - 启动 iterate 服务
3. **⏹️ 停止服务** - 停止当前服务
4. **🛑 停止所有服务** - 停止所有 iterate 进程（清理端口）

#### 命令面板

| 功能 | 命令 | 说明 |
|------|------|------|
| 复制开头语 | `iterate: 复制开头语` | 复制 AI 交互开头语 |
| 启动服务 | `iterate: 启动服务` | 手动启动服务 |
| 停止服务 | `iterate: 停止服务` | 停止当前服务 |

#### 自动功能

- ✅ **自动启动**：VSCode 启动时自动启动 iterate 服务（可配置）
- ✅ **健康检查**：每 5 秒检查服务状态，自动更新面板
- ✅ **端口管理**：自动查找可用端口，避免冲突
- ✅ **规则生成**：自动生成 `.windsurfrules` 文件

---

## Windows 启动端口问题排查

### 问题现象

- VSCode 扩展显示"未启动"
- 点击"启动服务"无反应或报错
- 端口号显示为 `-` 或 `0`

### 排查步骤

#### 1. 检查 cunzhi.exe 是否存在

```powershell
# 检查文件是否存在
Test-Path "$env:LOCALAPPDATA\寸止\bin\cunzhi.exe"
# 应返回: True

# 如果返回 False，需要重新安装
```

#### 2. 检查端口是否被占用

```powershell
# 检查端口 5310 是否被占用
netstat -ano | findstr :5310

# 如果有输出，说明端口被占用，查看进程 ID
# 然后杀掉进程：
taskkill /PID <进程ID> /F
```

#### 3. 手动测试启动

```powershell
# 手动启动服务测试
& "$env:LOCALAPPDATA\寸止\bin\cunzhi.exe" --serve --port 5310

# 应该看到类似输出：
# Server started on http://127.0.0.1:5310
```

#### 4. 检查防火墙

```powershell
# 检查防火墙规则
Get-NetFirewallRule -DisplayName "iterate"

# 如果没有规则，添加：
New-NetFirewallRule -DisplayName "iterate" -Direction Inbound -LocalPort 5310-5400 -Protocol TCP -Action Allow
```

#### 5. 检查 WebView2 Runtime

```powershell
# 检查 WebView2 是否安装
Get-AppxPackage -Name Microsoft.WebView2Runtime

# 如果未安装，下载安装：
# https://developer.microsoft.com/microsoft-edge/webview2/
```

#### 6. 查看 VSCode 开发者工具日志

1. 打开 VSCode
2. `Help` → `Toggle Developer Tools`
3. 切换到 `Console` 标签
4. 查找 `iterate` 相关错误信息

### 常见错误及解决方案

#### 错误 1: `ENOENT: no such file or directory`

**原因**：cunzhi.exe 路径配置错误

**解决方案**：
```json
// 在 VSCode 设置中修正路径
{
  "iterate.binaryPath": "C:\\Users\\username\\AppData\\Local\\寸止\\bin\\cunzhi.exe"
}
```

#### 错误 2: `spawn EACCES`

**原因**：权限不足或文件被杀毒软件拦截

**解决方案**：
```powershell
# 添加杀毒软件排除项
Add-MpPreference -ExclusionPath "$env:LOCALAPPDATA\寸止"
```

#### 错误 3: `Port already in use`

**原因**：端口被占用

**解决方案**：
```powershell
# 方式 1: 杀掉占用端口的进程
netstat -ano | findstr :5310
taskkill /PID <进程ID> /F

# 方式 2: 更改起始端口
# 在 VSCode 设置中：
{
  "iterate.startPort": 5320
}
```

#### 错误 4: `服务启动超时`

**原因**：服务启动慢或被防火墙阻止

**解决方案**：
```powershell
# 1. 检查防火墙
Get-NetFirewallRule -DisplayName "iterate"

# 2. 手动启动测试
& "$env:LOCALAPPDATA\寸止\bin\cunzhi.exe" --serve --port 5310

# 3. 等待 10 秒后检查健康
curl http://127.0.0.1:5310/health
```

### 完整诊断脚本

```powershell
# 保存为 diagnose-iterate.ps1
Write-Host "=== iterate Windows 诊断脚本 ===" -ForegroundColor Cyan

# 1. 检查 cunzhi.exe
$exePath = "$env:LOCALAPPDATA\寸止\bin\cunzhi.exe"
if (Test-Path $exePath) {
    Write-Host "✅ cunzhi.exe 存在: $exePath" -ForegroundColor Green
} else {
    Write-Host "❌ cunzhi.exe 不存在: $exePath" -ForegroundColor Red
    Write-Host "   请运行安装脚本或从 GitHub Releases 下载" -ForegroundColor Yellow
    exit 1
}

# 2. 检查端口占用
$port = 5310
$portInUse = netstat -ano | findstr ":$port"
if ($portInUse) {
    Write-Host "⚠️ 端口 $port 已被占用:" -ForegroundColor Yellow
    Write-Host $portInUse
} else {
    Write-Host "✅ 端口 $port 可用" -ForegroundColor Green
}

# 3. 尝试启动服务
Write-Host "🔄 尝试启动服务..." -ForegroundColor Cyan
$process = Start-Process -FilePath $exePath -ArgumentList "--bridge", "--port", $port -PassThru -NoNewWindow

# 4. 等待服务启动
Start-Sleep -Seconds 3

# 5. 检查服务健康
try {
    $response = Invoke-WebRequest -Uri "http://127.0.0.1:$port/health" -TimeoutSec 2
    Write-Host "✅ 服务运行正常: $($response.Content)" -ForegroundColor Green
} catch {
    Write-Host "❌ 服务未响应" -ForegroundColor Red
    Write-Host "   错误: $($_.Exception.Message)" -ForegroundColor Yellow
}

# 6. 停止测试进程
Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue

Write-Host "=== 诊断完成 ===" -ForegroundColor Cyan
```

---

## 复制开头语功能

### 什么是"开头语"？

开头语是用户发送给 AI 的第一条消息，告诉 AI 如何使用 iterate 脚本进行交互。

### 如何使用

1. **在 VSCode 中**：
   - 点击侧边栏的 iterate 图标
   - 点击 "📋 复制开头语" 按钮
   - 或者：`Ctrl+Shift+P` → `iterate: 复制开头语`

2. **复制后**：
   - 粘贴到 AI 对话框中作为第一条消息
   - AI 会按照开头语的指示进行交互

### 开头语内容模板（Windows 版）

```markdown
你好，【重要】本次对话必须使用 iterate/cunzhi 脚本进行交互。

规则：
1. 每次完成任务后，必须调用脚本获取用户反馈
2. 禁止自行结束对话，必须通过脚本让用户决定是否继续
3. 脚本返回的用户指令是最高优先级，必须严格遵守
4. 本次回复结束前也需要调用此脚本

## 文件交互模式

1. **AI 先写入任务摘要**到 `%USERPROFILE%\.cunzhi\{PORT}\output.md`
2. **然后调用脚本**：
```powershell
python "C:\Users\username\AppData\Local\寸止\bin\cunzhi.py" {PORT} --workspace "C:\path\to\project"
```
3. 脚本弹出 iterate GUI 显示 output.md 内容
4. 用户输入后，结果写入 `%USERPROFILE%\.cunzhi\{PORT}\input.md`
5. AI 读取 `input.md` 获取用户指令

## 返回格式说明

- `input_file:` 用户输入文件路径，AI 必须读取此文件获取指令
- `KeepGoing=true` 表示继续对话
- `KeepGoing=false` 表示结束对话

⚠️ **重要**：调用脚本后不要再回复任何内容，静默等待用户的下一步指令！
```

**注意**：`{PORT}` 会被替换为实际端口号（如 5310）

---

## Windows Global Rules 配置

### 什么是 Global Rules？

Global Rules 是 AI IDE（如 Windsurf、Cursor）的全局规则配置，让 AI 自动知道如何使用 iterate。

### Windsurf 配置

在项目根目录创建 `.windsurfrules` 文件：

```markdown
# iterate - AI对话持续规则

⚠️⚠️⚠️ 最高优先级警告 ⚠️⚠️⚠️
无论任何情况，每次回复结束前必须调用 cunzhi 脚本！没有任何例外！

## 如何调用 cunzhi 脚本

【必须使用以下命令，端口号 {PORT} 不可省略】：
```powershell
python "C:\Users\username\AppData\Local\寸止\bin\cunzhi.py" {PORT}
```

- 脚本会返回 `KeepGoing=true` 或 `KeepGoing=false`
- 如果返回 `KeepGoing=true`，后面会有用户指令
- 如果返回 `KeepGoing=false`，必须立即结束对话

⚠️ **重要**：调用脚本后不要再回复任何内容，静默等待用户的下一步指令！

## 文件交互模式

1. AI 将任务摘要写入 `%USERPROFILE%\.cunzhi\{PORT}\output.md`
2. AI 调用脚本: `python cunzhi.py {PORT}`
3. 脚本弹出 iterate GUI 显示内容
4. 用户输入后，结果写入 `%USERPROFILE%\.cunzhi\{PORT}\input.md`
5. AI 读取 `input.md` 获取用户指令

## 返回格式说明

- `input_file:` 用户输入文件路径，AI 必须读取此文件获取指令
- `KeepGoing=true` 表示继续对话
- `KeepGoing=false` 表示结束对话
```

### Cursor 配置

在项目根目录创建 `.cursorrules` 文件，内容与 `.windsurfrules` 相同。

### Claude Desktop 配置

Claude Desktop 需要使用 MCP（Model Context Protocol）方式：

```json
{
  "mcpServers": {
    "寸止": {
      "command": "C:\\Users\\username\\AppData\\Local\\寸止\\bin\\cunzhi.exe"
    }
  }
}
```

配置文件位置：`%APPDATA%\Claude\claude_desktop_config.json`

### 自动生成 Rules

VSCode 扩展会在启动服务时**自动生成** `.windsurfrules` 文件，无需手动创建。

---

## 手动启动服务（调试用）

如果扩展无法自动启动服务，可以手动启动：

```powershell
# 方式 1: 使用 exe
& "$env:LOCALAPPDATA\寸止\bin\cunzhi.exe" --serve --port 5310

# 方式 2: 使用 iterate 命令（如果在 PATH 中）
iterate --serve --port 5310
```

---

## AI 调用脚本格式

Windows 上 AI 应该这样调用：

```powershell
# 1. 写入任务摘要
$content = @"
## 任务摘要
...
"@
$content | Out-File -Encoding utf8 "$env:USERPROFILE\.cunzhi\5310\output.md"

# 2. 调用脚本
python "$env:LOCALAPPDATA\寸止\bin\cunzhi.py" 5310 --workspace "C:\path\to\project"

# 3. 读取用户输入
Get-Content "$env:USERPROFILE\.cunzhi\5310\input.md"
```

或者使用 bash/WSL 风格：

```bash
# 写入
cat > ~/.cunzhi/5310/output.md <<'MD'
## 任务摘要
...
MD

# 调用
python "${LOCALAPPDATA}/寸止/bin/cunzhi.py" 5310

# 读取
cat ~/.cunzhi/5310/input.md
```

---

## 完整端到端示例

> 从零开始配置并使用 iterate 的完整流程

### 场景：首次在 Windows 上使用 iterate + Windsurf

```powershell
# ========== 第一步：安装前置条件 ==========

# 1.1 检查 Python
python --version  # 需要 3.8+

# 如果没有 Python，安装它：
# 从 https://www.python.org/downloads/ 下载并安装
# 安装时勾选 "Add Python to PATH"

# 1.2 检查 Git
git --version

# 如果没有 Git，安装它：
# 从 https://git-scm.com/download/win 下载并安装

# ========== 第二步：安装 iterate ==========

# 2.1 克隆仓库
cd "$env:USERPROFILE\Projects"
git clone https://github.com/kexin94yyds/iterate-desktop.git
cd iterate-desktop

# 2.2 安装（选择一种方式）
# 方式 A: 下载预编译版本
# 从 https://github.com/kexin94yyds/iterate-releases/releases 下载
# 解压到 $env:LOCALAPPDATA\寸止

# 方式 B: 从源码构建
.\install-windows.ps1

# 2.3 验证安装
& "$env:LOCALAPPDATA\寸止\bin\cunzhi.exe" --version

# ========== 第三步（可选）：安装官方 VSCode 扩展 ==========

# 本 desktop-source 仓库不提供扩展。取得官方 VSIX 后安装：
code --install-extension "C:\path\to\iterate-official.vsix"

# 3.2 重启 VSCode

# ========== 第四步：配置 VSCode 扩展 ==========

# 在 VSCode 设置中添加：
# {
#   "iterate.cunzhiPath": "C:\\Users\\username\\AppData\\Local\\寸止\\bin\\cunzhi.py",
#   "iterate.binaryPath": "C:\\Users\\username\\AppData\\Local\\寸止\\bin\\cunzhi.exe",
#   "iterate.startPort": 5310,
#   "iterate.autoStart": true
# }

# ========== 第五步：开始使用 ==========

# 5.1 打开 VSCode 侧边栏的 iterate 面板
# 5.2 确认状态显示 "运行中" 和端口号（如 5310）
# 5.3 点击 "📋 复制开头语"
# 5.4 打开 AI 对话框（Windsurf/Cursor），粘贴开头语
# 5.5 开始对话，AI 会自动调用 iterate 脚本与你交互
```

### 验证流程是否正常

```powershell
# 1. 检查服务是否运行
curl http://127.0.0.1:5310/health
# 应返回: {"status":"ok","service":"cunzhi","port":5310}

# 2. 手动测试脚本
python "$env:LOCALAPPDATA\寸止\bin\cunzhi.py" 5310 --message "测试消息"
# 应弹出 GUI，输入后返回 KeepGoing=true 或 false

# 3. 检查端口文件
ls "$env:USERPROFILE\.cunzhi_ports"
# 应显示端口号文件，如 5310
```

---

## 故障排除检查清单

> 按顺序检查以下项目

### 检查清单

| # | 检查项 | 命令 | 预期结果 |
|---|--------|------|----------|
| 1 | Python 版本 | `python --version` | Python 3.8+ |
| 2 | cunzhi.exe 存在 | `Test-Path "$env:LOCALAPPDATA\寸止\bin\cunzhi.exe"` | True |
| 3 | cunzhi.py 存在 | `Test-Path "$env:LOCALAPPDATA\寸止\bin\cunzhi.py"` | True |
| 4 | PATH 配置 | `$env:PATH -like "*寸止*"` | True |
| 5 | 端口未被占用 | `netstat -ano \| findstr :5310` | 无输出或仅有 iterate 进程 |
| 6 | 服务健康 | `curl http://127.0.0.1:5310/health` | 返回 JSON |
| 7 | 端口文件存在 | `Test-Path "$env:USERPROFILE\.cunzhi_ports\5310"` | True |
| 8 | 数据目录存在 | `Test-Path "$env:USERPROFILE\.cunzhi\5310"` | True |

### 一键检查脚本

```powershell
# 保存为 check-iterate.ps1 并运行
Write-Host "=== iterate 健康检查 ===" -ForegroundColor Cyan

# 检查 Python
$python = python --version 2>&1
if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ Python: $python" -ForegroundColor Green
} else {
    Write-Host "❌ Python 未安装" -ForegroundColor Red
}

# 检查 cunzhi.exe
$exePath = "$env:LOCALAPPDATA\寸止\bin\cunzhi.exe"
if (Test-Path $exePath) {
    Write-Host "✅ cunzhi.exe 存在" -ForegroundColor Green
} else {
    Write-Host "❌ cunzhi.exe 不存在: $exePath" -ForegroundColor Red
}

# 检查 cunzhi.py
$pyPath = "$env:LOCALAPPDATA\寸止\bin\cunzhi.py"
if (Test-Path $pyPath) {
    Write-Host "✅ cunzhi.py 存在" -ForegroundColor Green
} else {
    Write-Host "❌ cunzhi.py 不存在: $pyPath" -ForegroundColor Red
}

# 检查服务
try {
    $response = Invoke-WebRequest -Uri "http://127.0.0.1:5310/health" -TimeoutSec 2
    Write-Host "✅ 服务运行中: $($response.Content)" -ForegroundColor Green
} catch {
    Write-Host "❌ 服务未运行或端口 5310 不可用" -ForegroundColor Red
}

Write-Host "=== 检查完成 ===" -ForegroundColor Cyan
```

---

## 防火墙与杀毒软件

### Windows Defender 防火墙

```powershell
# 添加防火墙规则（管理员权限）
New-NetFirewallRule -DisplayName "iterate" -Direction Inbound -LocalPort 5310-5400 -Protocol TCP -Action Allow

# 检查规则是否存在
Get-NetFirewallRule -DisplayName "iterate"

# 如果需要删除规则
Remove-NetFirewallRule -DisplayName "iterate"
```

### Windows Defender 杀毒软件

如果 cunzhi.exe 被误报为病毒：

```powershell
# 添加排除项（管理员权限）
Add-MpPreference -ExclusionPath "$env:LOCALAPPDATA\寸止"

# 查看当前排除项
Get-MpPreference | Select-Object -ExpandProperty ExclusionPath
```

### 企业环境特殊处理

如果在企业环境中：

1. **企业防火墙**：联系 IT 部门开放本地端口 5310-5400
2. **企业杀毒软件**：将 `%LOCALAPPDATA%\寸止` 添加到白名单
3. **组策略限制**：可能需要管理员权限安装

---

## 常见错误代码

### 错误: `KeepGoing=false` + `Port not available`

**原因**：服务未启动或端口被占用

**解决方案**：
```powershell
# 1. 检查端口占用
netstat -ano | findstr :5310

# 2. 如果被其他进程占用，杀掉它
taskkill /PID <进程ID> /F

# 3. 启动服务
& "$env:LOCALAPPDATA\寸止\bin\cunzhi.exe" --serve --port 5310
```

### 错误: `Cannot find running cunzhi server`

**原因**：服务未启动，且端口文件不存在

**解决方案**：
```powershell
# 启动服务
& "$env:LOCALAPPDATA\寸止\bin\cunzhi.exe" --serve --port 5310

# 或通过 VSCode 扩展启动
# Ctrl+Shift+P → "iterate: 启动服务"
```

### 错误: `Python 脚本无法执行`

**原因**：Python 未安装或路径问题

**解决方案**：
```powershell
# 检查 Python
python --version

# 如果找不到，手动指定路径
& "C:\Python310\python.exe" "$env:LOCALAPPDATA\寸止\bin\cunzhi.py" 5310
```

### 错误: `UnicodeDecodeError` 或 `编码错误`

**原因**：Windows 终端编码问题

**解决方案**：
```powershell
# 设置终端编码为 UTF-8
chcp 65001

# 设置环境变量
$env:PYTHONIOENCODING = "utf-8"

# 然后重新运行脚本
python "$env:LOCALAPPDATA\寸止\bin\cunzhi.py" 5310
```

### 错误: `GUI 无法显示`

**原因**：Tauri/WebView2 依赖问题

**解决方案**：
```powershell
# 1. 安装 WebView2 Runtime
# 从 https://developer.microsoft.com/microsoft-edge/webview2/ 下载

# 2. 检查 Windows 版本（需要 Windows 10 1809+）
winver

# 3. 更新 Windows 到最新版本
```

### 错误: `ECONNREFUSED` 或 `连接被拒绝`

**原因**：防火墙阻止或服务未监听

**解决方案**：
```powershell
# 1. 检查防火墙
Get-NetFirewallRule -DisplayName "iterate"

# 2. 检查服务是否监听
netstat -ano | findstr :5310

# 3. 添加防火墙规则
New-NetFirewallRule -DisplayName "iterate" -Direction Inbound -LocalPort 5310 -Protocol TCP -Action Allow
```

---

## 常见问题

### Q: 端口 5310 被占用
**A**: 更改端口号，使用 5311、5312 等。在 VSCode 设置中修改 `iterate.startPort`。

### Q: GUI 无法弹出
**A**: 
1. 检查 `cunzhi.exe` 是否存在
2. 检查 Windows Defender 是否拦截
3. 安装 WebView2 Runtime
4. 确保 Windows 版本 ≥ 10 1809

### Q: Python 脚本报 UTF-8 错误
**A**: 
1. 运行 `chcp 65001` 设置终端编码
2. 确保使用 Python 3.8+
3. 设置 `$env:PYTHONIOENCODING = "utf-8"`

### Q: 找不到 iterate 命令
**A**: 
1. 检查 PATH 环境变量
2. 使用完整路径: `& "$env:LOCALAPPDATA\寸止\bin\cunzhi.exe"`
3. 重启终端使 PATH 生效

### Q: VSCode 扩展无法启动服务
**A**:
1. 检查 `iterate.binaryPath` 配置是否正确
2. 手动运行 `cunzhi.exe --serve --port 5310` 看错误信息
3. 查看 VSCode 开发者工具 (Help → Toggle Developer Tools)

### Q: 多个 AI IDE 同时使用
**A**: 
- 每个 IDE 使用不同端口：Windsurf 用 5310，Cursor 用 5311
- 在各自的 `.windsurfrules` / `.cursorrules` 中指定不同端口

### Q: WSL 中使用
**A**:
- WSL 和 Windows 端口不互通
- 建议在 Windows 原生环境使用
- 或在 WSL 中单独安装 Linux 版本

---

## 文件结构（Windows）

```
%LOCALAPPDATA%\寸止\
├── bin\
│   ├── cunzhi.exe      # 主程序（Tauri 构建）
│   ├── cunzhi.py       # Python 客户端脚本
│   ├── 等一下.exe      # cunzhi.exe 的别名
│   └── 寸止.exe        # cunzhi.exe 的别名
└── ...

%USERPROFILE%\.cunzhi\
├── 5310\               # 端口数据目录
│   ├── output.md       # AI 写入的任务摘要
│   └── input.md        # 用户输入
└── ...

%USERPROFILE%\.cunzhi_ports\
└── 5310                # 端口注册文件
```
