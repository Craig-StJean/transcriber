#!/usr/bin/env bash
# Automatic update checker for Transcriber release installs.
# Called by the transcriber-update.timer systemd unit (installed only by
# install-remote.sh — source builds from install.sh never enable it).
# Downloads, verifies and installs new releases with desktop notifications.
#
# Auth, in order:
#   1. $CONFIG_DIR/github-token — a fine-grained, read-only token the user
#      created (install-remote.sh writes it with umask 077 when prompted).
#   2. `gh auth token`, read at use time and never written to disk.
set -euo pipefail

REPO="Craig-StJean/transcriber"
DATA_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/transcriber"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/transcriber"
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

for tool in curl jq sha256sum tar; do
    if ! command -v "$tool" &>/dev/null; then
        log "$tool not found — cannot check for updates"
        exit 0
    fi
done

# ── Read current version ─────────────────────────────────────────────────────

VERSION_FILE="$DATA_DIR/version"
if [[ ! -f "$VERSION_FILE" ]]; then
    log "No version file found — skipping update check"
    exit 0
fi
CURRENT="$(cat "$VERSION_FILE")"

# Source builds (install.sh / update.sh) mark their version "+git". Never
# replace a dev build with a release.
if [[ "$CURRENT" == *+* ]]; then
    log "Source build ($CURRENT) — auto-update disabled"
    exit 0
fi

# ── Resolve auth ─────────────────────────────────────────────────────────────

GITHUB_TOKEN=""
TOKEN_FILE="$CONFIG_DIR/github-token"

GH_TOKEN_NOW=""
if command -v gh &>/dev/null; then
    GH_TOKEN_NOW="$(gh auth token 2>/dev/null)" || GH_TOKEN_NOW=""
fi

if [[ -f "$TOKEN_FILE" ]]; then
    GITHUB_TOKEN="$(cat "$TOKEN_FILE")"
    # Older installers copied the full-scope `gh` token here. Drop that copy;
    # gh is asked at use time instead.
    if [[ -n "$GH_TOKEN_NOW" && "$GITHUB_TOKEN" == "$GH_TOKEN_NOW" ]]; then
        rm -f -- "$TOKEN_FILE"
        log "Removed stored copy of the gh CLI token; using gh at run time instead"
    fi
fi
if [[ -z "$GITHUB_TOKEN" || ! -f "$TOKEN_FILE" ]]; then
    GITHUB_TOKEN="$GH_TOKEN_NOW"
fi
unset GH_TOKEN_NOW

if [[ -z "$GITHUB_TOKEN" ]]; then
    log "No GitHub token available — cannot check for updates"
    exit 0
fi

# The Authorization header is fed through a file descriptor so the token
# never appears in the process list.
gh_api() {
    curl -fsSL --connect-timeout 10 --max-time 120 \
        -H @<(printf 'Authorization: Bearer %s\n' "$GITHUB_TOKEN") \
        -H "Accept: application/vnd.github+json" \
        "$@"
}

# ── Check latest release ─────────────────────────────────────────────────────

RELEASE_JSON="$(gh_api "https://api.github.com/repos/$REPO/releases/latest" 2>/dev/null)" || {
    log "Failed to query GitHub API"
    exit 0
}

TAG_NAME="$(jq -r '.tag_name // empty' <<<"$RELEASE_JSON")"
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

NEWER="$(printf '%s\n%s\n' "$CURRENT" "$LATEST" | sort -V | tail -1)"
if [[ "$NEWER" == "$CURRENT" ]]; then
    log "Current $CURRENT is newer than latest release $LATEST — skipping"
    exit 0
fi

log "Update available: $CURRENT → $LATEST"

# ── Download and verify ──────────────────────────────────────────────────────

TARBALL="transcriber-${LATEST}.tar.gz"

# API URL of the asset with exactly this name (works for private repos).
asset_url() {
    jq -r --arg n "$1" '.assets[] | select(.name == $n) | .url' <<<"$RELEASE_JSON" | head -n1
}

