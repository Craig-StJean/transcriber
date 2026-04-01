# Preferences and Dialogs

Libadwaita provides adaptive dialog widgets that present as centered floating windows on desktop and as bottom sheets on mobile. These replace the older GTK dialog and window-based patterns with a unified, HIG-compliant approach.

See also: [Libadwaita Dialogs Documentation](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/class.Dialog.html)

---

## AdwPreferencesDialog (since 1.5)

Dialog for presenting application preferences. Contains one or more `AdwPreferencesPage` instances, each containing `AdwPreferencesGroup` instances with row widgets. Replaces the deprecated `AdwPreferencesWindow`.

### Key Properties

| Property | Type | Description |
|---|---|---|
| `visible-page` | `AdwPreferencesPage` | The currently visible page |
| `visible-page-name` | `string` | Name of the currently visible page |
| `search-enabled` | `bool` | Whether the search feature is available (default: `true`) |

### Key Methods

| Method | Description |
|---|---|
| `add(&page)` | Add a preferences page |
| `remove(&page)` | Remove a preferences page |
| `present(parent)` | Present the dialog with the given parent widget |
| `push_subpage(&page)` | Push a subpage onto the navigation stack |
| `pop_subpage()` | Pop the current subpage |
| `add_toast(&toast)` | Show a toast notification within the dialog |

### Notes

- Derived from `AdwDialog`, so it inherits adaptive behavior (floating on desktop, bottom sheet on mobile).
- The parent widget passed to `present()` must be inside an `AdwApplicationWindow` or `AdwWindow`.
- Built-in search filters across all pages and groups.
- Supports subpages for drill-down within preferences (e.g., an "Advanced" subpage).

### Blueprint XML Example

```xml
using Adw 1;
using Gtk 4;

Adw.PreferencesDialog preferences_dialog {
  Adw.PreferencesPage {
    name: "general";
    title: _("General");
    icon-name: "preferences-system-symbolic";
    description: _("Basic application settings");

    Adw.PreferencesGroup {
      title: _("Appearance");

      Adw.ComboRow color_scheme_row {
        title: _("Color Scheme");
        model: StringList {
          strings ["Light", "Dark", "System"]
        };
        selected: 2;
      }

      Adw.SwitchRow compact_mode_row {
        title: _("Compact Mode");
        subtitle: _("Reduce spacing between elements");
      }
    }

    Adw.PreferencesGroup {
      title: _("Behavior");

      Adw.SwitchRow autostart_row {
        title: _("Start on Login");
        subtitle: _("Launch the application when you log in");
      }

      Adw.SwitchRow notifications_row {
        title: _("Desktop Notifications");
        active: true;
      }
    }
  }

  Adw.PreferencesPage {
    name: "audio";
    title: _("Audio");
    icon-name: "audio-input-microphone-symbolic";

    Adw.PreferencesGroup {
      title: _("Recording");

      Adw.ComboRow {
        title: _("Input Device");
        model: StringList {
          strings ["Default", "Built-in Microphone"]
        };
      }

      Adw.SpinRow {
        title: _("Sample Rate");
        adjustment: Adjustment {
          lower: 8000;
          upper: 48000;
          step-increment: 8000;
          value: 16000;
        };
      }

      Adw.SwitchRow {
        title: _("Noise Reduction");
        active: true;
      }
    }
  }
}
```

### Rust: Showing a Preferences Dialog

```rust
use adw::prelude::*;

fn show_preferences(window: &adw::ApplicationWindow) {
    let dialog = adw::PreferencesDialog::new();

    // Build the general page
    let general_page = adw::PreferencesPage::builder()
        .name("general")
        .title("General")
        .icon_name("preferences-system-symbolic")
        .build();

    let appearance_group = adw::PreferencesGroup::builder()
        .title("Appearance")
        .build();

    let dark_mode = adw::SwitchRow::builder()
        .title("Dark Mode")
        .build();
    appearance_group.add(&dark_mode);

    general_page.add(&appearance_group);
    dialog.add(&general_page);

    dialog.present(Some(window));
}
```

---

## AdwPreferencesPage

A scrollable page within a preferences dialog. Contains one or more `AdwPreferencesGroup` instances.

### Key Properties

| Property | Type | Description |
|---|---|---|
| `name` | `string` | Identifier for the page |
| `title` | `string` | Title displayed in the sidebar/switcher |
| `icon-name` | `string` | Icon displayed alongside the title |
| `description` | `string` | Description text shown at the top of the page |
| `description-centered` | `bool` | Whether the description is centered |

### Notes

