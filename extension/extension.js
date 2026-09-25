import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import Gio from 'gi://Gio';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';

import {
    BUS_NAME, OBJECT_PATH, INTERFACE,
    callDaemon, getCurrentState, isCancelledError,
} from './dbus.js';
import { RecordingOverlay } from './overlay.js';

// States in which a recording/transcription is in flight — the only time the
// cancel shortcut is grabbed, so Escape isn't stolen from apps otherwise.
const ACTIVE_STATES = new Set(['Recording', 'Transcribing', 'PostProcessing', 'Streaming']);
// States from which the toggle hotkey starts a new recording.
const IDLE_STATES = new Set(['Idle', 'Done', 'Error']);

const MODIFIER_POLL_MS  = 30;
const REPASTE_WAIT_MS   = 2000;
const ERROR_DISPLAY_MS  = 4000;
const ERROR_MAX_CHARS   = 90;

const SHIFT_PASTE_APPS = new Set([
    'alacritty', 'antigravity', 'code', 'cursor', 'gnome-terminal',
    'urxvt', 'vscodium', 'windsurf', 'xterm', 'com.mitchellh.ghostty',
    'deepin-terminal', 'dev.warp.warp', 'dev.zed.zed', 'foot', 'ghostty',
    'gnome-terminal-server', 'guake', 'io.elementary.terminal', 'kgx',
    'kitty', 'konsole', 'lxterminal', 'mate-terminal', 'org.gnome.console',
    'org.gnome.ptyxis', 'org.wezfurlong.wezterm', 'tabby', 'terminator',
    'tilda', 'tilix', 'xfce4-terminal', 'yakuake', 'zed',
]);

const M = Clutter.ModifierType;
const ANY_MODIFIER = M.SHIFT_MASK | M.CONTROL_MASK | M.MOD1_MASK | M.MOD4_MASK |
    M.SUPER_MASK | M.HYPER_MASK | M.META_MASK;

/**
 * Parse the modifier part of a GTK-style accelerator ("<Super>apostrophe")
 * into a list of Clutter modifier masks, one per required modifier.
 */
function accelModifierGroups(accel) {
    const groups = [];
    for (const [, name] of (accel ?? '').matchAll(/<([^>]+)>/g)) {
        switch (name.toLowerCase()) {
        case 'super':   groups.push(M.SUPER_MASK | M.MOD4_MASK); break;
        case 'control':
        case 'ctrl':
        case 'primary': groups.push(M.CONTROL_MASK); break;
        case 'shift':   groups.push(M.SHIFT_MASK); break;
        case 'alt':
        case 'mod1':    groups.push(M.MOD1_MASK); break;
        case 'meta':    groups.push(M.META_MASK | M.MOD1_MASK); break;
        case 'hyper':   groups.push(M.HYPER_MASK); break;
        }
    }
    return groups;
}

function currentModifiers() {
    const [, , mods] = global.get_pointer();
    return mods;
}

function summarizeError(message) {
    // Strip the "GDBus.Error:org.foo.Bar: " prefix from remote DBus errors.
    let s = (message ?? '').split('\n')[0].replace(/^GDBus\.Error:\S+:\s*/, '').trim();
    if (!s) return 'Error — check logs';
    if (s.length > ERROR_MAX_CHARS)
        s = `${s.slice(0, ERROR_MAX_CHARS - 1).trimEnd()}…`;
    return s;
}

export default class TranscriberExtension extends Extension {

    enable() {
        this._settings          = this.getSettings();
        this._overlay           = new RecordingOverlay();
        this._cancellable       = new Gio.Cancellable();
        this._signalIds         = [];
        this._watchId           = 0;
        this._nameOwner         = null;
        this._daemonState       = 'Idle';
        this._starting          = false;   // StartRecording in flight (maybe activating daemon)
        this._streamTyped       = '';      // text typed via chunks this session
        this._typedViaChunks    = false;
        this._streamMismatch    = false;   // final text didn't extend the typed chunks
        this._discardReason     = null;    // SessionDiscarded reason, consumed on Idle
        this._lastTranscription = '';
        this._lastError         = null;
        this._cancelled         = false;
        this._cancelBound       = false;
        this._overlayTimeoutId  = 0;
        this._repasteWaitId     = 0;
        this._pttWatchId        = 0;
        this._pttAwaitingStart  = false;
        this._pttStopPending    = false;

        this._bindShortcuts();
        this._watchDaemon();
    }

