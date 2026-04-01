# Adwaita CSS Style Classes Reference

Complete reference of all CSS style classes available in libadwaita. Apply these
with `widget.add_css_class("class-name")` in Rust or `<style><class name="class-name"/></style>`
in Blueprint/XML.

---

## Button Styles

| Class | Applies To | Description / Effect |
|-------|-----------|----------------------|
| `.suggested-action` | `GtkButton`, `AdwButtonRow` | Accent-colored background. Use for the primary action in a dialog or view. |
| `.destructive-action` | `GtkButton`, `AdwButtonRow` | Destructive styling. Since 1.6: transparent background with colored text (was solid red background before 1.6). |
| `.flat` | `GtkButton`, `GtkMenuButton`, `GtkToggleButton` | Flat appearance with no visible background until hovered. Default style inside toolbars. |
| `.raised` | `GtkButton`, `GtkMenuButton`, `GtkToggleButton` | Regular button appearance with visible background. Use inside toolbars when a button needs emphasis. |
| `.circular` | `GtkButton` | Round button shape. Best for icon-only buttons. Button should be square (same width/height). |
| `.pill` | `GtkButton` | Pill-shaped with large rounded corners. Use for standalone call-to-action buttons. |

### Button Style Notes

- `.flat` is automatically applied to buttons inside `.toolbar` containers and `AdwHeaderBar`.
- `.raised` counteracts the automatic `.flat` styling in toolbars.
- `.suggested-action` and `.destructive-action` are mutually exclusive.
- `.circular` and `.pill` are mutually exclusive.

### Usage Example (Rust)

```rust
let save_btn = gtk::Button::with_label("Save");
save_btn.add_css_class("suggested-action");

let delete_btn = gtk::Button::with_label("Delete");
delete_btn.add_css_class("destructive-action");

let icon_btn = gtk::Button::from_icon_name("list-add-symbolic");
icon_btn.add_css_class("circular");
```

---

## Toggle Group Styles

| Class | Applies To | Description / Effect |
|-------|-----------|----------------------|
| `.flat` | `AdwToggleGroup`, `AdwInlineViewSwitcher` | Flat toggle buttons without visible container. |
| `.round` | `AdwToggleGroup`, `AdwInlineViewSwitcher` | Rounded pill-shaped toggle group. |

---

## Layout Styles

| Class | Applies To | Description / Effect |
|-------|-----------|----------------------|
| `.linked` | `GtkBox` | Joins child widgets visually as a single group. Removes internal borders and rounds only outer corners. |
| `.toolbar` | `GtkBox` | Applies toolbar styling: flattens contained buttons, sets 6px margins and spacing. Compatible with `AdwToolbarView` as a top/bottom bar. |
| `.spacer` | `GtkSeparator` | Makes the separator invisible. Use for adding whitespace between toolbar items. |

### Layout Examples

```xml
<!-- Linked buttons -->
<object class="GtkBox">
  <style><class name="linked"/></style>
  <child><object class="GtkButton"><!-- ... --></object></child>
  <child><object class="GtkButton"><!-- ... --></object></child>
</object>

<!-- Custom toolbar -->
<object class="GtkBox">
  <style><class name="toolbar"/></style>
  <child><object class="GtkButton"><!-- auto-flattened --></object></child>
  <child>
    <object class="GtkSeparator">
      <style><class name="spacer"/></style>
    </object>
  </child>
  <child><object class="GtkButton"><!-- auto-flattened --></object></child>
</object>
```

---

## Typography Styles

