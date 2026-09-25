{
  description = "Transcriber — native Linux voice-to-text with global hotkey (Hyprland / wlroots / KDE)";

  inputs = {
    # This used to pin `staging-next`, because the settings app needs
    # `libadwaita-rs` v1_9 features (see app/Cargo.toml) and libadwaita 1.9.0
    # had not reached a release channel yet. That is over: 1.9.2 shipped in
    # nixos-26.05 (and nixos-unstable), so the pin came off on 2026-08-10.
    #
    # Do not put it back. staging-next is a mass-rebuild integration branch —
    # it is routinely mid-rebuild, uncached, and can fail to evaluate outright,
    # which is a poor thing to hand anyone consuming this flake. It also forced
    # every consumer to carry a second, parallel nixpkgs closure, because a
    # `follows` onto a release channel could not satisfy the libadwaita
    # requirement.
    #
    # A release channel rather than nixos-unstable so a standalone `nix build`
    # of this repo is reproducible-ish and cache-warm. Consumers that set
    # `inputs.nixpkgs.follows` (as system-config does) override this anyway, so
    # it only governs direct `nix build` / `nix run` of this flake. Bump it when
    # moving to a newer NixOS release.
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";
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

        # Same pinned toolchain (MSRV + clippy and the formatter) that rustup,
        # CI and the release build use. The fallback only matters while the
        # file is untracked (flakes can't see untracked files).
        rustToolchain =
          if builtins.pathExists ./rust-toolchain.toml
          then pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml
          else pkgs.rust-bin.stable.latest.default;

        # Single source of truth for the version: the workspace Cargo.toml.
        cargoVersion = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;

        rustPlatform = pkgs.makeRustPlatform {
          cargo = rustToolchain;
          rustc = rustToolchain;
        };

        # Native tools needed to *build* the workspace.
        # - pkg-config: locate system libs (gtk4, alsa, …)
        # - wrapGAppsHook4: wraps installed GTK4 binaries so they can find GSettings
        #   schemas, GDK-pixbuf loaders, libadwaita styling, etc. at runtime.
        nativeBuildInputs = with pkgs; [
          pkg-config
          wrapGAppsHook4
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

        transcriber = rustPlatform.buildRustPackage {
          pname = "transcriber";
          version = cargoVersion;

          src = ./.;
          cargoLock.lockFile = ./Cargo.lock;

          inherit nativeBuildInputs buildInputs;

          # Build only our three workspace binaries; skip dependency dev-targets.
          cargoBuildFlags = [
            "-p" "transcriber-daemon"
            "-p" "transcriber-overlay"
            "-p" "transcriber-settings"
          ];

          # Run every workspace crate's tests as part of the build. They are
          # pure logic (no display or network), so they run in the Nix sandbox;
          # buildInputs above already carries every C library the test
          # harnesses link (incl. gtk4-layer-shell for the overlay).
          doCheck = true;
          cargoTestFlags = [ "--workspace" ];

          # wrapGAppsHook4 will wrap every binary in $out/bin. The daemon doesn't
          # need GTK env, but wrapping is harmless for it. We just need the GTK
          # binaries (overlay, settings) to find their runtime resources.

          meta = with pkgs.lib; {
            description = "Native Linux voice-to-text tool with global hotkey";
            homepage = "https://github.com/Craig-StJean/transcriber";
            license = licenses.mit;
            platforms = platforms.linux;
            mainProgram = "transcriber-settings";
          };
        };
      in {
        packages = {
          default = transcriber;
          transcriber = transcriber;
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
          program = "${transcriber}/bin/transcriber-settings";
        };
      }
    ) // {
      # System-agnostic outputs. The HM module is curried with `self` so it can
      # default `programs.transcriber.package` to this flake's build of
      # the workspace.
      homeManagerModules.default = import ./nix/hm-module.nix self;
      homeManagerModules.transcriber = import ./nix/hm-module.nix self;
    };
}
