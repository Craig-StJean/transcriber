import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import Gio from 'gi://Gio';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';

import { createDaemonProxy } from './dbus.js';
import { RecordingOverlay } from './overlay.js';

export default class VoiceTranscriberExtension extends Extension {

    enable() {
        this._settings      = this.getSettings();
        this._overlay       = new RecordingOverlay();
        this._proxy         = null;
        this._signalIds     = [];
        this._watchId       = 0;
        this._daemonState   = 'Idle';

        this._bindShortcuts();
        this._watchDaemon();
    }

    disable() {
        this._unbindShortcuts();
        this._unwatchDaemon();
        this._disconnectSignals();
        this._overlay.destroy();
        this._overlay       = null;
        this._proxy         = null;
        this._settings      = null;
        this._daemonState   = 'Idle';
    }

    // ── Daemon connection management ─────────────────────────────────────────

    _watchDaemon() {
        // Automatically connect/disconnect as the daemon appears and disappears
        this._watchId = Gio.bus_watch_name(
            Gio.BusType.SESSION,
            'org.transcriber.Daemon',
            Gio.BusNameWatcherFlags.NONE,
            () => this._connectProxy(),    // appeared
            () => this._onDaemonGone(),    // vanished
        );
    }

    _unwatchDaemon() {
        if (this._watchId) {
            Gio.bus_unwatch_name(this._watchId);
            this._watchId = 0;
        }
    }

    _connectProxy() {
        createDaemonProxy().then(proxy => {
            this._proxy = proxy;
            // Seed local state from the proxy's cached property so toggle works
            // immediately on first press even before a StateChanged signal fires.
            this._daemonState = proxy.CurrentState ?? 'Idle';
            this._connectSignals();
            console.log('VoiceTranscriber: connected to daemon');
        }).catch(e => {
            console.error('VoiceTranscriber: failed to create proxy:', e.message);
        });
    }

    _onDaemonGone() {
        console.log('VoiceTranscriber: daemon disappeared');
        this._disconnectSignals();
        this._proxy = null;
        this._daemonState = 'Idle';
        this._overlay.destroy();
    }

    // ── DBus signal handling ─────────────────────────────────────────────────

    _connectSignals() {
        const bus  = Gio.DBus.session;
        const dest = 'org.transcriber.Daemon';
        const path = '/org/transcriber/Daemon';
        const iface = 'org.transcriber.Daemon';
        const flags = Gio.DBusSignalFlags.NONE;

        // Subscribe directly on the session-bus connection — immune to
        // ES-module/proxy caching issues.
        this._signalIds.push(
            bus.signal_subscribe(null, iface, 'StateChanged', path, null, flags,
                (_c, _s, _p, _i, _n, params) => {
                    this._onStateChanged(params.get_child_value(0).get_string()[0]);
                }),
            bus.signal_subscribe(null, iface, 'AudioLevel', path, null, flags,
                (_c, _s, _p, _i, _n, params) => {
                    this._overlay.setLevel(params.get_child_value(0).get_double());
                }),
            bus.signal_subscribe(null, iface, 'TranscriptionReady', path, null, flags,
                (_c, _s, _p, _i, _n, params) => {
                    const text = params.get_child_value(0).get_string()[0];
                    console.log(`VoiceTranscriber: TranscriptionReady (${text.length} chars)`);
                    try {
                        if (this._settings.get_boolean('direct-injection')) {
                            this._directInject(text);
                        } else {
                            St.Clipboard.get_default().set_text(St.ClipboardType.CLIPBOARD, text);
                            if (this._settings.get_boolean('auto-paste'))
                                this._autoPaste();
                        }
                    } catch (e) {
                        console.error('VoiceTranscriber: TranscriptionReady handler failed:', e.message);
                    }
                }),
        );
    }

    _disconnectSignals() {
        const bus = Gio.DBus.session;
        this._signalIds.forEach(id => bus.signal_unsubscribe(id));
        this._signalIds = [];
    }

    _onStateChanged(state) {
        this._daemonState = state;
        switch (state) {
        case 'Recording':
            this._overlay.show();
            break;
        case 'Transcribing':
            this._overlay.setLevel(0);
            this._overlay.update('Transcribing...');
            break;
        case 'Done': {
            const label = this._settings.get_boolean('direct-injection') ? '✓ Typed!'
                    : this._settings.get_boolean('auto-paste')        ? '✓ Pasted!'
                    : '✓ Copied!';
            this._overlay.update(label);
            GLib.timeout_add(GLib.PRIORITY_DEFAULT, 800, () => {
                this._overlay.fadeOut();
                return GLib.SOURCE_REMOVE;
            });
            break;
        }
        case 'Error':
            this._overlay.update('✗ Error — check logs');
            GLib.timeout_add(GLib.PRIORITY_DEFAULT, 2500, () => {
                this._overlay.fadeOut();
                return GLib.SOURCE_REMOVE;
            });
            break;
        case 'Idle':
            this._overlay.fadeOut();
            break;
        }
    }

