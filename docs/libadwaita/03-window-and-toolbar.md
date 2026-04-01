# Window & Toolbar

This document covers the libadwaita widgets used to structure an application's window: the window types, toolbar management, header bars, and bottom sheets. These widgets form the outermost shell of any GNOME application.

See also: [AdwApplicationWindow docs](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/class.ApplicationWindow.html), [AdwToolbarView docs](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/class.ToolbarView.html)

---

## AdwWindow

`AdwWindow` is the base window class in libadwaita. It provides two features beyond `GtkWindow`:

- A `content` property for setting the window's main widget
- Support for `AdwBreakpoint` for adaptive layouts

Use `AdwWindow` for utility windows or secondary windows that are not tied to a `GtkApplication` instance. For the main application window, use `AdwApplicationWindow` instead.

### Properties

| Property | Type | Description |
|----------|------|-------------|
| `content` | Widget | The window's content widget |
| `current-breakpoint` | AdwBreakpoint | The currently applied breakpoint (read-only, since 1.4) |
| `dialogs` | ListModel | Open dialogs (read-only, since 1.5) |
| `visible-dialog` | AdwDialog | The topmost visible dialog (read-only, since 1.5) |

---

## AdwApplicationWindow

`AdwApplicationWindow` is the primary window type for libadwaita applications. It subclasses `GtkApplicationWindow` through `AdwWindow`, combining application integration with libadwaita's layout features.

### Key Differences from GtkApplicationWindow

| Behavior | GtkApplicationWindow | AdwApplicationWindow |
|----------|---------------------|----------------------|
| Built-in titlebar | Yes (adds one automatically) | No (you must add your own AdwHeaderBar) |
| Content property | No (use `set_child`) | Yes (`set_content`) |
| Breakpoint support | No | Yes (since 1.4) |
| Default minimum size | None | 360x200 pixels (since 1.6) |

The absence of a built-in titlebar is the most important difference. With `GtkApplicationWindow`, calling `set_titlebar()` replaces the default titlebar. With `AdwApplicationWindow`, there is no titlebar at all until you add an `AdwHeaderBar` (typically inside an `AdwToolbarView`).

### Properties

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `content` | Widget | None | The window's content widget |
| `current-breakpoint` | AdwBreakpoint | None | Currently applied breakpoint (read-only, since 1.4) |
| `dialogs` | ListModel | - | Open dialogs (read-only, since 1.5) |
| `visible-dialog` | AdwDialog | None | Topmost visible dialog (read-only, since 1.5) |

### Rust Example

```rust
use adw::prelude::*;
use adw::{ApplicationWindow, HeaderBar, ToolbarView};
use gtk::Label;

fn build_window(app: &adw::Application) {
    let toolbar_view = ToolbarView::new();
    toolbar_view.add_top_bar(&HeaderBar::new());
    toolbar_view.set_content(Some(&Label::new(Some("Hello"))));

    let window = ApplicationWindow::builder()
        .application(app)
        .title("My App")
        .default_width(800)
        .default_height(600)
        .content(&toolbar_view)
        .build();

    window.present();
}
```

### Blueprint Example

```blueprint
using Gtk 4.0;
using Adw 1;

template $MyAppWindow: Adw.ApplicationWindow {
  title: "My App";
  default-width: 800;
  default-height: 600;

  content: Adw.ToolbarView {
    [top]
    Adw.HeaderBar {}

    content: Gtk.Label {
      label: "Hello";
    };
  };
}
```

---

## AdwToolbarView

*Available since libadwaita 1.4.*

`AdwToolbarView` is the recommended way to manage toolbars in a window. It holds a content widget plus any number of top and bottom bars. It replaces the pattern of manually stacking an `AdwHeaderBar` and content inside a `GtkBox`.

### Why Use AdwToolbarView

- Automatically applies flat styling to buttons in toolbar bars
- Manages undershoot shadows when the content scrolls under the toolbar
- Handles correct spacing and visual separation between bars and content
- Compatible with a specific set of bar widgets that it recognizes

