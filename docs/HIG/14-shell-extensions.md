# Shell Extensions

Guidelines specific to GNOME Shell extensions. Shell extensions modify the desktop itself and
run inside the Shell process, so they must be written with extreme care to avoid memory leaks,
crashes, and interference with other extensions.

## Extension Lifecycle

Every GNOME Shell extension has two critical entry points:

- **`enable()`** — Called when the extension is activated (at login, or when the user toggles
  it on). Set up all resources here: create widgets, connect signals, register keybindings,
  start timers, and open D-Bus connections.
- **`disable()`** — Called when the extension is deactivated (at logout, when the screen locks,
  or when the user toggles it off). **Clean up everything.** The extension must be fully
  reversible — after `disable()` returns, no trace of the extension should remain in the Shell.

### Why cleanup matters

Extensions share a single JavaScript runtime with GNOME Shell. A leaked signal connection can
fire after the extension is disabled, referencing destroyed objects and crashing the Shell. A
leaked widget remains visible and consumes resources. A leaked timer continues to fire
indefinitely.

## Cleanup Requirements

This is the most common source of bugs in GNOME Shell extensions. Follow this checklist
rigorously.

| Resource | Acquire in `enable()` | Release in `disable()` |
|---|---|---|
| Signal connections | `obj.connect('signal', handler)` → store returned ID | `obj.disconnect(id)` |
| Created widgets | `new St.Label(...)` | `widget.destroy()` |
| Children added to Shell UI | `Main.panel.addToStatusArea(...)` | `indicator.destroy()` |
| Timers and timeouts | `GLib.timeout_add(...)` → store returned ID | `GLib.source_remove(id)` |
| Keybindings | `Main.wm.addKeybinding(...)` | `Main.wm.removeKeybinding(...)` |
| D-Bus proxies | `new Gio.DBusProxy(...)` | Set to `null`; call `run_dispose()` if needed |
| Object references | Store in `this._variable` | Set `this._variable = null` |

### Recommended pattern

Store all connection IDs, timer IDs, and widget references as instance properties so they can
be cleaned up systematically:

```javascript
export default class MyExtension extends Extension {
    enable() {
        this._indicator = new PanelMenu.Button(0.0, this.metadata.name, false);
        Main.panel.addToStatusArea(this.metadata.uuid, this._indicator);

        this._settingsChangedId = this._settings.connect('changed', () => {
            this._onSettingsChanged();
        });

        this._timerId = GLib.timeout_add_seconds(GLib.PRIORITY_DEFAULT, 60, () => {
            this._poll();
            return GLib.SOURCE_CONTINUE;
        });
    }

    disable() {
        if (this._timerId) {
            GLib.source_remove(this._timerId);
            this._timerId = null;
        }

        if (this._settingsChangedId) {
            this._settings.disconnect(this._settingsChangedId);
            this._settingsChangedId = null;
        }

        if (this._indicator) {
            this._indicator.destroy();
            this._indicator = null;
        }
    }
}
```

## St Toolkit (Shell Toolkit) Widgets

Shell extensions use the St (Shell Toolkit) widget library, not GTK. St is a Clutter-based
toolkit optimized for the compositor process.

### Available Widgets

| Widget | Purpose |
|---|---|
| `St.Widget` | Base widget class — use for custom containers |
| `St.BoxLayout` | Horizontal or vertical container (like GtkBox) |
| `St.Label` | Text display |
| `St.Button` | Clickable button with press/release states |
| `St.Icon` | Icon display (symbolic icons from the icon theme) |
| `St.Entry` | Single-line text input |
| `St.Bin` | Single-child container with alignment and padding |
| `St.ScrollView` | Scrollable container for content that may overflow |

### St.BoxLayout Properties

| Property | Type | Description |
|---|---|---|
| `vertical` | Boolean | `true` for column layout, `false` for row layout |
| `style_class` | String | CSS class name(s) applied to the widget |
| `x_expand` | Boolean | Expand horizontally to fill available space |
| `y_expand` | Boolean | Expand vertically to fill available space |
| `x_align` | `Clutter.ActorAlign` | Horizontal alignment: `START`, `CENTER`, `END`, `FILL` |
| `y_align` | `Clutter.ActorAlign` | Vertical alignment: `START`, `CENTER`, `END`, `FILL` |

Example:

```javascript
const box = new St.BoxLayout({
    vertical: true,
    style_class: 'my-extension-container',
    x_expand: true,
    y_expand: true,
    x_align: Clutter.ActorAlign.FILL,
});

box.add_child(new St.Label({ text: 'Hello, world' }));
box.add_child(new St.Button({ label: 'Click Me' }));
```

## Clutter Animation

All animation in Shell extensions uses the Clutter animation framework.

- Use **`actor.ease()`** for property animations. This is the modern, preferred API.
- Common animation modes:
  - `Clutter.AnimationMode.EASE_OUT_QUAD` — smooth deceleration (default for most UI motion)
  - `Clutter.AnimationMode.EASE_IN_OUT_CUBIC` — smooth acceleration and deceleration
  - `Clutter.AnimationMode.LINEAR` — constant speed
- **Recommended durations**: 200--300ms for UI transitions (show/hide, slide); 150ms for hover
  effects and micro-interactions.
- Use the **`onComplete` callback** to chain animations or perform cleanup after the animation
  finishes.
- **Respect reduced-motion preferences.** Check the system setting and skip or shorten
  animations for users who have enabled reduced motion.

```javascript
actor.ease({
    opacity: 255,
    translation_y: 0,
    duration: 250,
    mode: Clutter.AnimationMode.EASE_OUT_QUAD,
    onComplete: () => {
        // Animation finished
    },
});
```

