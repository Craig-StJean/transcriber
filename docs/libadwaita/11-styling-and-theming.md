# Styling and Theming

This document covers libadwaita's styling system: the style manager for color scheme control, accent colors, the complete set of CSS variables, the GNOME color palette, CSS media queries, custom stylesheet loading, and best practices for theming GNOME applications.

---

## AdwStyleManager

`AdwStyleManager` is the central API for managing application appearance. It provides access to color scheme preferences, accent colors, and system capabilities. There is a single default instance shared across the application.

### Accessing the Style Manager

```rust
let style_manager = adw::StyleManager::default();
```

### Properties

| Property | Type | Access | Description |
|----------|------|--------|-------------|
| `color-scheme` | `ColorScheme` | Read/Write | The application's color scheme preference |
| `dark` | `bool` | Read-only | Whether the current appearance is dark |
| `high-contrast` | `bool` | Read-only | Whether high contrast mode is active |
| `accent-color` | `AccentColor` | Read-only | The current accent color enum value (since 1.6) |
| `accent-color-rgba` | `GdkRGBA` | Read-only | The accent color as an RGBA value (since 1.6) |
| `system-supports-color-schemes` | `bool` | Read-only | Whether the system provides a color scheme preference |

### Color Scheme Options

| Value | Constant | Behavior |
|-------|----------|----------|
| Default | `ColorScheme::Default` | Follow system preference; light if unsupported |
| Prefer Light | `ColorScheme::PreferLight` | Light unless system explicitly prefers dark |
| Prefer Dark | `ColorScheme::PreferDark` | Dark unless system explicitly prefers light |
| Force Light | `ColorScheme::ForceLight` | Always light, ignores system |
| Force Dark | `ColorScheme::ForceDark` | Always dark, ignores system |

The recommended default is `PreferLight`, which respects the system preference while defaulting to light when no system preference is available.

### Rust Example: Color Scheme Control

```rust
use adw::prelude::*;

let style_manager = adw::StyleManager::default();

// Follow system preference (recommended)
style_manager.set_color_scheme(adw::ColorScheme::Default);

// Check current state
if style_manager.is_dark() {
    println!("Dark mode is active");
}

// React to changes
style_manager.connect_dark_notify(|sm| {
    if sm.is_dark() {
        println!("Switched to dark mode");
    } else {
        println!("Switched to light mode");
    }
});
```

---

## Accent Colors

*Since 1.6.* Libadwaita supports nine standardized accent colors. The user selects an accent color in GNOME Settings, and applications automatically adapt via CSS variables. Applications should not offer their own accent color picker.

### Color Table

| Name | Enum Value | Light Hex | Description |
|------|------------|-----------|-------------|
| Blue | `AccentColor::Blue` | `#3584e4` | Default accent color |
| Teal | `AccentColor::Teal` | `#2190a4` | Blue-green |
| Green | `AccentColor::Green` | `#3a944a` | Green |
| Yellow | `AccentColor::Yellow` | `#c88800` | Warm yellow |
| Orange | `AccentColor::Orange` | `#ed5b00` | Orange |
| Red | `AccentColor::Red` | `#e62d42` | Red |
| Pink | `AccentColor::Pink` | `#d56199` | Pink |
| Purple | `AccentColor::Purple` | `#9141ac` | Purple |
| Slate | `AccentColor::Slate` | `#6f8396` | Neutral blue-gray |

### Accent Color CSS Variables

| Variable | Purpose |
|----------|---------|
| `--accent-bg-color` | Background for accent elements (buttons, switches, checks) |
| `--accent-fg-color` | Foreground on accent backgrounds (defaults to white) |
| `--accent-color` | Standalone accent color for text, icons, and focus rings (higher contrast than `--accent-bg-color`) |

### Best Practices

- Use `--accent-bg-color` for backgrounds of interactive elements (suggested-action buttons, switches, checkboxes)
- Use `--accent-color` for accent-colored text and icons -- it has higher contrast and is better suited for small elements on both light and dark backgrounds
- Use `--accent-fg-color` for text and icons placed on top of `--accent-bg-color` backgrounds
- Never use `--accent-bg-color` for text; it may have insufficient contrast against the window background
- The `.suggested-action` and `.accent` style classes use these variables automatically

### Rust Example: Reading Accent Color

