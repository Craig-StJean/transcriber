# Layout Widgets

This document covers libadwaita's layout and utility widgets: clamping containers, wrapping boxes, carousels, avatars, and button composition widgets. These building blocks are used throughout GNOME applications to create consistent, adaptive layouts.

---

## AdwClamp

`AdwClamp` constrains its child's maximum size while allowing the child to shrink freely. When the available space exceeds the maximum size, padding is added on both sides to center the child. This is the standard way to prevent content from stretching too wide on large screens.

### Key Properties

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `child` | `GtkWidget` | `None` | The content widget |
| `maximum-size` | `i32` | `600` | Maximum width (or height) in pixels/sp |
| `tightening-threshold` | `i32` | `400` | Width at which padding begins to tighten |
| `unit` | `LengthUnit` | `Pixel` | Unit for sizes: `Pixel` or `Sp` (since 1.4) |
| `orientation` | `GtkOrientation` | `Horizontal` | Clamp direction |

### How Tightening Works

- Below the tightening threshold: no padding, child fills available width
- Between threshold and maximum: padding gradually increases
- Above maximum: child is clamped at maximum width with symmetric padding

The tightening provides a smooth visual transition rather than an abrupt jump from full-width to clamped.

### Unit Property (since 1.4)

When `unit` is set to `Sp`, the `maximum-size` and `tightening-threshold` values scale with the text scale factor. This is recommended for content-heavy layouts where text size affects how much width is needed.

### Rust Example: Boxed List Pattern

The most common use of `AdwClamp` is wrapping a boxed list to prevent it from stretching across the entire window:

```rust
use adw::prelude::*;
use gtk::{ListBox, SelectionMode};

let list_box = ListBox::builder()
    .selection_mode(SelectionMode::None)
    .css_classes(["boxed-list"])
    .build();

let row1 = adw::ActionRow::builder()
    .title("Username")
    .subtitle("craig")
    .build();
let row2 = adw::SwitchRow::builder()
    .title("Enable notifications")
    .build();

list_box.append(&row1);
list_box.append(&row2);

let clamp = adw::Clamp::builder()
    .maximum_size(600)
    .tightening_threshold(400)
    .child(&list_box)
    .build();
```

### Blueprint

```xml
<object class="AdwClamp">
  <property name="maximum-size">600</property>
  <property name="tightening-threshold">400</property>
  <property name="child">
    <object class="GtkListBox">
      <property name="selection-mode">none</property>
      <style>
        <class name="boxed-list"/>
      </style>
      <child>
        <object class="AdwActionRow">
          <property name="title">Setting Name</property>
          <property name="subtitle">Description</property>
        </object>
      </child>
    </object>
  </property>
</object>
```

---

## AdwClampLayout

`AdwClampLayout` is a `GtkLayoutManager` version of `AdwClamp`. Assign it to any widget's layout manager to get clamping behavior without wrapping in an `AdwClamp` container.

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `maximum-size` | `i32` | `600` | Maximum child size |
| `tightening-threshold` | `i32` | `400` | Padding tightening threshold |
| `unit` | `LengthUnit` | `Pixel` | Unit for sizes (since 1.4) |

```rust
let layout = adw::ClampLayout::new();
layout.set_maximum_size(800);
my_widget.set_layout_manager(Some(layout));
```

---

## AdwClampScrollable

`AdwClampScrollable` is a scrollable version of `AdwClamp`, implementing `GtkScrollable`. Use it when the clamped content is inside a `GtkScrolledWindow`.

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `child` | `GtkWidget` | `None` | The scrollable content widget |
| `maximum-size` | `i32` | `600` | Maximum child size |
| `tightening-threshold` | `i32` | `400` | Padding tightening threshold |
| `unit` | `LengthUnit` | `Pixel` | Unit for sizes (since 1.4) |

---

## AdwWrapBox

`AdwWrapBox` arranges children like words in a paragraph: children are placed left-to-right (or right-to-left in RTL) and wrap to the next line when the available width is exhausted. Unlike a grid, children can have different widths.

### Key Properties

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `child-spacing` | `i32` | `0` | Horizontal spacing between children |
| `line-spacing` | `i32` | `0` | Vertical spacing between lines |
| `align` | `f32` | `0.0` | Line alignment (0.0 = start, 0.5 = center, 1.0 = end) |
| `justify` | `JustifyMode` | `None` | Justification mode for non-last lines |
| `justify-last-line` | `bool` | `false` | Whether to justify the last line |
| `line-homogeneous` | `bool` | `false` | Whether all children in a line share the same height |
| `wrap-policy` | `WrapPolicy` | `Natural` | `Natural` (wrap at natural size) or `Minimum` (wrap at minimum size) |
| `wrap-reverse` | `bool` | `false` | Whether to wrap lines in reverse order |

### Use Cases

- Tag/chip displays
- Button groups that should wrap rather than scroll
- Photo grids with variable-width items
- Any layout where items should flow like text

```rust
let wrap_box = adw::WrapBox::builder()
    .child_spacing(6)
    .line_spacing(6)
    .build();

for tag in &["Rust", "GNOME", "GTK4", "Libadwaita", "Linux"] {
    let label = gtk::Label::new(Some(tag));
    label.add_css_class("card");
    wrap_box.append(&label);
}
```

---

## AdwWrapLayout