TARBALL_URL="$(asset_url "$TARBALL")"
SUMS_URL="$(asset_url "SHA256SUMS")"
if [[ -z "$TARBALL_URL" || -z "$SUMS_URL" ]]; then
    log "Release $TAG_NAME lacks $TARBALL or SHA256SUMS — not installing"
    exit 0
fi

WORK_DIR="$(mktemp -d)"
trap 'rm -rf -- "$WORK_DIR"' EXIT

notify "Updating to v${LATEST}..."

if ! gh_api -H "Accept: application/octet-stream" "$TARBALL_URL" -o "$WORK_DIR/$TARBALL" \
    || ! gh_api -H "Accept: application/octet-stream" "$SUMS_URL" -o "$WORK_DIR/SHA256SUMS"; then
    log "Failed to download release"
    notify "Update failed: could not download v${LATEST}"
    exit 0
fi

# Check only the line for this exact file name; a missing line is a failure.
if ! grep -E "^[0-9a-f]{64}  \*?${TARBALL//./\\.}\$" "$WORK_DIR/SHA256SUMS" > "$WORK_DIR/expected" \
    || ! (cd "$WORK_DIR" && sha256sum --check --status expected); then
    log "Checksum verification failed for $TARBALL — not installing"
    notify "Update failed: v${LATEST} did not pass checksum verification"
    exit 0
fi

tar xzf "$WORK_DIR/$TARBALL" -C "$WORK_DIR" --no-same-owner
RELEASE_DIR="$WORK_DIR/transcriber-${LATEST}"
if [[ ! -d "$RELEASE_DIR/bin" ]]; then
    log "Unexpected tarball structure"
    exit 0
fi

# ── Stop daemon ──────────────────────────────────────────────────────────────

if systemctl --user is-active --quiet transcriber-daemon.service 2>/dev/null; then
    systemctl --user stop transcriber-daemon.service
fi

# ── Install files ────────────────────────────────────────────────────────────

# Install via a temp file + rename so a running binary ("Text file busy")
# is replaced atomically instead of failing.
install_bin() {
    install -m755 "$1" "$BIN_DIR/.$(basename "$1").new"
    mv -f "$BIN_DIR/.$(basename "$1").new" "$BIN_DIR/$(basename "$1")"
}

mkdir -p "$BIN_DIR" "$SYSTEMD_DIR" "$DBUS_SERVICES_DIR" "$APPLICATIONS_DIR"
install_bin "$RELEASE_DIR/bin/transcriber-daemon"
install_bin "$RELEASE_DIR/bin/transcriber-settings"
install_bin "$RELEASE_DIR/bin/transcriber-overlay"

install -m644 "$RELEASE_DIR/deploy/transcriber-daemon.service" "$SYSTEMD_DIR/"
install -m644 "$RELEASE_DIR/deploy/transcriber-update.service" "$SYSTEMD_DIR/"
install -m644 "$RELEASE_DIR/deploy/transcriber-update.timer"   "$SYSTEMD_DIR/"
if [[ -f "$RELEASE_DIR/deploy/transcriber-overlay.service" && -f "$SYSTEMD_DIR/transcriber-overlay.service" ]]; then
    install -m644 "$RELEASE_DIR/deploy/transcriber-overlay.service" "$SYSTEMD_DIR/"
fi
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
cp -r "$RELEASE_DIR/extension/." "$EXT_DIR/"
glib-compile-schemas "$EXT_DIR/schemas/"

# Update checker script
install -m755 "$RELEASE_DIR/scripts/check-update.sh" "$DATA_DIR/"

# ── Restart services ─────────────────────────────────────────────────────────

systemctl --user daemon-reload
systemctl --user start transcriber-daemon.service
systemctl --user try-restart transcriber-overlay.service 2>/dev/null || true

# ── Write version (last!) ────────────────────────────────────────────────────

echo "$LATEST" > "$DATA_DIR/version"

log "Updated to $LATEST"

if [[ "$EXT_CHANGED" == true ]]; then
    notify "Updated to v${LATEST}. Log out and back in to apply extension changes."
else
    notify "Updated to v${LATEST}"
fi
