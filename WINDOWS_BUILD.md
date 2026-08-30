# Windows 本地编译指南

> 历史说明：本文件偏向源码构建与早期 Windows 接入记录，不是当前普通用户的主安装入口。
>
> 当前对外交付请优先使用 [docs/INSTALLATION.md](docs/INSTALLATION.md) 和安装包中的 `INSTALLATION.md`。

在 Windows 上编译 iterate 应用。

## 1. 安装依赖

### 1.1 安装 Rust
```powershell
# 方式一：使用 winget
winget install Rustlang.Rustup

# 方式二：从官网下载安装器
# https://www.rust-lang.org/tools/install
# 下载 rustup-init.exe 并运行

# 重启终端后验证
rustc --version
cargo --version
```

### 1.2 安装 Node.js
```powershell
# 方式一：使用 winget
winget install OpenJS.NodeJS.LTS

# 方式二：从官网下载
# https://nodejs.org/
# 下载 LTS 版本安装

# 验证
node --version
npm --version
```

### 1.3 安装 pnpm
```powershell
npm install -g pnpm
# 验证
pnpm --version
```

## 2. 克隆项目

```powershell
git clone https://github.com/kexin94yyds/iterate.git
cd iterate
```

## 3. 构建前端

**重要：必须先构建前端，否则 Tauri 编译会失败！**

```powershell
# 步骤 1：安装前端依赖
pnpm install

# 步骤 2：构建前端（生成 dist 目录）
pnpm build
```

构建成功后，会生成 `dist/` 目录，包含前端静态文件。

### 验证前端构建
```powershell
# 检查 dist 目录是否存在
dir dist
```

如果看到 `index.html` 和其他文件，说明前端构建成功。

## 4. 编译 Rust 后端

```powershell
# 步骤 1：安装 Tauri CLI
cargo install tauri-cli --version "^2.0" --locked

# 步骤 2：编译（不打包）
cargo tauri build --no-bundle
```

编译时间约 10-15 分钟。

## 5. 运行

编译完成后，二进制文件在：
```
target\release\iterate.exe    # 主应用
target\release\mcp-server.exe # MCP server
```

### 启动 HTTP 服务器
```powershell
.\target\release\iterate.exe --serve --port 5311
```

### 配合 MCP Server 使用
参考 [MCP_INSTALL.md](MCP_INSTALL.md)

## 常见问题

### OpenSSL 错误
如果遇到 OpenSSL 相关错误，项目已配置使用 `native-tls`，应该不会出现此问题。

### 编译很慢
Windows 编译确实比 macOS/Linux 慢，首次编译约 15-20 分钟，后续增量编译会快很多。

### 中文文件名问题
确保终端编码为 UTF-8：
```powershell
chcp 65001
```