    // ── Hotkey handlers ──────────────────────────────────────────────────────

    _toggleRecording() {
        if (!this._proxy) {
            Main.notify('Voice Transcriber', 'Daemon is not running');
            return;
        }
        const state = this._daemonState;
        if (!state || state === 'Idle' || state === 'Done' || state === 'Error') {
            this._proxy.StartRecordingRemote((_, err) => {
                if (err) console.error('VoiceTranscriber: StartRecording failed:', err.message);
            });
        } else if (state === 'Recording') {
            this._proxy.StopRecordingRemote((_, err) => {
                if (err) console.error('VoiceTranscriber: StopRecording failed:', err.message);
            });
        }
    }

    // Simulate a paste keypress into the currently focused window.
    // Uses Ctrl+Shift+V for terminals, Ctrl+V everywhere else.
    _autoPaste() {
        try {
            const win = global.display.focus_window;
            const cls = win?.get_wm_class() ?? '';
            
            const SHIFT_PASTE_APPS = new Set([
                'alacritty', 'antigravity', 'code', 'cursor', 'gnome-terminal',
                'urxvt', 'vscodium', 'windsurf', 'xterm', 'com.mitchellh.ghostty',
                'deepin-terminal', 'dev.warp.warp', 'dev.zed.zed', 'foot', 'ghostty',
                'gnome-terminal-server', 'guake', 'io.elementary.terminal', 'kgx',
                'kitty', 'konsole', 'lxterminal', 'mate-terminal', 'org.gnome.console',
                'org.gnome.ptyxis', 'org.wezfurlong.wezterm', 'tabby', 'terminator',
                'tilda', 'tilix', 'xfce4-terminal', 'yakuake', 'zed'
            ]);
            
            const isTerminal = SHIFT_PASTE_APPS.has(cls);

            // Clutter.get_default_backend().get_default_seat() is the correct API.
            // notify_keyval expects time in microseconds — use GLib.get_monotonic_time().
            const seat = Clutter.get_default_backend().get_default_seat();
            const vk   = seat.create_virtual_device(Clutter.InputDeviceType.KEYBOARD_DEVICE);
            let t = GLib.get_monotonic_time();

            const PRESS   = Clutter.KeyState.PRESSED;
			const RELEASE = Clutter.KeyState.RELEASED;
            
			const vKeyval = isTerminal ? Clutter.KEY_V : Clutter.KEY_v;

			vk.notify_keyval(t++, Clutter.KEY_Control_L, PRESS);
            if (isTerminal) vk.notify_keyval(t++, Clutter.KEY_Shift_L, PRESS);
            vk.notify_keyval(t++, vKeyval, PRESS);
            vk.notify_keyval(t++, vKeyval, RELEASE);
            if (isTerminal) vk.notify_keyval(t++, Clutter.KEY_Shift_L, RELEASE);
            vk.notify_keyval(t++, Clutter.KEY_Control_L, RELEASE);
        } catch (e) {
            console.error('VoiceTranscriber: auto-paste failed:', e.message);
        }
    }

    _directInject(text) {
        try {
            const seat = Clutter.get_default_backend().get_default_seat();
            const vk = seat.create_virtual_device(Clutter.InputDeviceType.KEYBOARD_DEVICE);
            let t = GLib.get_monotonic_time();
            const PRESS   = Clutter.KeyState.PRESSED;
            const RELEASE = Clutter.KeyState.RELEASED;
            for (const char of text) {
                const cp = char.codePointAt(0);
                // Unicode keysyms: code points < 0x80 map directly;
                // others use the 0x01000000 | codepoint convention.
                const ks = cp < 0x80 ? cp : (0x01000000 | cp);
                vk.notify_keyval(t++, ks, PRESS);
                vk.notify_keyval(t++, ks, RELEASE);
            }
        } catch (e) {
            console.error('VoiceTranscriber: direct inject failed:', e.message);
        }
    }

    _cancelRecording() {
        if (!this._proxy) return;
        this._proxy.CancelRemote((_, err) => {
            if (err) console.error('VoiceTranscriber: Cancel failed:', err.message);
        });
    }

    // ── Keybindings ──────────────────────────────────────────────────────────

    _bindShortcuts() {
        Main.wm.addKeybinding(
            'toggle-recording',
            this._settings,
            Meta.KeyBindingFlags.NONE,
            Shell.ActionMode.ALL,
            this._toggleRecording.bind(this),
        );
        Main.wm.addKeybinding(
            'cancel-recording',
            this._settings,
            Meta.KeyBindingFlags.NONE,
            Shell.ActionMode.ALL,
            this._cancelRecording.bind(this),
        );
    }

    _unbindShortcuts() {
        Main.wm.removeKeybinding('toggle-recording');
        Main.wm.removeKeybinding('cancel-recording');
    }
}
