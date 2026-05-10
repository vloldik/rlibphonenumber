#!/usr/bin/env bash
set -euo pipefail

# ─────────────────────────────────────────────────────────────
# rlibphonenumber_cli installer
# ─────────────────────────────────────────────────────────────

REPO="vloldik/rlibphonenumber"
BIN_NAME="rpn"
TAG_PREFIX="rlibphonenumber_cli-"
API_BASE="https://api.github.com/repos/${REPO}/releases"

VERSION="${1:-}" # optional: "v0.1.0" or "0.1.0"

# ── Terminal output ──────────────────────────────────────────

info()    { printf '\033[1;34m[info]\033[0m %s\n' "$*"; }
success() { printf '\033[1;32m[done]\033[0m  %s\n' "$*"; }
warn()    { printf '\033[1;33m[warn]\033[0m %s\n' "$*"; }
error()   { printf '\033[1;31m[error]\033[0m %s\n' "$*" >&2; exit 1; }

need() { 
  command -v "$1" &>/dev/null || error "'$1' is required but not found in PATH."
}

# ── JSON parsing: jq → python3 → python ─────────────────────

json_field() {
  local json="$1" key="$2"
  if command -v jq &>/dev/null; then
    printf '%s' "$json" | jq -r ".$key // empty"
  elif command -v python3 &>/dev/null; then
    printf '%s' "$json" | python3 -c \
      "import sys,json; v=json.load(sys.stdin).get('$key',''); print(v if v else '', end='')"
  elif command -v python &>/dev/null; then
    printf '%s' "$json" | python -c \
      "import sys,json; v=json.load(sys.stdin).get('$key',''); print(v if v else '', end='')"
  else
    error "jq or python is required to parse GitHub API responses. Install one and retry."
  fi
}

# ── Platform detection ───────────────────────────────────────

detect_os() {
  case "$(uname -s)" in
    Linux*) echo linux ;;
    Darwin*) echo macos ;;
    MINGW*|MSYS*|CYGWIN*) echo windows ;;
    *) error "Unsupported OS: $(uname -s)" ;;
  esac
}

detect_arch() {
  case "$(uname -m)" in
    x86_64|amd64) echo x86_64 ;;
    aarch64|arm64) echo aarch64 ;;
    *) error "Unsupported architecture: $(uname -m)" ;;
  esac
}

asset_name() {
  local os="$1" arch="$2"
  case "${os}-${arch}" in
    linux-x86_64) echo "rpn-x86_64-unknown-linux-gnu" ;;
    linux-aarch64) echo "rpn-aarch64-unknown-linux-gnu" ;;
    macos-x86_64) echo "rpn-x86_64-apple-darwin" ;;
    macos-aarch64) echo "rpn-aarch64-apple-darwin" ;;
    windows-x86_64) echo "rpn-x86_64-pc-windows-msvc.exe" ;;
    windows-aarch64) echo "rpn-aarch64-pc-windows-msvc.exe" ;;
    *) error "No binary available for ${os}/${arch}" ;;
  esac
}

# ── Install directory ────────────────────────────────────────

resolve_install_dir() {
  if [ "${EUID:-$(id -u)}" -eq 0 ] ||[ -w "/usr/local/bin" ]; then
    echo "/usr/local/bin"
  else
    echo "${HOME}/.local/bin"
  fi
}

ensure_on_path() {
  local dir="$1"
  echo ":${PATH}:" | grep -q ":${dir}:" && return

  local profile
  if [ "$(basename "${SHELL:-sh}")" = "zsh" ] ||[ -n "${ZSH_VERSION:-}" ]; then
    profile="${HOME}/.zshrc"
  elif [ -f "${HOME}/.bash_profile" ]; then
    profile="${HOME}/.bash_profile"
  else
    profile="${HOME}/.bashrc"
  fi

  printf '\n# added by rlibphonenumber_cli installer\nexport PATH="%s:${PATH}"\n' "$dir" >> "$profile"
  warn "${dir} is not in PATH — added to ${profile}"
  warn "Restart your shell or run: export PATH=\"${dir}:\$PATH\""
}

# ── Checksum: parsed from release body at install time ───────

extract_checksum() {
  local body="$1" asset="$2"
  # FIXED: Added `|| true` so grep failing doesn't kill the script under `set -e`
  printf '%s' "$body" \
    | grep -A1 "${asset}" \
    | grep -o 'sha256:[a-f0-9]*' \
    | head -1 \
    | cut -d: -f2 || true
}

verify_checksum() {
  local file="$1" expected="$2" asset="$3"

  if [ -z "$expected" ]; then
    warn "Checksum not found in release notes for '${asset}' — skipping verification."
    return
  fi

  local actual
  if command -v sha256sum &>/dev/null; then
    actual=$(sha256sum "$file" | awk '{print $1}')
  elif command -v shasum &>/dev/null; then
    actual=$(shasum -a 256 "$file" | awk '{print $1}')
  else
    warn "sha256sum/shasum not available — skipping verification."
    return
  fi

  if [ "$actual" != "$expected" ]; then
    rm -f "$file"
    error "Checksum mismatch for '${asset}'!\n expected: ${expected}\n got:      ${actual}"
  fi
  info "Checksum OK ✓"
}

# ── Main ─────────────────────────────────────────────────────

main() {
  need curl

  local os arch asset install_dir dest api_url release_json tag body checksum download_url tmp

  os=$(detect_os)
  arch=$(detect_arch)
  asset=$(asset_name "$os" "$arch")
  install_dir=$(resolve_install_dir)
  dest="${install_dir}/${BIN_NAME}"
  
  if [ "$os" = "windows" ]; then
    dest="${dest}.exe"
  fi

  # ── Choose release endpoint ──────────────────────────────────
  if [ -z "$VERSION" ]; then
    api_url="${API_BASE}/latest"
    info "Fetching latest release…"
  else
    local ver="${VERSION#v}"
    api_url="${API_BASE}/tags/${TAG_PREFIX}v${ver}"
    info "Fetching release ${TAG_PREFIX}v${ver}…"
  fi

  # ── Call GitHub API ──────────────────────────────────────────
  release_json=$(curl -fsSL \
    -H "Accept: application/vnd.github+json" \
    -H "X-GitHub-Api-Version: 2022-11-28" \
    "$api_url") || error "Failed to reach GitHub API. Check your connection and version string."

  tag=$(json_field "$release_json" "tag_name")
  body=$(json_field "$release_json" "body")

  [ -z "$tag" ] && error "Release not found. Does version '${VERSION}' exist?"

  info "Tag : ${tag}"
  info "Platform : ${os}/${arch}"
  info "Asset : ${asset}"
  info "Dest : ${dest}"

  # ── Extract checksum live from release body ──────────────────
  checksum=$(extract_checksum "$body" "$asset")

  # ── Download binary ──────────────────────────────────────────
  download_url="https://github.com/${REPO}/releases/download/${tag}/${asset}"
  info "Downloading ${download_url} …"

  tmp=$(mktemp)
  trap 'rm -f "$tmp"' EXIT
  curl -fsSL --progress-bar -o "$tmp" "$download_url"

  verify_checksum "$tmp" "$checksum" "$asset"

  # ── Place binary ─────────────────────────────────────────────
  chmod +x "$tmp"
  mkdir -p "$install_dir"
  mv "$tmp" "$dest"

  ensure_on_path "$install_dir"

  success "Installed ${tag} → ${dest}"
  success "Run '${BIN_NAME} --help' to get started."
}

main "$@"