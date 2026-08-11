import Clutter from 'gi://Clutter';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const N_BARS = 8;

function _animationsEnabled() {
    return St.Settings.get().enable_animations;
}

export class RecordingOverlay {
    constructor() {
        this._box     = null;
        this._label   = null;
        this._bars    = null;
        this._sigId   = 0;
    }

    /**
     * Show (or re-show) the overlay with `message`.
     * Safe to call while already visible — updates the text.
     */
    show(message = '● Recording...') {
        if (this._box) {
            this._label.set_text(message);
            this._box.set_opacity(255);
            return;
        }

        const monitor = Main.layoutManager.primaryMonitor;
        const y = monitor.y + monitor.height - 140;

        this._box = new St.BoxLayout({
            style_class: 'transcriber-overlay',
            vertical: false,
            reactive: false,
        });

        this._label = new St.Label({
            text: message,
            style_class: 'transcriber-label',
            y_align: Clutter.ActorAlign.CENTER,
            visible: false,
        });

        // ── VU meter bars — sized entirely by CSS ─────────────────────────────
        const barsBox = new St.BoxLayout({
            style_class: 'transcriber-bars',
            y_align: Clutter.ActorAlign.CENTER,
        });
        this._bars = Array.from({ length: N_BARS }, () => {
            const bar = new St.Widget({ style_class: 'transcriber-bar' });
            bar.set_opacity(60);
            barsBox.add_child(bar);
            return bar;
        });

        this._box.add_child(barsBox);
        this._box.add_child(this._label);

        // Start invisible so there's no flicker before the first allocation.
        this._box.set_opacity(0);
        Main.uiGroup.add_child(this._box);

        // Reposition and reveal once Clutter gives us the real width.
        this._sigId = this._box.connect('notify::width', () => {
            const w = this._box.width;
            if (w > 0) {
                this._box.set_position(
                    monitor.x + Math.round((monitor.width - w) / 2),
                    y,
                );
                if (_animationsEnabled()) {
                    this._box.ease({ opacity: 255, duration: 120,
                                      mode: Clutter.AnimationMode.EASE_OUT_QUAD });
                } else {
                    this._box.set_opacity(255);
                }
                this._box.disconnect(this._sigId);
                this._sigId = 0;
            }
        });
    }

    /** Switch from bars-only to a status message (hides bars, shows label). */
    update(message) {
        if (!this._label) return;
        this._label.set_text(message);
        this._label.show();
        if (this._bars)
            this._bars.forEach(b => b.get_parent().hide());
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

    fadeOut(onComplete = null, duration = 700) {
        if (!this._box) {
            onComplete?.();
            return;
        }
        if (_animationsEnabled()) {
            this._box.ease({
                opacity: 0,
                duration,
                mode: Clutter.AnimationMode.EASE_OUT_QUAD,
                onComplete: () => {
                    this._destroy();
                    onComplete?.();
                },
            });
        } else {
            this._box.set_opacity(0);
            this._destroy();
            onComplete?.();
        }
    }

    _destroy() {
        if (this._sigId && this._box) {
            this._box.disconnect(this._sigId);
            this._sigId = 0;
        }
        if (this._box) {
            Main.uiGroup.remove_child(this._box);
            this._box.destroy();
            this._box   = null;
            this._label = null;
            this._bars  = null;
        }
    }

    destroy() {
        this._destroy();
    }
}
