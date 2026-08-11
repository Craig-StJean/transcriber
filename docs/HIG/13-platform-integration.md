# Platform Integration

Guidelines for integrating applications with the GNOME desktop and the underlying Linux
platform. Proper integration ensures that applications behave predictably, respect user
preferences, and cooperate with other software on the system.

## D-Bus Integration

D-Bus is the primary message bus for inter-application and desktop communication on Linux. It
provides two buses:

- **Session bus** — communication between applications within a single user session. Use this
  for data sharing, event notifications, and remote method calls between user-facing apps.
- **System bus** — communication with system-wide services such as NetworkManager, UPower, or
  logind. Applications should access the system bus read-only whenever possible.

### Bus Name Conventions

Use **reverse domain notation** for all bus names, object paths, and interface names.

| Element | Format | Example |
|---|---|---|
| Bus name | `org.example.AppName` | `org.gnome.TextEditor` |
| Object path | `/org/example/AppName` | `/org/gnome/TextEditor` |
| Interface name | `org.example.AppName.InterfaceName` | `org.gnome.TextEditor.Window` |

- Use **well-known names** so other applications can discover your service without knowing its
  process ID.
- Keep the bus name, object path, and interface name consistent with each other and with the
  application ID.

### D-Bus Activation

When a `.desktop` file declares `DBusActivatable=true`, the application can be started
automatically in response to a D-Bus method call. This provides several benefits:

- **Single-instance behavior.** The desktop environment reuses the running instance instead of
  launching a second copy.
- **Lazy startup.** The application launches only when needed.
- **Consistent application ID.** The desktop file name (without the `.desktop` suffix) must be
  a valid D-Bus bus name and serves as the application ID everywhere on the platform.

## GSettings

GSettings is the standard mechanism for storing and retrieving user preferences on GNOME.

- **Schema ID**: Use reverse domain notation that matches or extends your application ID
  (for example, `org.gnome.shell.extensions.transcriber`).
- **Define schemas in XML** with proper types, defaults, summaries, and descriptions for every
  key. This enables tools such as `dconf-editor` to display helpful information.

  ```xml
  <schemalist>
    <schema id="org.gnome.shell.extensions.transcriber"
            path="/org/gnome/shell/extensions/transcriber/">
      <key name="api-key" type="s">
        <default>''</default>
        <summary>API Key</summary>
        <description>Authentication key for the transcription service.</description>
      </key>
      <key name="save-history" type="b">
        <default>true</default>
        <summary>Save History</summary>
        <description>Whether to persist transcription history between sessions.</description>
      </key>
    </schema>
  </schemalist>
  ```

- **Compile schemas** before use: `glib-compile-schemas schemas/`
- **Access via code:**
  - GJS: `const settings = new Gio.Settings({ schema_id: 'org.example.App' });`
  - Rust: `let settings = gio::Settings::new("org.example.App");`

## XDG Base Directories

The XDG Base Directory Specification defines standard locations for application files. Always
use these directories — never hardcode paths relative to the home directory.

| Variable | Purpose | Default Path |
|---|---|---|
| `XDG_CONFIG_HOME` | Configuration files | `~/.config/` |
| `XDG_DATA_HOME` | Application data | `~/.local/share/` |
| `XDG_CACHE_HOME` | Non-essential cached data | `~/.cache/` |
| `XDG_STATE_HOME` | Persistent state (logs, history) | `~/.local/state/` |
| `XDG_RUNTIME_DIR` | Runtime files (sockets, locks, pipes) | `/run/user/$UID/` |

### Rules

- **Always read the environment variable first.** Fall back to the default path only if the
  variable is unset.
- **Create an application subdirectory** within each XDG directory:
  `$XDG_CONFIG_HOME/appname/`, `$XDG_CACHE_HOME/appname/`, and so on.
- **Never write to `XDG_RUNTIME_DIR` for persistent data.** This directory is cleared on
  logout.
- **Respect the user's choice.** If `XDG_CONFIG_HOME` points somewhere unusual, honor it.

