# Adaptive Design

This document covers libadwaita's adaptive layout system: breakpoints for responsive UI restructuring, multi-layout views for alternative arrangements, and common adaptive patterns used in GNOME applications. Adaptive design ensures applications work well across desktop monitors, laptops, tablets, and phones.

---

## AdwBreakpoint

*Since 1.4.* `AdwBreakpoint` enables UI restructuring based on available space. When a breakpoint condition is met, it applies a set of property changes (setters) to widgets. When the condition is no longer met, the properties revert to their original values.

### Conditions

Breakpoint conditions test the available width or height of the containing widget.

| Condition Type | Description |
|----------------|-------------|
| `max-width` | Matches when width is at or below the value |
| `min-width` | Matches when width is at or above the value |
| `max-height` | Matches when height is at or below the value |
| `min-height` | Matches when height is at or above the value |

### Scalable Pixels (sp)

Breakpoint values use `sp` (scalable pixel) units. Unlike raw pixels, scalable pixels adjust with the user's text scale factor. This means breakpoints trigger earlier when text is enlarged, providing more space for larger content.

- At 100% text scale: 1sp = 1px
- At 150% text scale: 1sp = 1.5px
- A `max-width: 550sp` condition triggers at 550px with default text scaling, but at 825px when text scale is 150%

### Operators

Conditions can be combined with `and` and `or` operators:

```xml
<!-- Both conditions must be true -->
<condition>max-width: 600sp and max-height: 400sp</condition>

<!-- Either condition can be true -->
<condition>max-width: 400sp or max-height: 300sp</condition>
```

### Setter Syntax

Setters define which widget properties change when the breakpoint is active:

```xml
<object class="AdwBreakpoint">
  <condition>max-width: 550sp</condition>
  <setter object="my_widget" property="visible">False</setter>
  <setter object="split_view" property="collapsed">True</setter>
  <setter object="title_label" property="label">Short Title</setter>
</object>
```

### Important: No Minimum Size

A widget that contains breakpoints has **no minimum size**. The breakpoint system assumes the UI can adapt to any size. You must manually set `width-request` and `height-request` on the window or container to prevent the widget from collapsing to zero.

```xml
<object class="AdwApplicationWindow">
  <property name="width-request">360</property>
  <property name="height-request">200</property>
  <!-- breakpoints here -->
</object>
```

The default minimum window size since libadwaita 1.6 is 360x200 pixels.

### Compatible Containers

Breakpoints work with these container widgets:

| Widget | Notes |
|--------|-------|
| `AdwWindow` | Top-level window |
| `AdwApplicationWindow` | Application window with GtkApplication integration |
| `AdwDialog` | Adaptive dialog (since 1.5) |
| `AdwBreakpointBin` | Standalone container for breakpoints anywhere |

### Rust Example

```rust
use adw::prelude::*;

let breakpoint = adw::Breakpoint::new(
    adw::BreakpointCondition::parse("max-width: 550sp").unwrap(),
);
breakpoint.add_setter(&view_switcher, "visible", &false.to_value());
breakpoint.add_setter(&switcher_bar, "reveal", &true.to_value());

let window = adw::ApplicationWindow::builder()
    .application(app)
    .title("My App")
    .width_request(360)
    .height_request(200)
    .content(&toolbar_view)
    .build();
window.add_breakpoint(breakpoint);
```

---

## AdwBreakpointBin

*Since 1.4.* `AdwBreakpointBin` is a standalone container that supports breakpoints. Use it when you need breakpoint-driven adaptation inside a widget that is not a window or dialog.

### Key Properties

| Property | Type | Description |
|----------|------|-------------|
| `child` | `GtkWidget` | The content widget |
| `width-request` | `i32` | Minimum width (must be set manually) |
| `height-request` | `i32` | Minimum height (must be set manually) |

### When to Use

- Inside a larger layout where only a portion needs to adapt
- In custom widgets that may be embedded in different contexts
- When the adapting container is not the top-level window

