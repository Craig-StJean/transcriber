#!/usr/bin/env bash
# Install transcriber from source for the current user.
# Run from anywhere; paths are resolved relative to this script.
#
# Source installs never auto-update: the release updater timer is left
# disabled (and disabled if a previous release install enabled it), and the
# version file is marked "+git" so check-update.sh refuses to replace it.
# Use `bash update.sh` to update a source install.
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="$HOME/.local/bin"
SYSTEMD_DIR="$HOME/.config/systemd/user"
DBUS_SERVICES_DIR="$HOME/.local/share/dbus-1/services"
APPLICATIONS_DIR="$HOME/.local/share/applications"
EXT_DIR="$HOME/.local/share/gnome-shell/extensions/transcriber@local"
DATA_DIR="$HOME/.local/share/transcriber"

# The wlroots/KDE overlay replaces the GNOME extension on other desktops.
IS_GNOME=false
[[ "${XDG_CURRENT_DESKTOP:-}" == *GNOME* ]] && IS_GNOME=true

PACKAGES=(-p transcriber-daemon -p transcriber-settings)
if [[ "$IS_GNOME" == false ]]; then
    PACKAGES+=(-p transcriber-overlay)
fi

echo "==> Building binaries (release)..."
cargo build --release "${PACKAGES[@]}" --manifest-path "$REPO/Cargo.toml"

# Temp file + rename, so a running binary is replaced instead of "Text file busy".
install_bin() {
    install -m755 "$1" "$BIN_DIR/.$(basename "$1").new"
    mv -f "$BIN_DIR/.$(basename "$1").new" "$BIN_DIR/$(basename "$1")"
}

echo "==> Installing binaries to $BIN_DIR..."
mkdir -p "$BIN_DIR"
install_bin "$REPO/target/release/transcriber-daemon"
install_bin "$REPO/target/release/transcriber-settings"
if [[ "$IS_GNOME" == false ]]; then
    install_bin "$REPO/target/release/transcriber-overlay"
fi

echo "==> Installing systemd user services..."
mkdir -p "$SYSTEMD_DIR"
install -m644 "$REPO/deploy/transcriber-daemon.service" "$SYSTEMD_DIR/"
if [[ "$IS_GNOME" == false ]]; then
    # Installed, not enabled — see the note printed at the end.
    install -m644 "$REPO/deploy/transcriber-overlay.service" "$SYSTEMD_DIR/"
fi

echo "==> Installing DBus activation file..."
mkdir -p "$DBUS_SERVICES_DIR"
install -m644 "$REPO/deploy/org.transcriber.Daemon.service" "$DBUS_SERVICES_DIR/"

echo "==> Installing .desktop file..."
mkdir -p "$APPLICATIONS_DIR"
install -m644 "$REPO/deploy/org.transcriber.Settings.desktop" "$APPLICATIONS_DIR/"
update-desktop-database "$APPLICATIONS_DIR/" 2>/dev/null || true

echo "==> Installing GNOME extension..."
mkdir -p "$EXT_DIR/schemas"
cp -r "$REPO/extension/." "$EXT_DIR/"

echo "==> Compiling GSettings schema..."
glib-compile-schemas "$EXT_DIR/schemas/"

echo "==> Writing version file..."
mkdir -p "$DATA_DIR"
VERSION="$(grep -m1 '^version' "$REPO/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/')"
echo "${VERSION}+git" > "$DATA_DIR/version"

echo "==> Reloading systemd and enabling services..."
systemctl --user daemon-reload
systemctl --user enable transcriber-daemon.service
# Pick up the freshly installed binary if the daemon was already running.
systemctl --user restart transcriber-daemon.service

if systemctl --user is-enabled --quiet transcriber-update.timer 2>/dev/null; then
    systemctl --user disable --now transcriber-update.timer
    echo "    Note: disabled transcriber-update.timer from a previous release install,"
    echo "    so release builds no longer overwrite this source build."
    echo "    Use 'bash update.sh' to update; re-run install-remote.sh to go back to releases."
fi

echo ""
echo "Done!  Next steps:"
echo "  1. Open 'Transcriber Settings' from the app launcher, or run: transcriber-settings"
if [[ "$IS_GNOME" == false ]]; then
    echo "  2. Not a GNOME session: enable the on-screen overlay + global hotkey:"
    echo "       systemctl --user enable --now transcriber-overlay.service"
    echo "     (needs a compositor with wlr-layer-shell: Sway, Hyprland, KDE Plasma 6, …)"
elif [ "${XDG_SESSION_TYPE:-}" = "wayland" ]; then
    echo "  2. Log out and back in so GNOME Shell picks up the extension."
    echo "     Then enable it:  gnome-extensions enable transcriber@local"
    echo "     (Wayland can't reload the shell in place — logout is required.)"
else
    echo "  2. Restart GNOME Shell with Alt+F2 → 'r', then enable the extension:"
    echo "       gnome-extensions enable transcriber@local"
fi
echo "  3. Press Super+' (Super + apostrophe) to start recording."
echo "     (Rebind in: Transcriber Settings, or GNOME Settings → Keyboard → Custom Shortcuts)"
