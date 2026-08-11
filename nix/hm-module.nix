flake:
{ config, lib, pkgs, ... }:

let
  cfg = config.programs.transcriber;
  system = pkgs.stdenv.hostPlatform.system;

  # DBus activation file content. Exec= is required by the spec but ignored
  # when SystemdService= is set, so we point at coreutils' `true` so the path
  # is valid on NixOS (where /bin/false does not exist).
  dbusServiceText = ''
    [D-BUS Service]
    Name=org.transcriber.Daemon
    Exec=${pkgs.coreutils}/bin/true
    SystemdService=transcriber-daemon.service
  '';
in {
  options.programs.transcriber = {
    enable = lib.mkEnableOption "transcriber (voice-to-text daemon + Wayland overlay)";

    package = lib.mkOption {
      type = lib.types.package;
      default = flake.packages.${system}.default;
      defaultText = lib.literalExpression "transcriber.packages.\${system}.default";
      description = ''
        The transcriber package providing the daemon, overlay, and
        settings binaries. Defaults to this flake's build of the workspace.
      '';
    };

    enableOverlay = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = ''
        Install and run the Wayland overlay (VU meter + recording indicator).
        Required for usable feedback on Hyprland / Sway / KDE.
      '';
    };

    enableSettingsApp = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Install a .desktop entry for the GTK settings GUI.";
    };

    autostart = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = ''
        Install the daemon's systemd unit with WantedBy=graphical-session.target
        so it starts with the desktop session. Requires Hyprland to be started
        via uwsm (or another systemd-managed launcher). If you launch Hyprland
        directly, set this to false and add the equivalent `exec-once = ...`
        lines to hyprland.conf instead.
      '';
    };

    waylandDisplay = lib.mkOption {
      type = lib.types.str;
      default = "wayland-1";
      description = ''
        Value of WAYLAND_DISPLAY exported into the daemon and overlay services.
        Hyprland's default socket is wayland-1; Sway / GNOME typically use
        wayland-0. Run `echo $WAYLAND_DISPLAY` inside your session to check.
      '';
    };

    config = lib.mkOption {
      type = lib.types.nullOr (lib.types.attrsOf lib.types.anything);
      default = null;
      example = lib.literalExpression ''
        {
          provider = "groq";
          groq_api_key = "sk-...";
          model = "whisper-large-v3-turbo";
          language = "en";
        }
      '';
      description = ''
        Declarative contents of ~/.config/transcriber/config.json.
        Leave null to let the daemon write its own defaults on first run,
        or to manage the file via the settings GUI. Note: if you set this,
        edits made through the settings GUI will be overwritten on the next
        home-manager switch.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = [ cfg.package ];

    # DBus activation: lets the daemon start on first method call (e.g. the
    # Hyprland hotkey bound to `dbus-send ... StartRecording`) without having
    # to be running already.
    xdg.dataFile."dbus-1/services/org.transcriber.Daemon.service".text = dbusServiceText;

    # ── Daemon ──────────────────────────────────────────────────────────────
    systemd.user.services.transcriber-daemon = {
      Unit = {
        Description = "Transcriber Daemon";
        Documentation = "https://github.com/Craig-StJean/transcriber";
        After = [ "graphical-session.target" ];
        PartOf = [ "graphical-session.target" ];
      };
      Service = {
        Type = "dbus";
        BusName = "org.transcriber.Daemon";
        ExecStart = "${cfg.package}/bin/transcriber-daemon";
        Restart = "on-failure";
        RestartSec = 3;
        Environment = [
          "RUST_LOG=info"
          "WAYLAND_DISPLAY=${cfg.waylandDisplay}"
        ];
      };
      Install = lib.mkIf cfg.autostart {
        WantedBy = [ "graphical-session.target" ];
      };
    };

    # ── Overlay ─────────────────────────────────────────────────────────────
    systemd.user.services.transcriber-overlay = lib.mkIf cfg.enableOverlay {
      Unit = {
        Description = "Transcriber Overlay (wlroots/KDE)";
        Documentation = "https://github.com/Craig-StJean/transcriber";
        After = [ "graphical-session.target" "transcriber-daemon.service" ];
        PartOf = [ "graphical-session.target" ];
      };
      Service = {
        Type = "simple";
        ExecStart = "${cfg.package}/bin/transcriber-overlay";
        Restart = "on-failure";
        RestartSec = 3;
        Environment = [
          "RUST_LOG=info"
          "WAYLAND_DISPLAY=${cfg.waylandDisplay}"
        ];
      };
      Install = lib.mkIf cfg.autostart {
        WantedBy = [ "graphical-session.target" ];
      };
    };

    # ── Settings GUI .desktop entry ─────────────────────────────────────────
    xdg.desktopEntries = lib.mkIf cfg.enableSettingsApp {
      transcriber-settings = {
        name = "Transcriber Settings";
        comment = "Configure voice transcription and view history";
        exec = "${cfg.package}/bin/transcriber-settings";
        icon = "audio-input-microphone";
        terminal = false;
        type = "Application";
        categories = [ "Utility" "AudioVideo" ];
        settings = {
          Keywords = "voice;transcription;microphone;speech;whisper;";
          StartupWMClass = "org.transcriber.Settings";
        };
      };
    };

    # ── Optional declarative config.json ───────────────────────────────────
    xdg.configFile."transcriber/config.json" = lib.mkIf (cfg.config != null) {
      text = builtins.toJSON cfg.config;
    };
  };
}