```rust
use adw::prelude::*;

let style_manager = adw::StyleManager::default();

// Get the accent color enum
let accent = style_manager.accent_color();
match accent {
    adw::AccentColor::Blue => println!("Blue accent"),
    adw::AccentColor::Red => println!("Red accent"),
    _ => println!("Other accent: {:?}", accent),
}

// Get the RGBA value for custom drawing
let rgba = style_manager.accent_color_rgba();
println!("Accent: ({}, {}, {}, {})", rgba.red(), rgba.green(), rgba.blue(), rgba.alpha());

// React to accent color changes
style_manager.connect_accent_color_notify(|sm| {
    let new_accent = sm.accent_color();
    println!("Accent changed to: {:?}", new_accent);
});
```

---

## CSS Variables Reference

All CSS variables automatically adapt to light mode, dark mode, high contrast, and accent color changes. Use these instead of hardcoded color values.

### Accent Colors

| Variable | Purpose |
|----------|---------|
| `--accent-bg-color` | Background for accent elements |
| `--accent-fg-color` | Foreground on accent backgrounds |
| `--accent-color` | Standalone accent (text, icons, focus) |

### Semantic Colors

Each semantic category has three variants: `-bg-color` (background), `-fg-color` (foreground on that background), and `-color` (standalone). Prefixes: `--destructive-*` (red), `--success-*` (green), `--warning-*` (yellow), `--error-*` (red).

### Window, View, and Surface Variables

| Variable | Purpose |
|----------|---------|
| `--window-bg-color` / `--window-fg-color` | Main window background and foreground |
| `--view-bg-color` / `--view-fg-color` | Content/scrollable area |
| `--card-bg-color` / `--card-fg-color` | Card and boxed list |
| `--card-shade-color` | Card row separators |
| `--dialog-bg-color` / `--dialog-fg-color` | Dialog |
| `--popover-bg-color` / `--popover-fg-color` | Popover |
| `--popover-shade-color` | Popover shadow |

### Header Bar

| Variable | Purpose |
|----------|---------|
| `--headerbar-bg-color` / `--headerbar-fg-color` | Header bar background and foreground |
| `--headerbar-border-color` | Vertical borders between header bars |
| `--headerbar-backdrop-color` | Background when window is unfocused |
| `--headerbar-shade-color` | Shadow/border below header bar |
| `--headerbar-darker-shade-color` | Darker raised border shade |

### Sidebar

| Variable | Purpose |
|----------|---------|
| `--sidebar-bg-color` / `--sidebar-fg-color` | Sidebar background and foreground |
| `--sidebar-backdrop-color` / `--sidebar-border-color` / `--sidebar-shade-color` | Unfocused, border, shadow |
| `--secondary-sidebar-bg-color` / `--secondary-sidebar-fg-color` | Middle pane (triple-pane layouts) |
| `--secondary-sidebar-backdrop-color` / `--secondary-sidebar-border-color` / `--secondary-sidebar-shade-color` | Middle pane state variants |

### Tab Overview and Toggle

| Variable | Purpose |
|----------|---------|
| `--overview-bg-color` / `--overview-fg-color` | Tab overview background and foreground |
| `--thumbnail-bg-color` / `--thumbnail-fg-color` | Tab thumbnail |
| `--active-toggle-bg-color` / `--active-toggle-fg-color` | Active toggle button |

### Miscellaneous

| Variable | Purpose |
|----------|---------|
| `--shade-color` | Scroll undershoots and transitions |
| `--scrollbar-outline-color` | Overlay scrollbar visibility outline |
| `--border-color` | Derived border color (from foreground) |
| `--border-opacity` | 15% regular, 50% high contrast |
| `--dim-opacity` | 55% regular, 90% high contrast |
| `--disabled-opacity` | 50% regular, 40% high contrast |
| `--window-radius` | Window corner radius (15px) |
| `--document-font-family` / `--document-font-size` | System document font |
| `--monospace-font-family` / `--monospace-font-size` | System monospace font |

---

## GNOME Palette

The GNOME color palette is available as CSS variables. Each color has five shades (1 = lightest, 5 = darkest). These are the raw palette colors; prefer semantic variables (`--accent-*`, `--success-*`, etc.) in most cases.

| Color | Variables |
|-------|-----------|
| Blue | `--blue-1` through `--blue-5` |
| Green | `--green-1` through `--green-5` |
| Yellow | `--yellow-1` through `--yellow-5` |
| Orange | `--orange-1` through `--orange-5` |
| Red | `--red-1` through `--red-5` |
| Purple | `--purple-1` through `--purple-5` |
| Brown | `--brown-1` through `--brown-5` |
| Light | `--light-1` through `--light-5` |
| Dark | `--dark-1` through `--dark-5` |

Use palette colors sparingly and only when no semantic variable fits. For example, use `--success-color` instead of `--green-3` for success indicators, so the color adapts correctly to dark mode and high contrast.

---

## CSS Media Queries

Libadwaita supports standard CSS media queries for adapting styles to system preferences.

