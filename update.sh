#!/usr/bin/env bash
# Quick update: rebuild changed components, install binaries, restart daemon.
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="$HOME/.local/bin"

bold="\033[1m"
dim="\033[2m"
green="\033[32m"
yellow="\033[33m"
cyan="\033[36m"
reset="\033[0m"

step() { echo -e "\n${cyan}${bold}::${reset}${bold} $1${reset}"; }
ok()   { echo -e "   ${green}done${reset} ${dim}$1${reset}"; }
skip() { echo -e "   ${yellow}skip${reset} ${dim}$1${reset}"; }

echo -e "${bold}Voice Transcriber${reset} ${dim}update${reset}"

# ── Build ────────────────────────────────────────────────────────────────────

step "Building release binaries..."
cargo build --release \
    -p voice-transcriber-daemon \
    -p voice-transcriber-settings \
    --manifest-path "$REPO/Cargo.toml" 2>&1 | tail -1
ok "compiled"

# ── Daemon ───────────────────────────────────────────────────────────────────

DAEMON_SRC="$REPO/target/release/voice-transcriber-daemon"
DAEMON_DST="$BIN_DIR/voice-transcriber-daemon"

step "Updating daemon..."
if [ -f "$DAEMON_DST" ] && cmp -s "$DAEMON_SRC" "$DAEMON_DST"; then
    skip "binary unchanged"
else
    install -m755 "$DAEMON_SRC" "$DAEMON_DST"
    ok "binary installed"
    systemctl --user daemon-reload
    systemctl --user restart voice-transcriber-daemon.service
    ok "daemon restarted"
fi

# ── Settings app ─────────────────────────────────────────────────────────────

APP_SRC="$REPO/target/release/voice-transcriber-settings"
APP_DST="$BIN_DIR/voice-transcriber-settings"

step "Updating settings app..."
if [ -f "$APP_DST" ] && cmp -s "$APP_SRC" "$APP_DST"; then
    skip "binary unchanged"
else
    # Handle "Text file busy" if the app is currently running
    if ! install -m755 "$APP_SRC" "$APP_DST" 2>/dev/null; then
        rm -f "$APP_DST"
        install -m755 "$APP_SRC" "$APP_DST"
    fi
    ok "binary installed (restart the app to use it)"
fi

# ── GNOME extension ─────────────────────────────────────────────────────────

EXT_DIR="$HOME/.local/share/gnome-shell/extensions/voice-transcriber@local"

step "Updating GNOME extension..."
if [ -d "$EXT_DIR" ]; then
    cp -r "$REPO/extension/"* "$EXT_DIR/"
    glib-compile-schemas "$EXT_DIR/schemas/"
    ok "extension files synced"
else
    skip "extension not installed"
fi

# ── Done ─────────────────────────────────────────────────────────────────────

echo -e "\n${green}${bold}All up to date.${reset}\n"
