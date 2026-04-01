# Navigation

Navigation widgets control how users move between views and content within an application. Libadwaita provides a set of navigation containers that implement the GNOME HIG patterns for browsing, split-pane layouts, and sidebar navigation.

See also: [Libadwaita Adaptive Layouts](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/adaptive-layouts.html)

---

## AdwNavigationView (since 1.4)

Implements the browsing/drill-down navigation pattern. Pages are pushed onto a stack and popped off to go back. Replaces the deprecated `AdwLeaflet` with `can-unfold=false`.

### Key Properties

| Property | Type | Description |
|---|---|---|
| `animate-transitions` | `bool` | Whether to animate push/pop transitions |
| `pop-on-escape` | `bool` | Whether pressing Escape pops the visible page (default: `true`) |

### Behavior

- Maintains a navigation stack of `AdwNavigationPage` children
- Automatic swipe-back gesture on touchscreens and touchpads
- Built-in `navigation.push` and `navigation.pop` actions usable from Blueprint/XML
- When an `AdwHeaderBar` is used inside a navigation page, it automatically displays the page title and a back button (with a context menu for multi-level back navigation)
- Pages can be added statically in the UI definition or pushed dynamically in code

### Actions

| Action | Parameter | Description |
|---|---|---|
| `navigation.push` | `string` (tag) | Push the page with the given tag onto the stack |
| `navigation.pop` | none | Pop the visible page from the stack |

### Signals

| Signal | Description |
|---|---|
| `pushed` | Emitted after a page is pushed |
| `popped` | Emitted after a page is popped, receives the popped page |
| `get-next-page` | Emitted when pushing a page by tag that is not a static child; return a new page dynamically |

### Rust Example

```rust
use adw::prelude::*;
use adw::NavigationView;

let nav_view = NavigationView::new();

// Push a page dynamically
let detail_page = adw::NavigationPage::builder()
    .title("Item Detail")
    .tag("detail")
    .child(&detail_content)
    .build();
nav_view.push(&detail_page);

// Pop back
nav_view.pop();

// Handle dynamic page creation
nav_view.connect_get_next_page(|_nav, tag| {
    if tag == "detail" {
        let page = create_detail_page();
        Some(page)
    } else {
        None
    }
});
```

---

## AdwNavigationPage (since 1.4)

Container widget representing a single page within an `AdwNavigationView` or `AdwNavigationSplitView`. Each page carries a title and an optional tag for identification.

### Key Properties

| Property | Type | Description |
|---|---|---|
| `title` | `string` | Title displayed in the header bar when this page is visible |
| `tag` | `string` | Unique identifier for the page, used with `navigation.push` action |
| `child` | `Widget` | The content widget for this page |
| `can-pop` | `bool` | Whether the page can be popped (default: `true`; since 1.8) |

### Usage Notes

- The `title` property is read by `AdwHeaderBar` to display the page title automatically.
- The `tag` property must be unique within a given navigation view. It is used by the `navigation.push` action and by `NavigationView::find_page()`.
- Both the sidebar and content children of `AdwNavigationSplitView` must be `AdwNavigationPage` instances.

---

## AdwNavigationSplitView (since 1.4)

Two-pane layout with a sidebar and a content area. Both panes are `AdwNavigationPage` instances. When collapsed (typically via a breakpoint), it transforms into a navigation view where the sidebar becomes the root page and the content becomes a subpage pushed on top.

### Key Properties

| Property | Type | Default | Description |
|---|---|---|---|
| `sidebar` | `NavigationPage` | | The sidebar pane |
| `content` | `NavigationPage` | | The content pane |
| `collapsed` | `bool` | `false` | Whether the split view is in collapsed (single-pane) mode |
| `show-content` | `bool` | `false` | When collapsed, whether the content page is shown (vs. the sidebar) |
| `sidebar-width-fraction` | `f64` | `0.25` | Fraction of total width allocated to the sidebar |
| `min-sidebar-width` | `f64` | `180.0` | Minimum sidebar width in scalable pixels |
| `max-sidebar-width` | `f64` | `280.0` | Maximum sidebar width in scalable pixels |

### Collapsed Behavior