    disable() {
        this._cancellable.cancel();
        this._unbindShortcuts();
        this._unwatchDaemon();
        this._disconnectSignals();
        this._clearOverlayTimeout();
        this._stopPttWatch();
        if (this._repasteWaitId) {
            GLib.source_remove(this._repasteWaitId);
            this._repasteWaitId = 0;
        }
        this._overlay.destroy();

        this._overlay           = null;
        this._cancellable       = null;
        this._settings          = null;
        this._nameOwner         = null;
        this._daemonState       = 'Idle';
        this._streamTyped       = '';
        this._lastTranscription = '';
        this._lastError         = null;
    }

    // ── Daemon connection management ─────────────────────────────────────────

    _watchDaemon() {
        this._watchId = Gio.bus_watch_name(
            Gio.BusType.SESSION,
            BUS_NAME,
            Gio.BusNameWatcherFlags.NONE,
            (_conn, _name, owner) => this._onDaemonAppeared(owner),
            () => this._onDaemonGone(),
        );
    }

    _unwatchDaemon() {
        if (this._watchId) {
            Gio.bus_unwatch_name(this._watchId);
            this._watchId = 0;
        }
    }

    _onDaemonAppeared(owner) {
        if (!this._settings || owner === this._nameOwner) return;
        this._disconnectSignals();
        this._nameOwner = owner;
        this._connectSignals(owner);
        console.log(`Transcriber: connected to daemon (${owner})`);
        this._syncState(true);
    }

    _onDaemonGone() {
        if (!this._settings) return;
        const wasConnected = this._nameOwner !== null;
        this._disconnectSignals();
        this._nameOwner = null;
        // While we're DBus-activating the daemon, the name is expected to be
        // unowned — don't tear down the "Starting daemon…" overlay.
        if (this._starting && !wasConnected) return;
        if (wasConnected) console.log('Transcriber: daemon disappeared');
        this._daemonState = 'Idle';
        this._resetTransient();
        this._overlay.destroy();
    }

    /** Clear per-session timers, grabs and flags (not settings/last text). */
    _resetTransient() {
        this._clearOverlayTimeout();
        this._stopPttWatch();
        this._setCancelBinding(false);
        this._pttAwaitingStart = false;
        this._pttStopPending   = false;
        this._streamTyped      = '';
        this._typedViaChunks   = false;
        this._streamMismatch   = false;
        this._discardReason    = null;
        this._lastError        = null;
        this._cancelled        = false;
    }

    /**
     * Fetch CurrentState and apply it if it differs from what we believe —
     * covers signals emitted before our subscription was in place (e.g. the
     * first StateChanged after DBus activation).
     */
    _syncState(onConnect = false) {
        getCurrentState(this._cancellable).then(state => {
            if (!this._settings || !this._nameOwner) return;
            if (!state || state === this._daemonState) return;
            // On (re)connect, a stale terminal state (e.g. an old Done) is
            // adopted silently rather than flashing "✓ Copied!" at login.
            if (onConnect && !this._starting && !ACTIVE_STATES.has(state))
                this._daemonState = state;
            else
                this._onStateChanged(state);
        }).catch(e => {
            if (!isCancelledError(e))
                console.warn('Transcriber: could not read CurrentState:', e.message);
        });
    }

    // ── DBus signal handling ─────────────────────────────────────────────────

    _connectSignals(owner) {
        const bus   = Gio.DBus.session;
        const flags = Gio.DBusSignalFlags.NONE;
        // Subscribing with the daemon's *unique* name means only the process
        // that actually owns org.transcriber.Daemon can drive us — another
        // client can't emit a spoofed TranscriptionReady and get text typed.
        const sub = (name, handler) => bus.signal_subscribe(
            owner, INTERFACE, name, OBJECT_PATH, null, flags,
            (_c, _s, _p, _i, _n, params) => {
                if (!this._settings) return;
                try {
                    handler(params);
                } catch (e) {
                    console.error(`Transcriber: ${name} handler failed:`, e.message);
                }
            });

        this._signalIds.push(
            sub('StateChanged', p =>
                this._onStateChanged(p.get_child_value(0).get_string()[0])),
            sub('AudioLevel', p =>
                this._overlay.setLevel(p.get_child_value(0).get_double())),
            sub('ErrorOccurred', p => {
                this._lastError = p.get_child_value(0).get_string()[0];
            }),
            // Emitted right before StateChanged("Idle") when a session ends
            // without text; the Idle handler shows the matching message.
            sub('SessionDiscarded', p => {
                this._discardReason = p.get_child_value(0).get_string()[0];
            }),
            sub('TranscriptionReady', p =>
                this._onTranscriptionReady(p.get_child_value(0).get_string()[0])),
            sub('TranscriptionChunk', p =>
                this._onTranscriptionChunk(p.get_child_value(0).get_string()[0])),
        );
    }

