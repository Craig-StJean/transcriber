# Lists and Rows

List rows are the primary building blocks for settings pages, option panels, and structured content in GNOME applications. Libadwaita provides a family of row widgets that present title/subtitle pairs with associated controls, all designed to be used inside boxed lists.

See also: [Libadwaita Widget Gallery](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/widget-gallery.html)

---

## AdwPreferencesGroup

Groups related preference rows under a title and optional description. This is the standard container for boxed lists in preferences pages.

### Key Properties

| Property | Type | Description |
|---|---|---|
| `title` | `string` | Group heading displayed above the boxed list |
| `description` | `string` | Descriptive text displayed below the title |
| `header-suffix` | `Widget` | Widget placed at the end of the header row (e.g., an "Add" button) |
| `separate-rows` | `bool` | Display each row as a separate card (since 1.6) |

### Key Methods

| Method | Since | Description |
|---|---|---|
| `add(&child)` | 1.0 | Add a row to the group |
| `remove(&child)` | 1.0 | Remove a row from the group |
| `bind_model(model, factory)` | 1.8 | Bind a `GListModel` to create rows dynamically |

### Notes

- Since 1.8, accepts non-`AdwPreferencesRow` children (plain widgets).
- Since 1.9, `AdwPreferencesPage` refuses non-`AdwPreferencesGroup` children, so all rows must be inside a group.
- The `separate-rows` property (since 1.6) gives each row its own card border, useful for `AdwButtonRow` items.

---

## AdwPreferencesRow

Base class for all preference row widgets. Provides the `title` property and the `use-underline` property for mnemonics. You rarely use this directly; instead use one of the specialized subclasses below.

| Property | Type | Description |
|---|---|---|
| `title` | `string` | Row title |
| `use-underline` | `bool` | Whether underscores indicate mnemonics |
| `title-selectable` | `bool` | Whether the title text can be selected/copied (since 1.2) |

---

## AdwActionRow

General-purpose row with a title, subtitle, optional icon, and slots for prefix and suffix widgets. This is the most flexible row type.

### Key Properties

| Property | Type | Description |
|---|---|---|
| `title` | `string` | Primary text |
| `subtitle` | `string` | Secondary text below the title |
| `icon-name` | `string` | Icon displayed at the start of the row |
| `activatable-widget` | `Widget` | Widget activated when the row is clicked |
| `subtitle-lines` | `int` | Max lines for subtitle (0 = unlimited) |
| `title-lines` | `int` | Max lines for title (0 = unlimited) |
| `subtitle-selectable` | `bool` | Whether the subtitle text can be selected (since 1.3) |

### Key Methods

| Method | Description |
|---|---|
| `add_prefix(&widget)` | Add a widget at the start of the row (before the title) |
| `add_suffix(&widget)` | Add a widget at the end of the row (after the title) |
| `remove(&widget)` | Remove a prefix or suffix widget |

### Style Classes

| Class | Effect |
|---|---|
| `.property` | Deemphasizes the title and emphasizes the subtitle (useful for displaying property values) |

### Rust Example

```rust
use adw::prelude::*;

let row = adw::ActionRow::builder()
    .title("Download Location")
    .subtitle("/home/user/Downloads")
    .build();
row.add_css_class("property");

// Add a "Change" button as a suffix
let button = gtk::Button::builder()
    .label("Change")
    .valign(gtk::Align::Center)
    .build();
row.add_suffix(&button);
row.set_activatable_widget(Some(&button));
```

---

## AdwSwitchRow (since 1.4)

Row with a built-in toggle switch. A convenient shorthand for an `AdwActionRow` with a `GtkSwitch` suffix.

### Key Properties

| Property | Type | Description |
|---|---|---|
| `title` | `string` | Row title |
| `subtitle` | `string` | Secondary text |
| `active` | `bool` | Whether the switch is on |

### Rust Example

```rust
let switch_row = adw::SwitchRow::builder()
    .title("Enable Notifications")
    .subtitle("Show desktop notifications for new messages")
    .active(true)
    .build();

switch_row.connect_active_notify(|row| {
    println!("Notifications: {}", if row.is_active() { "on" } else { "off" });
});
```

---

## AdwSpinRow (since 1.4)

Row with an embedded spin button for numeric input.

### Key Properties

| Property | Type | Description |
|---|---|---|
| `title` | `string` | Row title |
| `subtitle` | `string` | Secondary text |
| `adjustment` | `GtkAdjustment` | Value range and step configuration |
| `value` | `f64` | Current value |
| `digits` | `u32` | Number of decimal places |
| `numeric` | `bool` | Whether only numeric input is accepted |
| `wrap` | `bool` | Whether value wraps around at bounds |
| `snap-to-ticks` | `bool` | Whether value snaps to step increments |

