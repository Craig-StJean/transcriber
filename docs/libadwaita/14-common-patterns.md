# Common Patterns and Recipes

Self-contained, copy-pasteable patterns for building GNOME apps with Rust and
libadwaita. Each pattern includes Blueprint XML and/or Rust code.

---

## 1. Application Lifecycle

The minimal setup to get an `adw::Application` running.

### Rust

```rust
use adw::prelude::*;
use adw::{Application, ApplicationWindow};

fn main() -> glib::ExitCode {
    let app = Application::builder()
        .application_id("com.example.MyApp")
        .build();

    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &adw::Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("My App")
        .default_width(800)
        .default_height(600)
        .build();

    window.present();
}
```

`adw::Application` calls `adw::init()` internally, so you do not need to
initialize anything yourself. It also loads `style.css` from your application
resources automatically.

---

## 2. Window with Header Bar and Toolbar

The standard GNOME app structure: `AdwApplicationWindow` with `AdwToolbarView`
managing the header bar and content area.

### Blueprint

```xml
using Adw 1;
using Gtk 4.0;

Adw.ApplicationWindow window {
  default-width: 800;
  default-height: 600;

  content: Adw.ToolbarView {
    [top]
    Adw.HeaderBar {
      [start]
      Gtk.Button {
        icon-name: "list-add-symbolic";
        tooltip-text: "Add Item";
      }

      [end]
      Gtk.MenuButton {
        icon-name: "open-menu-symbolic";
        tooltip-text: "Main Menu";
        primary: true;
      }
    }

    content: Gtk.Label {
      label: "Hello, World!";
      styles ["title-1"]
    };
  };
}
```

### Rust

```rust
use adw::prelude::*;

fn build_ui(app: &adw::Application) {
    let header_bar = adw::HeaderBar::new();

    let add_button = gtk::Button::from_icon_name("list-add-symbolic");
    add_button.set_tooltip_text(Some("Add Item"));
    header_bar.pack_start(&add_button);

    let menu_button = gtk::MenuButton::new();
    menu_button.set_icon_name("open-menu-symbolic");
    menu_button.set_tooltip_text(Some("Main Menu"));
    header_bar.pack_end(&menu_button);

    let content = gtk::Label::builder()
        .label("Hello, World!")
        .build();
    content.add_css_class("title-1");

    let toolbar_view = adw::ToolbarView::new();
    toolbar_view.add_top_bar(&header_bar);
    toolbar_view.set_content(Some(&content));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("My App")
        .default_width(800)
        .default_height(600)
        .content(&toolbar_view)
        .build();

    window.present();
}
```

---

## 3. Preferences Dialog

A full preferences dialog with multiple pages, groups, and row types.

### Blueprint

```xml
using Adw 1;
using Gtk 4.0;

Adw.PreferencesDialog preferences_dialog {
  Adw.PreferencesPage {
    title: "General";
    icon-name: "preferences-system-symbolic";

    Adw.PreferencesGroup {
      title: "Appearance";
      description: "Customize how the application looks";

      Adw.SwitchRow dark_mode_row {
        title: "Dark Mode";
        subtitle: "Use dark color scheme";
      }

      Adw.ComboRow theme_row {
        title: "Accent Color";
        subtitle: "Choose the highlight color";
        model: Gtk.StringList {
          strings ["Blue", "Teal", "Green", "Yellow", "Orange", "Red", "Pink", "Purple"]
        };
      }
    }

    Adw.PreferencesGroup {
      title: "Behavior";

      Adw.SwitchRow {
        title: "Start Minimized";
        subtitle: "Start the application in the background";
      }

      Adw.SpinRow {
        title: "Auto-save Interval";
        subtitle: "Minutes between automatic saves";
        adjustment: Gtk.Adjustment {
          lower: 1;
          upper: 60;
          step-increment: 1;
          value: 5;
        };
      }

      Adw.EntryRow {
        title: "Default Project Name";
      }
    }
  }

  Adw.PreferencesPage {
    title: "Audio";
    icon-name: "audio-speakers-symbolic";

    Adw.PreferencesGroup {
      title: "Recording";

      Adw.ComboRow {
        title: "Sample Rate";
        model: Gtk.StringList {
          strings ["16000 Hz", "44100 Hz", "48000 Hz"]
        };
      }

      Adw.SwitchRow {
        title: "Noise Reduction";
      }
    }
  }
}
```

### Presenting the Dialog (Rust)

```rust
fn show_preferences(window: &adw::ApplicationWindow) {
    let dialog = adw::PreferencesDialog::new();
    // ... build pages and groups programmatically, or load from template
    dialog.present(Some(window));
}
```

---

## 4. Adaptive Navigation with Split View

A sidebar + content layout that collapses to a single-pane drill-down navigation
on narrow windows.

### Blueprint

