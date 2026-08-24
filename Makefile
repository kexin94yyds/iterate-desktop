# iterate (cunzhi) - 自动化 Makefile
# 用法: make <target>

SHELL := /bin/zsh
APP_NAME := iterate
BUNDLE_ID := com.kexin94yyds.iterate
PROJECT_DIR := $(shell pwd)

# ============================================================
# 开发
# ============================================================

.PHONY: dev
dev: ## 启动 Tauri 开发模式
	pnpm tauri:dev

.PHONY: dev-web
dev-web: ## 仅启动前端 Vite dev server
	pnpm dev

.PHONY: lint
lint: ## ESLint 修复
	pnpm lint

# ============================================================
# 构建 & 发布
# ============================================================

.PHONY: build
build: ## 开发构建 / 整包验证（不安装到 /Applications）
	pnpm tauri:build

.PHONY: build-frontend
build-frontend: ## 仅构建前端
	pnpm build

.PHONY: build-debug
build-debug: ## 构建 Tauri 应用 (debug)
	cargo tauri build --debug

.PHONY: build-mcp
build-mcp: ## 构建 MCP server 二进制
	cargo build --release --bin mcp-server

.PHONY: build-iterate-bin
build-iterate-bin: ## 构建 iterate CLI 二进制
	cargo build --release --bin iterate

.PHONY: local-delivery
local-delivery: ## 构建本地 signed delivery 产物
	pnpm delivery:macos

.PHONY: notarize-local
notarize-local: ## 如本地具备凭据则公证本地 delivery 产物
	@if xcrun notarytool history --keychain-profile "$${CUNZHI_NOTARY_PROFILE:-cunzhi-notary}" --output-format json --no-progress >/dev/null 2>&1; then \
		echo "🪪 检测到 notarytool keychain profile，执行本地公证..."; \
		pnpm notarize:macos; \
	elif [[ -n "$${APPLE_ID:-}" && -n "$${APPLE_APP_SPECIFIC_PASSWORD:-}" && -n "$${APPLE_TEAM_ID:-$${CUNZHI_NOTARY_TEAM_ID:-}}" ]]; then \
		echo "🪪 检测到 APPLE_ID / app-specific password 凭据，执行本地公证..."; \
		pnpm notarize:macos; \
	else \
		echo "ℹ️ 未检测到本地 notarization 凭据，跳过公证，继续使用已签名 delivery 产物。"; \
	fi

# ============================================================
# 安装 & 更新 (本地)
# ============================================================

.PHONY: install
install: local-delivery notarize-local ## 本地正式安装（signed delivery 路径）
	@echo "🔍 查找 signed delivery 产物..."
	@INSTALL_SCRIPT="./scripts/install-macos-dev-app.sh"; \
	DMG=$$(find target/release/delivery/macos -maxdepth 1 -name "*.dmg" 2>/dev/null | head -1); \
	if [ -n "$$DMG" ]; then \
		echo "📦 找到: $$DMG"; \
		echo "📂 挂载 DMG..."; \
		MOUNT=$$(hdiutil attach "$$DMG" -nobrowse | tail -1 | awk '{print $$NF}'); \
		echo "📋 通过保活安装脚本复制到 /Applications..."; \
		INSTALL_STATUS=0; \
		bash "$$INSTALL_SCRIPT" --skip-build --source-app "$$MOUNT/$(APP_NAME).app" --dest-app "/Applications/$(APP_NAME).app" --no-sign || INSTALL_STATUS=$$?; \
		hdiutil detach "$$MOUNT" -quiet; \
		if [ "$$INSTALL_STATUS" -ne 0 ]; then exit "$$INSTALL_STATUS"; fi; \
		echo "✅ 安装完成: /Applications/$(APP_NAME).app"; \
	else \
		echo "⚠️ 未找到 delivery DMG，回退到已签名 app bundle..."; \
		APP="target/release/bundle/macos/$(APP_NAME).app"; \
		if [ -d "$$APP" ]; then \
			bash "$$INSTALL_SCRIPT" --skip-build --source-app "$$APP" --dest-app "/Applications/$(APP_NAME).app" --no-sign; \
			echo "✅ 安装完成"; \
		else \
			echo "❌ 未找到构建产物"; \
			exit 1; \
		fi; \
	fi

.PHONY: reinstall
reinstall: ## 本地正式重装（走 signed delivery 路径）
	@$(MAKE) install --no-print-directory

.PHONY: install-dev
install-dev: ## 本地开发安装（构建 → 替换 /Applications，不误杀开发服务）
	bash ./scripts/install-macos-dev-app.sh

.PHONY: reinstall-dev
reinstall-dev: ## 本地开发重装（跳过构建 → 替换 /Applications）
	bash ./scripts/install-macos-dev-app.sh --skip-build