```rust
let bin = adw::BreakpointBin::builder()
    .child(&content)
    .width_request(200)
    .height_request(100)
    .build();

let breakpoint = adw::Breakpoint::new(
    adw::BreakpointCondition::parse("max-width: 400sp").unwrap(),
);
breakpoint.add_setter(&label, "visible", &false.to_value());
bin.add_breakpoint(breakpoint);
```

---

## AdwMultiLayoutView

*Since 1.6.* `AdwMultiLayoutView` allows you to define multiple completely different layouts and switch between them. Children are placed in named slots and automatically reparent between layouts when the active layout changes.

### Components

| Widget | Role |
|--------|------|
| `AdwMultiLayoutView` | Container that holds layouts and manages switching |
| `AdwLayout` | A single layout definition containing an arrangement of slots |
| `AdwLayoutSlot` | A named placeholder within a layout where a child is placed |

### How It Works

1. Define named children on the `AdwMultiLayoutView`
2. Define multiple `AdwLayout` objects, each containing `AdwLayoutSlot` placeholders
3. Each slot references a child by name
4. When you switch layouts (via `set_layout()`), children move to their new positions automatically
5. Combine with breakpoints to switch layouts based on available space

### Blueprint Example

```xml
<object class="AdwMultiLayoutView" id="multi_layout">
  <child type="layout">
    <object class="AdwLayout">
      <property name="name">wide</property>
      <property name="content">
        <object class="GtkBox">
          <property name="orientation">horizontal</property>
          <child>
            <object class="AdwLayoutSlot">
              <property name="slot-name">sidebar</property>
            </object>
          </child>
          <child>
            <object class="AdwLayoutSlot">
              <property name="slot-name">content</property>
            </object>
          </child>
        </object>
      </property>
    </object>
  </child>
  <child type="layout">
    <object class="AdwLayout">
      <property name="name">narrow</property>
      <property name="content">
        <object class="GtkBox">
          <property name="orientation">vertical</property>
          <child>
            <object class="AdwLayoutSlot">
              <property name="slot-name">content</property>
            </object>
          </child>
        </object>
      </property>
    </object>
  </child>
</object>

<!-- Switch layouts with a breakpoint -->
<object class="AdwBreakpoint">
  <condition>max-width: 500sp</condition>
  <setter object="multi_layout" property="layout-name">narrow</setter>
</object>
```

---

## Adaptive Patterns

These are the standard adaptive patterns used in GNOME applications. Each pattern uses breakpoints to transition between desktop and mobile layouts.

### View Switcher Pattern

Desktop: `AdwViewSwitcher` in the header bar. Mobile: `AdwViewSwitcherBar` at the bottom. Transition at approximately 550sp.

```xml
<object class="AdwToolbarView">
  <child type="top">
    <object class="AdwHeaderBar">
      <property name="title-widget">
        <object class="AdwViewSwitcher" id="view_switcher">
          <property name="stack">view_stack</property>
        </object>
      </property>
    </object>
  </child>
  <property name="content">
    <object class="AdwViewStack" id="view_stack">
      <child>
        <object class="AdwViewStackPage">
          <property name="name">page1</property>
          <property name="title">Library</property>
          <property name="icon-name">library-symbolic</property>
          <property name="child">
            <!-- page content -->
          </property>
        </object>
      </child>
    </object>
  </property>
  <child type="bottom">
    <object class="AdwViewSwitcherBar" id="switcher_bar">
      <property name="stack">view_stack</property>
      <property name="reveal">false</property>
    </object>
  </child>
</object>

<object class="AdwBreakpoint">
  <condition>max-width: 550sp</condition>
  <setter object="view_switcher" property="visible">False</setter>
  <setter object="switcher_bar" property="reveal">True</setter>
</object>
```

See also: [View Switching and Tabs](05-view-switching-and-tabs.md)

### Split View Pattern

Desktop: sidebar and content side-by-side. Mobile: collapsed into a navigation stack (NavigationSplitView) or overlay (OverlaySplitView). Transition at approximately 600sp.

