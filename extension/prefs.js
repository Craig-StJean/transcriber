import Adw from 'gi://Adw';
import Gdk from 'gi://Gdk';
import Gio from 'gi://Gio';
import Gtk from 'gi://Gtk';
import { ExtensionPreferences } from 'resource:///org/gnome/Shell/Extensions/js/extensions/prefs.js';

// AdwShortcutLabel arrived in libadwaita 1.8 (GNOME 49); GtkShortcutLabel is
// the fallback for GNOME 47/48. Both expose `accelerator` and `disabled-text`.
const ShortcutLabel = Adw.ShortcutLabel ?? Gtk.ShortcutLabel;

export default class TranscriberPreferences extends ExtensionPreferences {

    fillPreferencesWindow(window) {
        const settings = this.getSettings();
        // Keep the settings object alive for the window's lifetime and drop
        // our change handlers when it closes.
        window._transcriberSettings = settings;
        const handlerIds = [];
        window.connect('close-request', () => {
            handlerIds.forEach(id => settings.disconnect(id));
            return false;
        });

        const page = new Adw.PreferencesPage({
            title: 'General',
            icon_name: 'audio-input-microphone-symbolic',
        });

        const shortcuts = new Adw.PreferencesGroup({
            title: 'Keyboard Shortcuts',
            description: 'Click a shortcut to change it',
        });
        const rows = [
            ['toggle-recording', 'Toggle Recording', 'Start or stop a recording session'],
            ['cancel-recording', 'Cancel Recording', 'Discard an in-progress recording (only active while recording)'],
            ['repaste-last', 'Re-paste Last Transcription', 'Paste the most recent transcription again'],
        ];
        for (const [key, title, subtitle] of rows)
            shortcuts.add(this._makeShortcutRow(settings, handlerIds, key, title, subtitle));
        page.add(shortcuts);

        const behaviour = new Adw.PreferencesGroup({ title: 'Behaviour' });
        const ptt = new Adw.SwitchRow({
            title: 'Push-to-Talk',
            subtitle: 'Hold the toggle shortcut to record; release to stop. The shortcut must include a modifier such as Super.',
        });
        settings.bind('push-to-talk', ptt, 'active', Gio.SettingsBindFlags.DEFAULT);
        behaviour.add(ptt);
        page.add(behaviour);

        window.add(page);
    }

    _makeShortcutRow(settings, handlerIds, key, title, subtitle) {
        const row = new Adw.ActionRow({ title, subtitle, activatable: true });

        const label = new ShortcutLabel({
            disabled_text: 'Not set',
            valign: Gtk.Align.CENTER,
        });

        const reset = new Gtk.Button({
            icon_name: 'edit-undo-symbolic',
            tooltip_text: 'Reset to default',
            valign: Gtk.Align.CENTER,
            css_classes: ['flat'],
        });
        reset.connect('clicked', () => settings.reset(key));

        // GSettings stores accelerators as string[]; display the first entry
        const refresh = () => {
            label.set_accelerator(settings.get_strv(key)[0] ?? '');
            reset.set_sensitive(settings.get_user_value(key) !== null);
        };
        refresh();
        handlerIds.push(settings.connect(`changed::${key}`, refresh));

        row.add_suffix(label);
        row.add_suffix(reset);
        row.connect('activated', () => this._captureShortcut(row, settings, key, title));
        return row;
    }

    /**
     * Modal dialog that records the next key combination into `key`.
     * Esc cancels, Backspace clears (unbinds) the shortcut.
     *
     * Note: combinations GNOME Shell already grabs (including this
     * extension's current bindings) are consumed by the compositor and never
     * reach this dialog.
     */
    _captureShortcut(parent, settings, key, title) {
        const status = new Adw.StatusPage({
            icon_name: 'preferences-desktop-keyboard-shortcuts-symbolic',
            title: 'Press a key combination',
            description: 'Esc to cancel · Backspace to disable',
        });
        const toolbar = new Adw.ToolbarView({ content: status });
        toolbar.add_top_bar(new Adw.HeaderBar());

        const dialog = new Adw.Dialog({
            title,
            content_width: 420,
            child: toolbar,
        });

        const controller = new Gtk.EventControllerKey({
            propagation_phase: Gtk.PropagationPhase.CAPTURE,
        });
        controller.connect('key-pressed', (_c, keyval, _keycode, state) => {
            const mask  = state & Gtk.accelerator_get_default_mod_mask() & ~Gdk.ModifierType.LOCK_MASK;
            const lower = Gdk.keyval_to_lower(keyval);

            if (mask === 0 && lower === Gdk.KEY_Escape) {
                dialog.close();
                return Gdk.EVENT_STOP;
            }
            if (mask === 0 && lower === Gdk.KEY_BackSpace) {
                settings.set_strv(key, []);
                dialog.close();
                return Gdk.EVENT_STOP;
            }
            // Modifier-only presses aren't valid yet — wait for the full combo.
            if (!Gtk.accelerator_valid(lower, mask))
                return Gdk.EVENT_STOP;
            // Global shortcuts without a modifier would swallow ordinary typing.
            // The cancel shortcut is only grabbed while recording, so allow it.
            if (mask === 0 && key !== 'cancel-recording' && Gdk.keyval_to_unicode(lower) !== 0) {
                status.set_description('Add a modifier such as Ctrl, Alt or Super');
                return Gdk.EVENT_STOP;
            }

            settings.set_strv(key, [Gtk.accelerator_name(lower, mask)]);
            dialog.close();
            return Gdk.EVENT_STOP;
        });
        dialog.add_controller(controller);
        dialog.present(parent);
    }
}