When `collapsed` is `true`:
- The two panes behave as a single `AdwNavigationView`
- The sidebar becomes the root page
- The content becomes a subpage that can be pushed/popped
- `navigation.push` shows the content pane
- `navigation.pop` returns to the sidebar

### Actions

| Action | Parameter | Description |
|---|---|---|
| `navigation.push` | none | Show the content pane (when collapsed) |
| `navigation.pop` | none | Return to the sidebar (when collapsed) |

### Rust Example

```rust
use adw::prelude::*;

let split_view = adw::NavigationSplitView::new();

let sidebar_page = adw::NavigationPage::builder()
    .title("Sidebar")
    .tag("sidebar")
    .child(&sidebar_content)
    .build();

let content_page = adw::NavigationPage::builder()
    .title("Content")
    .tag("content")
    .child(&content_area)
    .build();

split_view.set_sidebar(Some(&sidebar_page));
split_view.set_content(Some(&content_page));

// Show the content pane when an item is activated in the sidebar
// (relevant when collapsed)
split_view.set_show_content(true);
```

---

## AdwOverlaySplitView (since 1.4)

Two-pane layout where the sidebar overlays the content when collapsed, rather than transforming into a navigation view. Replaces the deprecated `AdwFlap`.

### Key Properties

| Property | Type | Default | Description |
|---|---|---|---|
| `sidebar` | `Widget` | | The sidebar widget |
| `content` | `Widget` | | The content widget |
| `collapsed` | `bool` | `false` | Whether the sidebar overlays the content |
| `show-sidebar` | `bool` | `true` | Whether the sidebar is visible (when collapsed, controls the overlay) |
| `sidebar-position` | `GtkPackType` | `Start` | Position of the sidebar (`Start` = left, `End` = right) |
| `sidebar-width-fraction` | `f64` | `0.25` | Fraction of total width for sidebar |
| `min-sidebar-width` | `f64` | `180.0` | Minimum sidebar width in scalable pixels |
| `max-sidebar-width` | `f64` | `280.0` | Maximum sidebar width in scalable pixels |
| `pin-sidebar` | `bool` | `false` | Whether the sidebar stays visible alongside content even when collapsed |
| `enable-show-gesture` | `bool` | `true` | Whether swipe gesture can reveal sidebar |
| `enable-hide-gesture` | `bool` | `true` | Whether swipe gesture can hide sidebar |

### Collapsed Behavior

When `collapsed` is `true`:
- The sidebar slides over the content as an overlay
- A dimming layer appears behind the sidebar
- Clicking the dimmed content area hides the sidebar
- Swipe gestures can show/hide the sidebar
- Toggle visibility with the `show-sidebar` property

### When to Choose OverlaySplitView vs NavigationSplitView

| Scenario | Widget |
|---|---|
| Sidebar is primary navigation (selecting an item changes content) | `AdwNavigationSplitView` |
| Sidebar is supplementary (tools panel, file tree, properties) | `AdwOverlaySplitView` |
| Content should remain visible when sidebar is shown on mobile | `AdwOverlaySplitView` |
| Content replaces sidebar on mobile (drill-down) | `AdwNavigationSplitView` |

---

## AdwSidebar (since 1.9)

A dedicated sidebar widget with built-in support for selection, sections, tooltips, context menus, drop targets, suffix widgets, and search filtering. Designed to be used inside `AdwNavigationSplitView` or `AdwOverlaySplitView`.

### Key Properties

| Property | Type | Description |
|---|---|---|
| `model` | `GListModel` | The data model of `AdwSidebarItem` objects |
| `mode` | `AdwSidebarMode` | Display mode: `Auto`, `Sidebar`, `BoxedList` |
| `selected-item` | `AdwSidebarItem` | The currently selected item |
| `search-mode-enabled` | `bool` | Whether the search filter is active |

### Mode Property

| Mode | Description |
|---|---|
| `Auto` | Automatically switches between sidebar and boxed list based on context |
| `Sidebar` | Always display as a sidebar (`.navigation-sidebar` style) |
| `BoxedList` | Display as a boxed list (appropriate for mobile/narrow contexts) |

### Signals

| Signal | Description |
|---|---|
| `activated` | Emitted when an item is activated (clicked or selected) |
| `setup-context-menu` | Emitted to set up a context menu for an item |

### Rust Example