| Class | Applies To | Description / Effect |
|-------|-----------|----------------------|
| `.title-1` | `GtkLabel` | Largest heading. ~24px bold. For page/view titles. |
| `.title-2` | `GtkLabel` | Second-level heading. ~20px bold. |
| `.title-3` | `GtkLabel` | Third-level heading. ~18px bold. |
| `.title-4` | `GtkLabel` | Fourth-level heading. ~16px bold. |
| `.heading` | `GtkLabel` | Standard UI heading. Bold weight at default size. For group/section titles. |
| `.document` | `GtkLabel` | Larger font size with increased line height. For long-form reading content. |
| `.body` | `GtkLabel` | Default font size with increased line height. For multi-line descriptions. |
| `.caption-heading` | `GtkLabel` | Small bold text. For labeling captions or metadata. |
| `.caption` | `GtkLabel` | Small text. For secondary metadata or timestamps. |
| `.monospace` | `GtkLabel`, `GtkTextView` | Monospace font family. For code, paths, or technical values. |
| `.numeric` | `GtkLabel` | Tabular (monospace) figures. Digits align vertically across rows. |

### Typography Example

```rust
let title = gtk::Label::new(Some("Settings"));
title.add_css_class("title-1");

let description = gtk::Label::new(Some("Configure your preferences below."));
description.add_css_class("body");

let code = gtk::Label::new(Some("/usr/bin/transcriber"));
code.add_css_class("monospace");
```

---

## Color Styles

| Class | Applies To | Description / Effect |
|-------|-----------|----------------------|
| `.accent` | `GtkLabel`, `GtkImage`, most widgets | Applies the accent color to the widget. Uses `--accent-color` for text. |
| `.success` | `GtkLabel`, `GtkImage`, most widgets | Applies the success color (green). |
| `.warning` | `GtkLabel`, `GtkImage`, most widgets | Applies the warning color (yellow/orange). |
| `.error` | `GtkLabel`, `GtkImage`, most widgets | Applies the error color (red). |

These classes set the foreground color. They work well on labels and icons for
semantic coloring without needing custom CSS.

```rust
let status = gtk::Label::new(Some("Connected"));
status.add_css_class("success");

let error_label = gtk::Label::new(Some("Failed to save"));
error_label.add_css_class("error");
```

---

## Container Styles

| Class | Applies To | Description / Effect |
|-------|-----------|----------------------|
| `.boxed-list` | `GtkListBox` | Rounded card appearance with row separators. Requires `selection-mode` set to `none`. The standard settings/preferences list style. |
| `.boxed-list-separate` | `GtkListBox` | Each row rendered as a separate card with spacing between them. |
| `.card` | Any widget | Card appearance with rounded corners, background, and shadow. |
| `.activatable` | Used with `.card` | Adds hover and active states to a card. For clickable cards. |
| `.navigation-sidebar` | `GtkListBox` | Rounded row items with neutral selection color. Standard sidebar list style. |

### Boxed List Example

```xml
<object class="GtkListBox">
  <property name="selection-mode">none</property>
  <style><class name="boxed-list"/></style>
  <child>
    <object class="AdwActionRow">
      <property name="title">Username</property>
      <property name="subtitle">craig</property>
    </object>
  </child>
  <child>
    <object class="AdwSwitchRow">
      <property name="title">Enable Notifications</property>
    </object>
  </child>
</object>
```

### Card Example

```xml
<object class="GtkBox">
  <style>
    <class name="card"/>
    <class name="activatable"/>
  </style>
  <!-- card content -->
</object>
```

---

## Visual Styles

| Class | Applies To | Description / Effect |
|-------|-----------|----------------------|
| `.dimmed` | `GtkLabel`, most widgets | Partial transparency (55% opacity, 90% in high contrast). For secondary/less important text. Replaces deprecated `.dim-label`. |
| `.icon-dropshadow` | `GtkImage` | Adds a drop shadow. Use for app icons larger than 32px. |
| `.lowres-icon` | `GtkImage` | Adds a drop shadow optimized for small icons (32px or smaller). |
| `.selection-mode` | `GtkCheckButton` | Large round checkbuttons for item selection mode (like file managers). |
| `.osd` | Various | Dark semi-transparent overlay style. For on-screen display elements over content. |
| `.background` | Any widget | Applies `--window-bg-color` and `--window-fg-color`. |
| `.view` | Any widget | Applies `--view-bg-color` and `--view-fg-color`. For content areas. |
| `.frame` | Any widget | Adds the default border style. |

---

## Specialized Styles