- Since 1.9, only `AdwPreferencesGroup` children are accepted. Plain widgets must be wrapped in a group.
- When a `PreferencesDialog` has multiple pages, a sidebar or view switcher is shown for navigation between them.

---

## AdwDialog (since 1.5)

Base class for adaptive dialogs. Presents as a centered floating window on desktop and as a bottom sheet on mobile. Replaces the pattern of using separate `GtkWindow` subclasses for dialogs.

### Key Properties

| Property | Type | Description |
|---|---|---|
| `title` | `string` | Dialog title |
| `content-width` | `int` | Preferred content width |
| `content-height` | `int` | Preferred content height |
| `child` | `Widget` | The dialog content widget |
| `can-close` | `bool` | Whether the dialog can be closed by the user (default: `true`) |
| `presentation-mode` | `DialogPresentationMode` | How the dialog is presented |
| `current-breakpoint` | `AdwBreakpoint` | Currently applied breakpoint (read-only) |
| `follows-content-size` | `bool` | Whether the dialog resizes to match content |

### Presentation Modes

| Mode | Description |
|---|---|
| `Auto` | Automatically chooses based on parent size (default) |
| `Floating` | Always presents as a floating centered window |
| `BottomSheet` | Always presents as a bottom sheet |

### Key Methods

| Method | Description |
|---|---|
| `present(parent)` | Present the dialog with the given parent |
| `force_close()` | Close the dialog regardless of `can-close` |
| `close()` | Request to close (respects `can-close`) |
| `add_breakpoint(breakpoint)` | Add a breakpoint for adaptive layout within the dialog |

### Notes

- The parent must be inside an `AdwWindow` or `AdwApplicationWindow`.
- Supports `AdwBreakpoint` for responsive layout changes within the dialog itself.
- Automatically handles keyboard focus, escape-to-close, and click-outside-to-close.

---

## AdwAlertDialog (since 1.5)

Dialog for presenting important messages and choices to the user. Displays a heading, body text, and a set of response buttons. Replaces the deprecated `AdwMessageDialog`.

### Key Properties

| Property | Type | Description |
|---|---|---|
| `heading` | `string` | Primary message text |
| `body` | `string` | Secondary descriptive text |
| `heading-use-markup` | `bool` | Whether heading contains Pango markup |
| `body-use-markup` | `bool` | Whether body contains Pango markup |
| `extra-child` | `Widget` | Additional widget shown between body and buttons |
| `default-response` | `string` | Response activated by Enter |
| `close-response` | `string` | Response activated by Escape or close button |
| `prefer-wide-layout` | `bool` | Whether to prefer horizontal button layout |

### Response Appearance

| Appearance | Description |
|---|---|
| `Default` | Standard button appearance |
| `Suggested` | Accent-colored, draws attention to the recommended action |
| `Destructive` | Red/destructive coloring for dangerous actions |

### Key Methods

| Method | Description |
|---|---|
| `add_response(id, label)` | Add a response button |
| `set_response_appearance(id, appearance)` | Set the visual style of a response button |
| `set_response_enabled(id, enabled)` | Enable or disable a response button |
| `choose(parent, cancellable, callback)` | Present and get the chosen response asynchronously |
| `choose_future(parent)` | Present and get the chosen response as a `Future` |

### Blueprint XML Example

```xml
using Adw 1;

Adw.AlertDialog delete_dialog {
  heading: _("Delete Recording?");
  body: _("This will permanently delete the recording and its transcript. This action cannot be undone.");
  default-response: "cancel";
  close-response: "cancel";

  responses [
    cancel: _("Cancel"),
    delete: _("Delete") destructive
  ]
}
```

### Rust Example

```rust
use adw::prelude::*;

fn show_delete_confirmation(parent: &adw::ApplicationWindow) {
    let dialog = adw::AlertDialog::builder()
        .heading("Delete Recording?")
        .body("This will permanently delete the recording. This action cannot be undone.")
        .default_response("cancel")
        .close_response("cancel")
        .build();

    dialog.add_response("cancel", "Cancel");
    dialog.add_response("delete", "Delete");
    dialog.set_response_appearance(
        "delete",
        adw::ResponseAppearance::Destructive,
    );

    dialog.connect_response(None, |_dialog, response| {
        if response == "delete" {
            // Perform deletion
        }
    });

    dialog.present(Some(parent));
}

// Alternatively, using the async Future API:
async fn confirm_delete(parent: &adw::ApplicationWindow) -> bool {
    let dialog = adw::AlertDialog::builder()
        .heading("Delete Recording?")
        .body("This action cannot be undone.")
        .default_response("cancel")
        .close_response("cancel")
        .build();

    dialog.add_response("cancel", "Cancel");
    dialog.add_response("delete", "Delete");
    dialog.set_response_appearance(
        "delete",
        adw::ResponseAppearance::Destructive,
    );

    let response = dialog.choose_future(Some(parent)).await;
    response == "delete"
}
```

