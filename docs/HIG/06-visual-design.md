# Visual Design

This document covers typography, spacing, color usage, and styling conventions for GNOME applications using Adwaita. Consistent visual design creates a polished, professional experience and ensures your application feels native within the GNOME desktop.

---

## Typography

### System Fonts

GNOME uses **Adwaita Sans**, a custom variant of Inter, as the default interface font. Applications should always use the system font rather than hardcoding a specific typeface.

- **Never hardcode font names, sizes, or weights** in application code or CSS. Instead, use Adwaita's typography style classes.
- Use relative sizing (style classes) so that text scales correctly with user preferences and accessibility settings.
- Respect the user's system font settings; do not override them.

### Typography Style Classes

Apply these CSS classes to `GtkLabel` and other text widgets to achieve consistent typography throughout your application.

| Class | Usage |
|-------|-------|
| `body` | Default text for labels, descriptions, and general content. This is the baseline style. |
| `heading` | Window titles and control group headings. Larger and bolder than body text. |
| `caption` | Sub-text accompanying body text. Smaller and lighter, used for supplementary information. |
| `caption-heading` | Bold caption text. Used for labels within caption-sized contexts. |
| `large-title` | Large display headings for hero areas or prominent page titles. Use sparingly. |
| `title-1` | The largest section heading. Suitable for top-level page titles. |
| `title-2` | Second-level section heading. |
| `title-3` | Third-level section heading. |
| `title-4` | Fourth-level section heading. The smallest title size. |

### Typography Guidelines

- **Minimize variation.** Use as few different font sizes and weights as possible. A clean hierarchy uses 2-3 levels (heading, body, caption).
- **Secondary information** should be presented in lighter or smaller text (use `caption` or the `.dim-label` class) to establish visual hierarchy without competing with primary content.
- **Avoid italics.** They reduce readability, especially at small sizes and on low-resolution displays.
- **Avoid all-caps.** All-caps text is harder to read and can feel aggressive. Use title case or sentence case instead.
- **Avoid hardcoded styles.** Do not set `font-size`, `font-weight`, or `font-family` directly in CSS. Use style classes.

### Unicode Best Practices

Use correct typographic characters throughout the interface:

| Character | Unicode | Usage |
|-----------|---------|-------|
| Left double quote | U+201C (\u201c) | Opening quotation marks |
| Right double quote | U+201D (\u201d) | Closing quotation marks |
| Left single quote | U+2018 (\u2018) | Opening single quotes, apostrophes |
| Right single quote | U+2019 (\u2019) | Closing single quotes, apostrophes |
| Ellipsis | U+2026 (\u2026) | Truncated text, ongoing actions (\u201cLoading\u2026\u201d) |
| En dash | U+2013 (\u2013) | Ranges (\u201c3\u201315 items\u201d), parenthetical clauses |
| Multiplication sign | U+00D7 (\u00d7) | Dimensions (\u201c1920 \u00d7 1080\u201d) |

Do not use ASCII substitutes (`"` for curly quotes, `...` for ellipsis, `x` for multiplication, `-` for en dash).

---

## Spacing

### Base Unit

All spacing in GNOME applications is built on a **6-pixel base unit**. Use multiples of 6 for all margins, padding, and gaps.

### Spacing Reference

| Context | Spacing | Multiple |
|---------|---------|----------|
| Between an icon and its label | 6px | 1x |
| Between a label and its associated component | 12px (horizontal) | 2x |
| Between groups of related components | 18px (vertical) | 3x |
| Dialog padding from window borders | 12px | 2x |
| Element indentation for visual hierarchy | 12px | 2x |
| Standard margin around content areas | 12px-24px | 2x-4x |
| Gap between cards in a grid | 12px-18px | 2x-3x |

### Spacing Guidelines

- **Align content precisely.** Use consistent spacing to create a clear visual grid. Misaligned elements look sloppy and reduce trust.
- **Group related controls** by placing them closer together. Use wider spacing to separate unrelated groups. Spacing communicates relationships more effectively than borders.
- **Avoid visible borders and separators.** Use whitespace and background color differences to create visual groupings. Borders add visual noise.
- **Use `GtkBox` spacing** and `margin-*` CSS properties to control spacing. Avoid manual pixel positioning.
- **Consistent margins:** Use the same margin values throughout the application. Do not vary margins between similar views.

---

## Styling with Adwaita

Adwaita is not just a theme -- it is the visual design system for GNOME. Use it as your foundation and minimize custom styling.

### Principles

