use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gtk4::glib;
use gtk4::prelude::*;
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};

const N_BARS: usize = 8;
const FADE_STEP_MS: u64 = 16; // ~60 fps
const FADE_DURATION_MS: f64 = 700.0;

// ── CSS ───────────────────────────────────────────────────────────────────────

const STYLE: &str = r#"
window {
    background: transparent;
}
.overlay-box {
    background-color: rgba(28, 28, 28, 0.88);
    border-radius: 10px;
    padding: 10px 18px;
    border: 1px solid rgba(255, 255, 255, 0.12);
}
.overlay-label {
    color: white;
    font-size: 15px;
    font-weight: bold;
    margin-left: 10px;
}
.vu-bar {
    background-color: #4CAF50;
    border-radius: 2px;
    min-width: 5px;
    min-height: 22px;
    margin: 0 2px;
}
"#;

// ── OverlayWindow ─────────────────────────────────────────────────────────────

struct Inner {
    window:   gtk4::ApplicationWindow,
    bars:     Vec<gtk4::Frame>,
    label:    gtk4::Label,
    bars_box: gtk4::Box,
    /// Monotonically-increasing counter; incremented on every show/hide cycle
    /// so fade closures can detect they've been superseded.
    generation: Cell<u32>,
}

/// Cheap-to-clone handle to the overlay.  All methods must be called on the
/// GTK main thread.
#[derive(Clone)]
pub struct OverlayWindow(Rc<Inner>);

impl OverlayWindow {
    pub fn new(app: &gtk4::Application) -> Self {
        // ── Load CSS ──────────────────────────────────────────────────────────
        let css = gtk4::CssProvider::new();
        css.load_from_data(STYLE);
        gtk4::style_context_add_provider_for_display(
            &gtk4::gdk::Display::default().expect("no GDK display"),
            &css,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        // ── Build window ──────────────────────────────────────────────────────
        let window = gtk4::ApplicationWindow::builder()
            .application(app)
            .decorated(false)
            .build();

        // ── Configure layer-shell ─────────────────────────────────────────────
        // This panics if the compositor does not support wlr-layer-shell.
        // main.rs checks gtk4_layer_shell::is_layer_shell_supported() first.
        window.init_layer_shell();
        window.set_layer(Layer::Overlay);
        window.set_anchor(Edge::Bottom, true);
        window.set_anchor(Edge::Left, false);
        window.set_anchor(Edge::Right, false);
        window.set_anchor(Edge::Top, false);
        window.set_margin(Edge::Bottom, 140);
        // Pass keyboard events only when the window is focused (Esc to cancel).
        window.set_keyboard_mode(KeyboardMode::OnDemand);

        // ── Build VU meter bars ───────────────────────────────────────────────
        let bars_box = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .spacing(0)
            .build();
        let bars: Vec<gtk4::Frame> = (0..N_BARS)
            .map(|_| {
                let bar = gtk4::Frame::new(None);
                bar.add_css_class("vu-bar");
                bar.set_opacity(0.24);
                bars_box.append(&bar);
                bar
            })
            .collect();

        // ── Build status label (hidden until Transcribing/Done/Error) ─────────
        let label = gtk4::Label::builder()
            .label("● Recording...")
            .visible(false)
            .build();
        label.add_css_class("overlay-label");

        // ── Outer container ───────────────────────────────────────────────────
        let outer = gtk4::Box::builder()
            .orientation(gtk4::Orientation::Horizontal)
            .build();
        outer.add_css_class("overlay-box");
        outer.append(&bars_box);
        outer.append(&label);

        window.set_child(Some(&outer));
        window.set_visible(false);

        Self(Rc::new(Inner {
            window,
            bars,
            label,
            bars_box,
            generation: Cell::new(0),
        }))
    }

    // ── Public API (mirrors the GNOME overlay.js interface) ───────────────────

    /// Update VU meter bars.  `level` is in 0.0–1.0.
    pub fn set_level(&self, level: f64) {
        let active = (level * N_BARS as f64).round() as usize;
        for (i, bar) in self.0.bars.iter().enumerate() {
            bar.set_opacity(if i < active { 1.0 } else { 0.24 });
        }
    }

    /// Handle a daemon state transition.
    pub fn handle_state(&self, state: &str) {
        match state {
            "Recording" => self.show_recording(),
            "Transcribing" => {
                self.set_level(0.0);
                self.update("Transcribing...");
            }
            "Done" => {
                self.update("✓ Copied!");
                self.schedule_fade(800);
            }
            "Error" => {
                self.update("✗ Error — check logs");
                self.schedule_fade(2500);
            }
            "Idle" => self.schedule_fade(0),
            _ => {}
        }
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    fn show_recording(&self) {
        let inner = &self.0;
        inner.generation.set(inner.generation.get().wrapping_add(1));
        // Reset to full opacity in case a fade was in progress.
        inner.window.set_opacity(1.0);
        // Show bars, hide label.
        inner.bars_box.set_visible(true);
        inner.label.set_visible(false);
        inner.label.set_label("● Recording...");
        inner.window.set_visible(true);
    }

    fn update(&self, msg: &str) {
        let inner = &self.0;
        inner.label.set_label(msg);
        inner.bars_box.set_visible(false);
        inner.label.set_visible(true);
        if !inner.window.is_visible() {
            inner.window.set_opacity(1.0);
            inner.window.set_visible(true);
        }
    }

    /// Schedule a fade-out that starts after `delay_ms` milliseconds.
    fn schedule_fade(&self, delay_ms: u32) {
        let this = self.clone();
        let gen = self.0.generation.get();
        glib::timeout_add_local_once(Duration::from_millis(delay_ms as u64), move || {
            if this.0.generation.get() == gen {
                this.start_fade(gen);
            }
        });
    }

    fn start_fade(&self, gen: u32) {
        let weak = Rc::downgrade(&self.0);
        let start = Instant::now();
        glib::timeout_add_local(Duration::from_millis(FADE_STEP_MS), move || {
            let Some(inner) = weak.upgrade() else {
                return glib::ControlFlow::Break;
            };
            // Superseded by a newer show — stop.
            if inner.generation.get() != gen {
                return glib::ControlFlow::Break;
            }
            let elapsed = start.elapsed().as_millis() as f64;
            let opacity = 1.0 - (elapsed / FADE_DURATION_MS).clamp(0.0, 1.0);
            inner.window.set_opacity(opacity);
            if opacity <= 0.0 {
                inner.window.set_visible(false);
                inner.window.set_opacity(1.0); // reset for next show
                return glib::ControlFlow::Break;
            }
            glib::ControlFlow::Continue
        });
    }
}