```rust
use adw::prelude::*;

let sidebar = adw::Sidebar::new();
sidebar.set_model(Some(&sidebar_model));

sidebar.connect_activated(|sidebar, item| {
    let selected = sidebar.selected_item();
    // Update the content pane based on selection
});
```

---

## Adaptive Navigation with Breakpoint

A common pattern combines `AdwNavigationSplitView` with `AdwBreakpoint` to create a layout that collapses on narrow windows. Each pane gets its own `AdwToolbarView` and `AdwHeaderBar`.

### Blueprint XML Example

```xml
using Adw 1;
using Gtk 4;

Adw.ApplicationWindow window {
  width-request: 360;
  height-request: 200;
  default-width: 900;
  default-height: 600;

  content: Adw.NavigationSplitView split_view {
    sidebar: Adw.NavigationPage {
      title: _("Sidebar");
      tag: "sidebar";

      child: Adw.ToolbarView {
        [top]
        Adw.HeaderBar {}

        content: Gtk.ScrolledWindow {
          child: Gtk.ListBox sidebar_list {
            styles ["navigation-sidebar"]
          };
        };
      };
    };

    content: Adw.NavigationPage {
      title: _("Content");
      tag: "content";

      child: Adw.ToolbarView {
        [top]
        Adw.HeaderBar {}

        content: Adw.StatusPage {
          title: _("Select an Item");
          icon-name: "document-open-symbolic";
        };
      };
    };
  };

  [breakpoint]
  Adw.Breakpoint {
    condition ("max-width: 600sp")
    setters {
      split_view.collapsed: true;
    }
  }
}
```

### Rust: Sidebar Activation Handling

When the split view is collapsed, activating a sidebar item should show the content pane. The pattern differs between `NavigationSplitView` and `OverlaySplitView`.

```rust
use adw::prelude::*;

// --- NavigationSplitView ---
// When an item is selected, show the content pane.
// In collapsed mode this pushes the content as a subpage.
fn setup_nav_split_activation(
    sidebar_list: &gtk::ListBox,
    split_view: &adw::NavigationSplitView,
) {
    let sv = split_view.clone();
    sidebar_list.connect_row_activated(move |_list, row| {
        // Update content based on selected row...
        update_content_for_row(row);

        // Show the content pane (pushes subpage when collapsed)
        sv.set_show_content(true);
    });
}

// --- OverlaySplitView ---
// When an item is selected, hide the sidebar overlay if collapsed.
fn setup_overlay_split_activation(
    sidebar_list: &gtk::ListBox,
    split_view: &adw::OverlaySplitView,
) {
    let sv = split_view.clone();
    sidebar_list.connect_row_activated(move |_list, row| {
        // Update content based on selected row...
        update_content_for_row(row);

        // Hide the sidebar overlay when collapsed
        if sv.is_collapsed() {
            sv.set_show_sidebar(false);
        }
    });
}
```

---

## Navigation Pattern Selection

| Pattern | Widget | Use When |
|---|---|---|
| **Browsing** (push/pop) | `AdwNavigationView` | Content has parent-child hierarchy (folders, detail views) |
| **Sidebar + content** (two-pane) | `AdwNavigationSplitView` | Sidebar drives content selection; collapses to drill-down on mobile |
| **Sidebar overlay** | `AdwOverlaySplitView` | Sidebar is supplementary; content stays visible on mobile |
| **View switching** | `AdwViewStack` | 2-5 peer views; see [05 View Switching](05-view-switching-and-tabs.md) |

---

## Further Reading

- [09 Adaptive Design](09-adaptive-design.md) - Breakpoint conditions, scalable pixels, and adaptive layout patterns
- [03 Window & Toolbar](03-window-and-toolbar.md) - AdwToolbarView and AdwHeaderBar usage
- [05 View Switching & Tabs](05-view-switching-and-tabs.md) - View stack and tab navigation
- [HIG: Layout & Navigation](../HIG/03-layout-and-navigation.md) - Design guidelines for navigation patterns
- [AdwNavigationView (upstream)](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/class.NavigationView.html)
- [AdwNavigationSplitView (upstream)](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/class.NavigationSplitView.html)
- [AdwOverlaySplitView (upstream)](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/class.OverlaySplitView.html)
