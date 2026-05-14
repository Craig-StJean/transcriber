# Voice Transcriber on NixOS + Hyprland

This guide covers running voice-transcriber on NixOS under Hyprland. The
GNOME Shell extension does not apply; instead, the Wayland-native `overlay`
binary provides the on-screen VU meter, and global hotkeys are bound directly
in `hyprland.conf` via `dbus-send`.

## 1. Install via the flake

### Option A — Home Manager (recommended)

Add the flake to your inputs and import the module:

```nix
# flake.nix (your system config)
{
  inputs.voice-transcriber.url = "github:YOUR_USER/voice-transcriber";
  # …
}
```

```nix
# home.nix
{ inputs, ... }: {
  imports = [ inputs.voice-transcriber.homeManagerModules.default ];

  programs.voice-transcriber = {
    enable = true;
    waylandDisplay = "wayland-1";  # Hyprland default; check with `echo $WAYLAND_DISPLAY`

    # Optional: declaratively manage ~/.config/voice-transcriber/config.json.
    # Omit this and use the settings GUI instead if you'd rather edit in place.
    config = {
      provider = "groq";
      groq_api_key = "sk-...";   # prefer sops-nix / agenix for real keys
      model = "whisper-large-v3-turbo";
      language = "en";
    };
  };
}
```

Apply with `home-manager switch` (or `nh home switch`).

### Option B — Ad-hoc

```bash
nix profile install github:YOUR_USER/voice-transcriber
```

Then write the systemd units and DBus activation file yourself (see
`deploy/` for templates), or just run the binaries manually.

## 2. NixOS prerequisites

In your system configuration:

```nix
{
  # Hyprland (uwsm is recommended so systemd user services pick up
  # graphical-session.target — the daemon and overlay both Want= it).
  programs.hyprland.enable = true;
  programs.uwsm.enable = true;

  # PipeWire (cpal auto-detects this backend).
  services.pipewire = {
    enable = true;
    alsa.enable = true;
    pulse.enable = true;
  };

  # The daemon writes to the Wayland clipboard via wl-clipboard-rs, which
  # talks to the compositor directly — no extra packages needed.
}
```

If you are **not** using `uwsm` to launch Hyprland, set
`programs.voice-transcriber.autostart = false;` in your home config and add
`exec-once` lines to `hyprland.conf` instead (see step 4).

## 3. Bind the global hotkey

Add to `~/.config/hypr/hyprland.conf`:

```conf
# Toggle / cancel recording. Goes straight through DBus — no XDG portal
# approval dialog, no GNOME extension needed.
bind = SUPER, apostrophe, exec, dbus-send --session --type=method_call \
    --dest=org.transcriber.Daemon \
    /org/transcriber/Daemon org.transcriber.Daemon.StartRecording

bind = SUPER SHIFT, apostrophe, exec, dbus-send --session --type=method_call \
    --dest=org.transcriber.Daemon \
    /org/transcriber/Daemon org.transcriber.Daemon.StopRecording

bind = SUPER CTRL, apostrophe, exec, dbus-send --session --type=method_call \
    --dest=org.transcriber.Daemon \
    /org/transcriber/Daemon org.transcriber.Daemon.Cancel
```

VAD will auto-stop after ~1.5 s of silence, so the explicit Stop bind is
optional. The daemon is DBus-activated, so the first hotkey press will spawn
it if it isn't already running.

## 4. Optional: exec-once if not using uwsm

If you launch Hyprland without `uwsm`, `graphical-session.target` isn't
reached and the systemd-managed overlay never starts. Add:

```conf
exec-once = systemctl --user start voice-transcriber-overlay.service
```

(The daemon starts on demand via DBus activation, so it doesn't need an
`exec-once` line.)

## 5. Configure your API key

Open **Voice Transcriber Settings** from your launcher (or run
`voice-transcriber-settings`) and paste a Groq / OpenAI / Deepgram API key.
Skip this step if you set `programs.voice-transcriber.config` declaratively.

## 6. First recording

1. Press `Super + '` (apostrophe).
2. The VU meter overlay appears at the bottom of the screen.
3. Speak. VAD will stop the recording automatically, or press `Super + '`
   followed by `Super + Shift + '` to stop manually.
4. The transcribed text lands in the Wayland clipboard. Paste with `Ctrl+V`.
5. To inject text directly into the focused window without using the
   clipboard, set `direct_injection = true` in the settings GUI.

## Troubleshooting

**Daemon not responding to hotkey:**

```bash
systemctl --user status voice-transcriber-daemon.service
journalctl --user -u voice-transcriber-daemon.service -f
```

Test the DBus path manually:

```bash
dbus-send --session --print-reply --dest=org.transcriber.Daemon \
    /org/transcriber/Daemon org.transcriber.Daemon.StartRecording
```

**Overlay never appears:**

- Confirm `WAYLAND_DISPLAY` is set in the service (`systemctl --user show-environment`).
- Confirm `gtk4-layer-shell` is the runtime version Hyprland supports (it should be — Hyprland implements `wlr-layer-shell-unstable-v1`).
- Look for "compositor does not support wlr-layer-shell" in the overlay's
  log: `journalctl --user -u voice-transcriber-overlay.service`.

**Clipboard empty after transcription:**

Make sure `waylandDisplay` matches your actual socket. On Hyprland it is
usually `wayland-1`, but it varies — `echo $WAYLAND_DISPLAY` from a terminal
inside the session is authoritative.

**Direct injection types nothing / wrong characters:**

`enigo` uses the `virtual_keyboard_v1` Wayland protocol. Hyprland supports
it, but the keymap must be a sensible Latin layout — non-Latin layouts may
type garbage. Disable `direct_injection` and use the clipboard path instead.