### prefers-color-scheme

Matches the current color scheme (light or dark):

```css
/* Light mode specific styles */
@media (prefers-color-scheme: light) {
    .my-widget {
        background-color: rgba(255, 255, 255, 0.1);
    }
}

/* Dark mode specific styles */
@media (prefers-color-scheme: dark) {
    .my-widget {
        background-color: rgba(0, 0, 0, 0.2);
    }
}
```

### prefers-contrast

Matches the high contrast accessibility setting:

```css
@media (prefers-contrast: more) {
    .my-custom-border {
        border-width: 2px;
        border-color: var(--border-color);
    }
}
```

### prefers-reduced-motion

*Since 1.9.* Matches the reduced motion accessibility setting:

```css
.my-animated-widget {
    transition: opacity 300ms ease;
}

@media (prefers-reduced-motion) {
    .my-animated-widget {
        transition: none;
    }
}
```

---

## Custom Stylesheet Loading

### Automatic Loading

`AdwApplication` automatically loads `style.css` from the application's GResource bundle. Place your stylesheet at the resource path corresponding to your application ID:

```
/com/example/myapp/style.css
```

No additional code is needed. The stylesheet is loaded when the application starts.

### Deprecated Conditional Stylesheets

Before 1.9, applications could provide separate stylesheets for different modes:

| File | Condition |
|------|-----------|
| `style.css` | Always loaded |
| `style-dark.css` | Loaded in dark mode |
| `style-hc.css` | Loaded in high contrast |
| `style-hc-dark.css` | Loaded in high contrast + dark mode |

**These are deprecated since 1.9.** Use a single `style.css` with CSS media queries instead.

### Recommended Approach (since 1.9)

Use a single `style.css` with media queries for mode-specific styles:

```css
/* Base styles (always applied) */
.my-sidebar {
    padding: 12px;
}

/* Dark mode adjustments */
@media (prefers-color-scheme: dark) {
    .my-sidebar {
        border-right: 1px solid var(--border-color);
    }
}

/* High contrast adjustments */
@media (prefers-contrast: more) {
    .my-sidebar {
        border-right: 2px solid var(--border-color);
    }
}
```

### Manual Stylesheet Loading

If you are not using `AdwApplication`, or need to load additional stylesheets, use GTK's CSS provider API:

```rust
use adw::prelude::*;
use gtk::CssProvider;

let provider = CssProvider::new();
provider.load_from_string("
    .my-class {
        background-color: var(--view-bg-color);
        color: var(--view-fg-color);
    }
");

gtk::style_context_add_provider_for_display(
    &gdk::Display::default().unwrap(),
    &provider,
    gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
);
```

---

## Accent Color Best Practices

### Use the Right Variable

| Scenario | Variable | Why |
|----------|----------|-----|
| Button backgrounds, switch tracks, checkmarks | `--accent-bg-color` | Designed for filled backgrounds |
| Text labels, icons, focus rings | `--accent-color` | Higher contrast, readable at small sizes |
| Text on accent-colored backgrounds | `--accent-fg-color` | Ensures readability on `--accent-bg-color` |

### Common Mistakes

- **Do not** use `--accent-bg-color` for text -- it may lack contrast against the window background. Use `--accent-color` instead.
- **Do not** use `--accent-color` for large filled areas -- it is too saturated. Use `--accent-bg-color` for backgrounds.
- The `.suggested-action` and `.accent` style classes handle this automatically.

### Custom CSS with Accent Colors

```css
.badge {
    background-color: var(--accent-bg-color);
    color: var(--accent-fg-color);
    border-radius: 99px;
    padding: 2px 8px;
}

.custom-link {
    color: var(--accent-color);
}
```

---

## Rust Example: Complete Theme Setup

```rust
use adw::prelude::*;

fn setup_styling() {
    let style_manager = adw::StyleManager::default();
    style_manager.set_color_scheme(adw::ColorScheme::Default);

    // React to dark mode changes
    style_manager.connect_dark_notify(|sm| {
        println!("Dark mode: {}", sm.is_dark());
    });

    // React to accent color changes
    style_manager.connect_accent_color_notify(|sm| {
        let rgba = sm.accent_color_rgba();
        println!("Accent: ({}, {}, {})", rgba.red(), rgba.green(), rgba.blue());
    });
}
```

---

## Cross-References

- [Visual Design](../HIG/06-visual-design.md) -- typography classes, spacing system, and styling principles
- [Color Palette](../HIG/07-color-palette.md) -- GNOME HIG color reference and semantic color guidelines
- [Overview](01-overview.md) -- version history for accent colors (1.6), CSS media queries (1.9)
