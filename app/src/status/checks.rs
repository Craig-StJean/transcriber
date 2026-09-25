//! Setup checks. Every `run` function may spawn processes or read files, so
//! callers run them on a worker thread.

use common::config;

use crate::ext_settings::is_gnome;

/// `Err` carries a specific failure message, or an empty string to use the
/// check's generic `fail_msg`.
pub type CheckResult = Result<(), String>;

pub enum FixAction {
    RunCommand {
        btn_label: &'static str,
        program:   &'static str,
        args:      &'static [&'static str],
    },
    CopyText {
        btn_label: &'static str,
        text:      &'static str,
    },
    GoToSettings,
}

pub struct StatusCheck {
    pub title:    &'static str,
    pub ok_msg:   &'static str,
    pub fail_msg: &'static str,
    pub run:      fn() -> CheckResult,
    pub fix:      Option<FixAction>,
}

fn flag(ok: bool) -> CheckResult {
    if ok { Ok(()) } else { Err(String::new()) }
}

/// Whether `name` is installed, checking both the install.sh location
/// (~/.local/bin) and anywhere on `$PATH`. The PATH check covers NixOS /
/// home-manager, where binaries live in the read-only Nix store rather than
/// ~/.local/bin.
fn binary_available(name: &str, local_subpath: &str) -> bool {
    if dirs::home_dir()
        .map(|h| h.join(local_subpath).exists())
        .unwrap_or(false)
    {
        return true;
    }
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|p| p.join(name).exists()))
        .unwrap_or(false)
}

fn service_active(service: &str) -> bool {
    std::process::Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", service])
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn check_daemon_installed() -> CheckResult {
    flag(binary_available("transcriber-daemon", ".local/bin/transcriber-daemon"))
}

fn check_daemon_running() -> CheckResult {
    flag(service_active("transcriber-daemon.service"))
}

fn check_overlay_installed() -> CheckResult {
    flag(binary_available("transcriber-overlay", ".local/bin/transcriber-overlay"))
}

fn check_overlay_running() -> CheckResult {
    flag(service_active("transcriber-overlay.service"))
}

fn check_extension_installed() -> CheckResult {
    flag(
        dirs::data_local_dir()
            .map(|d| d.join("gnome-shell/extensions/transcriber@local").exists())
            .unwrap_or(false),
    )
}

fn check_extension_loaded() -> CheckResult {
    flag(
        std::process::Command::new("gnome-extensions")
            .args(["list"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("transcriber@local"))
            .unwrap_or(false),
    )
}

fn check_extension_enabled() -> CheckResult {
    flag(
        std::process::Command::new("gsettings")
            .args(["get", "org.gnome.shell", "enabled-extensions"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).contains("transcriber@local"))
            .unwrap_or(false),
    )
}

/// Checks every key the current configuration will actually use, mirroring
/// the daemon's choice of path: streaming (only if post-processing is off)
/// needs the Deepgram key instead of the batch key, and post-processing
/// needs its own key on top.
fn check_api_keys() -> CheckResult {
    let cfg = config::load().map_err(|e| format!("config.json could not be read: {e}"))?;
    let streaming = cfg.streaming_enabled && !cfg.postprocess_enabled;
    let mut missing = Vec::new();
    if streaming {
        if cfg.deepgram_api_key.is_empty() {
            missing.push("Deepgram");
        }
    } else if cfg.active_key().is_empty() {
        missing.push("transcription");
    }
    if cfg.postprocess_enabled && cfg.active_postprocess_key().is_empty() {
        missing.push("post-processing");
    }
    match missing.as_slice() {
        [] => Ok(()),
        [one] => Err(format!("No {one} API key — transcription will fail")),
        many => Err(format!("No {} API keys — transcription will fail", many.join(" or "))),
    }
}

const DAEMON_INSTALLED_CHECK: StatusCheck = StatusCheck {
    title:    "Daemon installed",
    ok_msg:   "Binary found in ~/.local/bin or on $PATH",
    fail_msg: "Binary not found — run install.sh (or rebuild your Nix config)",
    run:      check_daemon_installed,
    fix:      Some(FixAction::CopyText {
        btn_label: "Copy command",
        text:      "bash install.sh",
    }),
};

const DAEMON_RUNNING_CHECK: StatusCheck = StatusCheck {
    title:    "Daemon running",
    ok_msg:   "systemd service is active",
    fail_msg: "Service not running",
    run:      check_daemon_running,
    fix:      Some(FixAction::RunCommand {
        btn_label: "Start daemon",
        program:   "systemctl",
        args:      &["--user", "enable", "--now", "transcriber-daemon.service"],
    }),
};

const API_KEY_CHECK: StatusCheck = StatusCheck {
    title:    "API keys configured",
    ok_msg:   "Every provider in use has a key",
    fail_msg: "Missing API key — transcription will fail",
    run:      check_api_keys,
    fix:      Some(FixAction::GoToSettings),
};

/// Checks shown on GNOME Shell sessions: the hotkey + VU meter come from the
/// GNOME extension.
const GNOME_CHECKS: &[StatusCheck] = &[
    DAEMON_INSTALLED_CHECK,
    DAEMON_RUNNING_CHECK,
    StatusCheck {
        title:    "Extension installed",
        ok_msg:   "Found in ~/.local/share/gnome-shell/extensions/",
        fail_msg: "Extension not found — run install.sh",
        run:      check_extension_installed,
        fix:      Some(FixAction::CopyText {
            btn_label: "Copy command",
            text:      "bash install.sh",
        }),
    },
    StatusCheck {
        title:    "Extension loaded by GNOME Shell",
        ok_msg:   "GNOME Shell has scanned the extension",
        fail_msg: "Not loaded yet — log out and back in on Wayland",
        run:      check_extension_loaded,
        fix:      Some(FixAction::CopyText {
            btn_label: "Copy logout command",
            text:      "gnome-session-quit --logout",
        }),
    },
    StatusCheck {
        title:    "Extension enabled",
        ok_msg:   "Enabled in GNOME Shell",
        fail_msg: "Extension loaded but not enabled",
        run:      check_extension_enabled,
        fix:      Some(FixAction::RunCommand {
            btn_label: "Enable extension",
            program:   "gnome-extensions",
            args:      &["enable", "transcriber@local"],
        }),
    },
    API_KEY_CHECK,
];

/// Checks shown on wlroots compositors (Hyprland / Sway / KDE): the hotkey +
/// VU meter come from the standalone overlay binary, not a GNOME extension.
const WLROOTS_CHECKS: &[StatusCheck] = &[
    DAEMON_INSTALLED_CHECK,
    DAEMON_RUNNING_CHECK,
    StatusCheck {
        title:    "Overlay installed",
        ok_msg:   "Binary found in ~/.local/bin or on $PATH",
        fail_msg: "Overlay not found — run install.sh (or enable it in your Nix config)",
        run:      check_overlay_installed,
        fix:      Some(FixAction::CopyText {
            btn_label: "Copy command",
            text:      "bash install.sh",
        }),
    },
    StatusCheck {
        title:    "Overlay running",
        ok_msg:   "systemd service is active",
        fail_msg: "Overlay not running",
        run:      check_overlay_running,
        fix:      Some(FixAction::RunCommand {
            btn_label: "Start overlay",
            program:   "systemctl",
            args:      &["--user", "enable", "--now", "transcriber-overlay.service"],
        }),
    },
    API_KEY_CHECK,
];

pub fn active_checks() -> &'static [StatusCheck] {
    if is_gnome() { GNOME_CHECKS } else { WLROOTS_CHECKS }
}
