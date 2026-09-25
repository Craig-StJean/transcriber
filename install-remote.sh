#!/usr/bin/env bash
# One-command installer for Transcriber.
# Downloads pre-built binaries from a GitHub Release and installs everything.
#
# Usage:
#   # With gh CLI already authenticated:
#   bash install-remote.sh
#
#   # With a GitHub token (not stored):
#   GITHUB_TOKEN=github_pat_xxx bash install-remote.sh
#
# Auto-updates use either ~/.config/transcriber/github-token (a fine-grained,
# read-only token — written only when you paste one at the prompt below) or
# `gh auth token` at run time. The gh token itself is never written to disk.
#
#   # Install a specific version:
#   VERSION=v0.2.0 bash install-remote.sh
set -euo pipefail

REPO="Craig-StJean/transcriber"

BIN_DIR="$HOME/.local/bin"
SYSTEMD_DIR="$HOME/.config/systemd/user"
DBUS_SERVICES_DIR="$HOME/.local/share/dbus-1/services"
APPLICATIONS_DIR="$HOME/.local/share/applications"
EXT_DIR="$HOME/.local/share/gnome-shell/extensions/transcriber@local"
DATA_DIR="$HOME/.local/share/transcriber"
CONFIG_DIR="$HOME/.config/transcriber"

bold="\033[1m"
dim="\033[2m"
green="\033[32m"
red="\033[31m"
yellow="\033[33m"
cyan="\033[36m"
reset="\033[0m"

info()  { echo -e "${cyan}${bold}::${reset}${bold} $1${reset}"; }
ok()    { echo -e "   ${green}done${reset} ${dim}$1${reset}"; }
warn()  { echo -e "   ${yellow}warn${reset} $1"; }
err()   { echo -e "${red}${bold}error:${reset} $1" >&2; exit 1; }

# ── Environment checks ──────────────────────────────────────────────────────

info "Checking environment..."

if ! grep -qi fedora /etc/os-release 2>/dev/null; then
    warn "This installer is designed for Fedora. Proceeding anyway..."
fi

# ── Install runtime dependencies ─────────────────────────────────────────────

info "Installing runtime dependencies (may prompt for sudo)..."
sudo dnf install -y gtk4 libadwaita glib2 alsa-lib glib2-devel gtk4-layer-shell jq >/dev/null 2>&1 || {
    warn "Some packages may not have installed. Continuing..."
}
ok "system dependencies"

for tool in curl jq sha256sum tar; do
    command -v "$tool" &>/dev/null || err "$tool is required but not installed."
done

# ── Resolve GitHub authentication ────────────────────────────────────────────

TOKEN_FILE="$CONFIG_DIR/github-token"

resolve_auth() {
    local gh_token=""
    if command -v gh &>/dev/null && gh auth status &>/dev/null; then
        gh_token="$(gh auth token 2>/dev/null)" || gh_token=""
    fi

    # Older installers stored a copy of the full-scope gh token. Remove it;
    # gh is asked at run time instead.
    if [[ -f "$TOKEN_FILE" && -n "$gh_token" && "$(cat "$TOKEN_FILE")" == "$gh_token" ]]; then
        rm -f -- "$TOKEN_FILE"
        warn "removed stored copy of your gh CLI token (auto-updates now ask gh directly)"
    fi

    # 1. Env var (used for this run only, never stored)
    if [[ -n "${GITHUB_TOKEN:-}" ]]; then
        return 0
    fi

    # 2. User-provided fine-grained token from a previous install
    if [[ -f "$TOKEN_FILE" ]]; then
        GITHUB_TOKEN="$(cat "$TOKEN_FILE")"
        return 0
    fi

    # 3. gh CLI (read at use time, never stored)
    if [[ -n "$gh_token" ]]; then
        GITHUB_TOKEN="$gh_token"
        return 0
    fi

    # 4. Interactive prompt — stored for auto-updates, readable only by you
    echo ""
    echo -e "${yellow}This is a private repository. A GitHub token with repo read access is required.${reset}"
    echo "Create one at: https://github.com/settings/tokens"
    echo "(Select: Fine-grained → Only select repositories → ${REPO} → Contents: Read-only)"
    echo ""
    read -rsp "GitHub token: " GITHUB_TOKEN
    echo ""
    if [[ -z "$GITHUB_TOKEN" ]]; then
        err "No token provided. Cannot continue."
    fi
    mkdir -p "$CONFIG_DIR"
    (umask 077; printf '%s\n' "$GITHUB_TOKEN" > "$TOKEN_FILE")
    ok "token saved to $TOKEN_FILE (mode 0600) for auto-updates"
}