`AdwWrapLayout` is a `GtkLayoutManager` version of `AdwWrapBox`. Assign it to any container to get wrap behavior.

Properties mirror `AdwWrapBox`: `child-spacing`, `line-spacing`, `align`, `justify`, `justify-last-line`, `line-homogeneous`, `wrap-policy`, `wrap-reverse`.

---

## AdwBin

`AdwBin` is a simple single-child container with no visual styling. It serves as a minimal wrapper when you need a container but do not need any special behavior.

| Property | Type | Description |
|----------|------|-------------|
| `child` | `GtkWidget` | The single child widget |

### When to Use

- As a base for custom composite widgets (subclass `AdwBin` instead of `GtkWidget`)
- As a simple wrapper to apply CSS classes or margins to a single widget
- When you need a named container for breakpoint setters

---

## AdwCarousel

`AdwCarousel` provides swipe-based page navigation. Pages are arranged horizontally (or vertically) and the user swipes between them. It is commonly used for onboarding flows, image galleries, and card-based interfaces.

### Key Properties

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `interactive` | `bool` | `true` | Whether swipe gestures are enabled |
| `spacing` | `u32` | `0` | Space between pages in pixels |
| `n-pages` | `u32` | Read-only | Number of pages |
| `position` | `f64` | Read-only | Current scroll position |
| `allow-mouse-drag` | `bool` | `true` | Whether mouse dragging works like touch swiping |
| `allow-scroll-wheel` | `bool` | `true` | Whether scroll wheel navigates pages |
| `allow-long-swipes` | `bool` | `false` | Whether swiping can skip pages |
| `reveal-duration` | `u32` | `0` | Animation duration when adding/removing pages |
| `scroll-params` | `SpringParams` | Default spring | Animation parameters for scrolling |

### Indicator Widgets

Two companion widgets show the current page position:

| Widget | Appearance |
|--------|------------|
| `AdwCarouselIndicatorDots` | Row of dots (best for few pages) |
| `AdwCarouselIndicatorLines` | Row of lines (more compact) |

Both have a `carousel` property to connect to an `AdwCarousel`.

### Rust Example

```rust
use adw::prelude::*;

let carousel = adw::Carousel::builder()
    .spacing(12)
    .allow_long_swipes(false)
    .build();

// Add pages
for i in 0..3 {
    let page = adw::StatusPage::builder()
        .icon_name("emblem-default-symbolic")
        .title(&format!("Page {}", i + 1))
        .build();
    carousel.append(&page);
}

// Add dot indicators
let dots = adw::CarouselIndicatorDots::builder()
    .carousel(&carousel)
    .build();

let container = gtk::Box::new(gtk::Orientation::Vertical, 0);
container.append(&carousel);
container.append(&dots);
```

---

## AdwAvatar

`AdwAvatar` displays a user avatar image with an automatic fallback to the user's initials when no image is available. The initials are displayed on a colored background derived from the text.

### Key Properties

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `text` | `String` | `None` | User's name (used for initials fallback) |
| `custom-image` | `GdkPaintable` | `None` | Custom avatar image |
| `show-initials` | `bool` | `false` | Whether to show initials instead of a generic icon |
| `size` | `i32` | `0` | Avatar diameter in pixels |
| `icon-name` | `String` | `None` | Custom fallback icon (since 1.8) |

```rust
let avatar = adw::Avatar::builder()
    .text("Craig Smith")
    .show_initials(true)
    .size(48)
    .build();
```

---

## AdwSplitButton

`AdwSplitButton` combines a button with a dropdown arrow. Clicking the main area triggers the primary action, while clicking the arrow opens a dropdown menu (typically a `GtkPopover` with a `GMenuModel`).

### Key Properties

| Property | Type | Description |
|----------|------|-------------|
| `label` | `String` | Button text |
| `icon-name` | `String` | Button icon |
| `child` | `GtkWidget` | Custom button content |
| `menu-model` | `GMenuModel` | Dropdown menu model |
| `popover` | `GtkPopover` | Custom dropdown popover |
| `dropdown-tooltip` | `String` | Tooltip for the dropdown arrow (since 1.2) |
| `can-shrink` | `bool` | Whether the button can shrink (since 1.4) |

```rust
let split_button = adw::SplitButton::builder()
    .label("Open")
    .menu_model(&menu_model)
    .build();
split_button.connect_clicked(|_| {
    // Handle primary action
});
```

---

## AdwButtonContent

`AdwButtonContent` provides an icon-and-label combination for use inside buttons. It handles proper spacing and alignment between the icon and text. Use it as the child of a `GtkButton` or `AdwSplitButton`.

### Key Properties

| Property | Type | Description |
|----------|------|-------------|
| `label` | `String` | Button text |
| `icon-name` | `String` | Symbolic icon name |
| `use-underline` | `bool` | Whether underscores indicate mnemonics |
| `can-shrink` | `bool` | Whether the label can ellipsize (since 1.4) |

```rust
let content = adw::ButtonContent::builder()
    .icon_name("document-open-symbolic")
    .label("Open")
    .build();

let button = gtk::Button::builder()
    .child(&content)
    .build();
```

---

## Cross-References

- [Adaptive Design](09-adaptive-design.md) -- breakpoints and responsive patterns using layout widgets
- [Visual Design](../HIG/06-visual-design.md) -- spacing guidelines (6px base unit) that apply to layout widget configuration
- [Overview](01-overview.md) -- complete widget list and version history
