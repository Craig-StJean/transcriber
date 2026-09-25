flake:
{ config, lib, pkgs, ... }:

let
  cfg = config.programs.transcriber;
  system = pkgs.stdenv.hostPlatform.system;

  # Where the daemon keeps its files. Passed to the units explicitly so the
  # daemon's paths and its sandbox's ReadWritePaths can't disagree.
  configDir = "${config.xdg.configHome}/transcriber";
  dataDir = "${config.xdg.dataHome}/transcriber";
  xdgEnvironment = [
    "XDG_CONFIG_HOME=${config.xdg.configHome}"
    "XDG_DATA_HOME=${config.xdg.dataHome}"
  ];

  # DBus activation file content. Exec= is required by the spec but ignored
  # when SystemdService= is set, so we point at coreutils' `true` so the path
  # is valid on NixOS (where /bin/false does not exist).
  dbusServiceText = ''
    [D-BUS Service]
    Name=org.transcriber.Daemon
    Exec=${pkgs.coreutils}/bin/true
    SystemdService=transcriber-daemon.service
  '';

  # The daemon's sandbox (ProtectSystem=strict) leaves $HOME read-only except
  # its two ReadWritePaths, so create those — and carry over the pre-rename
  # "voice-transcriber" dirs — outside the sandbox ("+" prefix) before start.
  prepareDirs = pkgs.writeShellScript "transcriber-prepare-dirs" ''
    set -eu
    for d in ${lib.escapeShellArg config.xdg.configHome} ${lib.escapeShellArg config.xdg.dataHome}; do
      if [ -d "$d/voice-transcriber" ] && [ ! -e "$d/transcriber" ]; then
        ${pkgs.coreutils}/bin/mv "$d/voice-transcriber" "$d/transcriber"
      fi
      ${pkgs.coreutils}/bin/mkdir -p "$d/transcriber"
    done
  '';

  # Merges the declared `config` attrs onto the existing config.json (declared
  # fields win, recursively), keeping it a regular 0600 file the settings GUI
  # can still save to. Replaces a store symlink left by older versions of
  # this module.
  declaredConfig = pkgs.writeText "transcriber-config.json" (builtins.toJSON cfg.config);
  mergeConfig = pkgs.writeShellScript "transcriber-merge-config" ''
    set -eu
    dir="$1"
    file="$dir/config.json"
    ${pkgs.coreutils}/bin/mkdir -p "$dir"
    existing='{}'
    if [ -f "$file" ]; then
      # Unparseable file degrades to {} rather than failing activation.
      existing="$(${pkgs.jq}/bin/jq -c '.' "$file" 2>/dev/null || true)"
      [ -n "$existing" ] || existing='{}'
    fi
    tmp="$(${pkgs.coreutils}/bin/mktemp "$dir/.config.json.XXXXXX")"   # mode 0600
    trap '${pkgs.coreutils}/bin/rm -f -- "$tmp"' EXIT
    printf '%s' "$existing" \
      | ${pkgs.jq}/bin/jq -s '.[0] * .[1]' - ${declaredConfig} > "$tmp"
    ${pkgs.coreutils}/bin/mv -f "$tmp" "$file"
    trap - EXIT
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
      type = lib.types.nullOr lib.types.str;
      default = null;
      visible = false;
      description = ''
        Deprecated and ignored. The units now inherit WAYLAND_DISPLAY from
        the session environment that the compositor imports into the systemd
        user manager (home-manager's Hyprland/Sway modules, uwsm and KDE all
        do this).
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
        Fields to merge into `''${xdg.configHome}/transcriber/config.json` on every
        home-manager switch. The merge is recursive and the declared values
        win; every field you don't declare keeps whatever is on disk, so
        settings changed in the GUI survive unless they are declared here.
        The file stays a regular, writable file (mode 0600), created if
        missing. Leave null to leave the file entirely to the daemon and GUI.

        These values are copied into the world-readable Nix store, so don't
        put API keys here — write them from a secret file in your own
        activation script instead.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    home.packages = [ cfg.package ];

    warnings = lib.optional (cfg.waylandDisplay != null) ''
      programs.transcriber.waylandDisplay is deprecated and ignored: the
      services now inherit WAYLAND_DISPLAY from the session environment.
      Remove it from your configuration.
    '';

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
        ExecStartPre = "+${prepareDirs}";
        ExecStart = "${cfg.package}/bin/transcriber-daemon";
        Restart = "on-failure";
        RestartSec = 3;
        Environment = [ "RUST_LOG=info" ] ++ xdgEnvironment;

        # Hardening. No ProtectHome: audio goes through PipeWire/PulseAudio
        # sockets under $XDG_RUNTIME_DIR, and the config/data dirs are in $HOME.
        NoNewPrivileges = true;
        PrivateTmp = true;
        ProtectSystem = "strict";
        ReadWritePaths = [ configDir dataDir ];
        RestrictAddressFamilies = "AF_UNIX AF_INET AF_INET6 AF_NETLINK";
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
        # Exits 0 without wlr-layer-shell, so this can't restart-loop on GNOME.
        Restart = "on-failure";
        RestartSec = 3;
        # XDG dirs as for the daemon: the overlay reads config.json too.
        Environment = [ "RUST_LOG=info" ] ++ xdgEnvironment;
        # Kept minimal: needs the Wayland socket, the session bus and the
        # virtual-keyboard protocol for direct text injection.
        NoNewPrivileges = true;
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
    # Merged onto the live file rather than linked from the store, so the
    # settings GUI can keep saving to it. See the `config` option.
    home.activation.transcriberConfig = lib.mkIf (cfg.config != null)
      (lib.hm.dag.entryAfter [ "writeBoundary" ] ''
        run ${mergeConfig} ${lib.escapeShellArg configDir}
      '');
  };
}
