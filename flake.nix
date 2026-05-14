{
  description = "Voice Transcriber — native Linux voice-to-text with global hotkey (Hyprland / wlroots / KDE)";

  inputs = {
    # staging-next instead of nixos-unstable because the settings app pins
    # `libadwaita-rs` v1_9 features (see app/Cargo.toml), and libadwaita 1.9.0
    # is only present on staging-next as of mid-2026. Once it lands in
    # nixos-unstable this can be moved back.
    nixpkgs.url = "github:NixOS/nixpkgs/staging-next";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default;

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };

        # Native tools needed to *build* the workspace.
        # - pkg-config: locate system libs (gtk4, alsa, …)
        # - wrapGAppsHook4: wraps installed GTK4 binaries so they can find GSettings
        #   schemas, GDK-pixbuf loaders, libadwaita styling, etc. at runtime.
        # - blueprint-compiler: invoked by gtk4-rs build script for the
        #   "blueprint" feature (used by the settings app to compile .blp → .ui).
        nativeBuildInputs = with pkgs; [
          pkg-config
          wrapGAppsHook4
          blueprint-compiler
        ];

        # Runtime + link-time C libs the three binaries collectively need.
        # Listing all of them in one derivation is harmless: each binary only
        # links what it actually references.
        buildInputs = with pkgs; [
          # daemon: audio capture via cpal → ALSA backend.
          alsa-lib
          # daemon: rusqlite is bundled (no system libsqlite3 needed).

          # overlay + settings: GTK4 stack.
          gtk4
          glib
          libadwaita
          graphene
          gdk-pixbuf
          pango
          cairo

          # overlay: wlr-layer-shell window placement.
          gtk4-layer-shell

          # overlay: Wayland text injection via enigo, plus general Wayland client.
          wayland
          libxkbcommon
        ];

        voice-transcriber = rustPlatform.buildRustPackage {
          pname = "voice-transcriber";
          version = "0.1.0";

          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          inherit nativeBuildInputs buildInputs;

          # Build only our three workspace binaries; skip dependency dev-targets.
          cargoBuildFlags = [
            "-p" "voice-transcriber-daemon"
            "-p" "voice-transcriber-overlay"
            "-p" "voice-transcriber-settings"
          ];

          # The workspace has no unit tests at the moment.
          doCheck = false;

          # wrapGAppsHook4 will wrap every binary in $out/bin. The daemon doesn't
          # need GTK env, but wrapping is harmless for it. We just need the GTK
          # binaries (overlay, settings) to find their runtime resources.

          meta = with pkgs.lib; {
            description = "Native Linux voice-to-text tool with global hotkey";
            homepage = "https://github.com/local/voice-transcriber";
            platforms = platforms.linux;
            mainProgram = "voice-transcriber-settings";
          };
        };
      in {
        packages = {
          default = voice-transcriber;
          voice-transcriber = voice-transcriber;
        };

        # `nix develop` — full build environment for `cargo build` locally.
        devShells.default = pkgs.mkShell {
          inherit buildInputs;
          nativeBuildInputs = nativeBuildInputs ++ [
            rustToolchain
            pkgs.rust-analyzer
          ];
        };

        # `nix run` defaults to opening the settings GUI.
        apps.default = {
          type = "app";
          program = "${voice-transcriber}/bin/voice-transcriber-settings";
        };
      }
    ) // {
      # System-agnostic outputs. The HM module is curried with `self` so it can
      # default `programs.voice-transcriber.package` to this flake's build of
      # the workspace.
      homeManagerModules.default = import ./nix/hm-module.nix self;
      homeManagerModules.voice-transcriber = import ./nix/hm-module.nix self;
    };
}
