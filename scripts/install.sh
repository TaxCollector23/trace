#!/bin/sh
# TraceGuard installer for macOS and Linux.
#
#   curl -fsSL https://raw.githubusercontent.com/TaxCollector23/TraceGuard/main/scripts/install.sh | sh
#
# Downloads the correct `trg` binary from GitHub Releases, installs it to
# ~/.traceguard/bin/trg, makes it executable, and prints PATH instructions.
set -eu

REPO="TaxCollector23/TraceGuard"
INSTALL_DIR="${HOME}/.traceguard/bin"
BIN="${INSTALL_DIR}/trg"

err() { printf 'error: %s\n' "$1" >&2; exit 1; }

# --- Detect OS ---
os="$(uname -s)"
case "$os" in
  Darwin) os_tag="macos" ;;
  Linux)  os_tag="linux" ;;
  *) err "unsupported OS: $os (TraceGuard supports macOS, Linux, Windows)" ;;
esac

# --- Detect architecture ---
arch="$(uname -m)"
case "$arch" in
  arm64|aarch64) arch_tag="arm64" ;;
  x86_64|amd64)  arch_tag="x64" ;;
  *) err "unsupported architecture: $arch" ;;
esac

asset="trg-${os_tag}-${arch_tag}"
version="${TRACEGUARD_VERSION:-latest}"
if [ "$version" = "latest" ]; then
  url="https://github.com/${REPO}/releases/latest/download/${asset}"
else
  url="https://github.com/${REPO}/releases/download/${version}/${asset}"
fi

printf 'Installing TraceGuard (%s) ...\n' "$asset"
mkdir -p "$INSTALL_DIR"

if command -v curl >/dev/null 2>&1; then
  curl -fSL "$url" -o "$BIN" || err "download failed from $url"
elif command -v wget >/dev/null 2>&1; then
  wget -qO "$BIN" "$url" || err "download failed from $url"
else
  err "neither curl nor wget is available"
fi

chmod +x "$BIN"

# Provide the `traceguard` long alias as a symlink to the same binary.
ln -sf "$BIN" "${INSTALL_DIR}/traceguard"

printf '\nInstalled trg to %s\n' "$BIN"

# --- PATH guidance ---
case ":${PATH}:" in
  *":${INSTALL_DIR}:"*)
    printf 'TraceGuard is on your PATH. Run: trg --help\n'
    ;;
  *)
    printf '\nAdd TraceGuard to your PATH by adding this line to your shell profile\n'
    printf '(~/.zshrc, ~/.bashrc, or ~/.profile):\n\n'
    printf '  export PATH="%s:$PATH"\n\n' "$INSTALL_DIR"
    printf 'Then restart your shell and run: trg --help\n'
    ;;
esac