.PHONY: launch
launch: ## 启动已安装的应用
	@open /Applications/$(APP_NAME).app

.PHONY: restart
restart: ## 重启应用（关闭 → 启动）
	@echo "⏳ 关闭 $(APP_NAME)..."
	@DEST_BIN="/Applications/$(APP_NAME).app/Contents/MacOS/$(APP_NAME)"; \
	PIDS=$$(pgrep -f "$$DEST_BIN" 2>/dev/null || true); \
	STOP_PIDS=""; \
	for PID in $$PIDS; do \
		CMD=$$(ps -p "$$PID" -o command= 2>/dev/null || true); \
		case "$$CMD" in \
			*" --bridge-only"*|*" --relay-mac-client"*) echo "保留后台进程 $$PID";; \
			*) STOP_PIDS="$$STOP_PIDS $$PID";; \
		esac; \
	done; \
	if [ -n "$$STOP_PIDS" ]; then kill $$STOP_PIDS 2>/dev/null || true; fi; \
	sleep 1
	@echo "🚀 启动 $(APP_NAME)..."
	@open /Applications/$(APP_NAME).app
	@echo "✅ 已重启"

.PHONY: update
update: ## 拉取最新代码 → signed 本地安装 → 启动
	@echo "📥 拉取最新代码..."
	git pull --rebase
	@echo "📦 安装依赖..."
	pnpm install --frozen-lockfile 2>/dev/null || pnpm install
	@$(MAKE) install --no-print-directory
	@$(MAKE) launch --no-print-directory
	@echo "✅ 更新完成"

# ============================================================
# 清理
# ============================================================

.PHONY: clean
clean: ## 清理构建产物
	rm -rf dist/
	cargo clean

.PHONY: clean-deps
clean-deps: ## 清理 node_modules（释放磁盘空间）
	rm -rf node_modules/
	@echo "✅ node_modules 已删除，运行 pnpm install 恢复"

.PHONY: clean-all
clean-all: clean clean-deps ## 清理所有（构建产物 + 依赖）
	@echo "✅ 全部清理完成"

.PHONY: clean-deep
clean-deep: ## 深度清理：包括 Rust target 缓存
	rm -rf dist/ node_modules/ target/
	@echo "✅ 深度清理完成（需要完全重新编译）"

# ============================================================
# 多项目资源管理
# ============================================================

.PHONY: disk-usage
disk-usage: ## 显示项目磁盘占用
	@echo "📊 磁盘占用统计:"
	@echo "---"
	@du -sh node_modules/ 2>/dev/null || echo "  node_modules/: (不存在)"
	@du -sh target/ 2>/dev/null || echo "  target/: (不存在)"
	@du -sh dist/ 2>/dev/null || echo "  dist/: (不存在)"
	@echo "---"
	@du -sh . 2>/dev/null | awk '{print "  总计: " $$1}'

# ============================================================
# MCP & 服务
# ============================================================

.PHONY: mcp
mcp: ## 启动 MCP server
	cargo run --release --bin mcp-server

.PHONY: iterate-cli
iterate-cli: ## 运行 iterate CLI
	cargo run --release --bin iterate

.PHONY: relay-mac-configure
relay-mac-configure: ## 配置 Mac relay client（需 RELAY_URL，可选 RELAY_TOKEN/RELAY_ALLOW_RECOVER）
	/bin/bash scripts/install-relay-mac-client.sh configure

.PHONY: relay-mac-install
relay-mac-install: ## 安装 Mac relay client LaunchAgent（不加载）
	/bin/bash scripts/install-relay-mac-client.sh install

.PHONY: relay-mac-load
relay-mac-load: ## 加载 Mac relay client LaunchAgent
	/bin/bash scripts/install-relay-mac-client.sh load

.PHONY: relay-mac-restart
relay-mac-restart: ## 重启 Mac relay client LaunchAgent
	/bin/bash scripts/install-relay-mac-client.sh restart

.PHONY: relay-mac-status
relay-mac-status: ## 查看 Mac relay client LaunchAgent 状态
	/bin/bash scripts/install-relay-mac-client.sh status

.PHONY: relay-mac-doctor
relay-mac-doctor: ## 检查 Mac relay client 安装前置条件
	/bin/bash scripts/install-relay-mac-client.sh doctor

# ============================================================
# Git 快捷
# ============================================================

.PHONY: save
save: ## git add + commit (用法: make save m="commit message")
	git add -A
	git commit -m "$(m)"

.PHONY: push
push: save ## save + push
	git push

# ============================================================
# 帮助
# ============================================================

.PHONY: help
help: ## 显示所有可用命令
	@echo "iterate (cunzhi) - 可用命令:"
	@echo ""
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

.DEFAULT_GOAL := help
