# Color Palette

This document provides the complete GNOME color reference, including the named color palette, semantic CSS variables, usage guidelines, and contrast requirements. All GNOME applications should use these colors through CSS variables and style classes rather than hardcoding hex values.

---

## GNOME Named Color Palette

The GNOME palette consists of 10 color families, each with 5 shades ranging from lightest (1) to darkest (5). These colors are used throughout the GNOME platform for icons, illustrations, and UI accents.

| Color | 1 (Lightest) | 2 | 3 | 4 | 5 (Darkest) |
|-------|:------------:|:-:|:-:|:-:|:------------:|
| **Blue** | `#99c1f1` | `#62a0ea` | `#3584e4` | `#1c71d8` | `#1a5fb4` |
| **Green** | `#8ff0a4` | `#57e389` | `#33d17a` | `#2ec27e` | `#26a269` |
| **Yellow** | `#f9f06b` | `#f8e45c` | `#f6d32d` | `#f5c211` | `#e5a50a` |
| **Orange** | `#ffbe6f` | `#ffa348` | `#ff7800` | `#e66100` | `#c64600` |
| **Red** | `#f66151` | `#ed333b` | `#e01b24` | `#c01c28` | `#a51d2d` |
| **Purple** | `#dc8add` | `#c061cb` | `#9141ac` | `#813d9c` | `#613583` |
| **Brown** | `#cdab8f` | `#b5835a` | `#986a44` | `#865e3c` | `#63452c` |
| **Light** | `#ffffff` | `#f6f5f4` | `#deddda` | `#c0bfbc` | `#9a9996` |
| **Dark** | `#77767b` | `#5e5c64` | `#3d3846` | `#241f31` | `#000000` |

### Shade Usage

- **Shade 3** is the standard/primary shade for each color family. Use it as the default choice.
- **Shades 1-2** are lighter tints, suitable for backgrounds, hover states, and light-mode accents.
- **Shades 4-5** are darker tones, suitable for dark-mode accents, pressed states, and high-contrast outlines.
- Use adjacent shades (e.g., 2 and 4) for hover/active state progressions of a base shade-3 color.

---

## Semantic CSS Color Variables

These CSS variables are the primary way to use color in application UI. They adapt automatically to light mode, dark mode, and high-contrast settings. **Always use these variables instead of hardcoded hex values.**

### Background and Surface Colors

| Variable | Purpose |
|----------|---------|
| `@window_bg_color` | Main window background. The outermost background of the application. |
| `@view_bg_color` | Content area background. Used for scrollable content regions, text views, and list views. |
| `@card_bg_color` | Card and boxed list background. Slightly elevated surface for grouped content. |
| `@headerbar_bg_color` | Header bar background. May differ from window background for visual separation. |
| `@popover_bg_color` | Popover and dropdown menu background. |
| `@dialog_bg_color` | Dialog window background. |
| `@sidebar_bg_color` | Sidebar and navigation panel background. |

### Foreground Colors

| Variable | Purpose |
|----------|---------|
| `@window_fg_color` | Default text color on window backgrounds. |
| `@view_fg_color` | Text color on view/content backgrounds. |
| `@card_fg_color` | Text color on card backgrounds. |
| `@headerbar_fg_color` | Text and icon color in the header bar. |
| `@accent_fg_color` | Text color on accent-colored backgrounds. Guaranteed readable against `@accent_bg_color`. |
| `@destructive_fg_color` | Text on destructive-colored backgrounds. |
| `@success_fg_color` | Text on success-colored backgrounds. |
| `@warning_fg_color` | Text on warning-colored backgrounds. |
| `@error_fg_color` | Text on error-colored backgrounds. |

### Accent and State Colors

| Variable | Purpose |
|----------|---------|
| `@accent_bg_color` | Background for accent-colored elements (suggested buttons, selected items, active toggles). |
| `@accent_color` | Accent color for text, icons, links, and focus indicators. Use on standard backgrounds. |
| `@destructive_bg_color` | Background for destructive action buttons. |
| `@destructive_color` | Destructive color for text and icons on standard backgrounds. |
| `@success_bg_color` | Background for success indicators and elements. |
| `@success_color` | Success color for text and icons on standard backgrounds. |
| `@warning_bg_color` | Background for warning indicators and elements. |
| `@warning_color` | Warning color for text and icons on standard backgrounds. |
| `@error_bg_color` | Background for error indicators and elements. |
| `@error_color` | Error color for text and icons on standard backgrounds. |

### Pairing Backgrounds and Foregrounds

Always pair `_bg_color` with the corresponding `_fg_color` to ensure readability:

```css
.my-accent-badge {
    background-color: @accent_bg_color;
    color: @accent_fg_color;  /* guaranteed contrast */
}
```

Never pair `@accent_bg_color` with `@window_fg_color` -- the contrast is not guaranteed.

---

## Color Usage Guidelines

### General Principles

- **Use bright colors sparingly.** Color should highlight the most important elements, not flood the interface. Too much bright color creates visual noise, causes fatigue, and makes it impossible to distinguish what is important.
- **Reserve color for meaning.** Each color used in the UI should communicate something specific: an interactive element, a status, a category.
- **Neutral by default.** Most UI surfaces, text, and icons should use neutral grays and blacks/whites from the Light and Dark palette rows. Color is the exception, not the rule.