resolve_auth

# The Authorization header is fed through a file descriptor so the token
# never appears in the process list.
gh_api() {
    curl -fsSL \
        -H @<(printf 'Authorization: Bearer %s\n' "$GITHUB_TOKEN") \
        -H "Accept: application/vnd.github+json" \
        "$@"
}

# ── Determine version to install ─────────────────────────────────────────────

info "Querying latest release..."

if [[ -n "${VERSION:-}" ]]; then
    RELEASE_URL="https://api.github.com/repos/$REPO/releases/tags/$VERSION"
else
    RELEASE_URL="https://api.github.com/repos/$REPO/releases/latest"
fi

RELEASE_JSON="$(gh_api "$RELEASE_URL")" || err "Failed to query GitHub API. Check your token and network."

TAG_NAME="$(jq -r '.tag_name // empty' <<<"$RELEASE_JSON")"
[[ -n "$TAG_NAME" ]] || err "Could not parse the release tag."

INSTALL_VERSION="${TAG_NAME#v}"
ok "found $TAG_NAME"

# Check if already at this version
if [[ -f "$DATA_DIR/version" ]]; then
    CURRENT="$(cat "$DATA_DIR/version")"
    if [[ "$CURRENT" == "$INSTALL_VERSION" ]]; then
        echo -e "\n${green}Already at version $INSTALL_VERSION. Nothing to do.${reset}"
        exit 0
    fi
    info "Upgrading from $CURRENT → $INSTALL_VERSION"
fi

# ── Download and verify release ──────────────────────────────────────────────

TARBALL="transcriber-${INSTALL_VERSION}.tar.gz"

# API URL of the asset with exactly this name (works for private repos).
asset_url() {
    jq -r --arg n "$1" '.assets[] | select(.name == $n) | .url' <<<"$RELEASE_JSON" | head -n1
}

TARBALL_URL="$(asset_url "$TARBALL")"
SUMS_URL="$(asset_url "SHA256SUMS")"
[[ -n "$TARBALL_URL" ]] || err "Release $TAG_NAME has no $TARBALL asset."
[[ -n "$SUMS_URL" ]]    || err "Release $TAG_NAME has no SHA256SUMS asset — refusing to install unverified binaries."

WORK_DIR="$(mktemp -d)"
trap 'rm -rf -- "$WORK_DIR"' EXIT

info "Downloading $TAG_NAME..."
gh_api -H "Accept: application/octet-stream" "$TARBALL_URL" -o "$WORK_DIR/$TARBALL" \
    || err "Failed to download $TARBALL"
gh_api -H "Accept: application/octet-stream" "$SUMS_URL" -o "$WORK_DIR/SHA256SUMS" \
    || err "Failed to download SHA256SUMS"
ok "downloaded"

info "Verifying checksum..."
grep -E "^[0-9a-f]{64}  \*?${TARBALL//./\\.}\$" "$WORK_DIR/SHA256SUMS" > "$WORK_DIR/expected" \
    || err "SHA256SUMS has no entry for $TARBALL"
(cd "$WORK_DIR" && sha256sum --check --status expected) \
    || err "Checksum mismatch for $TARBALL — refusing to install."
ok "sha256 verified"

info "Extracting..."
tar xzf "$WORK_DIR/$TARBALL" -C "$WORK_DIR" --no-same-owner
RELEASE_DIR="$WORK_DIR/transcriber-${INSTALL_VERSION}"
[[ -d "$RELEASE_DIR/bin" ]] || err "Unexpected tarball structure"
ok "extracted"

# ── Stop daemon if upgrading ─────────────────────────────────────────────────

