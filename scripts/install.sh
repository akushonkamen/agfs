#!/bin/sh
set -e

# AGFS Installation Script
# AGFS has been rewritten in Rust - this script helps install from source or pre-built binaries

REPO="c4pt0r/agfs"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"
AGFS_SHELL_DIR="${AGFS_SHELL_DIR:-$HOME/.local/agfs-shell}"
INSTALL_SERVER="${INSTALL_SERVER:-yes}"
INSTALL_CLIENT="${INSTALL_CLIENT:-yes}"

# Detect OS and architecture
detect_platform() {
    OS=$(uname -s | tr '[:upper:]' '[:lower:]')
    ARCH=$(uname -m)

    case "$OS" in
        linux)
            OS="linux"
            ;;
        darwin)
            OS="darwin"
            ;;
        mingw* | msys* | cygwin*)
            OS="windows"
            ;;
        *)
            echo "Error: Unsupported operating system: $OS"
            exit 1
            ;;
    esac

    case "$ARCH" in
        x86_64 | amd64)
            ARCH="amd64"
            ;;
        aarch64 | arm64)
            ARCH="arm64"
            ;;
        *)
            echo "Error: Unsupported architecture: $ARCH"
            exit 1
            ;;
    esac

    echo "Detected platform: $OS-$ARCH"
}

# Check if Rust is installed
check_rust() {
    if command -v cargo >/dev/null 2>&1; then
        echo "✓ Rust/Cargo found"
        return 0
    else
        echo "✗ Rust not found"
        return 1
    fi
}

# Check Python version
check_python() {
    if ! command -v python3 >/dev/null 2>&1; then
        echo "Warning: python3 not found. agfs-shell requires Python 3.10+"
        return 1
    fi

    PYTHON_VERSION=$(python3 -c 'import sys; print(".".join(map(str, sys.version_info[:2])))')
    PYTHON_MAJOR=$(echo "$PYTHON_VERSION" | cut -d. -f1)
    PYTHON_MINOR=$(echo "$PYTHON_VERSION" | cut -d. -f2)

    if [ "$PYTHON_MAJOR" -lt 3 ] || { [ "$PYTHON_MAJOR" -eq 3 ] && [ "$PYTHON_MINOR" -lt 10 ]; }; then
        echo "Warning: Python $PYTHON_VERSION found, but agfs-shell requires Python 3.10+"
        return 1
    fi

    echo "✓ Found Python $PYTHON_VERSION"
    return 0
}

# Install agfs-server from source
install_server_from_source() {
    echo ""
    echo "Installing agfs-server from source..."

    if ! check_rust; then
        echo ""
        echo "Rust is required to build agfs-server from source."
        echo "Install Rust from https://rustup.rs/"
        echo ""
        echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        echo ""
        exit 1
    fi

    # Check if we're in the agfs directory
    if [ ! -f "src/Cargo.toml" ]; then
        echo "Error: Please run this script from the agfs repository root"
        echo ""
        echo "Clone the repository first:"
        echo "  git clone https://github.com/$REPO.git"
        echo "  cd agfs"
        echo "  ./install.sh"
        exit 1
    fi

    cd src

    echo "Building agfs-server (this may take a few minutes)..."
    if cargo build --release 2>&1 | while read -r line; do echo "  $line"; done; then
        echo ""
        echo "✓ Build successful"
    else
        echo "Error: Build failed"
        exit 1
    fi

    # Create install directory if it doesn't exist
    mkdir -p "$INSTALL_DIR"

    # Install binary
    cp target/release/agfs-server "$INSTALL_DIR/agfs-server"
    chmod +x "$INSTALL_DIR/agfs-server"

    echo "✓ agfs-server installed to $INSTALL_DIR/agfs-server"

    # Install agfs-fuse if built
    if [ -f "target/release/agfs-fuse" ]; then
        cp target/release/agfs-fuse "$INSTALL_DIR/agfs-fuse"
        chmod +x "$INSTALL_DIR/agfs-fuse"
        echo "✓ agfs-fuse installed to $INSTALL_DIR/agfs-fuse"
    fi

    cd - > /dev/null
}

# Install agfs-shell from source
install_client_from_source() {
    echo ""
    echo "Installing agfs-shell..."

    # Check Python
    if ! check_python; then
        echo "Skipping agfs-shell installation (Python requirement not met)"
        return 1
    fi

    if [ ! -d "rust-src/agfs-shell" ]; then
        echo "Error: agfs-shell directory not found"
        return 1
    fi

    # Install using pip
    if pip3 install -e rust-src/agfs-shell 2>&1 | while read -r line; do echo "  $line"; done; then
        echo "✓ agfs-shell installed"
    else
        echo "Warning: agfs-shell installation failed"
        return 1
    fi
}

show_completion() {
    echo ""
    echo "----------------------------------"
    echo "    Installation completed!"
    echo "----------------------------------"
    echo ""

    if [ "$INSTALL_SERVER" = "yes" ] && [ -f "$INSTALL_DIR/agfs-server" ]; then
        echo "Server: agfs-server"
        echo "  Location: $INSTALL_DIR/agfs-server"
        echo "  Usage: agfs-server --help"
        echo ""
    fi

    if command -v agfs >/dev/null 2>&1; then
        echo "Client: agfs"
        echo "  Location: $(command -v agfs)"
        echo "  Usage: agfs --help"
        echo "  Interactive: agfs"
        echo ""
    fi

    # Check if install dir is in PATH
    case ":$PATH:" in
        *":$INSTALL_DIR:"*)
            ;;
        *)
            echo "Note: $INSTALL_DIR is not in your PATH."
            echo "Add it to your PATH by adding this to ~/.bashrc or ~/.zshrc:"
            echo "  export PATH=\"\$PATH:$INSTALL_DIR\""
            echo ""
            ;;
    esac

    echo "Quick Start:"
    echo "  1. Start server: agfs-server --config config.yaml"
    echo "  2. Use client: agfs"
    echo ""
    echo "For more information, see https://github.com/$REPO"
}

main() {
    echo ""
    echo "----------------------------------"
    echo "          AGFS Installer           "
    echo "----------------------------------"
    echo ""
    echo "AGFS is now written in Rust!"
    echo "This installer will build from source."
    echo ""

    detect_platform

    if [ "$INSTALL_SERVER" = "yes" ]; then
        install_server_from_source
    fi

    if [ "$INSTALL_CLIENT" = "yes" ]; then
        install_client_from_source || true  # Don't fail if client install fails
    fi

    show_completion
}

main
