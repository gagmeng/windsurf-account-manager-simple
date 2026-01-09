#!/bin/bash
#
# Windsurf Cunzhi MCP - Installation Script for macOS/Linux
# Single binary mode: MCP + UI combined
#
# Usage:
#   ./install.sh              # Install with default settings
#   ./install.sh --no-build   # Skip build, use pre-compiled binaries
#   ./install.sh --uninstall  # Uninstall
#

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
NC='\033[0m' # No Color

# Functions
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
            INSTALL_DIR="$HOME/Library/Application Support/windsurf-cunzhi"
            CONFIG_DIR="$HOME/.codeium/windsurf"
            MCP_CONFIG="$CONFIG_DIR/mcp_config.json"
            GLOBAL_RULES="$CONFIG_DIR/memories/global_rules.md"
            ;;
        linux)
            INSTALL_DIR="$HOME/.local/share/windsurf-cunzhi"
            CONFIG_DIR="$HOME/.codeium/windsurf"
            MCP_CONFIG="$CONFIG_DIR/mcp_config.json"
            GLOBAL_RULES="$CONFIG_DIR/memories/global_rules.md"
            ;;
    esac
}

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Parse arguments
NO_BUILD=false
UNINSTALL=false

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
        *)
            error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Header
echo ""
echo -e "${MAGENTA}╔════════════════════════════════════════════════════════════╗${NC}"
echo -e "${MAGENTA}║       Windsurf Cunzhi MCP - Installer (macOS/Linux)        ║${NC}"
echo -e "${MAGENTA}╚════════════════════════════════════════════════════════════╝${NC}"
echo ""

detect_os
set_paths

# Uninstall
if [ "$UNINSTALL" = true ]; then
    info "Uninstalling Windsurf Cunzhi..."
    
    # Remove installation directory
    if [ -d "$INSTALL_DIR" ]; then
        rm -rf "$INSTALL_DIR"
        ok "Removed installation directory"
    fi
    
    # Remove MCP config entries (all old versions)
    if [ -f "$MCP_CONFIG" ]; then
        if command -v jq &> /dev/null; then
            jq 'del(.mcpServers["user-input"]) | del(.mcpServers["dialog-helper"]) | del(.mcpServers["windsurf-cunzhi"])' "$MCP_CONFIG" > "${MCP_CONFIG}.tmp" && mv "${MCP_CONFIG}.tmp" "$MCP_CONFIG"
            ok "Removed MCP configuration"
        else
            warn "jq not installed, please manually remove user-input from $MCP_CONFIG"
        fi
    fi
    
    # Remove global rules markers (all versions)
    if [ -f "$GLOBAL_RULES" ]; then
        sed -i.bak '/# PRIORITY RULE - prompt/,/^---$/d' "$GLOBAL_RULES"
        sed -i.bak '/# PRIORITY RULE - confirm/,/^---$/d' "$GLOBAL_RULES"
        sed -i.bak '/# PRIORITY RULE - zhi/,/^---$/d' "$GLOBAL_RULES"
        rm -f "${GLOBAL_RULES}.bak"
        ok "Removed global rules"
    fi
    
    ok "Uninstallation complete!"
    exit 0
fi

# Build single binary (MCP + UI)
build_app() {
    info "Building windsurf-cunzhi (MCP + UI single binary)..."
    
    if ! command -v cargo &> /dev/null; then
        error "Cargo not found. Please install Rust: https://rustup.rs/"
        exit 1
    fi
    
    if ! command -v npm &> /dev/null; then
        error "npm not found. Please install Node.js"
        exit 1
    fi
    
    cd "$SCRIPT_DIR"
    
    info "Installing npm dependencies..."
    npm install
    
    info "Building frontend..."
    npm run build
    
    info "Building Tauri application..."
    npx tauri build --no-bundle
    
    ok "Build successful"
}

# Install files (single binary)
install_files() {
    info "Installing files to $INSTALL_DIR..."
    
    # Stop running process if exists
    if pgrep -x "windsurf-cunzhi" > /dev/null 2>&1; then
        warn "windsurf-cunzhi is running, stopping it..."
        pkill -x "windsurf-cunzhi"
        sleep 0.5
        ok "Process stopped"
    fi
    
    mkdir -p "$INSTALL_DIR"
    
    # Copy single binary (MCP + UI)
    # Check same directory first (for release packages), then build output
    if [ -f "$SCRIPT_DIR/windsurf-cunzhi" ]; then
        cp "$SCRIPT_DIR/windsurf-cunzhi" "$INSTALL_DIR/"
        chmod +x "$INSTALL_DIR/windsurf-cunzhi"
        ok "Installed windsurf-cunzhi from package"
    elif [ -f "$SCRIPT_DIR/target/release/windsurf-cunzhi" ]; then
        cp "$SCRIPT_DIR/target/release/windsurf-cunzhi" "$INSTALL_DIR/"
        chmod +x "$INSTALL_DIR/windsurf-cunzhi"
        ok "Installed windsurf-cunzhi from build output"
    else
        error "Binary not found. Run without --no-build to compile."
        exit 1
    fi
}