---

## AdwShortcutsDialog (since 1.8)

Dialog for displaying keyboard shortcuts. Simpler structure than the deprecated `GtkShortcutsWindow`, dropping rarely-used sections/views. Contains `AdwShortcutsSection` and `AdwShortcutsItem` children.

### AdwShortcutsSection

Groups shortcuts under a titled section.

| Property | Type | Description |
|---|---|---|
| `title` | `string` | Section heading |

### AdwShortcutsItem

Individual shortcut entry showing a description and the key combination.

| Property | Type | Description |
|---|---|---|
| `title` | `string` | Description of what the shortcut does |
| `accelerator` | `string` | Key combination (GTK accelerator format, e.g., `<Control>s`) |
| `action-name` | `string` | Action name (alternative to accelerator; the shortcut is looked up from the action) |
| `subtitle` | `string` | Additional description |
| `icon-name` | `string` | Icon representing the shortcut |

### Blueprint XML Example

```xml
using Adw 1;

Adw.ShortcutsDialog shortcuts_dialog {
  Adw.ShortcutsSection {
    title: _("General");

    Adw.ShortcutsItem {
      title: _("New Recording");
      accelerator: "<Control>n";
    }

    Adw.ShortcutsItem {
      title: _("Stop Recording");
      accelerator: "<Control>period";
    }

    Adw.ShortcutsItem {
      title: _("Preferences");
      accelerator: "<Control>comma";
    }

    Adw.ShortcutsItem {
      title: _("Keyboard Shortcuts");
      accelerator: "<Control>question";
    }

    Adw.ShortcutsItem {
      title: _("Quit");
      accelerator: "<Control>q";
    }
  }

  Adw.ShortcutsSection {
    title: _("Playback");

    Adw.ShortcutsItem {
      title: _("Play / Pause");
      accelerator: "space";
    }

    Adw.ShortcutsItem {
      title: _("Seek Forward");
      accelerator: "Right";
    }

    Adw.ShortcutsItem {
      title: _("Seek Backward");
      accelerator: "Left";
    }
  }
}
```

### Rust: Showing a Shortcuts Dialog

```rust
use adw::prelude::*;

fn show_shortcuts(window: &adw::ApplicationWindow) {
    let dialog = adw::ShortcutsDialog::new();

    let section = adw::ShortcutsSection::builder()
        .title("General")
        .build();

    let item = adw::ShortcutsItem::builder()
        .title("New Recording")
        .accelerator("<Control>n")
        .build();
    section.add_item(&item);

    let item = adw::ShortcutsItem::builder()
        .title("Quit")
        .accelerator("<Control>q")
        .build();
    section.add_item(&item);

    dialog.add_section(&section);
    dialog.present(Some(window));
}
```

---

## Dialog Pattern Summary

| Need | Widget | Notes |
|---|---|---|
| Application preferences | `AdwPreferencesDialog` | Multi-page, searchable, adaptive |
| Custom dialog with arbitrary content | `AdwDialog` | Floating or bottom sheet |
| Confirmation / alert message | `AdwAlertDialog` | Heading, body, response buttons |
| Keyboard shortcuts reference | `AdwShortcutsDialog` | Sections and items |
| About dialog | `AdwAboutDialog` | Not covered here; see upstream docs |

### Deprecated Alternatives (do not use)

| Deprecated | Replacement | Since |
|---|---|---|
| `AdwPreferencesWindow` | `AdwPreferencesDialog` | 1.6 |
| `AdwMessageDialog` | `AdwAlertDialog` | 1.5 |
| `GtkShortcutsWindow` | `AdwShortcutsDialog` | 1.8 |
| `AdwAboutWindow` | `AdwAboutDialog` | 1.6 |

---

## Further Reading

- [06 Lists & Rows](06-lists-and-rows.md) - Row widgets used inside preferences groups
- [04 Navigation](04-navigation.md) - Navigation patterns for dialog subpages
- [HIG: Layout & Navigation](../HIG/03-layout-and-navigation.md) - Design guidelines for dialog and window patterns
- [AdwPreferencesDialog (upstream)](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/class.PreferencesDialog.html)
- [AdwDialog (upstream)](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/class.Dialog.html)
- [AdwAlertDialog (upstream)](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/class.AlertDialog.html)
- [AdwShortcutsDialog (upstream)](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/class.ShortcutsDialog.html)