    _disconnectSignals() {
        const bus = Gio.DBus.session;
        this._signalIds.forEach(id => bus.signal_unsubscribe(id));
        this._signalIds = [];
    }

    _onTranscriptionReady(text) {
        if (this._cancelled) return;
        console.log(`Transcriber: TranscriptionReady (${text.length} chars)`);
        if (text) this._lastTranscription = text;

        if (this._typedViaChunks) {
            // Streaming: the chunks already typed a prefix. Type only what
            // extends it; never retype text that's already in the window.
            if (text.startsWith(this._streamTyped)) {
                const rest = text.slice(this._streamTyped.length);
                if (rest) this._directInject(rest);
                this._streamTyped = text;
            } else {
                console.warn(`Transcriber: final text (${text.length} chars) does not extend ` +
                    `the ${this._streamTyped.length} chars already typed — copying instead`);
                St.Clipboard.get_default().set_text(St.ClipboardType.CLIPBOARD, text);
                this._streamMismatch = true;
            }
        } else if (this._settings.get_boolean('direct-injection')) {
            this._directInject(text);
        } else {
            St.Clipboard.get_default().set_text(St.ClipboardType.CLIPBOARD, text);
            if (this._settings.get_boolean('auto-paste'))
                this._autoPaste();
        }
    }

    _onTranscriptionChunk(text) {
        if (this._cancelled) return;
        // Chunks after the session has ended (cancel, Done) are stale.
        if (this._daemonState !== 'Recording' && this._daemonState !== 'Streaming') return;
        // Chunks are cumulative: type only the suffix beyond what was typed.
        if (!text.startsWith(this._streamTyped)) {
            console.warn(`Transcriber: stream chunk (${text.length} chars) does not extend ` +
                `the ${this._streamTyped.length} chars already typed — not typing it`);
            return;
        }
        const delta = text.slice(this._streamTyped.length);
        if (!delta) return;
        this._directInject(delta);
        this._streamTyped    = text;
        this._typedViaChunks = true;
    }

    _onStateChanged(state) {
        const prev = this._daemonState;
        this._daemonState = state;
        this._starting = false;

        // Any pending fade from a previous Done/Error/Cancel must not touch the
        // overlay for this new state.
        this._clearOverlayTimeout();
        this._setCancelBinding(ACTIVE_STATES.has(state));

        // Leaving Recording (stop, VAD, cancel, error) ends any push-to-talk
        // session. Don't clear on other transitions: an unrelated Done→Idle
        // arriving between the key press and Recording must not orphan it.
        if ((prev === 'Recording' && state !== 'Recording') || state === 'Error') {
            this._stopPttWatch();
            this._pttAwaitingStart = false;
            this._pttStopPending   = false;
        }

        switch (state) {
        case 'Recording':
            this._cancelled      = false;
            this._streamTyped    = '';
            this._typedViaChunks = false;
            this._streamMismatch = false;
            this._discardReason  = null;
            this._lastError      = null;
            this._overlay.show();
            if (this._pttAwaitingStart) {
                this._pttAwaitingStart = false;
                if (this._pttStopPending) {
                    // Key was released before the daemon started recording.
                    this._pttStopPending = false;
                    this._stopPttWatch();
                    this._callDaemon('StopRecording');
                }
            }
            break;
        case 'Transcribing':
            this._showStatus('Transcribing…');
            break;
        case 'PostProcessing':
            this._showStatus('Polishing…');
            break;
        case 'Streaming':
            this._showStatus('Finalizing…');
            break;
        case 'Done': {
            if (this._cancelled) {
                this._flash('Cancelled', 700);
                break;
            }
            if (this._streamMismatch) {
                this._flash('Copied (stream mismatch)', 1500);
                break;
            }
            const typed = this._settings.get_boolean('direct-injection') ||
                this._typedViaChunks;
            const label = typed ? '✓ Typed!'
                : this._settings.get_boolean('auto-paste') ? '✓ Pasted!'
                : '✓ Copied!';
            this._flash(label, 800);
            break;
        }
        case 'Error':
            if (this._cancelled) {
                this._flash('Cancelled', 700);
            } else {
                this._flash(`✗ ${summarizeError(this._lastError)}`, ERROR_DISPLAY_MS);
            }
            this._lastError = null;
            break;
        case 'Idle': {
            // Only the daemon's SessionDiscarded says why a session ended
            // without text; any other Idle (e.g. a daemon restart) just hides.
            const reason = this._discardReason;
            this._discardReason = null;
            if (reason === 'cancelled') {
                this._flash('Cancelled', 700);
            } else if (reason === 'no-speech') {
                this._flash('No speech detected', 1200);
            } else {
                if (reason !== null)
                    console.warn(`Transcriber: unknown SessionDiscarded reason "${reason}"`);
                this._overlay.fadeOut();
            }
            break;
        }
        }
    }