## Desktop Files (.desktop)

Desktop entry files describe applications to the desktop environment. They control how the
application appears in the app launcher, what MIME types it handles, and how it is activated.

- **Location**: `~/.local/share/applications/` for per-user installs;
  `/usr/share/applications/` for system-wide installs.
- **File name**: Must match the application ID with a `.desktop` suffix
  (for example, `org.gnome.TextEditor.desktop`).

### Required Keys

| Key | Description | Example |
|---|---|---|
| `Type` | Entry type | `Application` |
| `Name` | Human-readable application name | `Text Editor` |
| `Exec` | Command to launch the application | `gnome-text-editor %U` |
| `Icon` | Icon name (without extension) | `org.gnome.TextEditor` |

### Recommended Keys

| Key | Description | Example |
|---|---|---|
| `Categories` | Freedesktop categories | `Utility;TextEditor;` |
| `Keywords` | Search terms (semicolon-separated) | `write;notepad;code;` |
| `Comment` | Short description (one sentence) | `Edit text files` |
| `DBusActivatable` | Enable D-Bus activation | `true` |
| `MimeType` | Supported MIME types | `text/plain;text/markdown;` |

## Systemd User Services

For applications or background processes that should start automatically or be managed as
services, use systemd user units.

- **Location**: `~/.config/systemd/user/`
- Use `Type=dbus` with a `BusName=` directive for services that are D-Bus activated.
- Use `Type=simple` for long-running daemons that do not register on D-Bus.
- **Enable and start**: `systemctl --user enable --now service-name`
- **View logs**: `journalctl --user -u service-name`

Example unit file:

```ini
[Unit]
Description=Transcriber Background Service

[Service]
Type=dbus
BusName=org.gnome.shell.extensions.transcriber
ExecStart=/usr/bin/transcriber-daemon

[Install]
WantedBy=default.target
```

## XDG Desktop Portals

Desktop portals provide sandboxed applications with controlled access to system features.
Even non-sandboxed applications benefit from portals because they present consistent,
desktop-integrated dialogs.

### Common Portals

| Portal | Purpose |
|---|---|
| `org.freedesktop.portal.FileChooser` | Native file open/save dialogs |
| `org.freedesktop.portal.GlobalShortcuts` | System-wide keyboard shortcuts |
| `org.freedesktop.portal.Notification` | Desktop notifications |
| `org.freedesktop.portal.Screenshot` | Screen capture and color picking |
| `org.freedesktop.portal.Background` | Request permission to run in the background |
| `org.freedesktop.portal.Settings` | Read desktop settings (color scheme, accent color) |

### Access Libraries

- **Rust**: Use the `ashpd` crate, which provides safe, async wrappers around portal D-Bus
  interfaces.
- **GJS**: Use `xdg-desktop-portal` bindings or call portal D-Bus methods directly via
  `Gio.DBusProxy`.

## Notifications

- Use `Gio.Notification` to send desktop notifications.
- **Only notify for events the user needs to know about** when the application window is not
  focused. Do not notify for routine operations that complete silently.
- **Support notification actions.** Add buttons to the notification so the user can respond
  without switching windows.
- **Respect "Do Not Disturb" mode.** The notification daemon handles this automatically when
  you use `Gio.Notification`, but avoid any custom notification mechanisms that might bypass it.
- **Use standard categories** to help the notification daemon prioritize:
  `im.received`, `email.arrived`, `transfer.complete`, `device.added`, and so on.

## MIME Types and File Associations

- Declare supported MIME types in the `MimeType` key of your `.desktop` file.
- Use **standard MIME types** defined by IANA (for example, `audio/wav`, `text/plain`,
  `application/json`).
- If your application defines a custom file format, register a custom MIME type via a
  `.xml` file in `$XDG_DATA_HOME/mime/packages/` and run `update-mime-database`.

## See Also

- [Shell Extensions](14-shell-extensions.md) — extension-specific integration patterns
- [Keyboard Shortcuts](10-keyboard-shortcuts.md) — shortcut registration and conventions
