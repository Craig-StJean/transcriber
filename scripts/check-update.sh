#!/usr/bin/env bash
# Automatic update checker for Transcriber.
# Called by the transcriber-update.timer systemd unit.
# Downloads and installs new releases silently with desktop notifications.
set -euo pipefail

REPO="Craig-StJean/transcriber"
DATA_DIR="$HOME/.local/share/transcriber"
CONFIG_DIR="$HOME/.config/transcriber"
BIN_DIR="$HOME/.local/bin"
SYSTEMD_DIR="$HOME/.config/systemd/user"
DBUS_SERVICES_DIR="$HOME/.local/share/dbus-1/services"
APPLICATIONS_DIR="$HOME/.local/share/applications"
EXT_DIR="$HOME/.local/share/gnome-shell/extensions/transcriber@local"

LOG_TAG="transcriber-update"

log() { echo "$LOG_TAG: $1"; }

notify() {
    # Try notify-send; fall back to just logging
    notify-send -a "Transcriber" "Transcriber" "$1" 2>/dev/null || true
}

# ── Read current version ─────────────────────────────────────────────────────

VERSION_FILE="$DATA_DIR/version"
if [[ ! -f "$VERSION_FILE" ]]; then
    log "No version file found — skipping update check"
    exit 0
fi
CURRENT="$(cat "$VERSION_FILE")"

# ── Resolve auth ─────────────────────────────────────────────────────────────

GITHUB_TOKEN=""
TOKEN_FILE="$CONFIG_DIR/github-token"

if [[ -f "$TOKEN_FILE" ]]; then
    GITHUB_TOKEN="$(cat "$TOKEN_FILE")"
elif command -v gh &>/dev/null; then
    GITHUB_TOKEN="$(gh auth token 2>/dev/null)" || true
fi

if [[ -z "$GITHUB_TOKEN" ]]; then
    log "No GitHub token available — cannot check for updates"
    exit 0
fi

gh_api() {
    curl -fsSL --connect-timeout 10 --max-time 30 \
        -H "Authorization: token $GITHUB_TOKEN" \
        -H "Accept: application/vnd.github+json" \
        "$@"
}

# ── Check latest release ─────────────────────────────────────────────────────

RELEASE_JSON="$(gh_api "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null)" || {
    log "Failed to query GitHub API"
    exit 0
}

TAG_NAME="$(echo "$RELEASE_JSON" | grep -oP '"tag_name"\s*:\s*"\K[^"]+')"
LATEST="${TAG_NAME#v}"

if [[ -z "$LATEST" ]]; then
    log "Could not parse latest version"
    exit 0
fi

# Compare versions
if [[ "$CURRENT" == "$LATEST" ]]; then
    log "Already at $CURRENT — no update needed"
    exit 0
fi

NEWER="$(printf '%s\n%s' "$CURRENT" "$LATEST" | sort -V | tail -1)"
if [[ "$NEWER" == "$CURRENT" ]]; then
    log "Current $CURRENT is newer than latest release $LATEST — skipping"
    exit 0
fi

log "Update available: $CURRENT → $LATEST"
notify "Updating to v${LATEST}..."

# ── Download and extract ─────────────────────────────────────────────────────

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

ASSET_URL="$(echo "$RELEASE_JSON" | grep -oP '"url"\s*:\s*"\K[^"]+assets/[0-9]+')"

gh_api -H "Accept: application/octet-stream" "$ASSET_URL" \
    -o "$TMPDIR/release.tar.gz" || {
    # Fallback to browser_download_url
    BROWSER_URL="$(echo "$RELEASE_JSON" | grep -oP '"browser_download_url"\s*:\s*"\K[^"]+\.tar\.gz')"
    gh_api "$BROWSER_URL" -o "$TMPDIR/release.tar.gz" || {
        log "Failed to download release"
        notify "Update failed: could not download v${LATEST}"
        exit 0
    }
}

tar xzf "$TMPDIR/release.tar.gz" -C "$TMPDIR"
RELEASE_DIR="$(find "$TMPDIR" -maxdepth 1 -type d -name 'transcriber-*' | head -1)"
if [[ ! -d "$RELEASE_DIR" ]]; then
    log "Unexpected tarball structure"
    exit 0
fi

# ── Stop daemon ──────────────────────────────────────────────────────────────

if systemctl --user is-active --quiet transcriber-daemon.service 2>/dev/null; then
    systemctl --user stop transcriber-daemon.service
fi

# ── Install files ────────────────────────────────────────────────────────────

install -m755 "$RELEASE_DIR/bin/transcriber-daemon"   "$BIN_DIR/"
install -m755 "$RELEASE_DIR/bin/transcriber-settings"  "$BIN_DIR/" 2>/dev/null || {
    rm -f "$BIN_DIR/transcriber-settings"
    install -m755 "$RELEASE_DIR/bin/transcriber-settings" "$BIN_DIR/"
}
install -m755 "$RELEASE_DIR/bin/transcriber-overlay"   "$BIN_DIR/" 2>/dev/null || {
    rm -f "$BIN_DIR/transcriber-overlay"
    install -m755 "$RELEASE_DIR/bin/transcriber-overlay" "$BIN_DIR/"
}

install -m644 "$RELEASE_DIR/deploy/transcriber-daemon.service" "$SYSTEMD_DIR/"
install -m644 "$RELEASE_DIR/deploy/transcriber-update.service" "$SYSTEMD_DIR/"
install -m644 "$RELEASE_DIR/deploy/transcriber-update.timer"   "$SYSTEMD_DIR/"
install -m644 "$RELEASE_DIR/deploy/org.transcriber.Daemon.service"   "$DBUS_SERVICES_DIR/"
install -m644 "$RELEASE_DIR/deploy/org.transcriber.Settings.desktop"  "$APPLICATIONS_DIR/"

# Extension
EXT_CHANGED=false
if [[ -d "$EXT_DIR" ]]; then
    if ! diff -rq "$RELEASE_DIR/extension/" "$EXT_DIR/" >/dev/null 2>&1; then
        EXT_CHANGED=true
    fi
fi
mkdir -p "$EXT_DIR/schemas"
cp -r "$RELEASE_DIR/extension/"* "$EXT_DIR/"
glib-compile-schemas "$EXT_DIR/schemas/"

# Update checker script
install -m755 "$RELEASE_DIR/scripts/check-update.sh" "$DATA_DIR/"

# ── Restart services ─────────────────────────────────────────────────────────

systemctl --user daemon-reload
systemctl --user start transcriber-daemon.service

# ── Write version (last!) ────────────────────────────────────────────────────

echo "$LATEST" > "$DATA_DIR/version"

log "Updated to $LATEST"

if [[ "$EXT_CHANGED" == true ]]; then
    notify "Updated to v${LATEST}. Log out and back in to apply extension changes."
else
    notify "Updated to v${LATEST}"
fi