    _showStatus(text) {
        this._overlay.setLevel(0);
        this._overlay.update(text);
    }

    /** Show `text`, then fade the overlay out after `ms`. */
    _flash(text, ms) {
        this._overlay.update(text);
        this._clearOverlayTimeout();
        this._overlayTimeoutId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, ms, () => {
            this._overlayTimeoutId = 0;
            this._overlay?.fadeOut();
            return GLib.SOURCE_REMOVE;
        });
    }

    _clearOverlayTimeout() {
        if (this._overlayTimeoutId) {
            GLib.source_remove(this._overlayTimeoutId);
            this._overlayTimeoutId = 0;
        }
    }

    // ── Daemon method calls ──────────────────────────────────────────────────

    _callDaemon(method, opts) {
        return callDaemon(method, this._cancellable, opts).then(() => true).catch(e => {
            if (!isCancelledError(e))
                console.error(`Transcriber: ${method} failed:`, e.message);
            return isCancelledError(e) ? null : e;
        });
    }

    _startRecording() {
        this._starting = true;
        if (!this._nameOwner) {
            // Not on the bus yet: the call below DBus-activates the daemon.
            this._clearOverlayTimeout();
            this._overlay.update('Starting daemon…');
        }
        this._callDaemon('StartRecording').then(result => {
            if (!this._settings || result === null) return;
            this._starting = false;
            if (result !== true) {
                this._pttAwaitingStart = false;
                this._stopPttWatch();
                this._flash(`✗ ${summarizeError(result.message)}`, ERROR_DISPLAY_MS);
                return;
            }
            // In case StateChanged(Recording) raced our signal subscription.
            if (this._nameOwner && this._daemonState !== 'Recording')
                this._syncState();
        });
    }

    // ── Hotkey handlers ──────────────────────────────────────────────────────

    _onToggleKey() {
        const state = this._daemonState;
        const idle  = IDLE_STATES.has(state) && !this._starting;

        if (this._settings.get_boolean('push-to-talk')) {
            if (!idle) {
                // Normally the key release stops the recording; if no release
                // watch is running (e.g. VAD/other client started it), a press
                // while recording still stops it.
                if (state === 'Recording' && !this._pttWatchId)
                    this._callDaemon('StopRecording');
                return;
            }
            const accel  = this._settings.get_strv('toggle-recording')[0];
            const groups = accelModifierGroups(accel);
            if (groups.length > 0) {
                this._pttAwaitingStart = true;
                this._pttStopPending   = false;
                this._startPttWatch(groups);
                this._startRecording();
                return;
            }
            // No modifier to watch — fall through to toggle behaviour.
            console.warn('Transcriber: push-to-talk needs a shortcut with a modifier; using toggle');
        }

        if (idle)
            this._startRecording();
        else if (state === 'Recording')
            this._callDaemon('StopRecording');
    }

    _cancelRecording() {
        if (!ACTIVE_STATES.has(this._daemonState)) return;
        this._cancelled     = true;
        this._stopPttWatch();
        this._pttAwaitingStart = false;
        this._pttStopPending   = false;
        this._clearOverlayTimeout();
        this._overlay.update('Cancelled');
        this._callDaemon('Cancel', { autoStart: false }).then(result => {
            if (this._settings && result !== true && result !== null)
                this._cancelled = false;
        });
    }

    _repasteLast() {
        const text = this._lastTranscription;
        if (!text) return;
        // The repaste shortcut's own modifiers are still held when this fires;
        // injecting now would turn "v" into e.g. Super+V. Wait for release.
        if (this._repasteWaitId) GLib.source_remove(this._repasteWaitId);
        const deadline = GLib.get_monotonic_time() + REPASTE_WAIT_MS * 1000;
        this._repasteWaitId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, MODIFIER_POLL_MS, () => {
            if ((currentModifiers() & ANY_MODIFIER) && GLib.get_monotonic_time() < deadline)
                return GLib.SOURCE_CONTINUE;
            this._repasteWaitId = 0;
            if (this._settings.get_boolean('direct-injection')) {
                this._directInject(text);
            } else {
                St.Clipboard.get_default().set_text(St.ClipboardType.CLIPBOARD, text);
                this._autoPaste();
            }
            return GLib.SOURCE_REMOVE;
        });
    }

    // ── Push-to-talk ─────────────────────────────────────────────────────────
    //
    // Mutter keybindings only fire on press, and key-release events for app
    // windows never reach the stage. Instead, poll the seat's modifier state:
    // once any modifier of the push-to-talk accelerator is let go, stop.

    _startPttWatch(groups) {
        this._stopPttWatch();
        this._pttWatchId = GLib.timeout_add(GLib.PRIORITY_DEFAULT, MODIFIER_POLL_MS, () => {
            const mods = currentModifiers();
            if (groups.every(g => mods & g))
                return GLib.SOURCE_CONTINUE;
            this._pttWatchId = 0;
            this._onPttRelease();
            return GLib.SOURCE_REMOVE;
        });
    }

    _stopPttWatch() {
        if (this._pttWatchId) {
            GLib.source_remove(this._pttWatchId);
            this._pttWatchId = 0;
        }
    }

    _onPttRelease() {
        if (this._daemonState === 'Recording')
            this._callDaemon('StopRecording');
        else if (this._pttAwaitingStart)
            this._pttStopPending = true;
    }

    // ── Text output ──────────────────────────────────────────────────────────

    // Simulate a paste keypress into the currently focused window.
    // Uses Ctrl+Shift+V for terminals, Ctrl+V everywhere else.
    _autoPaste() {
        try {
            const win = global.display.focus_window;
            const cls = win?.get_wm_class() ?? '';
            const isTerminal = SHIFT_PASTE_APPS.has(cls);

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
            console.error('Transcriber: auto-paste failed:', e.message);
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
                let ks;
                if (char === '\n')      ks = Clutter.KEY_Return;
                else if (char === '\t') ks = Clutter.KEY_Tab;
                else {
                    const cp = char.codePointAt(0);
                    // Drop \r and other control characters; they have no
                    // sensible keysym and would type garbage.
                    if (cp < 0x20 || (cp >= 0x7f && cp < 0xa0)) continue;
                    // ASCII maps directly; everything else uses the
                    // 0x01000000 | codepoint Unicode keysym convention.
                    ks = cp < 0x7f ? cp : (0x01000000 | cp);
                }
                vk.notify_keyval(t++, ks, PRESS);
                vk.notify_keyval(t++, ks, RELEASE);
            }
        } catch (e) {
            console.error('Transcriber: direct inject failed:', e.message);
        }
    }

    // ── Keybindings ──────────────────────────────────────────────────────────

    _bindShortcuts() {
        Main.wm.addKeybinding(
            'toggle-recording',
            this._settings,
            // Ignore autorepeat so holding the key (push-to-talk, or a slow
            // release in toggle mode) doesn't start/stop repeatedly.
            Meta.KeyBindingFlags.IGNORE_AUTOREPEAT,
            Shell.ActionMode.ALL,
            this._onToggleKey.bind(this),
        );
        Main.wm.addKeybinding(
            'repaste-last',
            this._settings,
            Meta.KeyBindingFlags.IGNORE_AUTOREPEAT,
            Shell.ActionMode.NORMAL | Shell.ActionMode.OVERVIEW,
            this._repasteLast.bind(this),
        );
    }

    _unbindShortcuts() {
        Main.wm.removeKeybinding('toggle-recording');
        Main.wm.removeKeybinding('repaste-last');
        this._setCancelBinding(false);
    }

    /** Grab the cancel shortcut only while something is in flight. */
    _setCancelBinding(bound) {
        if (bound === this._cancelBound) return;
        if (bound) {
            Main.wm.addKeybinding(
                'cancel-recording',
                this._settings,
                Meta.KeyBindingFlags.IGNORE_AUTOREPEAT,
                Shell.ActionMode.ALL,
                this._cancelRecording.bind(this),
            );
        } else {
            Main.wm.removeKeybinding('cancel-recording');
        }
        this._cancelBound = bound;
    }
}
