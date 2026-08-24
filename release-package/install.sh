#!/bin/bash
#
# Iterate (寸止) - Installation Script for macOS/Linux
# GUI input tool via run_command
#
# Usage:
#   ./install.sh                         # Build and install
#   ./install.sh --no-build              # Use pre-compiled binary (skip build)
#   ./install.sh --uninstall             # Uninstall
#   ./install.sh --client windsurf       # Configure one client only
#   ./install.sh --client windsurf,codex # Configure multiple clients

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m'

info() { echo -e "${CYAN}[INFO]${NC} $1"; }
ok() { echo -e "${GREEN}[OK]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Detect OS
detect_os() {
    case "$(uname -s)" in
        Darwin*)    OS="macos" ;;
        Linux*)     OS="linux" ;;
        *)          error "Unsupported OS: $(uname -s)"; exit 1 ;;
    esac
    info "Detected OS: $OS"
}

# Set paths based on OS
set_paths() {
    case "$OS" in
        macos)
            INSTALL_DIR="$HOME/Library/Application Support/iterate"
            LEGACY_INSTALL_DIR="$HOME/Library/Application Support/windsurf-cunzhi"
            BIN_DIR="$HOME/bin"
            CONFIG_DIR="$HOME/.codeium/windsurf"
            GLOBAL_RULES="$CONFIG_DIR/memories/global_rules.md"
            RULES_DIR="$CONFIG_DIR/rules"
            ;;
        linux)
            INSTALL_DIR="$HOME/.local/share/iterate"
            LEGACY_INSTALL_DIR="$HOME/.local/share/windsurf-cunzhi"
            BIN_DIR="$HOME/.local/bin"
            CONFIG_DIR="$HOME/.codeium/windsurf"
            GLOBAL_RULES="$CONFIG_DIR/memories/global_rules.md"
            RULES_DIR="$CONFIG_DIR/rules"
            ;;
    esac
}

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Parse arguments
NO_BUILD=false
UNINSTALL=false
CLIENTS_RAW=""
SELECTED_CLIENTS="all"
CONFIGURED_CLIENTS=()

while [[ $# -gt 0 ]]; do
    case $1 in
        --no-build)
            NO_BUILD=true
            shift
            ;;
        --uninstall)
            UNINSTALL=true
            shift
            ;;
        --client)
            if [[ -z "${2:-}" ]]; then
                error "--client requires a value (windsurf,cursor,codex,all)"
                exit 1
            fi
            CLIENTS_RAW="$2"
            shift 2
            ;;
        *)
            error "Unknown option: $1"
            exit 1
            ;;
    esac
done

echo ""
echo -e "${MAGENTA}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${MAGENTA}║       Iterate Installer                                    ║${NC}"
echo -e "${MAGENTA}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

detect_os
set_paths

normalize_clients() {
    echo "$1" | tr '[:upper:]' '[:lower:]' | tr -d ' '
}

validate_clients() {
    local raw="$1"
    local item

    if [[ "$raw" == *"all"* && "$raw" != "all" ]]; then
        error "\"all\" cannot be combined with other clients"
        exit 1
    fi

    IFS=',' read -r -a items <<< "$raw"
    for item in "${items[@]}"; do
        case "$item" in
            all|windsurf|cursor|codex)
                ;;
            "")
                ;;
            *)
                error "Unsupported client: $item"
                exit 1
                ;;
        esac
    done
}

select_clients() {
    local selected

    selected="$(normalize_clients "${CLIENTS_RAW:-}")"

    if [ -z "$selected" ] && [ -t 0 ]; then
        info "Choose client(s) to configure [windsurf,cursor,codex,all] (default: all)"
        read -r selected || true
        selected="$(normalize_clients "${selected:-}")"
    fi

    [ -z "$selected" ] && selected="all"
    validate_clients "$selected"
    SELECTED_CLIENTS="$selected"
    info "Client targets: $SELECTED_CLIENTS"
}

client_selected() {
    local target="$1"
    if [ "$SELECTED_CLIENTS" = "all" ]; then
        return 0
    fi

    case ",$SELECTED_CLIENTS," in
        *",$target,"*) return 0 ;;
        *) return 1 ;;
    esac
}

append_configured_client() {
    local label="$1"
    CONFIGURED_CLIENTS+=("$label")
}

select_clients