```xml
using Adw 1;
using Gtk 4.0;

Adw.ApplicationWindow window {
  default-width: 900;
  default-height: 600;
  width-request: 360;
  height-request: 200;

  content: Adw.NavigationSplitView split_view {
    sidebar: Adw.NavigationPage {
      title: "Sidebar";
      tag: "sidebar";

      child: Adw.ToolbarView {
        [top]
        Adw.HeaderBar {}

        content: Gtk.ListBox sidebar_list {
          styles ["navigation-sidebar"]
        };
      };
    };

    content: Adw.NavigationPage {
      title: "Content";
      tag: "content";

      child: Adw.ToolbarView {
        [top]
        Adw.HeaderBar {}

        content: Gtk.Label {
          label: "Select an item from the sidebar";
          styles ["dimmed"]
        };
      };
    };
  };

  Adw.Breakpoint {
    condition ("max-width: 600sp")
    setters {
      split_view.collapsed: true;
    }
  }
}
```

### Rust: Handling Sidebar Activation

```rust
use adw::prelude::*;

fn setup_sidebar(split_view: &adw::NavigationSplitView, sidebar_list: &gtk::ListBox) {
    sidebar_list.connect_row_activated(
        glib::clone!(@weak split_view => move |_, row| {
            // Update content based on selection...

            // When collapsed, this navigates to the content page.
            // When expanded, this is a no-op (content is already visible).
            split_view.set_show_content(true);
        }),
    );
}
```

---

## 5. View Switcher (Desktop and Mobile)

Desktop: tabs in the header bar. Mobile: bottom bar with tabs.
A breakpoint toggles between the two layouts.

### Blueprint

```xml
using Adw 1;
using Gtk 4.0;

Adw.ApplicationWindow window {
  default-width: 600;
  default-height: 500;
  width-request: 360;

  content: Adw.ToolbarView {
    [top]
    Adw.HeaderBar {
      title-widget: Adw.ViewSwitcher view_switcher {
        stack: view_stack;
      };
    }

    content: Adw.ViewStack view_stack {
      Adw.ViewStackPage {
        name: "page1";
        title: "Home";
        icon-name: "user-home-symbolic";

        child: Gtk.Label {
          label: "Home Page";
        };
      }

      Adw.ViewStackPage {
        name: "page2";
        title: "Search";
        icon-name: "system-search-symbolic";

        child: Gtk.Label {
          label: "Search Page";
        };
      }

      Adw.ViewStackPage {
        name: "page3";
        title: "Settings";
        icon-name: "preferences-system-symbolic";

        child: Gtk.Label {
          label: "Settings Page";
        };
      }
    };

    [bottom]
    Adw.ViewSwitcherBar switcher_bar {
      stack: view_stack;
    }
  };

  Adw.Breakpoint {
    condition ("max-width: 550sp")
    setters {
      view_switcher.visible: false;
      switcher_bar.reveal: true;
    }
  }
}
```

On wide windows, the `AdwViewSwitcher` appears in the header bar and the bottom
bar is hidden. Below 550sp, the header switcher hides and the bottom bar reveals.

---

## 6. Boxed List Settings

The standard GNOME settings pattern: a clamped, boxed list of action rows.

### Blueprint

```xml
using Adw 1;
using Gtk 4.0;

Adw.Clamp {
  maximum-size: 600;
  margin-top: 24;
  margin-bottom: 24;
  margin-start: 12;
  margin-end: 12;

  child: Gtk.Box {
    orientation: vertical;
    spacing: 24;

    Gtk.ListBox {
      selection-mode: none;
      styles ["boxed-list"]

      Adw.ActionRow {
        title: "Application Version";
        subtitle: "1.4.2";
        styles ["property"]
      }

      Adw.ActionRow {
        title: "Storage Used";
        subtitle: "142 MB";
        styles ["property"]
      }
    }

    Gtk.ListBox {
      selection-mode: none;
      styles ["boxed-list"]

      Adw.SwitchRow auto_start_row {
        title: "Start Automatically";
        subtitle: "Launch when you log in";
      }

      Adw.SwitchRow notifications_row {
        title: "Notifications";
        subtitle: "Show desktop notifications";
      }

      Adw.ComboRow language_row {
        title: "Language";
        model: Gtk.StringList {
          strings ["English", "Spanish", "French", "German", "Japanese"]
        };
      }

      Adw.ExpanderRow advanced_row {
        title: "Advanced";
        subtitle: "Additional settings";

        Adw.SwitchRow {
          title: "Debug Logging";
        }

        Adw.EntryRow {
          title: "Custom Endpoint";
        }
      }
    }
  };
}
```

Key points:
- `AdwClamp` constrains width for readability on wide screens.
- `GtkListBox` must have `selection-mode: none` for `.boxed-list` to work.
- Multiple boxed lists can be stacked vertically with spacing.

---