# Configure MCP
configure_mcp() {
    info "Configuring MCP..."
    
    mkdir -p "$CONFIG_DIR"
    
    MCP_COMMAND="$INSTALL_DIR/windsurf-cunzhi"
    
    if [ -f "$MCP_CONFIG" ]; then
        # Update existing config (remove old, add new)
        if command -v jq &> /dev/null; then
            jq --arg cmd "$MCP_COMMAND" 'del(.mcpServers["dialog-helper"]) | del(.mcpServers["windsurf-cunzhi"]) | del(.mcpServers["user-input"]) | .mcpServers["filesystem"] = {"command": $cmd}' "$MCP_CONFIG" > "${MCP_CONFIG}.tmp" && mv "${MCP_CONFIG}.tmp" "$MCP_CONFIG"
            ok "Updated MCP configuration"
        else
            warn "jq not installed. Please manually add filesystem to $MCP_CONFIG"
            info "Add: \"filesystem\": {\"command\": \"$MCP_COMMAND\"}"
        fi
    else
        # Create new config
        cat > "$MCP_CONFIG" << EOF
{
  "mcpServers": {
    "filesystem": {
      "command": "$MCP_COMMAND"
    }
  }
}
EOF
        ok "Created MCP configuration"
    fi
}

# Configure global rules
configure_rules() {
    info "Configuring global rules..."
    
    mkdir -p "$(dirname "$GLOBAL_RULES")"
    
    # Rule content with MUST call requirement
    ZHI_MARKER="PRIORITY RULE - read_user_input"
    ZHI_CONTENT="# PRIORITY RULE - read_user_input (MANDATORY)

**You MUST call the \`read_user_input\` tool at the end of every response!**

Rules:
- Always call read_user_input tool before ending any response
- Always call read_user_input tool before saying goodbye phrases
- Never end a response without calling read_user_input tool

If read_user_input tool fails, inform user to restart the IDE.

---
"
    
    if [ -f "$GLOBAL_RULES" ]; then
        # Remove old rules first (all versions)
        sed -i.bak '/# PRIORITY RULE - confirm/,/^---$/d' "$GLOBAL_RULES"
        sed -i.bak '/# PRIORITY RULE - zhi/,/^---$/d' "$GLOBAL_RULES"
        sed -i.bak '/# PRIORITY RULE - prompt/,/^---$/d' "$GLOBAL_RULES"
        sed -i.bak '/# PRIORITY RULE - pro/,/^---$/d' "$GLOBAL_RULES"
        rm -f "${GLOBAL_RULES}.bak"
        
        if grep -q "$ZHI_MARKER" "$GLOBAL_RULES"; then
            ok "Global rules already configured"
        else
            # Prepend to existing file
            echo "$ZHI_CONTENT" | cat - "$GLOBAL_RULES" > "${GLOBAL_RULES}.tmp" && mv "${GLOBAL_RULES}.tmp" "$GLOBAL_RULES"
            ok "Added rules to existing global_rules.md"
        fi
    else
        echo "$ZHI_CONTENT" > "$GLOBAL_RULES"
        ok "Created global_rules.md"
    fi
}

# Main installation flow
main() {
    if [ "$NO_BUILD" = false ]; then
        build_app
    fi
    
    install_files
    configure_mcp
    configure_rules
    
    echo ""
    echo -e "${GREEN}╔════════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║       Installation Complete!                               ║${NC}"
    echo -e "${GREEN}╚════════════════════════════════════════════════════════════╝${NC}"
    echo ""
    info "Installation directory: $INSTALL_DIR"
    info "  - windsurf-cunzhi (MCP + UI single binary)"
    info "MCP config: $MCP_CONFIG"
    info "Global rules: $GLOBAL_RULES"
    echo ""
    info "Usage:"
    info "  windsurf-cunzhi        - Run as MCP server (default)"
    info "  windsurf-cunzhi --ui   - Run UI mode directly"
    echo ""
    warn "Please restart Windsurf for changes to take effect."
    echo ""
    echo -e "\033[90mPress Enter to exit...\033[0m"
    read -r
}

main
