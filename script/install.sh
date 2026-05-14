#!/usr/bin/env bash
set -euo pipefail

REPO="NewtonChutney/quads-tui"
INSTALL_DIR="${INSTALL_DIR:-$HOME/.local/bin}"

main() {
    local platform arch version url tmpdir

    platform="$(detect_platform)"
    arch="$(detect_arch)"
    version="$(latest_version)"

    if [ -z "$version" ]; then
        echo "Error: could not determine latest release version." >&2
        exit 1
    fi

    local asset_name
    case "$platform" in
        linux)   asset_name="quads-tui-linux-${arch}" ;;
        macos)   asset_name="quads-tui-macos-${arch}" ;;
        *)
            echo "Error: unsupported platform: $platform" >&2
            exit 1
            ;;
    esac

    url="https://github.com/${REPO}/releases/download/${version}/${asset_name}"

    echo "Downloading quads-tui ${version} for ${platform}-${arch}..."
    tmpdir="$(mktemp -d)"
    trap 'rm -rf "$tmpdir"' EXIT

    fetch "$url" > "$tmpdir/quads-tui"
    chmod +x "$tmpdir/quads-tui"

    mkdir -p "$INSTALL_DIR"
    mv "$tmpdir/quads-tui" "$INSTALL_DIR/quads-tui"

    echo "Installed quads-tui to ${INSTALL_DIR}/quads-tui"
    check_path
}

detect_platform() {
    local os
    os="$(uname -s)"
    case "$os" in
        Linux*)  echo "linux" ;;
        Darwin*) echo "macos" ;;
        *)
            echo "Error: unsupported OS: $os" >&2
            exit 1
            ;;
    esac
}

detect_arch() {
    local arch
    arch="$(uname -m)"
    case "$arch" in
        x86_64|amd64)     echo "x86_64" ;;
        arm64|aarch64)    echo "aarch64" ;;
        *)
            echo "Error: unsupported architecture: $arch" >&2
            exit 1
            ;;
    esac
}

latest_version() {
    local url="https://api.github.com/repos/${REPO}/releases/latest"
    fetch "$url" | grep -o '"tag_name":[[:space:]]*"[^"]*"' | head -1 | cut -d'"' -f4
}

fetch() {
    if command -v curl > /dev/null 2>&1; then
        curl -fsSL "$1"
    elif command -v wget > /dev/null 2>&1; then
        wget -qO- "$1"
    else
        echo "Error: curl or wget is required." >&2
        exit 1
    fi
}

check_path() {
    if ! echo "$PATH" | tr ':' '\n' | grep -qx "$INSTALL_DIR"; then
        echo ""
        echo "Add ${INSTALL_DIR} to your PATH:"
        if [ -n "${ZSH_VERSION:-}" ] || [ "$(basename "${SHELL:-}")" = "zsh" ]; then
            echo "  echo 'export PATH=\"${INSTALL_DIR}:\$PATH\"' >> ~/.zshrc"
        elif [ "$(basename "${SHELL:-}")" = "fish" ]; then
            echo "  fish_add_path ${INSTALL_DIR}"
        else
            echo "  echo 'export PATH=\"${INSTALL_DIR}:\$PATH\"' >> ~/.bashrc"
        fi
    fi
}

main
