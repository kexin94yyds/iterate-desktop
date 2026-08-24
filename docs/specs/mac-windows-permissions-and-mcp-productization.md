# Mac / Windows 权限与 MCP 产品化方案

## 背景

当前 iterate 的实际使用门槛不只在 MCP 配置，还在：

- 用户是否装对主程序
- IDE 是否写对 MCP command / args
- 本地权限是否完整
- 平台运行时依赖是否齐全

如果继续让用户理解多个二进制、手改配置、自己排查权限，成功率会很低。

目标应该是：

1. 用户只安装一个主程序。
2. 主程序内置 MCP 入口。
3. 首次启动自动做权限与依赖检查。
4. 一键生成或安装推荐的 Tier 4 MCP 配置。

## 历史经验结论

### 1. MCP 应尽量内置到主程序

过去已有方案建议把：

- `iterate.exe / iterate.app`
- `mcp-server.exe`
- `寸止.exe`

收敛成单一产品入口，例如：

```bash
iterate --mcp 5311
```

这样用户不需要区分“GUI 主程序”和“单独 MCP server”。

### 2. macOS 真正难点是 TCC

历史问题反复说明：

- 辅助功能、输入监控、屏幕录制等权限都受 TCC 管理
- 权限绑定代码签名
- 如果频繁替换 `.app/Contents` 或使用不稳定签名，权限会掉

所以 macOS 产品化不能只写文档，必须把“权限诊断 + 引导”做进 app。

### 3. Windows 真正难点是依赖和安全放行

Windows 一般不会像 macOS 那样要求用户手动开一串 TCC 权限，但会卡在：

- Defender / SmartScreen
- 防火墙
- WebView2 Runtime
- 开机自启 / 后台常驻
- 全局热键冲突

本质上这是 Windows 的“隐形权限层”。

## 产品目标

### 用户视角

- 安装 iterate
- 打开 iterate
- 跟着权限向导完成授权
- 点击“一键安装到 Windsurf / Cursor / Cline”
- 直接开始用

### 工程视角

- 主程序内置 MCP 能力
- 平台权限状态可检测
- 推荐 MCP 配置可生成
- 缺失项可视化提示

## 总体方案

## 1. 单一主程序分发

### macOS

- 分发 `iterate.app`
- app 内置：
  - GUI
  - `--serve`
  - `--mcp`
  - 权限诊断页
  - MCP 配置导出/安装页

### Windows

- 分发 `iterate.exe` 或 MSI 安装包
- 主程序同样内置：
  - GUI
  - `--serve`
  - `--mcp`
  - 依赖检查页
  - MCP 配置导出/安装页

## 2. 首次启动向导

首次启动不要直接把用户扔进主界面，而是跑一个 onboarding：

1. 环境检查
2. 权限检查
3. IDE 选择
4. MCP 配置安装
5. 测试连通性

## 平台权限清单

## macOS

### 必需权限

#### 1. 辅助功能（Accessibility）

适用功能：

- 全局快捷键
- 模拟按键
- 激活/控制前台应用
- 粘贴注入

引导方式：

- 应用内明确说明用途
- 一键跳转到系统设置对应页
- 返回后重新检测

#### 2. 输入监控（Input Monitoring）

适用功能：

- 监听全局按键
- 快捷键捕获

历史经验表明：

- 只检查辅助功能不够
- 全局键盘监听往往需要 Accessibility + Input Monitoring 双重检查

#### 3. 屏幕录制（Screen Recording）

适用功能：

- 截图
- 读屏
- 视觉识别
- 采集屏幕内容

建议：

- 不要默认索取
- 只在用户触发截图/视觉功能时再申请

#### 4. 通知（Notifications）

适用功能：

- 系统提醒
- 状态通知

建议：

- 作为二级权限处理
- 非核心流程不阻塞产品主链路

#### 5. Automation

适用功能：

- AppleScript 控制其他 app
- 深度跨应用协作

建议：

- 只有真的需要时才申请
- 不要在第一屏统一要求

### macOS 产品要求

#### 稳定签名

必须保证：

- 稳定 bundle identifier
- 稳定签名身份
- 最好 notarization

否则每次更新后，TCC 权限可能重置，用户要重复授权。

#### 深链跳转

应用内应支持一键打开系统设置目标页面：

- Accessibility
- Input Monitoring
- Screen Recording
- Notifications

#### 权限状态检测

应用内需有明确状态：

