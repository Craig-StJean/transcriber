import Adw from 'gi://Adw';
import Gtk from 'gi://Gtk';
import { ExtensionPreferences } from 'resource:///org/gnome/Shell/Extensions/js/extensions/prefs.js';

export default class TranscriberPreferences extends ExtensionPreferences {

    fillPreferencesWindow(window) {
        const settings = this.getSettings();

        const page = new Adw.PreferencesPage({
            title: 'General',
            icon_name: 'audio-input-microphone-symbolic',
        });

        const group = new Adw.PreferencesGroup({ title: 'Keyboard Shortcuts' });

        group.add(this._makeShortcutRow(
            settings,
            'toggle-recording',
            'Toggle Recording',
            'Start or stop a recording session',
        ));
        group.add(this._makeShortcutRow(
            settings,
            'cancel-recording',
            'Cancel Recording',
            'Discard an in-progress recording',
        ));

        page.add(group);
        window.add(page);
    }

    _makeShortcutRow(settings, key, title, subtitle) {
        const row = new Adw.ActionRow({ title, subtitle });

        // AdwShortcutLabel — libadwaita 1.8 replacement for GtkShortcutLabel
        const label = new Adw.ShortcutLabel({
            disabled_text: 'Not set',
            valign: Gtk.Align.CENTER,
        });

        // GSettings stores accelerators as string[]; display the first entry
        const refresh = () => {
            const accels = settings.get_strv(key);
            label.set_accelerator(accels[0] ?? '');
        };
        refresh();
        settings.connect(`changed::${key}`, refresh);

        row.add_suffix(label);
        return row;
    }
}