### Compatible Bar Widgets

These widgets can be added as top or bottom bars:

| Widget | Typical Position |
|--------|-----------------|
| AdwHeaderBar | Top |
| AdwTabBar | Top |
| GtkSearchBar | Top |
| GtkActionBar | Bottom |
| AdwViewSwitcherBar | Bottom |
| GtkBox with `.toolbar` style class | Top or bottom |

### Properties

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `content` | Widget | None | The main content widget |
| `top-bar-style` | ToolbarStyle | Flat | Visual style for top bars |
| `bottom-bar-style` | ToolbarStyle | Flat | Visual style for bottom bars |
| `reveal-top-bars` | bool | true | Whether top bars are visible |
| `reveal-bottom-bars` | bool | true | Whether bottom bars are visible |
| `extend-content-to-top-edge` | bool | false | Content extends behind top bars |
| `extend-content-to-bottom-edge` | bool | false | Content extends behind bottom bars |

### ToolbarStyle Values

| Style | Effect |
|-------|--------|
| `Flat` | No visual separation (default, standard for most windows) |
| `Raised` | Shadow between bar and content |
| `RaisedBorder` | Border line between bar and content |

### Methods

```rust
use adw::ToolbarView;

let toolbar_view = ToolbarView::new();

// Add bars
toolbar_view.add_top_bar(&header_bar);
toolbar_view.add_bottom_bar(&action_bar);

// Set content
toolbar_view.set_content(Some(&scrolled_window));

// Remove bars
toolbar_view.remove(toolbar_view.top_bar_at(0));
```

### Rust Example: Window with Top and Bottom Bars

```rust
use adw::prelude::*;
use adw::{ApplicationWindow, HeaderBar, ToolbarView};
use gtk::{ActionBar, Button, Label, ScrolledWindow};

fn build_window(app: &adw::Application) {
    // Create toolbar view
    let toolbar_view = ToolbarView::new();

    // Top bar: header bar with a button
    let header_bar = HeaderBar::new();
    let menu_button = Button::builder()
        .icon_name("open-menu-symbolic")
        .build();
    header_bar.pack_end(&menu_button);
    toolbar_view.add_top_bar(&header_bar);

    // Content: scrollable area
    let label = Label::builder()
        .label("Main content area")
        .vexpand(true)
        .build();
    let scrolled = ScrolledWindow::builder()
        .child(&label)
        .build();
    toolbar_view.set_content(Some(&scrolled));

    // Bottom bar: action bar
    let action_bar = ActionBar::new();
    let save_button = Button::builder()
        .label("Save")
        .css_classes(["suggested-action"])
        .build();
    action_bar.pack_end(&save_button);
    toolbar_view.add_bottom_bar(&action_bar);

    // Window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("My App")
        .content(&toolbar_view)
        .default_width(600)
        .default_height(400)
        .build();

    window.present();
}
```

### Blueprint Example: Multiple Bars

```blueprint
using Gtk 4.0;
using Adw 1;

template $MyAppWindow: Adw.ApplicationWindow {
  title: "My App";
  default-width: 600;
  default-height: 400;

  content: Adw.ToolbarView {
    [top]
    Adw.HeaderBar {
      [end]
      Gtk.Button {
        icon-name: "open-menu-symbolic";
      }
    }

    [top]
    Gtk.SearchBar search_bar {
      Gtk.SearchEntry search_entry {}
    }

    content: Gtk.ScrolledWindow {
      Gtk.Label {
        label: "Main content";
        vexpand: true;
      }
    };

    [bottom]
    Gtk.ActionBar {
      [end]
      Gtk.Button {
        label: "Save";
        styles ["suggested-action"]
      }
    }
  };
}
```

---

## AdwHeaderBar

`AdwHeaderBar` mirrors the API of `GtkHeaderBar` with several GNOME-specific additions:

