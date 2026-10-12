#!/bin/sh
# Trace installer for macOS and Linux.
#
#   curl -fsSL https://raw.githubusercontent.com/TaxCollector23/trace/main/scripts/install.sh | sh
#
# Downloads the correct `trace` binary from GitHub Releases, installs it to
# ~/.trace/bin/trc, makes it executable, and prints PATH instructions.
set -eu

REPO="TaxCollector23/trace"
INSTALL_DIR="${HOME}/.trace/bin"
BIN="${INSTALL_DIR}/trc"

err() {
  mkdir -p "${HOME}/.trace" 2>/dev/null || true
  printf '%s %s\n' "$(date -u +%FT%TZ)" "$1" >> "${HOME}/.trace/install-errors.log" 2>/dev/null || true
  exit 1
}

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
release_base_url="${TRACE_RELEASE_BASE_URL:-https://github.com/${REPO}/releases}"
release_base_url="${release_base_url%/}"
if [ "$version" = "latest" ]; then
  url="${release_base_url}/latest/download/${asset}"
else
  url="${release_base_url}/download/${version}/${asset}"
fi

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
    curl -fsSL "$1"
  elif command -v wget >/dev/null 2>&1; then
    wget -qO- "$1"
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
trap 'rm -f "$tmp"' EXIT
fetch_to "$url" "$tmp" || err "download failed from $url"

# Verify the SHA-256 checksum published next to the asset before trusting it.
if [ "${TRACE_ALLOW_UNSIGNED:-0}" = "1" ]; then
  printf 'warning: TRACE_ALLOW_UNSIGNED=1; skipping checksum verification\n' >&2
else
  checksum_payload=$(fetch_text "${url}.sha256") || err "checksum download failed for $asset"
  published=$(printf '%s\n' "$checksum_payload" | awk 'NF { print tolower($1); exit }')
  printf '%s\n' "$published" | awk 'length($0) == 64 && $0 !~ /[^0-9a-f]/ { found=1 } END { exit(found ? 0 : 1) }' \
    || err "no valid SHA-256 checksum published for $asset"
  local_sum=$(sha256_of "$tmp")
  [ -n "$local_sum" ] || err "no SHA-256 tool available to verify $asset"
  if [ "$local_sum" != "$published" ]; then
    err "checksum mismatch for $asset (expected $published, got $local_sum)"
  fi
  printf 'Checksum verified.\n'
fi

chmod +x "$tmp"
mv "$tmp" "$BIN"

# Start live review immediately. The binary records details in daemon.log;
# install failures are written to the dashboard diagnostics file by err().
if ! "$BIN" daemon start >/dev/null 2>&1; then
  printf '%s daemon did not start; open Trace diagnostics in the local dashboard\n' "$(date -u +%FT%TZ)" >> "${HOME}/.trace/install-errors.log" 2>/dev/null || true
fi
if [ ! -f "${HOME}/.trace/install-banner-shown" ]; then
  : > "${HOME}/.trace/install-banner-shown"
  printf '\n  TRACE — see what your AI changes\n'
  printf '  ───────────────────────────────\n'
fi
printf 'to install all integrations, run trc integrations install all\n'
printf 'then start the dashboard with trc dashboard\n'
printf '\nNext steps:\n'
printf '  1. trc integrations install all\n'
printf '  2. Restart the agents you use (Codex may ask you to approve /hooks)\n'
printf '  3. trc dashboard\n'
if [ -f "${HOME}/.trace/daemon.json" ]; then
  port=$(sed -n 's/.*"port"[[:space:]]*:[[:space:]]*\([0-9][0-9]*\).*/\1/p' "${HOME}/.trace/daemon.json")
  [ -n "$port" ] && printf 'dashboard: http://127.0.0.1:%s\n' "$port"
fi