# Uninstall
if [ "$UNINSTALL" = true ]; then
    info "Uninstalling..."
    
    [ -d "$INSTALL_DIR" ] && rm -rf "$INSTALL_DIR" && ok "Removed $INSTALL_DIR"
    [ -d "$LEGACY_INSTALL_DIR" ] && rm -rf "$LEGACY_INSTALL_DIR" && ok "Removed legacy $LEGACY_INSTALL_DIR"
    [ -f "$BIN_DIR/windsurf-cunzhi" ] && rm -f "$BIN_DIR/windsurf-cunzhi" && ok "Removed legacy windsurf-cunzhi from PATH"
    [ -f "$BIN_DIR/iterate" ] && rm -f "$BIN_DIR/iterate" && ok "Removed iterate from PATH"
    
    ok "Uninstallation complete!"
    exit 0
fi

# Build
build_app() {
    info "Building iterate..."
    
    if ! command -v cargo &> /dev/null; then
        error "Cargo not found. Please install Rust: https://rustup.rs/"
        exit 1
    fi
    
    cd "$SCRIPT_DIR"
    
    if command -v pnpm &> /dev/null; then
        info "Building with pnpm tauri build..."
        pnpm tauri build --no-bundle
    elif command -v npm &> /dev/null; then
        info "Building with npm..."
        npm install
        npm run build
        npx tauri build --no-bundle
    else
        error "npm or pnpm not found. Please install Node.js"
        exit 1
    fi
    
    ok "Build successful"
}

# Install files
install_files() {
    info "Installing files..."
    
    mkdir -p "$INSTALL_DIR"
    mkdir -p "$BIN_DIR"
    
    # Find binary
    if [ -f "$SCRIPT_DIR/target/release/iterate" ]; then
        BINARY="$SCRIPT_DIR/target/release/iterate"
    elif [ -f "$SCRIPT_DIR/iterate" ]; then
        BINARY="$SCRIPT_DIR/iterate"
    else
        error "Binary not found. Run without --no-build to compile."
        exit 1
    fi
    
    # Install to both locations
    cp "$BINARY" "$INSTALL_DIR/iterate"
    chmod +x "$INSTALL_DIR/iterate"
    
    cp "$BINARY" "$BIN_DIR/iterate"
    chmod +x "$BIN_DIR/iterate"

    # Remove legacy binary if present
    if [ -f "$BIN_DIR/windsurf-cunzhi" ] && [ ! -L "$BIN_DIR/windsurf-cunzhi" ]; then
        rm -f "$BIN_DIR/windsurf-cunzhi" && ok "Removed legacy $BIN_DIR/windsurf-cunzhi"
    elif [ -L "$BIN_DIR/windsurf-cunzhi" ]; then
        rm -f "$BIN_DIR/windsurf-cunzhi" && ok "Removed legacy symlink $BIN_DIR/windsurf-cunzhi"
    fi
    
    # Install mcp-server binary
    MCP_SERVER_BINARY=""
    if [ -f "$SCRIPT_DIR/target/release/mcp-server" ]; then
        MCP_SERVER_BINARY="$SCRIPT_DIR/target/release/mcp-server"
    elif [ -f "$SCRIPT_DIR/mcp-server" ]; then
        MCP_SERVER_BINARY="$SCRIPT_DIR/mcp-server"
    fi

    if [ -n "$MCP_SERVER_BINARY" ]; then
        cp "$MCP_SERVER_BINARY" "$INSTALL_DIR/mcp-server"
        chmod +x "$INSTALL_DIR/mcp-server"
        cp "$MCP_SERVER_BINARY" "$BIN_DIR/mcp-server"
        chmod +x "$BIN_DIR/mcp-server"
        ok "Installed mcp-server"
    else
        warn "mcp-server binary not found, skipping (MCP stdio mode unavailable)"
    fi

    # Remove quarantine on macOS
    if [ "$OS" = "macos" ]; then
        xattr -d com.apple.quarantine "$INSTALL_DIR/iterate" 2>/dev/null || true
        xattr -d com.apple.quarantine "$BIN_DIR/iterate" 2>/dev/null || true
        [ -f "$INSTALL_DIR/mcp-server" ] && xattr -d com.apple.quarantine "$INSTALL_DIR/mcp-server" 2>/dev/null || true
        [ -f "$BIN_DIR/mcp-server" ] && xattr -d com.apple.quarantine "$BIN_DIR/mcp-server" 2>/dev/null || true
    fi
    
    ok "Installed iterate"
}