### Primary Color Uses

| Context | Approach |
|---------|----------|
| **App icons and illustrations** | Use the full palette freely. Icons and illustrations are artistic elements where vivid color is appropriate. |
| **Interactive elements** | Use `@accent_color` / `@accent_bg_color` for links, focus rings, suggested actions, and selected states. |
| **Destructive actions** | Use `@destructive_bg_color` on buttons that delete, discard, or permanently alter data. |
| **Status indicators** | Use `@success_color` for success, `@warning_color` for warnings, `@error_color` for errors. |
| **Text** | Almost always use `@window_fg_color` or `@view_fg_color`. Reserve colored text for links (`@accent_color`) and error messages (`@error_color`). |

### Anti-Patterns

- **Colored backgrounds everywhere.** Large colored surfaces overwhelm the user. Use `@window_bg_color` and `@card_bg_color` for most areas.
- **Color as the only indicator.** Never rely solely on color to convey information. Always supplement with text labels, icons, or patterns. This is essential for colorblind users and high-contrast mode.
- **Hardcoded hex values.** Using raw hex codes (`#3584e4`) bypasses dark mode, high contrast, and accent color adaptation. Always use CSS variables.
- **Too many accent colors.** Stick to the system accent color. Do not introduce multiple competing accent colors in the same interface.

---

## WCAG Contrast Requirements

All text and meaningful visual elements must meet Web Content Accessibility Guidelines (WCAG) 2.1 AA contrast requirements.

### Minimum Contrast Ratios

| Element | Minimum Ratio | Example |
|---------|:-------------:|---------|
| Normal text (below 18px regular, below 14px bold) | **4.5:1** | Body text, labels, descriptions |
| Large text (18px+ regular, or 14px+ bold) | **3:1** | Headings, title text |
| UI components (borders, icons, form controls) | **3:1** | Button borders, input outlines, icons |
| Graphical objects (charts, diagrams, indicators) | **3:1** | Status dots, progress bars, chart elements |

### Testing Contrast

1. **WebAIM Contrast Checker** (https://webaim.org/resources/contrastchecker/) -- enter foreground and background hex values to verify the ratio.
2. **GNOME high-contrast mode** -- enable it in Settings > Accessibility and visually verify all elements remain clear.
3. **Automated testing** -- tools like `axe` or `Lighthouse` can audit contrast in web-based previews.

### Common Pitfalls

- Light gray text on white backgrounds often fails 4.5:1. Use `@window_fg_color` for body text.
- Colored text on colored backgrounds (e.g., orange text on yellow) rarely provides sufficient contrast.
- Placeholder text in inputs (`dim-label` class) is exempt from the 4.5:1 rule only if it is not the sole label -- a visible label must be present.

---

## Accent Color API (libadwaita 1.6+)

Starting with libadwaita 1.6, GNOME supports a **system-wide accent color** that users can choose in GNOME Settings. Applications that use `@accent_color`, `@accent_bg_color`, and `@accent_fg_color` variables automatically adapt to the user's chosen accent color.

### Guidelines

- **Use `@accent_color` variables** instead of hardcoding Blue 3 (`#3584e4`) or any other specific accent color.
- **Do not assume the accent color is blue.** Users may choose green, purple, orange, or other colors. Design your UI so it works well with any accent color.
- **Test with multiple accent colors** to ensure your layout and contrast hold up regardless of the user's choice.
- **Brand colors in icons are fine.** App icons are not expected to change with the accent color. Use your brand color freely in your app icon.
- **Interactive elements should use `@accent_color`.** This includes focus rings, selected states, links, and suggested-action buttons.

### Checking Accent Color Support

```python
style_manager = Adw.StyleManager.get_default()

# The accent color variables are always available in libadwaita 1.6+.
# No special code is needed -- just use @accent_color in your CSS.
```

If you need to react programmatically to accent color changes (e.g., to regenerate a custom graphic), connect to the `AdwStyleManager::notify::accent-color` signal.

---

## Quick Reference: Color by Purpose

| Purpose | CSS Variable | Palette Origin |
|---------|-------------|----------------|
| Primary accent / brand | `@accent_bg_color` | Blue 3 (default, user-changeable) |
| Links and interactive text | `@accent_color` | Derived from accent |
| Destructive actions | `@destructive_bg_color` | Red 3 |
| Success states | `@success_bg_color` | Green 4 |
| Warning states | `@warning_bg_color` | Yellow 4 |
| Error states | `@error_bg_color` | Red 3 |
| Standard text | `@window_fg_color` | Dark row |
| Secondary text | `.dim-label` class | Dark row, reduced opacity |
| Window background | `@window_bg_color` | Light row (light mode) / Dark row (dark mode) |
| Card/elevated surface | `@card_bg_color` | Slightly offset from window background |

---

## Cross-References

- [Visual Design](06-visual-design.md) -- typography, spacing, and style class usage.
- [Accessibility](11-accessibility.md) -- comprehensive accessibility guidelines including contrast testing and screen reader support.
