#!/usr/bin/env bash
# Quick update for a source install: pull, rebuild, reinstall binaries and
# deploy files, restart what changed. Idempotent — safe to run repeatedly.
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="$HOME/.local/bin"
SYSTEMD_DIR="$HOME/.config/systemd/user"
DBUS_SERVICES_DIR="$HOME/.local/share/dbus-1/services"
APPLICATIONS_DIR="$HOME/.local/share/applications"
EXT_DIR="$HOME/.local/share/gnome-shell/extensions/transcriber@local"
DATA_DIR="$HOME/.local/share/transcriber"

bold="\033[1m"
dim="\033[2m"
green="\033[32m"
yellow="\033[33m"
cyan="\033[36m"
reset="\033[0m"

step() { echo -e "\n${cyan}${bold}::${reset}${bold} $1${reset}"; }
ok()   { echo -e "   ${green}done${reset} ${dim}$1${reset}"; }
skip() { echo -e "   ${yellow}skip${reset} ${dim}$1${reset}"; }

# install_file MODE SRC DEST_DIR — install only if different.
# Returns 0 if the file changed, 1 if it was already up to date.
# Uses temp file + rename so a running binary is replaced, not "Text file busy".
install_file() {
    local mode="$1" src="$2" dir="$3" name dst
    name="$(basename "$src")"
    dst="$dir/$name"
    if [[ -f "$dst" ]] && cmp -s "$src" "$dst"; then
        return 1
    fi
    mkdir -p "$dir"
    install -m"$mode" "$src" "$dir/.$name.new"
    mv -f "$dir/.$name.new" "$dst"
    return 0
}

echo -e "${bold}Transcriber${reset} ${dim}update${reset}"

# ── Pull latest ──────────────────────────────────────────────────────────────

step "Pulling latest changes..."
if git -C "$REPO" pull --ff-only 2>&1 | tail -1; then
    ok "up to date"
else
    skip "pull failed (check manually)"
fi

# ── Build ────────────────────────────────────────────────────────────────────

# The overlay is only rebuilt if it was installed (non-GNOME sessions).
OVERLAY_INSTALLED=false
[[ -f "$BIN_DIR/transcriber-overlay" ]] && OVERLAY_INSTALLED=true

PACKAGES=(-p transcriber-daemon -p transcriber-settings)
[[ "$OVERLAY_INSTALLED" == true ]] && PACKAGES+=(-p transcriber-overlay)

step "Building release binaries..."
cargo build --release "${PACKAGES[@]}" --manifest-path "$REPO/Cargo.toml" 2>&1 | tail -1
ok "compiled"

# ── Binaries ─────────────────────────────────────────────────────────────────

DAEMON_CHANGED=false
OVERLAY_CHANGED=false

step "Updating binaries..."
if install_file 755 "$REPO/target/release/transcriber-daemon" "$BIN_DIR"; then
    DAEMON_CHANGED=true
    ok "daemon installed"
else
    skip "daemon unchanged"
fi
if install_file 755 "$REPO/target/release/transcriber-settings" "$BIN_DIR"; then
    ok "settings app installed (restart the app to use it)"
else
    skip "settings app unchanged"
fi
if [[ "$OVERLAY_INSTALLED" == true ]]; then
    if install_file 755 "$REPO/target/release/transcriber-overlay" "$BIN_DIR"; then
        OVERLAY_CHANGED=true
        ok "overlay installed"
    else
        skip "overlay unchanged"
    fi
fi

# ── Deploy files (units, DBus activation, .desktop) ─────────────────────────

step "Updating service files..."
if install_file 644 "$REPO/deploy/transcriber-daemon.service" "$SYSTEMD_DIR"; then
    DAEMON_CHANGED=true
    ok "daemon unit"
fi
if [[ "$OVERLAY_INSTALLED" == true ]] \
    && install_file 644 "$REPO/deploy/transcriber-overlay.service" "$SYSTEMD_DIR"; then
    OVERLAY_CHANGED=true
    ok "overlay unit"
fi
if install_file 644 "$REPO/deploy/org.transcriber.Daemon.service" "$DBUS_SERVICES_DIR"; then
    ok "DBus activation file"
fi
if install_file 644 "$REPO/deploy/org.transcriber.Settings.desktop" "$APPLICATIONS_DIR"; then
    update-desktop-database "$APPLICATIONS_DIR/" 2>/dev/null || true
    ok ".desktop file"
fi

systemctl --user daemon-reload
ok "systemd reloaded"

# ── Restart what changed ────────────────────────────────────────────────────

step "Restarting services..."
if [[ "$DAEMON_CHANGED" == true ]]; then
    # try-restart: only if running; otherwise DBus activation starts the new one.
    systemctl --user try-restart transcriber-daemon.service
    ok "daemon restarted"
else
    skip "daemon unchanged"
fi
if [[ "$OVERLAY_CHANGED" == true ]]; then
    systemctl --user try-restart transcriber-overlay.service
    ok "overlay restarted"
fi

# ── GNOME extension ─────────────────────────────────────────────────────────

step "Updating GNOME extension..."
if [[ -d "$EXT_DIR" ]]; then
    cp -r "$REPO/extension/." "$EXT_DIR/"
    glib-compile-schemas "$EXT_DIR/schemas/"
    ok "extension files synced"
else
    skip "extension not installed"
fi

# ── Version file ────────────────────────────────────────────────────────────

step "Updating version file..."
mkdir -p "$DATA_DIR"
VERSION="$(grep -m1 '^version' "$REPO/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/')+git"
echo "$VERSION" > "$DATA_DIR/version"
ok "$VERSION"

# ── Release auto-updater ────────────────────────────────────────────────────

# A source install must not be overwritten by release builds.
if systemctl --user is-enabled --quiet transcriber-update.timer 2>/dev/null; then
    systemctl --user disable --now transcriber-update.timer
    ok "disabled release auto-update timer (this is a source install)"
fi

# ── Done ─────────────────────────────────────────────────────────────────────

echo -e "\n${green}${bold}All up to date.${reset}\n"