| Class | Applies To | Description / Effect |
|-------|-----------|----------------------|
| `.compact` | `AdwStatusPage` | Reduces spacing for use in constrained contexts like sidebars, popovers, or bottom sheets. |
| `.menu` | `GtkPopover` | Menu-like popover styling with appropriate padding and sizing. |
| `.devel` | `AdwHeaderBar`, `AdwToolbarView` | Adds diagonal stripes to the header bar. Visual indicator for development/nightly builds. |
| `.inline` | `GtkSearchBar`, `AdwTabBar`, `GtkTextView` | Neutral background that blends with surrounding content instead of using headerbar styling. |
| `.property` | `AdwActionRow`, `AdwExpanderRow` | Deemphasizes the title and emphasizes the subtitle. Use when the value (subtitle) is more important than the label (title). |

### Property Style Example

```xml
<!-- Without .property: title is bold, subtitle is dimmed -->
<!-- With .property: title is dimmed, subtitle is emphasized -->
<object class="AdwActionRow">
  <property name="title">Version</property>
  <property name="subtitle">1.4.2</property>
  <style><class name="property"/></style>
</object>
```

---

## Deprecated Style Classes

These classes still work but should be replaced with their modern equivalents.

| Deprecated Class | Replacement | Notes |
|-----------------|-------------|-------|
| `.content` | `.boxed-list` | Same visual effect, renamed for clarity. |
| `.sidebar` | `.navigation-sidebar` | Same visual effect, renamed for clarity. |
| `.app-notification` | `AdwToastOverlay` | Use the `AdwToast` widget instead of styling a custom notification. |
| `.large-title` | `.title-1` | Part of the new title hierarchy system. |
| `.dim-label` | `.dimmed` | Renamed. `.dimmed` applies more broadly than just labels. |

---

## Combining Style Classes

Some classes combine naturally:

```rust
// Pill-shaped suggested action button
let btn = gtk::Button::with_label("Get Started");
btn.add_css_class("suggested-action");
btn.add_css_class("pill");

// Flat circular icon button
let btn = gtk::Button::from_icon_name("view-more-symbolic");
btn.add_css_class("flat");
btn.add_css_class("circular");

// Dimmed caption text
let label = gtk::Label::new(Some("Last updated 2 hours ago"));
label.add_css_class("caption");
label.add_css_class("dimmed");
```

---

## Quick Reference by Widget

| Widget | Common Classes |
|--------|---------------|
| `GtkButton` | `.suggested-action`, `.destructive-action`, `.flat`, `.raised`, `.circular`, `.pill` |
| `GtkLabel` | `.title-1` to `.title-4`, `.heading`, `.body`, `.caption`, `.monospace`, `.numeric`, `.dimmed`, `.accent`, `.success`, `.warning`, `.error` |
| `GtkListBox` | `.boxed-list`, `.boxed-list-separate`, `.navigation-sidebar` |
| `GtkBox` | `.linked`, `.toolbar` |
| `GtkSeparator` | `.spacer` |
| `GtkImage` | `.icon-dropshadow`, `.lowres-icon` |
| `AdwStatusPage` | `.compact` |
| `AdwHeaderBar` | `.devel` |
| `AdwActionRow` | `.property` |
| `AdwExpanderRow` | `.property` |
| `AdwTabBar` | `.inline` |
| `GtkSearchBar` | `.inline` |
| `GtkPopover` | `.menu` |

---

## Applying Style Classes

### In Rust

```rust
// Add a class
widget.add_css_class("boxed-list");

// Remove a class
widget.remove_css_class("flat");

// Check if a class is applied
if widget.has_css_class("suggested-action") {
    // ...
}
```

### In Blueprint XML

```xml
<object class="GtkButton">
  <style>
    <class name="suggested-action"/>
    <class name="pill"/>
  </style>
</object>
```

### In Custom CSS

Style classes can also be targeted in your `style.css`:

```css
/* Target all buttons with .pill class */
button.pill {
  padding: 12px 32px;
}

/* Target labels with .accent inside a .boxed-list */
.boxed-list label.accent {
  font-weight: bold;
}
```
