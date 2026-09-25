import Clutter from 'gi://Clutter';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const N_BARS = 8;

function _animationsEnabled() {
    return St.Settings.get().enable_animations;
}

export class RecordingOverlay {
    constructor() {
        this._box      = null;
        this._label    = null;
        this._barsBox  = null;
        this._bars     = null;
        this._sigId    = 0;
        this._revealed = false;
    }

    /**
     * Show the overlay in "recording" mode: VU bars visible, label hidden.
     * Safe to call while the overlay already exists (including mid-fade) —
     * the box is reused and reset to its recording appearance.
     */
    show() {
        if (!this._ensureBox()) return;
        this._cancelFade();
        this._label.hide();
        this._barsBox.show();
        this.setLevel(0);
    }

    /** Show a status message (hides the bars). Creates the overlay if needed. */
    update(message) {
        if (!this._ensureBox()) return;
        this._cancelFade();
        this._label.set_text(message);
        this._label.show();
        this._barsBox.hide();
    }

    /**
     * Update the VU meter bars (0.0 – 1.0).
     * The number of lit bars scales linearly with level.
     */
    setLevel(level) {
        if (!this._bars) return;
        const active = Math.round(level * this._bars.length);
        this._bars.forEach((bar, i) => bar.set_opacity(i < active ? 255 : 60));
    }

    fadeOut(duration = 700) {
        const box = this._box;
        if (!box) return;
        if (_animationsEnabled()) {
            box.ease({
                opacity: 0,
                duration,
                mode: Clutter.AnimationMode.EASE_OUT_QUAD,
                // Only tear down the box this fade was started on; show()/update()
                // cancel the transition, so onComplete won't fire for a reused box.
                onComplete: () => {
                    if (this._box === box) this._destroy();
                },
            });
        } else {
            this._destroy();
        }
    }

    destroy() {
        this._destroy();
    }

    // ── internals ────────────────────────────────────────────────────────────

    _cancelFade() {
        this._box.remove_all_transitions();
        if (this._revealed) this._box.set_opacity(255);
    }

    _ensureBox() {
        if (this._box) return true;

        const monitor = Main.layoutManager.primaryMonitor;
        if (!monitor) return false;

        this._box = new St.BoxLayout({
            style_class: 'transcriber-overlay',
            vertical: false,
            reactive: false,
        });

        this._label = new St.Label({
            style_class: 'transcriber-label',
            y_align: Clutter.ActorAlign.CENTER,
            visible: false,
        });

        // ── VU meter bars — sized entirely by CSS ─────────────────────────────
        this._barsBox = new St.BoxLayout({
            style_class: 'transcriber-bars',
            y_align: Clutter.ActorAlign.CENTER,
        });
        this._bars = Array.from({ length: N_BARS }, () => {
            const bar = new St.Widget({ style_class: 'transcriber-bar' });
            bar.set_opacity(60);
            this._barsBox.add_child(bar);
            return bar;
        });

        this._box.add_child(this._barsBox);
        this._box.add_child(this._label);

        // Start invisible so there's no flicker before the first allocation.
        this._box.set_opacity(0);
        this._revealed = false;
        Main.uiGroup.add_child(this._box);

        // Re-centre whenever the width changes (label text varies); reveal on
        // the first real allocation.
        const y = monitor.y + monitor.height - 140;
        this._sigId = this._box.connect('notify::width', () => {
            const w = this._box.width;
            if (w <= 0) return;
            this._box.set_position(monitor.x + Math.round((monitor.width - w) / 2), y);
            if (!this._revealed) {
                this._revealed = true;
                if (_animationsEnabled()) {
                    this._box.ease({ opacity: 255, duration: 120,
                                     mode: Clutter.AnimationMode.EASE_OUT_QUAD });
                } else {
                    this._box.set_opacity(255);
                }
            }
        });
        return true;
    }

    _destroy() {
        if (!this._box) return;
        if (this._sigId) {
            this._box.disconnect(this._sigId);
            this._sigId = 0;
        }
        this._box.destroy();
        this._box      = null;
        this._label    = null;
        this._barsBox  = null;
        this._bars     = null;
        this._revealed = false;
    }
}