- 已授权
- 未授权
- 需要重启应用后生效

## Windows

### 核心检查项

#### 1. WebView2 Runtime

适用原因：

- 桌面壳 / 内嵌 Web UI 依赖

策略：

- 首次启动自动检测
- 缺失时提供官方下载或静默安装引导

#### 2. Defender / SmartScreen 放行

适用原因：

- 首次启动可执行文件常被拦截

策略：

- 安装文档明确提示
- 应用内提供“如果启动被拦截，请这样操作”的引导图

#### 3. 防火墙

适用原因：

- 本地 localhost 服务
- IDE 到本地服务通信

策略：

- 明确说明 iterate 仅使用本机回环
- 必要时引导用户允许防火墙放行

#### 4. 自启动 / 后台常驻

适用功能：

- 剪贴板监听
- 全局快捷键
- 后台服务

策略：

- 产品内提供“一键开机启动”
- 启动失败时有明确提示

#### 5. 热键冲突检查

适用原因：

- QQ / 微信 / 浏览器 / 输入法 / 系统快捷键经常抢占

策略：

- 启动时检测注册失败
- 提供默认备用快捷键

### Windows 产品要求

#### 避免把“管理员权限”当默认要求

原则：

- 只有确实需要系统级写入或服务安装时才申请 admin
- 普通 MCP / 本地服务 / GUI 产品链路尽量不要求管理员

#### 安装器能力

建议 MSI 或安装器支持：

- 注册开机启动
- 检查/安装 WebView2
- 写入本地配置模板
- 生成桌面快捷方式

## MCP 产品化方案

## 1. MCP 内置，不让用户理解多个二进制

理想状态：

```bash
iterate --mcp 5311
iterate --serve --port 5311
```

IDE 看到的只是一个 command：

- macOS: `/Applications/iterate.app/Contents/MacOS/iterate`
- Windows: `C:\\Program Files\\iterate\\iterate.exe`

不再让用户区分：

- GUI 主程序
- 独立 `mcp-server`
- 备用 `寸止.exe`

## 2. Tier 4 配置不要只是文档

推荐把 Tier 4 配置做成产品里的功能：

### 方式 A：一键写入

应用内按钮：

- 安装到 Windsurf
- 安装到 Cursor
- 安装到 Cline

点击后自动写入对应 MCP 配置文件。

### 方式 B：一键复制

如果用户不愿意自动写文件：

- 应用展示平台对应 JSON/TOML 模板
- 一键复制
- 一键打开配置目录

### 方式 C：导出配置包

导出一个带说明的配置文件，方便企业/团队分发。

## 3. 首次连通性测试

在用户完成配置后，app 内做一次自测：

1. 本地服务是否已启动
2. MCP 命令是否能正常握手
3. 是否能弹出 iterate 窗口

如果失败，给出明确原因：

- 权限缺失
- 运行时缺失
- 配置路径错误
- 防火墙/安全软件阻断

## 推荐的用户体验

## macOS 用户路径

1. 安装 `iterate.app`
2. 首次打开
3. 向导检测：
   - Accessibility
   - Input Monitoring
   - Screen Recording（按需）
4. 点击“一键安装 MCP 到 Windsurf/Cursor/Cline”
5. 应用执行测试握手
6. 成功后显示“你可以开始用了”

## Windows 用户路径

1. 安装 `iterate.exe` / MSI
2. 首次打开
3. 向导检测：
   - WebView2
   - Defender/SmartScreen
   - 防火墙
   - 后台常驻/自启动
   - 热键冲突
4. 点击“一键安装 MCP”
5. 执行测试握手
6. 成功后进入主界面

## 优先级建议

### Phase 1: 文档 + 配置模板

- 输出 Mac / Windows 权限清单
- 输出 Windsurf / Cursor / Cline 推荐配置
- 应用内提供复制模板按钮

### Phase 2: 主程序内置 `--mcp`

- 统一入口
- 降低用户认知负担

### Phase 3: 权限向导

- 平台诊断
- 深链跳转
- 状态回读

### Phase 4: 一键安装 MCP 配置

- 自动写入主流 IDE 配置
- 自动做连通性验证

## 结论

最佳实践不是“让用户下载 MCP 再照文档配”，而是：

1. **下载 iterate 主程序**
2. **主程序内置 MCP**
3. **主程序负责权限向导**
4. **主程序负责导出/安装 Tier 4 配置**

这样才是真正的产品化路径。