- Split properties for start and end title buttons (rather than GTK's single `show-title-buttons` property)
- Automatic title and back button management when used inside `AdwNavigationView`
- Title hiding when used with `AdwBottomSheet` drag handles

### Properties

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `title-widget` | Widget | None | Custom widget replacing the title (e.g., AdwViewSwitcher) |
| `show-start-title-buttons` | bool | true | Show window controls (close/minimize) on the start side |
| `show-end-title-buttons` | bool | true | Show window controls on the end side |
| `show-title` | bool | true | Whether the title is visible |
| `show-back-button` | bool | true | Show back button in NavigationView context |
| `centering-policy` | CenteringPolicy | Loose | How the title widget is centered |
| `decoration-layout` | String | None | Custom window button layout (overrides system setting) |

### CenteringPolicy

| Value | Behavior |
|-------|----------|
| `Loose` | Title centered in available space between packed widgets |
| `Strict` | Title centered in the full header bar width (may overlap packed widgets) |

### Packing Widgets

```rust
use adw::HeaderBar;
use gtk::Button;

let header_bar = HeaderBar::new();

// Pack a button at the start (left in LTR)
let back_button = Button::builder()
    .icon_name("go-previous-symbolic")
    .build();
header_bar.pack_start(&back_button);

// Pack a button at the end (right in LTR)
let menu_button = Button::builder()
    .icon_name("open-menu-symbolic")
    .build();
header_bar.pack_end(&menu_button);
```

### NavigationView Integration

When an `AdwHeaderBar` is placed inside an `AdwNavigationView` page, it automatically:

1. Displays the page's title
2. Shows a back button when the page is not the root
3. Provides a context menu on the back button for multi-level navigation

No additional configuration is needed. This behavior can be disabled by setting `show-back-button` to `false` or `show-title` to `false`.

---

## AdwWindowTitle

`AdwWindowTitle` displays a title and optional subtitle in a header bar. It is the standard way to show two lines of text in the title area.

### Properties

| Property | Type | Description |
|----------|------|-------------|
| `title` | String | The primary title text |
| `subtitle` | String | The secondary subtitle text (empty string hides it) |

### Rust Example

```rust
use adw::{HeaderBar, WindowTitle};

let title_widget = WindowTitle::builder()
    .title("Document.txt")
    .subtitle("~/Documents")
    .build();

let header_bar = HeaderBar::new();
header_bar.set_title_widget(Some(&title_widget));
```

### Blueprint Example

```blueprint
Adw.HeaderBar {
  title-widget: Adw.WindowTitle {
    title: "Document.txt";
    subtitle: "~/Documents";
  };
}
```

---

## AdwBottomSheet

*Available since libadwaita 1.6.*

`AdwBottomSheet` provides a persistent bottom sheet that slides up from the bottom of its parent. Unlike `AdwDialog`, a bottom sheet is not destroyed when dismissed -- it remains part of the widget tree and can be shown and hidden repeatedly.

### Properties

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `content` | Widget | None | The main content behind the sheet |
| `sheet` | Widget | None | The sheet content |
| `bottom-bar` | Widget | None | Optional bar shown when sheet is closed |
| `open` | bool | false | Whether the sheet is visible |
| `show-drag-handle` | bool | true | Show the drag handle on the sheet |
| `modal` | bool | true | Whether the sheet dims the background |
| `can-close` | bool | true | Whether the user can dismiss the sheet |
| `align` | float | 0.5 | Horizontal alignment of the sheet (0.0 left, 1.0 right) |
| `full-width` | bool | false | Whether the sheet spans the full width |
| `sheet-height` | int | - | Natural height of the sheet (read-only) |

### Transition Behavior

The transition between open and closed states depends on whether a bottom bar is present:

| Bottom Bar | Open Transition | Close Transition |
|------------|----------------|-----------------|
| None | Crossfade in | Crossfade out |
| Present | Slide up from bottom bar | Slide down to bottom bar |

When a bottom bar is set, it is shown when the sheet is closed and hidden when the sheet is open. This is useful for presenting a collapsed summary that expands into full detail.

### Rust Example

```rust
use adw::prelude::*;
use adw::BottomSheet;
use gtk::{Box, Button, Label, Orientation};

let bottom_sheet = BottomSheet::new();

// Main content (always visible behind the sheet)
let main_content = Label::new(Some("Main application content"));
bottom_sheet.set_content(Some(&main_content));

// Sheet content (shown when open)
let sheet_content = Box::new(Orientation::Vertical, 12);
sheet_content.append(&Label::new(Some("Sheet detail view")));
bottom_sheet.set_sheet(Some(&sheet_content));

// Bottom bar (shown when sheet is closed)
let bottom_bar = Button::builder()
    .label("Show Details")
    .build();
bottom_sheet.set_bottom_bar(Some(&bottom_bar));

// Open the sheet
bottom_bar.connect_clicked(glib::clone!(
    #[weak] bottom_sheet,
    move |_| {
        bottom_sheet.set_open(true);
    }
));
```

### Blueprint Example

```blueprint
using Gtk 4.0;
using Adw 1;

Adw.BottomSheet {
  open: false;
  show-drag-handle: true;

  content: Gtk.Label {
    label: "Main content";
    vexpand: true;
  };

  [sheet]
  Gtk.Box {
    orientation: vertical;
    spacing: 12;
    margin-start: 12;
    margin-end: 12;
    margin-top: 12;
    margin-bottom: 12;

    Gtk.Label {
      label: "Sheet content";
      styles ["title-2"]
    }
  }

  [bottom-bar]
  Gtk.Button {
    label: "Show Details";
  }
}
```

---

## Common Window Skeleton

A typical GNOME application window combines `AdwApplicationWindow`, `AdwToolbarView`, and `AdwHeaderBar`. This skeleton serves as a starting point for most applications.

### Rust Skeleton

```rust
use adw::prelude::*;
use adw::{Application, ApplicationWindow, HeaderBar, ToolbarView};
use gtk::{Box, Button, Orientation, ScrolledWindow};

fn main() -> glib::ExitCode {
    let app = Application::builder()
        .application_id("com.example.MyApp")
        .build();

    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &adw::Application) {
    let toolbar_view = ToolbarView::new();

    // Header bar
    let header_bar = HeaderBar::new();
    let menu_button = Button::builder()
        .icon_name("open-menu-symbolic")
        .tooltip_text("Main Menu")
        .build();
    header_bar.pack_end(&menu_button);
    toolbar_view.add_top_bar(&header_bar);

    // Scrollable content area
    let content = Box::new(Orientation::Vertical, 12);
    let scrolled = ScrolledWindow::builder()
        .child(&content)
        .vexpand(true)
        .build();
    toolbar_view.set_content(Some(&scrolled));

    // Window
    let window = ApplicationWindow::builder()
        .application(app)
        .title("My App")
        .default_width(800)
        .default_height(600)
        .content(&toolbar_view)
        .build();

    window.present();
}
```

### Blueprint Skeleton

```blueprint
using Gtk 4.0;
using Adw 1;

template $MyAppWindow: Adw.ApplicationWindow {
  title: "My App";
  default-width: 800;
  default-height: 600;

  content: Adw.ToolbarView {
    [top]
    Adw.HeaderBar {
      [end]
      Gtk.MenuButton {
        icon-name: "open-menu-symbolic";
        tooltip-text: "Main Menu";
        primary: true;
      }
    }

    content: Gtk.ScrolledWindow {
      vexpand: true;

      Gtk.Box {
        orientation: vertical;
        spacing: 12;
        margin-start: 12;
        margin-end: 12;
        margin-top: 12;
        margin-bottom: 12;
      }
    };
  };
}
```

---

## Further Reading

- [Application Setup](02-application-setup.md) - Project configuration and application lifecycle
- [Overview](01-overview.md) - Full widget list and version history
- [HIG Layout & Navigation](../HIG/03-layout-and-navigation.md) - Design guidance for window structure and navigation patterns
- [HIG Controls & Inputs](../HIG/04-controls-and-inputs.md) - Guidance on header bar button placement
