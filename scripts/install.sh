#!/bin/sh
# Trace installer for macOS and Linux.
#
#   curl -fsSL https://raw.githubusercontent.com/TaxCollector23/trace/main/scripts/install.sh | sh
#
# Downloads the correct `trace` binary from GitHub Releases, installs it to
# ~/.trace/bin/trc, makes it executable, and prints PATH instructions.
set -eu

REPO="TaxCollector23/trc"
INSTALL_DIR="${HOME}/.trace/bin"
BIN="${INSTALL_DIR}/trc"

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

fetch_to() { # fetch <url> <out-file>; nonzero on failure
  if command -v curl >/dev/null 2>&1; then
    curl -fSL "$1" -o "$2"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "$2" "$1"
  else
    err "neither curl nor wget is available"
  fi
}
fetch_text() { # fetch <url> to stdout, quietly; empty on failure
  if command -v curl >/dev/null 2>&1; then
    curl -fsSL "$1" 2>/dev/null
  elif command -v wget >/dev/null 2>&1; then
    wget -qO- "$1" 2>/dev/null
  fi
}
sha256_of() {
  if command -v sha256sum >/dev/null 2>&1; then
    sha256sum "$1" | awk '{print $1}'
  elif command -v shasum >/dev/null 2>&1; then
    shasum -a 256 "$1" | awk '{print $1}'
  else
    echo ""
  fi
}

tmp="${BIN}.download.$$"
fetch_to "$url" "$tmp" || err "download failed from $url"

# Verify the SHA-256 checksum published next to the asset before trusting it.
# Missing checksum (older releases) is allowed unless TRACE_REQUIRE_CHECKSUM.
published=$(fetch_text "${url}.sha256" | awk '{print $1}' | head -1)
if [ -n "$published" ]; then
  local_sum=$(sha256_of "$tmp")
  if [ -z "$local_sum" ]; then
    printf 'note: no sha256 tool found; skipping checksum verification\n' >&2
  elif [ "$local_sum" != "$published" ]; then
    rm -f "$tmp"
    err "checksum mismatch for $asset (expected $published, got $local_sum)"
  else
    printf 'Checksum verified.\n'
  fi
elif [ -n "${TRACE_REQUIRE_CHECKSUM:-}" ]; then
  rm -f "$tmp"
  err "no checksum published for $asset and TRACE_REQUIRE_CHECKSUM is set"
else
  printf 'note: no checksum published for this release; skipping verification\n' >&2
fi

chmod +x "$tmp"
mv "$tmp" "$BIN"

printf '\nInstalled trc to %s\n' "$BIN"

# --- PATH guidance ---
case ":${PATH}:" in
  *":${INSTALL_DIR}:"*)
    printf 'Trace is on your PATH. Run: trc --help\n'
    ;;
  *)
    printf '\nAdd Trace to your PATH by adding this line to your shell profile\n'
    printf '(~/.zshrc, ~/.bashrc, or ~/.profile):\n\n'
    printf '  export PATH="%s:$PATH"\n\n' "$INSTALL_DIR"
    printf 'Then restart your shell and run: trc --help\n'
    ;;
esac
