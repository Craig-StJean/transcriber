#!/usr/bin/env bash
# Install voice-transcriber daemon + GNOME extension for the current user.
# Run from the repository root.
set -euo pipefail

REPO="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN_DIR="$HOME/.local/bin"
SYSTEMD_DIR="$HOME/.config/systemd/user"
DBUS_SERVICES_DIR="$HOME/.local/share/dbus-1/services"
APPLICATIONS_DIR="$HOME/.local/share/applications"
EXT_DIR="$HOME/.local/share/gnome-shell/extensions/voice-transcriber@local"

echo "==> Building binaries (release)..."
cargo build --release \
    -p voice-transcriber-daemon \
    -p voice-transcriber-settings \
    --manifest-path "$REPO/Cargo.toml"

echo "==> Installing binaries to $BIN_DIR..."
mkdir -p "$BIN_DIR"
install -m755 "$REPO/target/release/voice-transcriber-daemon"  "$BIN_DIR/"
install -m755 "$REPO/target/release/voice-transcriber-settings" "$BIN_DIR/"

DATA_DIR="$HOME/.local/share/voice-transcriber"

echo "==> Installing systemd user service..."
mkdir -p "$SYSTEMD_DIR"
install -m644 "$REPO/deploy/voice-transcriber-daemon.service" "$SYSTEMD_DIR/"
install -m644 "$REPO/deploy/voice-transcriber-update.service" "$SYSTEMD_DIR/"
install -m644 "$REPO/deploy/voice-transcriber-update.timer"   "$SYSTEMD_DIR/"

echo "==> Installing DBus activation file..."
mkdir -p "$DBUS_SERVICES_DIR"
install -m644 "$REPO/deploy/org.transcriber.Daemon.service" "$DBUS_SERVICES_DIR/"

echo "==> Installing .desktop file..."
mkdir -p "$APPLICATIONS_DIR"
install -m644 "$REPO/deploy/org.transcriber.Settings.desktop" "$APPLICATIONS_DIR/"
update-desktop-database "$APPLICATIONS_DIR/" 2>/dev/null || true

echo "==> Installing GNOME extension..."
mkdir -p "$EXT_DIR/schemas"
cp -r "$REPO/extension/"* "$EXT_DIR/"

echo "==> Compiling GSettings schema..."
glib-compile-schemas "$EXT_DIR/schemas/"

echo "==> Installing update checker..."
mkdir -p "$DATA_DIR"
install -m755 "$REPO/scripts/check-update.sh" "$DATA_DIR/"

echo "==> Writing version file..."
# Read version from Cargo workspace
VERSION="$(grep '^version' "$REPO/Cargo.toml" | head -1 | sed 's/.*"\(.*\)".*/\1/')"
echo "$VERSION" > "$DATA_DIR/version"

echo "==> Reloading systemd and enabling services..."
systemctl --user daemon-reload
systemctl --user enable --now voice-transcriber-daemon.service
systemctl --user enable --now voice-transcriber-update.timer

echo ""
echo "Done!  Next steps:"
echo "  1. Open 'Voice Transcriber Settings' from the app launcher, or run: voice-transcriber-settings"
echo "  2. Enable extension:  gnome-extensions enable voice-transcriber@local"
echo "     (or restart GNOME Shell with Alt+F2 → 'r', then enable via Extensions app)"
echo "  3. Press Super+\` to start recording."