### Rust Example

```rust
let adjustment = gtk::Adjustment::new(
    5.0,   // value
    1.0,   // lower
    60.0,  // upper
    1.0,   // step increment
    5.0,   // page increment
    0.0,   // page size
);

let spin_row = adw::SpinRow::builder()
    .title("Timeout (seconds)")
    .adjustment(&adjustment)
    .build();
```

---

## AdwComboRow

Row with a dropdown selection. Shows the current selection inline and opens a popover or dialog for choosing a different option.

### Key Properties

| Property | Type | Description |
|---|---|---|
| `title` | `string` | Row title |
| `subtitle` | `string` | Secondary text |
| `model` | `GListModel` | List of options |
| `selected` | `u32` | Index of the selected item |
| `selected-item` | `GObject` | The selected item object |
| `factory` | `GtkListItemFactory` | Factory for rendering items |
| `list-factory` | `GtkListItemFactory` | Factory for items in the dropdown popup |
| `header-factory` | `GtkListItemFactory` | Factory for section headers (since 1.6) |
| `enable-search` | `bool` | Whether search is available in the dropdown (since 1.4) |
| `search-match-mode` | `GtkStringFilterMatchMode` | How search matches are computed (since 1.6) |
| `use-subtitle` | `bool` | Whether to show selected item as subtitle |

### Rust Example

```rust
use gtk::StringList;

let model = StringList::new(&["Light", "Dark", "System"]);

let combo_row = adw::ComboRow::builder()
    .title("Color Scheme")
    .model(&model)
    .selected(2) // "System"
    .build();

combo_row.connect_selected_item_notify(|row| {
    if let Some(item) = row.selected_item() {
        let string_obj = item.downcast_ref::<gtk::StringObject>().unwrap();
        println!("Selected: {}", string_obj.string());
    }
});
```

---

## AdwEntryRow

Row with an inline text entry field. Suitable for forms and settings that accept text input.

### Key Properties

| Property | Type | Description |
|---|---|---|
| `title` | `string` | Placeholder/label text (displayed as floating label) |
| `text` | `string` | Current text content |
| `show-apply-button` | `bool` | Whether to show an apply button (emits `apply` signal) |
| `input-purpose` | `GtkInputPurpose` | Hint for virtual keyboards |
| `input-hints` | `GtkInputHints` | Additional input hints |
| `max-length` | `int` | Maximum character count (since 1.6) |
| `text-length` | `u32` | Current text length (read-only; since 1.6) |

### Key Methods

| Method | Description |
|---|---|
| `add_prefix(&widget)` | Add a widget before the entry |
| `add_suffix(&widget)` | Add a widget after the entry |
| `grab_focus_without_selecting()` | Focus the entry without selecting existing text |

### Signals

| Signal | Description |
|---|---|
| `apply` | Emitted when the apply button is clicked or Enter is pressed (if `show-apply-button` is set) |
| `entry-activated` | Emitted when Enter is pressed |

---

## AdwPasswordEntryRow

Specialized entry row for password input. Includes a visibility toggle button and sets appropriate input hints automatically.

| Property | Type | Description |
|---|---|---|
| `title` | `string` | Placeholder/label text |
| `text` | `string` | Current text content |

Behaves identically to `AdwEntryRow` but with password-specific defaults (hidden text, visibility toggle icon).

---

## AdwExpanderRow

Row that expands to reveal child rows. Useful for grouping advanced options under a parent toggle.

### Key Properties

| Property | Type | Description |
|---|---|---|
| `title` | `string` | Row title |
| `subtitle` | `string` | Secondary text |
| `expanded` | `bool` | Whether the row is expanded |
| `show-enable-switch` | `bool` | Whether to show an enable/disable switch |
| `enable-expansion` | `bool` | Whether expansion is enabled (controlled by the switch) |
| `icon-name` | `string` | Icon at the start of the row |
| `title-lines` | `int` | Max lines for title |
| `subtitle-lines` | `int` | Max lines for subtitle |

### Style Classes

| Class | Effect |
|---|---|
| `.property` | Deemphasizes the title, emphasizes the subtitle |

### Rust Example

```rust
let expander = adw::ExpanderRow::builder()
    .title("Advanced Options")
    .subtitle("Configure additional settings")
    .show_enable_switch(true)
    .build();

let child_row_1 = adw::SwitchRow::builder()
    .title("Verbose Logging")
    .build();
let child_row_2 = adw::SpinRow::builder()
    .title("Max Retries")
    .adjustment(&gtk::Adjustment::new(3.0, 0.0, 10.0, 1.0, 1.0, 0.0))
    .build();

expander.add_row(&child_row_1);
expander.add_row(&child_row_2);
```