- **Use style classes over custom CSS.** Adwaita provides classes for nearly every visual pattern (`.card`, `.boxed-list`, `.flat`, `.suggested-action`, etc.). Prefer these to writing your own CSS.
- **Minimize custom CSS.** Every line of custom CSS is a maintenance burden. Custom styles may break with Adwaita updates and may not adapt correctly to dark mode or high contrast.
- **Use CSS variables for colors.** Never hardcode color values. Use Adwaita's named CSS variables so your application adapts to light mode, dark mode, high contrast, and accent color changes.

### Key CSS Variables

These variables automatically adjust for light/dark mode and high contrast:

| Variable | Purpose |
|----------|---------|
| `@window_bg_color` | Main window background |
| `@view_bg_color` | Content view background (scrollable areas) |
| `@card_bg_color` | Card and boxed list background |
| `@headerbar_bg_color` | Header bar background |
| `@popover_bg_color` | Popover background |
| `@dialog_bg_color` | Dialog background |
| `@sidebar_bg_color` | Sidebar background |
| `@accent_bg_color` | Accent/brand color background |
| `@accent_fg_color` | Text on accent-colored backgrounds |
| `@accent_color` | Accent color for text, icons, and focus rings |
| `@destructive_bg_color` | Destructive action background (red) |
| `@destructive_fg_color` | Text on destructive backgrounds |
| `@success_bg_color` | Success state background (green) |
| `@success_fg_color` | Text on success backgrounds |
| `@warning_bg_color` | Warning state background (yellow) |
| `@warning_fg_color` | Text on warning backgrounds |
| `@error_bg_color` | Error state background (red) |
| `@error_fg_color` | Text on error backgrounds |

### Common Style Classes

| Class | Effect |
|-------|--------|
| `.card` | Rounded card with subtle background and shadow |
| `.boxed-list` | A list with card-like appearance and separators |
| `.flat` | Removes button/header bar background |
| `.suggested-action` | Accent-colored button for primary actions |
| `.destructive-action` | Red button for dangerous actions |
| `.dim-label` | Reduced opacity for secondary text |
| `.numeric` | Monospaced/tabular figures for numbers |
| `.linked` | Joins adjacent widgets into a single visual group |
| `.toolbar` | Toolbar styling for a box of controls |
| `.osd` | On-screen display overlay styling |

---

## Light and Dark Modes

GNOME supports system-wide light and dark color scheme preferences. Every application must support both modes.

### Implementation

Use `AdwStyleManager` to manage color scheme support:

```python
style_manager = Adw.StyleManager.get_default()

# Follow the system preference (recommended default)
style_manager.set_color_scheme(Adw.ColorScheme.PREFER_LIGHT)
```

### Color Scheme Options

Applications can offer users a choice:

| Option | Behavior |
|--------|----------|
| System | Follow the system-wide preference (recommended default) |
| Light | Always use light mode |
| Dark | Always use dark mode |

### Guidelines

- **Default to "System"** so the application respects the user's desktop-wide preference.
- **Use named CSS variables** (`@window_bg_color`, `@card_bg_color`, etc.) for all colors. These switch automatically between light and dark values.
- **Use style classes** (`.card`, `.dim-label`, etc.) rather than hardcoded colors. Style classes adapt to the current mode.
- **Do not hardcode `#ffffff` or `#000000`** for backgrounds or text. These will break in the opposite mode.
- **Test in both modes.** Switch between light and dark mode frequently during development. Check every view, dialog, and state.
- **Custom illustrations** may need separate light and dark variants. Use `AdwStyleManager` to detect the current mode and swap assets.

---

## High Contrast

GNOME provides a high-contrast mode for users with low vision. Applications that use Adwaita style classes and CSS variables adapt automatically, but custom elements require manual verification.

### Guidelines

- **Test all UI elements** in high-contrast mode (enable it in GNOME Settings > Accessibility).
- **Never use color as the only distinguishing factor.** Supplement color with icons, text labels, or patterns. For example, do not indicate errors solely with red text; also add an error icon.
- **Ensure text contrast** meets WCAG requirements:
  - Normal text: minimum **4.5:1** contrast ratio against its background.
  - Large text (18px+ regular, or 14px+ bold): minimum **3:1**.
  - UI components and graphical objects: minimum **3:1**.
- **Borders and focus rings** must be visible in high contrast. Adwaita handles this for standard widgets; verify custom widgets.
- **Test with a contrast checker** (such as the WebAIM Contrast Checker) when using any custom colors.
- **Avoid low-contrast decorative text.** If text is important enough to display, it should be readable.

---

## Cross-References

- [Color Palette](07-color-palette.md) -- the complete GNOME color reference and semantic color variables.
- [Accessibility](11-accessibility.md) -- broader accessibility guidelines including contrast, screen readers, and keyboard navigation.
