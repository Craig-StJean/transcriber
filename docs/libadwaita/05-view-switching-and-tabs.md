# View Switching and Tabs

View switching and tab widgets provide navigation between peer-level views or user-managed sessions. View switchers are for a fixed set of application-defined views; tabs are for dynamic, user-created content like documents or terminal sessions.

See also: [Libadwaita Widget Gallery](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/widget-gallery.html)

---

## View Stack

### AdwViewStack

Container that shows one child page at a time. Does not provide any visible switching UI on its own; pair it with a view switcher widget.

#### Key Properties

| Property | Type | Description |
|---|---|---|
| `visible-child` | `Widget` | The currently visible child widget |
| `visible-child-name` | `string` | Name of the currently visible child |
| `hhomogeneous` | `bool` | Whether all children get the same horizontal size allocation |
| `vhomogeneous` | `bool` | Whether all children get the same vertical size allocation |
| `pages` | `GtkSelectionModel` | Selection model of `AdwViewStackPage` objects |

#### Page Properties (AdwViewStackPage)

| Property | Type | Description |
|---|---|---|
| `name` | `string` | Identifier for the page |
| `title` | `string` | Human-readable title shown in switchers |
| `icon-name` | `string` | Icon shown alongside the title |
| `needs-attention` | `bool` | Whether the page has unread content (shows a dot indicator) |
| `badge-number` | `u32` | Number badge displayed on the page tab |
| `use-underline` | `bool` | Whether underscores in the title indicate mnemonics |
| `visible` | `bool` | Whether the page appears in switchers |
| `section-name` | `string` | Section grouping for sidebar display (since 1.9) |

#### Notes

- No built-in transition animation between pages.
- Pages model supports sections (since 1.9) for use with `AdwViewSwitcherSidebar`.
- Add pages with `add_titled()` or `add_named()`.

---

## View Switcher Widgets

### AdwViewSwitcher

Desktop-style horizontal view switcher. Typically placed as the `title-widget` of an `AdwHeaderBar`.

| Property | Type | Description |
|---|---|---|
| `stack` | `AdwViewStack` | The view stack to control |
| `policy` | `ViewSwitcherPolicy` | `Narrow` (icons only) or `Wide` (icons + labels) |

Best used when there is enough horizontal space for labels. On narrow windows, hide this widget and reveal an `AdwViewSwitcherBar` instead (see the adaptive pattern below).

### AdwViewSwitcherBar

Mobile-style bottom bar view switcher. Designed to be placed as a bottom bar inside `AdwToolbarView`.

| Property | Type | Description |
|---|---|---|
| `stack` | `AdwViewStack` | The view stack to control |
| `reveal` | `bool` | Whether the bar is visible (default: `false`) |

The `reveal` property is typically toggled by a breakpoint. When `reveal` is `false`, the bar takes no space.

### AdwInlineViewSwitcher

Inline view switcher placed within content areas rather than in the header bar. Useful for switching between sub-views within a page.

| Property | Type | Description |
|---|---|---|
| `stack` | `AdwViewStack` | The view stack to control |
| `display-mode` | `InlineViewSwitcherDisplayMode` | Display mode for labels and icons |

#### Style Classes

| Class | Effect |
|---|---|
| `.flat` | Flat appearance, blends with surrounding content |
| `.round` | Rounded toggle group appearance |

### AdwViewSwitcherSidebar (since 1.9)

Sidebar view switcher that replaces `GtkStackSidebar`. Displays view stack pages in a sidebar with section support, search filtering, and item activation signals.

| Property | Type | Description |
|---|---|---|
| `stack` | `AdwViewStack` | The view stack to control |
| `search-mode-enabled` | `bool` | Whether the search filter is active |

Use this when the application has enough views to warrant a sidebar rather than a compact switcher. Pages with the `section-name` property set are grouped under section headers.

---

## Tab Widgets

### AdwTabView

Dynamic tab container. Does not display any visible tab switching UI on its own. Pair it with `AdwTabBar`, `AdwTabButton`, and/or `AdwTabOverview`.

#### Key Properties

| Property | Type | Description |
|---|---|---|
| `n-pages` | `int` | Number of open tabs (read-only) |
| `n-pinned-pages` | `int` | Number of pinned tabs (read-only) |
| `selected-page` | `AdwTabPage` | The currently selected tab |
| `is-transferring-page` | `bool` | Whether a tab is being dragged between windows |
| `default-icon` | `Gio.Icon` | Default icon for tabs without one |
| `menu-model` | `Gio.MenuModel` | Context menu model for tabs |
| `shortcuts` | `TabViewShortcuts` | Keyboard shortcuts enabled for the tab view |

#### Key Methods

| Method | Description |
|---|---|
| `append(&child)` | Add a new tab at the end |
| `prepend(&child)` | Add a new tab at the beginning |
| `insert(&child, position)` | Add a tab at a specific position |
| `close_page(&page)` | Close a tab (emits `close-page` signal) |
| `set_page_pinned(&page, pinned)` | Pin or unpin a tab |
| `reorder_page(&page, position)` | Move a tab to a new position |
| `transfer_page(&page, other_view, position)` | Move a tab to another tab view |

#### Key Signals

| Signal | Description |
|---|---|
| `page-attached` | A page was added to the tab view |
| `page-detached` | A page was removed from the tab view |
| `close-page` | A page close was requested; call `close_page_finish()` to confirm or cancel |
| `create-window` | A tab is being dragged out; return a new window's tab view to transfer the tab |

### AdwTabBar

Scrolling tab bar widget. Placed as a top bar inside `AdwToolbarView`.