---

## AdwButtonRow (since 1.6)

A list row styled as a button. Activating the row triggers its `activated` signal. Useful for action items within a preferences group.

### Key Properties

| Property | Type | Description |
|---|---|---|
| `title` | `string` | Button label |
| `start-icon-name` | `string` | Icon at the start of the row |
| `end-icon-name` | `string` | Icon at the end of the row |

### Style Classes

| Class | Effect |
|---|---|
| `.destructive-action` | Red/destructive styling for dangerous actions |
| `.suggested-action` | Accent-colored styling for primary actions |

### Rust Example

```rust
let button_row = adw::ButtonRow::builder()
    .title("Delete All Data")
    .start_icon_name("user-trash-symbolic")
    .build();
button_row.add_css_class("destructive-action");

button_row.connect_activated(|_row| {
    // Show confirmation dialog before deleting
});
```

Use `separate-rows` on the parent `AdwPreferencesGroup` to give each `AdwButtonRow` its own card border.

---

## Boxed List Pattern

The standard GNOME pattern for presenting structured settings and options. A `GtkListBox` with the `.boxed-list` style class, wrapped in an `AdwClamp` for maximum width control.

### Blueprint XML

```xml
using Adw 1;
using Gtk 4;

Adw.PreferencesGroup {
  title: _("Audio");
  description: _("Configure audio recording settings");

  Adw.ComboRow {
    title: _("Input Device");
    model: StringList {
      strings ["Default", "Built-in Microphone", "USB Headset"]
    };
  }

  Adw.SpinRow sample_rate_row {
    title: _("Sample Rate");
    subtitle: _("Hz");
    adjustment: Adjustment {
      lower: 8000;
      upper: 48000;
      step-increment: 8000;
      value: 16000;
    };
  }

  Adw.SwitchRow {
    title: _("Noise Reduction");
    subtitle: _("Apply noise filtering to input audio");
    active: true;
  }

  Adw.EntryRow {
    title: _("Custom Device Path");
    show-apply-button: true;
  }
}
```

### Manual Boxed List (without AdwPreferencesGroup)

When you need a boxed list outside of a preferences context, construct it manually:

```xml
using Adw 1;
using Gtk 4;

Adw.Clamp {
  maximum-size: 600;
  margin-start: 12;
  margin-end: 12;
  margin-top: 12;
  margin-bottom: 12;

  child: Gtk.ListBox {
    selection-mode: none;
    styles ["boxed-list"]

    Adw.ActionRow {
      title: _("Name");
      subtitle: "example-project";
      styles ["property"]
    }

    Adw.ActionRow {
      title: _("Location");
      subtitle: "/home/user/projects";
      styles ["property"]
    }
  };
}
```

### Rust: Building a Boxed List Programmatically

```rust
use adw::prelude::*;
use gtk::{ListBox, SelectionMode};

let list_box = ListBox::builder()
    .selection_mode(SelectionMode::None)
    .css_classes(vec!["boxed-list"])
    .build();

let row = adw::ActionRow::builder()
    .title("Version")
    .subtitle("1.4.2")
    .build();
row.add_css_class("property");
list_box.append(&row);

let clamp = adw::Clamp::builder()
    .maximum_size(600)
    .child(&list_box)
    .build();
```

---

## Row Selection Summary

| Need | Widget | Notes |
|---|---|---|
| Generic row with custom suffix widgets | `AdwActionRow` | Most flexible |
| Boolean toggle | `AdwSwitchRow` | Built-in switch |
| Numeric input | `AdwSpinRow` | Built-in spin button |
| Dropdown selection | `AdwComboRow` | Searchable since 1.4 |
| Text input | `AdwEntryRow` | Floating label, apply button |
| Password input | `AdwPasswordEntryRow` | Visibility toggle |
| Expandable group of child rows | `AdwExpanderRow` | Optional enable switch |
| Action button in a list | `AdwButtonRow` | Destructive/suggested styles |

---

## Further Reading

- [07 Preferences & Dialogs](07-preferences-and-dialogs.md) - AdwPreferencesDialog and AdwPreferencesPage for organizing groups into full preference UIs
- [HIG: Controls & Inputs](../HIG/04-controls-and-inputs.md) - Design guidelines for control widgets
- [03 Window & Toolbar](03-window-and-toolbar.md) - AdwToolbarView for placing list content
- [AdwActionRow (upstream)](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/class.ActionRow.html)
- [AdwPreferencesGroup (upstream)](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/class.PreferencesGroup.html)
- [Boxed Lists Style Class (upstream)](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/style-classes.html#boxed-lists)