## 7. Toast with Undo Action

Show a notification toast with an undo button. The toast auto-dismisses after a
timeout, and the action is only committed if the user does not press undo.

### Rust

```rust
use adw::prelude::*;

fn delete_item(
    toast_overlay: &adw::ToastOverlay,
    item_id: u32,
) {
    // Remove from UI immediately (optimistic delete)
    // ... remove item from list model ...

    let toast = adw::Toast::new("Item deleted");
    toast.set_button_label(Some("Undo"));
    toast.set_action_name(Some("win.undo-delete"));
    toast.set_timeout(5);

    // When toast is dismissed without undo, commit the deletion
    toast.connect_dismissed(move |_| {
        // Permanently delete item_id from database
        println!("Permanently deleting item {item_id}");
    });

    toast_overlay.add_toast(toast);
}

// In your window setup, register the undo action:
fn setup_actions(window: &adw::ApplicationWindow) {
    let action = gio::SimpleAction::new("undo-delete", None);
    action.connect_activate(move |_, _| {
        // Restore the item to the list model
        println!("Undo: restoring item");
    });
    window.add_action(&action);
}
```

### Toast Variants

```rust
// Simple notification
let toast = adw::Toast::new("File saved successfully");
toast_overlay.add_toast(toast);

// High priority (not auto-dismissed by other toasts)
let toast = adw::Toast::new("Connection lost");
toast.set_priority(adw::ToastPriority::High);
toast_overlay.add_toast(toast);

// With markup
let toast = adw::Toast::new("<b>3 files</b> moved to trash");
toast.set_use_markup(true);
toast_overlay.add_toast(toast);
```

---

## 8. Keyboard Shortcuts Dialog

Display application keyboard shortcuts in a structured dialog.

### Blueprint

```xml
using Adw 1;
using Gtk 4.0;

Adw.ShortcutsDialog shortcuts_dialog {
  Adw.ShortcutsSection {
    title: "General";

    Adw.ShortcutsItem {
      title: "New Window";
      action-name: "app.new-window";
    }

    Adw.ShortcutsItem {
      title: "Quit";
      action-name: "app.quit";
    }
  }

  Adw.ShortcutsSection {
    title: "Editing";

    Adw.ShortcutsItem {
      title: "Undo";
      accelerator: "<Control>z";
    }

    Adw.ShortcutsItem {
      title: "Redo";
      accelerator: "<Control><Shift>z";
    }

    Adw.ShortcutsItem {
      title: "Select All";
      accelerator: "<Control>a";
    }
  }

  Adw.ShortcutsSection {
    title: "Navigation";

    Adw.ShortcutsItem {
      title: "Go Back";
      accelerator: "<Alt>Left";
    }

    Adw.ShortcutsItem {
      title: "Search";
      accelerator: "<Control>f";
    }

    Adw.ShortcutsItem {
      title: "Show Shortcuts";
      accelerator: "<Control>question";
    }
  }
}
```

### Presenting the Dialog (Rust)

```rust
use adw::prelude::*;

fn show_shortcuts(window: &adw::ApplicationWindow) {
    // If using a template, load it. Otherwise build programmatically:
    let dialog = adw::ShortcutsDialog::new();

    // Build sections and items
    let section = adw::ShortcutsSection::new();
    section.set_title(Some("General"));

    let item = adw::ShortcutsItem::new();
    item.set_title(&"Quit");
    item.set_accelerator(Some("<Control>q"));
    section.add_item(&item);

    dialog.add_section(&section);
    dialog.present(Some(window));
}

// Register the shortcut action on your application:
fn setup_shortcuts(app: &adw::Application) {
    app.set_accels_for_action("app.quit", &["<Control>q"]);
    app.set_accels_for_action("app.shortcuts", &["<Control>question"]);
    app.set_accels_for_action("app.new-window", &["<Control>n"]);
}
```

---

## Pattern Summary

| Pattern | Key Widgets | Use Case |
|---------|------------|----------|
| App Lifecycle | `AdwApplication` | Every app needs this. |
| Window + Toolbar | `AdwToolbarView`, `AdwHeaderBar` | Standard window layout. |
| Preferences | `AdwPreferencesDialog`, `AdwPreferencesPage`, `AdwPreferencesGroup` | App settings. |
| Adaptive Nav | `AdwNavigationSplitView`, `AdwBreakpoint` | Sidebar + content that collapses. |
| View Switcher | `AdwViewSwitcher`, `AdwViewSwitcherBar`, `AdwViewStack` | Desktop/mobile tab switching. |
| Boxed List | `AdwClamp`, `GtkListBox.boxed-list` | Settings pages, info displays. |
| Toast + Undo | `AdwToastOverlay`, `AdwToast` | User feedback with actions. |
| Shortcuts | `AdwShortcutsDialog` | Keyboard shortcut reference. |