if systemctl --user is-active --quiet transcriber-daemon.service 2>/dev/null; then
    info "Stopping daemon for upgrade..."
    systemctl --user stop transcriber-daemon.service
    ok "daemon stopped"
fi

# ── Install files ────────────────────────────────────────────────────────────

info "Installing binaries to $BIN_DIR..."
mkdir -p "$BIN_DIR"
# Temp file + rename, so a running binary is replaced instead of "Text file busy".
install_bin() {
    install -m755 "$1" "$BIN_DIR/.$(basename "$1").new"
    mv -f "$BIN_DIR/.$(basename "$1").new" "$BIN_DIR/$(basename "$1")"
}
install_bin "$RELEASE_DIR/bin/transcriber-daemon"
install_bin "$RELEASE_DIR/bin/transcriber-settings"
install_bin "$RELEASE_DIR/bin/transcriber-overlay"
ok ""

info "Installing systemd services..."
mkdir -p "$SYSTEMD_DIR"
install -m644 "$RELEASE_DIR/deploy/transcriber-daemon.service" "$SYSTEMD_DIR/"
install -m644 "$RELEASE_DIR/deploy/transcriber-update.service" "$SYSTEMD_DIR/"
install -m644 "$RELEASE_DIR/deploy/transcriber-update.timer"   "$SYSTEMD_DIR/"
# Overlay unit is installed but not enabled — it's only for wlroots/KDE.
install -m644 "$RELEASE_DIR/deploy/transcriber-overlay.service" "$SYSTEMD_DIR/"
ok ""

info "Installing DBus activation file..."
mkdir -p "$DBUS_SERVICES_DIR"
install -m644 "$RELEASE_DIR/deploy/org.transcriber.Daemon.service" "$DBUS_SERVICES_DIR/"
ok ""

info "Installing .desktop file..."
mkdir -p "$APPLICATIONS_DIR"
install -m644 "$RELEASE_DIR/deploy/org.transcriber.Settings.desktop" "$APPLICATIONS_DIR/"
update-desktop-database "$APPLICATIONS_DIR/" 2>/dev/null || true
ok ""

info "Installing GNOME extension..."
mkdir -p "$EXT_DIR/schemas"
cp -r "$RELEASE_DIR/extension/." "$EXT_DIR/"
ok ""

info "Compiling GSettings schema..."
glib-compile-schemas "$EXT_DIR/schemas/"
ok ""

info "Installing update checker..."
mkdir -p "$DATA_DIR"
install -m755 "$RELEASE_DIR/scripts/check-update.sh" "$DATA_DIR/"
ok ""

# Write version file last (so partial installs retry)
echo "$INSTALL_VERSION" > "$DATA_DIR/version"

# ── Enable services ──────────────────────────────────────────────────────────

info "Enabling services..."
systemctl --user daemon-reload
systemctl --user enable --now transcriber-daemon.service
systemctl --user enable --now transcriber-update.timer
systemctl --user try-restart transcriber-overlay.service 2>/dev/null || true
ok ""

# ── Done ─────────────────────────────────────────────────────────────────────

echo ""
echo -e "${green}${bold}Transcriber $TAG_NAME installed successfully!${reset}"
echo ""
echo "Next steps:"
echo "  1. Open 'Transcriber Settings' from the app launcher, or run: transcriber-settings"
if [[ "${XDG_CURRENT_DESKTOP:-}" != *GNOME* ]]; then
    echo "  2. Not a GNOME session: enable the on-screen overlay + hotkey instead of the extension:"
    echo "       systemctl --user enable --now transcriber-overlay.service"
elif [ "${XDG_SESSION_TYPE:-}" = "wayland" ]; then
    echo "  2. Log out and back in so GNOME Shell picks up the extension."
    echo "     Then enable it:  gnome-extensions enable transcriber@local"
    echo "     (Wayland can't reload the shell in place — logout is required.)"
else
    echo "  2. Restart GNOME Shell with Alt+F2 → 'r', then enable the extension:"
    echo "       gnome-extensions enable transcriber@local"
fi
echo "  3. Press Super+' (Super + apostrophe) to start recording."
echo ""
echo -e "${dim}Auto-updates are enabled. The app will check for updates daily.${reset}"