## CSS Styling in Shell Extensions

Shell extensions use a `stylesheet.css` file in the extension directory for custom styling.

### Supported CSS Properties

St supports a **subset** of CSS. The following properties are commonly available:

- `background-color`, `background-gradient-direction`, `background-gradient-start`,
  `background-gradient-end`
- `color`, `font-size`, `font-weight`, `font-family`
- `border`, `border-color`, `border-width`, `border-radius`
- `padding`, `padding-top`, `padding-right`, `padding-bottom`, `padding-left`
- `margin`, `margin-top`, `margin-right`, `margin-bottom`, `margin-left`
- `box-shadow`
- `opacity`
- `width`, `height`, `min-width`, `min-height`, `max-width`, `max-height`
- `icon-size` (St-specific, for `St.Icon`)

### Limitations

- **No CSS variables** — unlike libadwaita, St does not support `var(--name)`. You must
  hardcode color values.
- **No flexbox or grid layout** — use `St.BoxLayout` for layout instead.
- **No advanced selectors** — pseudo-classes like `:hover`, `:focus`, `:active`, and
  `:checked` are supported, but complex combinators are limited.

### Applying Styles

- Use the `style_class` property to apply CSS classes defined in `stylesheet.css`.
- Use the inline `style` property for dynamic styles that change at runtime.

```css
/* stylesheet.css */
.my-extension-panel-button {
    padding: 0 8px;
    color: #ffffff;
}

.my-extension-panel-button:hover {
    background-color: rgba(255, 255, 255, 0.1);
}

.my-extension-popup {
    background-color: #303030;
    border-radius: 12px;
    padding: 12px;
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.4);
}
```

## Overlay and Monitor Positioning

When placing UI elements on screen, use the Shell's layout manager to query monitor geometry.

- **`Main.layoutManager.primaryMonitor`** — returns the geometry of the primary monitor
  (`x`, `y`, `width`, `height`).
- **`Main.layoutManager.monitors`** — array of all monitor geometries.
- Position actors **relative to monitor bounds**, not absolute screen coordinates.
- **Account for the panel height** and other Shell chrome when positioning overlays. The panel
  is typically 32px tall but may vary.
- Use **`Clutter.ActorAlign`** values for alignment within containers rather than manual
  coordinate calculation.

## Panel Integration

Panel buttons are the most common way for extensions to present themselves in the Shell.

- Extend **`PanelMenu.Button`** to create a panel indicator.
- Specify the panel box position: `'left'`, `'center'`, or `'right'`.
- Include an **indicator icon** (symbolic icon from the theme) and optionally a text label.
- Use **`PanelMenu.Button.menu`** to attach a popup menu that opens when the indicator is
  clicked.
- Add the indicator to the panel with `Main.panel.addToStatusArea(uuid, indicator)`.

## Extension Preferences

Extension preferences are displayed in a separate process (the Extensions app or
`gnome-extensions prefs`), not inside GNOME Shell.

- Extend the **`ExtensionPreferences`** base class.
- Implement the **`fillPreferencesWindow(window)`** method.
- The `window` parameter is an `AdwPreferencesWindow`. Build the preferences UI using:
  - `Adw.PreferencesPage` — a page within the preferences window
  - `Adw.PreferencesGroup` — a labeled group of rows
  - `Adw.ActionRow`, `Adw.SwitchRow`, `Adw.ComboRow`, `Adw.SpinRow` — individual settings
- Access GSettings via **`this.getSettings()`** within the preferences class.
- Bind settings to widgets using `settings.bind()` for automatic two-way synchronization.

## GSettings in Extensions

- Place the schema XML file in the `schemas/` subdirectory of the extension.
- **Compile schemas** during installation: `glib-compile-schemas schemas/`
- Access settings in the extension class via `this.getSettings()` (which automatically locates
  the schema within the extension directory).
- In the preferences class, also use `this.getSettings()`.

## Shell Version Compatibility

GNOME Shell extensions must declare which Shell versions they support.

- Set the **`shell-version`** array in `metadata.json`:
  ```json
  {
      "shell-version": ["47", "48"]
  }
  ```
- **GNOME 45 and later** use ESM (ECMAScript Module) imports:
  ```javascript
  import St from 'gi://St';
  import * as Main from 'resource:///org/gnome/shell/ui/main.js';
  import { Extension } from 'resource:///org/gnome/shell/extensions/extension.js';
  ```
- **GNOME 44 and earlier** use the legacy import system:
  ```javascript
  const St = imports.gi.St;
  const Main = imports.ui.main;
  ```
- **Test on all declared versions.** Behavior and API availability can differ between Shell
  releases.

## Debugging

| Technique | How |
|---|---|
| Looking Glass | Press Alt+F2, type `lg`, press Enter. Interactive JS console inside the Shell. |
| Log output | Call `log('message')` or `console.log('message')` in extension code. |
| View logs | `journalctl -f -o cat /usr/bin/gnome-shell` |
| Enable/disable | `gnome-extensions enable extension@id` / `gnome-extensions disable extension@id` |
| List installed | `gnome-extensions list --enabled` |
| Reload stylesheet | In Looking Glass: `St.ThemeContext.get_for_stage(global.stage).get_theme().load_stylesheet(file)` |

## See Also

- [Visual Design](06-visual-design.md) — color, spacing, and typography conventions
- [Platform Integration](13-platform-integration.md) — D-Bus, GSettings, and desktop file
  integration
- [Keyboard Shortcuts](10-keyboard-shortcuts.md) — keybinding registration and conventions
