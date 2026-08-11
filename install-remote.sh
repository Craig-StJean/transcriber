#!/usr/bin/env bash
# One-command installer for Transcriber.
# Downloads pre-built binaries from a GitHub Release and installs everything.
#
# Usage:
#   # With gh CLI already authenticated:
#   bash install-remote.sh
#
#   # With a GitHub token:
#   GITHUB_TOKEN=ghp_xxx bash install-remote.sh
#
#   # Install a specific version:
#   VERSION=v0.2.0 bash install-remote.sh
set -euo pipefail

REPO="Craig-StJean/transcriber"
APP_NAME="transcriber"

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
sudo dnf install -y gtk4 libadwaita glib2 alsa-lib glib2-devel >/dev/null 2>&1 || {
    warn "Some packages may not have installed. Continuing..."
}
ok "system dependencies"

# ── Resolve GitHub authentication ────────────────────────────────────────────

resolve_auth() {
    # 1. Env var
    if [[ -n "${GITHUB_TOKEN:-}" ]]; then
        return 0
    fi

    # 2. Stored token from previous install
    local token_file="$CONFIG_DIR/github-token"
    if [[ -f "$token_file" ]]; then
        GITHUB_TOKEN="$(cat "$token_file")"
        return 0
    fi

    # 3. gh CLI
    if command -v gh &>/dev/null && gh auth status &>/dev/null 2>&1; then
        GITHUB_TOKEN="$(gh auth token 2>/dev/null)" || true
        if [[ -n "${GITHUB_TOKEN:-}" ]]; then
            return 0
        fi
    fi

    # 4. Interactive prompt
    echo ""
    echo -e "${yellow}This is a private repository. A GitHub token with repo read access is required.${reset}"
    echo "Create one at: https://github.com/settings/tokens"
    echo "(Select: Fine-grained → Only select repositories → ${REPO} → Contents: Read-only)"
    echo ""
    read -rp "GitHub token: " GITHUB_TOKEN
    if [[ -z "$GITHUB_TOKEN" ]]; then
        err "No token provided. Cannot continue."
    fi
}

resolve_auth

# Store token for auto-updates
mkdir -p "$CONFIG_DIR"
echo "$GITHUB_TOKEN" > "$CONFIG_DIR/github-token"
chmod 600 "$CONFIG_DIR/github-token"

gh_api() {
    curl -fsSL \
        -H "Authorization: token $GITHUB_TOKEN" \
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

TAG_NAME="$(echo "$RELEASE_JSON" | grep -oP '"tag_name"\s*:\s*"\K[^"]+')"
ASSET_URL="$(echo "$RELEASE_JSON" | grep -oP '"browser_download_url"\s*:\s*"\K[^"]+\.tar\.gz')"

if [[ -z "$TAG_NAME" || -z "$ASSET_URL" ]]; then
    err "Could not find a release tarball. Is there a release published?"
fi

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

# ── Download release ─────────────────────────────────────────────────────────

TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

info "Downloading $TAG_NAME..."
gh_api -H "Accept: application/octet-stream" \
    "$(echo "$RELEASE_JSON" | grep -oP '"url"\s*:\s*"\K[^"]+assets/[0-9]+')" \
    -o "$TMPDIR/release.tar.gz" || {
    # Fallback: try browser_download_url directly
    gh_api "$ASSET_URL" -o "$TMPDIR/release.tar.gz"
}
ok "downloaded"

info "Extracting..."
tar xzf "$TMPDIR/release.tar.gz" -C "$TMPDIR"
RELEASE_DIR="$(find "$TMPDIR" -maxdepth 1 -type d -name 'transcriber-*' | head -1)"
[[ -d "$RELEASE_DIR" ]] || err "Unexpected tarball structure"
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
install -m755 "$RELEASE_DIR/bin/transcriber-daemon"   "$BIN_DIR/"
install -m755 "$RELEASE_DIR/bin/transcriber-settings"  "$BIN_DIR/"
install -m755 "$RELEASE_DIR/bin/transcriber-overlay"   "$BIN_DIR/"
ok ""

info "Installing systemd services..."
mkdir -p "$SYSTEMD_DIR"
install -m644 "$RELEASE_DIR/deploy/transcriber-daemon.service" "$SYSTEMD_DIR/"
install -m644 "$RELEASE_DIR/deploy/transcriber-update.service" "$SYSTEMD_DIR/"
install -m644 "$RELEASE_DIR/deploy/transcriber-update.timer"   "$SYSTEMD_DIR/"
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
cp -r "$RELEASE_DIR/extension/"* "$EXT_DIR/"
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
ok ""

# ── Done ─────────────────────────────────────────────────────────────────────

echo ""
echo -e "${green}${bold}Transcriber $TAG_NAME installed successfully!${reset}"
echo ""
echo "Next steps:"
echo "  1. Open 'Transcriber Settings' from the app launcher, or run: transcriber-settings"
if [ "${XDG_SESSION_TYPE:-}" = "wayland" ]; then
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