| Property | Type | Description |
|---|---|---|
| `view` | `AdwTabView` | The tab view to control |
| `autohide` | `bool` | Whether to hide when there is only one tab |
| `start-action-widget` | `Widget` | Widget placed before tabs |
| `end-action-widget` | `Widget` | Widget placed after tabs |
| `expand-tabs` | `bool` | Whether tabs expand to fill available space |
| `inverted` | `bool` | Whether the drag indicator is at the top |

Pinned tabs are displayed as compact icon-only tabs at the start of the bar and cannot be scrolled out of view.

### AdwTabButton

Button that displays the current tab count and opens the `AdwTabOverview` when clicked. Typically placed in the header bar on narrow windows where the tab bar is hidden.

| Property | Type | Description |
|---|---|---|
| `view` | `AdwTabView` | The tab view to display count for |

### AdwTabOverview

Grid overview of all open tabs, similar to the tab overview in mobile browsers. Displays tab thumbnails that users can select, close, or reorder.

| Property | Type | Description |
|---|---|---|
| `view` | `AdwTabView` | The tab view to display |
| `child` | `Widget` | The main content shown when the overview is closed |
| `open` | `bool` | Whether the overview is visible |
| `enable-new-tab` | `bool` | Whether to show a "New Tab" button |
| `enable-search` | `bool` | Whether to show a search entry |
| `inverted` | `bool` | Whether new tabs appear at the start |
| `search-active` | `bool` | Whether search is currently active (read-only) |

#### Signals

| Signal | Description |
|---|---|
| `create-tab` | Emitted when the "New Tab" button is clicked; return the new page |

---

## When to Use Which

| Pattern | Widget Combination | Use When |
|---|---|---|
| **View switcher** | `AdwViewStack` + `AdwViewSwitcher` / `AdwViewSwitcherBar` | 2-5 fixed, app-defined peer views (e.g., Home, Search, Library) |
| **Inline switcher** | `AdwViewStack` + `AdwInlineViewSwitcher` | Sub-view switching within a page |
| **Sidebar switcher** | `AdwViewStack` + `AdwViewSwitcherSidebar` | 6+ views, or views with sections |
| **Tabs** | `AdwTabView` + `AdwTabBar` + `AdwTabOverview` | User-created dynamic content (documents, sessions, terminals) |
| **Navigation** | `AdwNavigationView` | Hierarchical parent-child browsing; see [04 Navigation](04-navigation.md) |

---

## Adaptive View Switcher Pattern

The standard desktop/mobile view switcher pattern uses an `AdwViewSwitcher` in the header bar on wide windows and an `AdwViewSwitcherBar` at the bottom on narrow windows. A breakpoint toggles between them.

### Blueprint XML

```xml
using Adw 1;
using Gtk 4;

Adw.ApplicationWindow window {
  width-request: 360;
  height-request: 200;
  default-width: 800;
  default-height: 600;

  content: Adw.ToolbarView {
    [top]
    Adw.HeaderBar {
      title-widget: Adw.ViewSwitcher view_switcher {
        stack: view_stack;
        policy: wide;
      };
    }

    content: Adw.ViewStack view_stack {
      Adw.ViewStackPage {
        name: "home";
        title: _("Home");
        icon-name: "go-home-symbolic";

        child: Adw.StatusPage {
          title: _("Home");
          icon-name: "go-home-symbolic";
        };
      }

      Adw.ViewStackPage {
        name: "search";
        title: _("Search");
        icon-name: "edit-find-symbolic";

        child: Adw.StatusPage {
          title: _("Search");
          icon-name: "edit-find-symbolic";
        };
      }

      Adw.ViewStackPage {
        name: "library";
        title: _("Library");
        icon-name: "library-symbolic";

        child: Adw.StatusPage {
          title: _("Library");
          icon-name: "library-symbolic";
        };
      }
    };

    [bottom]
    Adw.ViewSwitcherBar switcher_bar {
      stack: view_stack;
    }
  };

  [breakpoint]
  Adw.Breakpoint {
    condition ("max-width: 550sp")
    setters {
      view_switcher.visible: false;
      switcher_bar.reveal: true;
    }
  }
}
```

### Rust: View Stack Setup

```rust
use adw::prelude::*;

fn build_view_stack() -> adw::ViewStack {
    let stack = adw::ViewStack::new();

    // Add pages
    let home_page = create_home_page();
    let page = stack.add_titled(&home_page, Some("home"), "Home");
    page.set_icon_name(Some("go-home-symbolic"));

    let search_page = create_search_page();
    let page = stack.add_titled(&search_page, Some("search"), "Search");
    page.set_icon_name(Some("edit-find-symbolic"));

    let library_page = create_library_page();
    let page = stack.add_titled(&library_page, Some("library"), "Library");
    page.set_icon_name(Some("library-symbolic"));

    // React to page changes
    stack.connect_visible_child_name_notify(|stack| {
        if let Some(name) = stack.visible_child_name() {
            println!("Switched to: {}", name);
        }
    });

    stack
}
```

---

## Further Reading

- [04 Navigation](04-navigation.md) - Navigation views and split views
- [09 Adaptive Design](09-adaptive-design.md) - Breakpoint conditions and adaptive layout patterns
- [03 Window & Toolbar](03-window-and-toolbar.md) - AdwToolbarView for placing tab bars and switcher bars
- [HIG: Layout & Navigation](../HIG/03-layout-and-navigation.md) - When to use view switchers vs tabs vs sidebars
- [AdwViewStack (upstream)](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/class.ViewStack.html)
- [AdwTabView (upstream)](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/class.TabView.html)