```xml
<object class="AdwNavigationSplitView" id="split_view">
  <property name="sidebar">
    <object class="AdwNavigationPage">
      <property name="title">Sidebar</property>
      <property name="child">
        <object class="AdwToolbarView">
          <child type="top">
            <object class="AdwHeaderBar"/>
          </child>
          <property name="content">
            <!-- sidebar content -->
          </property>
        </object>
      </property>
    </object>
  </property>
  <property name="content">
    <object class="AdwNavigationPage">
      <property name="title">Content</property>
      <property name="child">
        <object class="AdwToolbarView">
          <child type="top">
            <object class="AdwHeaderBar"/>
          </child>
          <property name="content">
            <!-- main content -->
          </property>
        </object>
      </property>
    </object>
  </property>
</object>

<object class="AdwBreakpoint">
  <condition>max-width: 600sp</condition>
  <setter object="split_view" property="collapsed">True</setter>
</object>
```

See also: [Navigation](04-navigation.md)

### Tab Pattern

Desktop: `AdwTabBar` visible in the toolbar. Mobile: `AdwTabOverview` accessible via `AdwTabButton`. Transition at approximately 500sp. Use a breakpoint to hide the tab bar and reveal the tab button:

```xml
<object class="AdwBreakpoint">
  <condition>max-width: 500sp</condition>
  <setter object="tab_bar" property="visible">False</setter>
  <setter object="tab_button" property="visible">True</setter>
</object>
```

See also: [View Switching and Tabs](05-view-switching-and-tabs.md)

### Triple-Pane Pattern

For applications with three panes (e.g., accounts / folders / messages), nest two `AdwNavigationSplitView` widgets and collapse them at different breakpoint widths. Collapse the inner pane first (wider breakpoint), then the outer pane (narrower breakpoint):

```xml
<!-- Collapse inner pane first -->
<object class="AdwBreakpoint">
  <condition>max-width: 860sp</condition>
  <setter object="inner_split" property="collapsed">True</setter>
</object>

<!-- Then collapse outer pane -->
<object class="AdwBreakpoint">
  <condition>max-width: 500sp</condition>
  <setter object="inner_split" property="collapsed">True</setter>
  <setter object="outer_split" property="collapsed">True</setter>
</object>
```

---

## Sidebar Item Activation

When using split views, sidebar item selection behavior differs depending on the split view type.

### NavigationSplitView

When collapsed, selecting a sidebar item should push the content page into view:

```rust
fn on_sidebar_item_activated(split_view: &adw::NavigationSplitView) {
    // Update the content page with selected item data
    // Then show the content pane
    split_view.set_show_content(true);
}
```

The `set_show_content(true)` call is a no-op when the split view is not collapsed (both panes are already visible), so it is safe to call unconditionally.

### OverlaySplitView

When collapsed, selecting a sidebar item should hide the overlay sidebar:

```rust
fn on_sidebar_item_activated(split_view: &adw::OverlaySplitView) {
    // Update the content area with selected item data
    // Then hide the sidebar if it is overlaying
    if split_view.is_collapsed() {
        split_view.set_show_sidebar(false);
    }
}
```

The `is_collapsed()` check prevents hiding the sidebar when both panes are shown side-by-side.

---

## Best Practices

- **Always use `sp` units** in breakpoint conditions so your layouts adapt to text scale settings
- **Set `width-request` and `height-request`** on any widget that uses breakpoints to prevent zero-size collapse
- **Test at multiple sizes** by resizing the window. Pay attention to the transition points
- **Use standard breakpoint values**: ~550sp for view switcher transitions, ~600sp for sidebar collapse, ~500sp for tab transitions
- **Prefer split views over custom solutions** -- `AdwNavigationSplitView` and `AdwOverlaySplitView` handle the complex transition logic for you
- **Use `AdwMultiLayoutView`** when the desktop and mobile layouts are fundamentally different, not just visibility toggles

---

## Cross-References

- [Navigation](04-navigation.md) -- AdwNavigationView, AdwNavigationSplitView, AdwOverlaySplitView
- [View Switching and Tabs](05-view-switching-and-tabs.md) -- AdwViewSwitcher, AdwTabBar, AdwTabOverview
- [Layout Widgets](10-layout-widgets.md) -- AdwClamp, AdwWrapBox, and other layout helpers
- [Overview](01-overview.md) -- version history and when each widget was introduced
