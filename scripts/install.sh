#!/bin/sh
# Trace installer for macOS and Linux.
#
#   curl -fsSL https://raw.githubusercontent.com/TaxCollector23/trace/main/scripts/install.sh | sh
#
# Downloads the correct `trace` binary from GitHub Releases, installs it to
# ~/.trace/bin/trace, makes it executable, and prints PATH instructions.
set -eu

REPO="TaxCollector23/trace"
INSTALL_DIR="${HOME}/.trace/bin"
BIN="${INSTALL_DIR}/trace"

err() { printf 'error: %s\n' "$1" >&2; exit 1; }

# --- Detect OS ---
os="$(uname -s)"
case "$os" in
  Darwin) os_tag="macos" ;;
  Linux)  os_tag="linux" ;;
  *) err "unsupported OS: $os (Trace supports macOS, Linux, Windows)" ;;
esac

# --- Detect architecture ---
arch="$(uname -m)"
case "$arch" in
  arm64|aarch64) arch_tag="arm64" ;;
  x86_64|amd64)  arch_tag="x64" ;;
  *) err "unsupported architecture: $arch" ;;
esac

asset="trace-${os_tag}-${arch_tag}"
version="${TRACE_VERSION:-latest}"
if [ "$version" = "latest" ]; then
  url="https://github.com/${REPO}/releases/latest/download/${asset}"
else
  url="https://github.com/${REPO}/releases/download/${version}/${asset}"
fi

printf 'Installing Trace (%s) ...\n' "$asset"
mkdir -p "$INSTALL_DIR"

if command -v curl >/dev/null 2>&1; then
  curl -fSL "$url" -o "$BIN" || err "download failed from $url"
elif command -v wget >/dev/null 2>&1; then
  wget -qO "$BIN" "$url" || err "download failed from $url"
else
  err "neither curl nor wget is available"
fi

chmod +x "$BIN"

printf '\nInstalled trace to %s\n' "$BIN"

# --- PATH guidance ---
case ":${PATH}:" in
  *":${INSTALL_DIR}:"*)
    printf 'Trace is on your PATH. Run: trace --help\n'
    ;;
  *)
    printf '\nAdd Trace to your PATH by adding this line to your shell profile\n'
    printf '(~/.zshrc, ~/.bashrc, or ~/.profile):\n\n'
    printf '  export PATH="%s:$PATH"\n\n' "$INSTALL_DIR"
    printf 'Then restart your shell and run: trace --help\n'
    ;;
esac