configure_json_client() {
    local config_path="$1"
    local client_label="$2"

    mkdir -p "$(dirname "$config_path")"

    if command -v python3 &>/dev/null; then
        python3 - <<PYEOF
import json
from pathlib import Path

path = Path(r"""$config_path""").expanduser()
try:
    cfg = json.loads(path.read_text())
except Exception:
    cfg = {}

cfg.setdefault("mcpServers", {})
cfg["mcpServers"]["iterate-zhi"] = {
    "command": r"""$MCP_SERVER_PATH""",
    "args": []
}

path.parent.mkdir(parents=True, exist_ok=True)
path.write_text(json.dumps(cfg, indent=2, ensure_ascii=False) + "\n")
print(path)
PYEOF
        ok "$client_label MCP config written: $config_path"
    elif [ ! -f "$config_path" ]; then
        cat > "$config_path" << EOF
{
  "mcpServers": {
    "iterate-zhi": {
      "command": "$MCP_SERVER_PATH",
      "args": []
    }
  }
}
EOF
        ok "$client_label MCP config written: $config_path"
    else
        warn "python3 not found, please manually add iterate-zhi to $config_path"
        return
    fi

    append_configured_client "$client_label"
}

configure_codex_client() {
    local config_path="$HOME/.codex/config.toml"
    mkdir -p "$(dirname "$config_path")"

    if command -v python3 &>/dev/null; then
        python3 "$SCRIPT_DIR/scripts/merge-codex-mcp-config.py" "$config_path" "$MCP_SERVER_PATH"
        ok "Codex CLI MCP config written: $config_path"
    elif [ ! -f "$config_path" ]; then
        cat > "$config_path" << EOF
[mcp_servers."iterate-zhi"]
command = "$MCP_SERVER_PATH"
args = ["5311"]
disabled = false
enabled = true
tool_timeout_sec = 315360000
EOF
        ok "Codex CLI MCP config written: $config_path"
    else
        warn "python3 not found, please manually add iterate-zhi to $config_path"
        return
    fi

    append_configured_client "Codex CLI"
}

configure_clients() {
    info "Configuring client integrations..."

    MCP_SERVER_PATH="$BIN_DIR/mcp-server"
    if [ ! -f "$MCP_SERVER_PATH" ]; then
        warn "mcp-server not installed, skipping client configuration"
        return
    fi

    if client_selected windsurf; then
        configure_json_client "$HOME/.codeium/windsurf/mcp_config.json" "Windsurf"
    fi

    if client_selected cursor; then
        configure_json_client "$HOME/.cursor/mcp.json" "Cursor"
    fi

    if client_selected codex; then
        configure_codex_client
    fi
}

# Configure Windsurf rules
configure_windsurf_rules() {
    if ! client_selected windsurf; then
        return
    fi

    info "Configuring Windsurf rules..."
    
    mkdir -p "$(dirname "$GLOBAL_RULES")"
    mkdir -p "$RULES_DIR"
    
    # Create/update rules file with trigger: always_on
    RULE_FILE="$RULES_DIR/00-zhi.md"
    
    cat > "$RULE_FILE" << 'EOF'
---
trigger: always_on
---

# 寸止 (zhi) - 对话控制规则

## 核心原则
- **寸止调用**：任何对话结束前必须调用 `zhi`（寸止）
- **禁止替代**：不得仅输出文字 "zhi"，必须真正调用工具
- **对话终止权**：AI 永远不能主动判断任务完成或切断对话

## 调用方式
使用 `run_command` 工具执行：
```bash
iterate --ui --message "消息内容" --options "选项1,选项2" --workspace "/项目路径"
```

**参数说明**：
- `--ui`：必须，启动 GUI 弹窗
- `--message`：显示给用户的消息（支持 Markdown）
- `--options`：预定义选项，逗号分隔
- `--workspace`：当前项目路径

**重要**：`Blocking` 必须为 `true`，等待用户输入后继续。
EOF
    
    ok "Created $RULE_FILE"
}

# Main
main() {
    if [ "$NO_BUILD" = false ]; then
        build_app
    fi
    
    install_files
    configure_clients
    configure_windsurf_rules
    
    echo ""
    echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║       Installation Complete!                               ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    info "Binary: $BIN_DIR/iterate"
    info "MCP server: $BIN_DIR/mcp-server"
    if [ "${#CONFIGURED_CLIENTS[@]}" -gt 0 ]; then
        info "Configured clients: ${CONFIGURED_CLIENTS[*]}"
    fi
    if client_selected windsurf; then
        info "Windsurf rules: $RULES_DIR/00-zhi.md"
    fi
    echo ""
    info "Usage:"
    info "  iterate --ui --message \"Hello\" --options \"A,B,C\""
    echo ""
    
    # Check PATH
    if [[ ":$PATH:" != *":$BIN_DIR:"* ]]; then
        warn "Add to PATH: export PATH=\"\$PATH:$BIN_DIR\""
    fi
    
    warn "Please restart your configured client(s) for changes to take effect."
}

main
