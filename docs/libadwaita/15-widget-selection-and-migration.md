# Widget Selection and Migration Guide

How to pick the right widget for your use case, migrate from deprecated widgets,
and understand version availability.

---

## Widget Decision Table

"I need X" mapped to the right widget.

### Application Structure

| I need... | Use this widget | Notes |
|-----------|----------------|-------|
| An app entry point | `AdwApplication` | Handles init, stylesheets, translations. |
| A main window | `AdwApplicationWindow` | Has no built-in titlebar -- add your own `AdwHeaderBar`. |
| A secondary window (no app binding) | `AdwWindow` | Same as above but without `GtkApplication` integration. |
| A header bar | `AdwHeaderBar` | Split start/end title button control. Auto-integrates with `AdwNavigationView`. |
| A toolbar layout | `AdwToolbarView` | Manages top/bottom bars with correct styling and scroll undershoots. |

### Navigation

| I need... | Use this widget | Notes |
|-----------|----------------|-------|
| Drill-down page navigation | `AdwNavigationView` | Stack-based. Auto back buttons, gestures. |
| Sidebar + content (responsive) | `AdwNavigationSplitView` | Collapses to drill-down on narrow windows. |
| Overlay sidebar | `AdwOverlaySplitView` | Sidebar slides over content when collapsed. |
| Tabbed views (desktop style) | `AdwViewSwitcher` + `AdwViewStack` | Place in header bar title widget. |
| Tabbed views (mobile style) | `AdwViewSwitcherBar` + `AdwViewStack` | Bottom bar, toggle with breakpoint. |
| Browser-style tabs | `AdwTabView` + `AdwTabBar` | Dynamic tabs with reordering, pinning. |
| Tab overview (mobile) | `AdwTabOverview` + `AdwTabButton` | Grid overview of all tabs. |
| Swipeable pages | `AdwCarousel` | With `CarouselIndicatorDots` or `CarouselIndicatorLines`. |

### Dialogs and Feedback

| I need... | Use this widget | Notes |
|-----------|----------------|-------|
| A modal dialog | `AdwDialog` | Adaptive: floating on desktop, bottom sheet on mobile. |
| A confirmation/alert | `AdwAlertDialog` | Vertical button layout, accent-aware. Since 1.5. |
| Settings/preferences UI | `AdwPreferencesDialog` | Multi-page, searchable. Since 1.5. |
| An about dialog | `AdwAboutDialog` | App info, credits, license. Since 1.5. |
| Keyboard shortcuts reference | `AdwShortcutsDialog` | Sections with shortcut items. Since 1.8. |
| A toast notification | `AdwToastOverlay` + `AdwToast` | Non-modal feedback with optional action button. |
| A persistent banner | `AdwBanner` | Top-of-content info bar with optional button. Since 1.3. |
| An empty/error state | `AdwStatusPage` | Icon, title, description, and child widget. |

### Settings Rows

| I need... | Use this widget | Notes |
|-----------|----------------|-------|
| A labeled row with subtitle | `AdwActionRow` | Add prefix/suffix widgets. |
| A toggle switch setting | `AdwSwitchRow` | Built-in switch. Since 1.4. |
| A dropdown selection | `AdwComboRow` | Model-based, supports search. |
| A text input field | `AdwEntryRow` | Inline text entry in a list row. |
| A password field | `AdwPasswordEntryRow` | Entry with show/hide toggle. |
| A numeric spinner | `AdwSpinRow` | Embedded `GtkSpinButton`. Since 1.4. |
| An expandable group | `AdwExpanderRow` | Expands to reveal child rows. |
| A clickable action row | `AdwButtonRow` | Looks like a button in a list. Since 1.6. |
| A row grouping | `AdwPreferencesGroup` | Groups rows with a title and optional description. |

### Layout and Containers

| I need... | Use this widget | Notes |
|-----------|----------------|-------|
| Constrained max width | `AdwClamp` | Standard for settings pages. Scales with text size. |
| Responsive breakpoints | `AdwBreakpoint` + window/dialog | Modify properties based on available size. |
| Breakpoints in arbitrary context | `AdwBreakpointBin` | Standalone container for breakpoint support. |
| Multiple switchable layouts | `AdwMultiLayoutView` | Define layouts with slots, switch via breakpoints. Since 1.6. |
| Wrapping flow layout | `AdwWrapBox` | Children wrap like words in text. |
| A bottom sheet | `AdwBottomSheet` | Persistent, draggable sheet. Since 1.6. |
| A simple container | `AdwBin` | Single child, minimal overhead. |

### Other

| I need... | Use this widget | Notes |
|-----------|----------------|-------|
| A loading spinner | `AdwSpinner` | Custom-drawn, no dithering. Since 1.6. |
| A spinner in a paintable context | `AdwSpinnerPaintable` | For `GtkImage` or custom paintable use. Since 1.6. |
| A user avatar | `AdwAvatar` | Fallback to initials. |
| A button with dropdown | `AdwSplitButton` | Primary action + menu arrow. |
| Icon + label button content | `AdwButtonContent` | Accessible icon-and-text combination. |
| Title + subtitle in header | `AdwWindowTitle` | Two-line title widget for header bars. |
| A keyboard shortcut label | `AdwShortcutLabel` | Updated keycap style. Since 1.8. |
| Style/theme management | `AdwStyleManager` | Color scheme, accent color, high contrast detection. |

---

## Deprecated Widgets and Replacements

Widgets marked deprecated will be removed in a future major version. Migrate to
their replacements.

| Deprecated Widget | Replacement | Since | Migration Notes |
|-------------------|-------------|-------|-----------------|
| `AdwLeaflet` | `AdwNavigationView` or `AdwNavigationSplitView` | 1.4 | `NavigationView` for drill-down, `NavigationSplitView` for sidebar+content. |
| `AdwLeafletPage` | `AdwNavigationPage` | 1.4 | Pages are now `AdwNavigationPage` with `title` and `tag`. |
| `AdwFlap` | `AdwOverlaySplitView` | 1.4 | Direct replacement for overlay sidebar pattern. |
| `AdwSqueezer` | `AdwViewSwitcher` + breakpoints | 1.4 | Use breakpoints to toggle between desktop/mobile switchers. |
| `AdwViewSwitcherTitle` | `AdwViewSwitcher` + `AdwViewSwitcherBar` + breakpoint | 1.4 | Manually toggle visibility with a breakpoint at ~550sp. |
| `AdwMessageDialog` | `AdwAlertDialog` | 1.5 | Updated styling, vertical button layout, derived from `AdwDialog`. |
| `AdwPreferencesWindow` | `AdwPreferencesDialog` | 1.5 | Dialog-based, adaptive presentation. |
| `AdwAboutWindow` | `AdwAboutDialog` | 1.5 | Dialog-based, adaptive presentation. |
| `GtkShortcutsWindow` | `AdwShortcutsDialog` | 1.8 | Simpler structure with `AdwShortcutsSection` and `AdwShortcutsItem`. |
| `GtkSpinner` | `AdwSpinner` | 1.6 | Custom-drawn, no gradient dithering at large sizes. |
| `GtkInfoBar` | `AdwBanner` | 1.3 | Simpler API: title + optional button at top of content. |

---

## Version Availability Table

Which widget appeared in which libadwaita version, and the corresponding Rust
feature flag to enable it.

### Since 1.0 (GNOME 42) -- no feature flag needed

| Widget |
|--------|
| `AdwApplication`, `AdwApplicationWindow`, `AdwWindow` |
| `AdwHeaderBar`, `AdwWindowTitle` |
| `AdwActionRow`, `AdwComboRow`, `AdwEntryRow`, `AdwExpanderRow`, `AdwPasswordEntryRow`, `AdwPreferencesRow` |
| `AdwPreferencesGroup`, `AdwPreferencesPage`, `AdwPreferencesWindow` |
| `AdwViewStack`, `AdwViewSwitcher`, `AdwViewSwitcherBar` |
| `AdwTabView`, `AdwTabBar`, `AdwTabButton`, `AdwTabOverview` |
| `AdwCarousel`, `AdwCarouselIndicatorDots`, `AdwCarouselIndicatorLines` |
| `AdwClamp`, `AdwClampLayout`, `AdwClampScrollable` |
| `AdwAvatar`, `AdwBin`, `AdwSplitButton`, `AdwButtonContent`, `AdwStatusPage` |
| `AdwToastOverlay`, `AdwToast` |
| `AdwStyleManager` |
| `AdwTimedAnimation`, `AdwSpringAnimation`, `AdwCallbackAnimationTarget`, `AdwPropertyAnimationTarget`, `AdwSpringParams` |
| `AdwLeaflet` (deprecated), `AdwFlap` (deprecated), `AdwSqueezer` (deprecated) |

### Since 1.3 (GNOME 44) -- `v1_3`

| Widget |
|--------|
| `AdwBanner` |

### Since 1.4 (GNOME 45) -- `v1_4`

| Widget |
|--------|
| `AdwToolbarView` |
| `AdwNavigationView`, `AdwNavigationPage`, `AdwNavigationSplitView`, `AdwOverlaySplitView` |
| `AdwBreakpoint`, `AdwBreakpointBin` |
| `AdwSwitchRow`, `AdwSpinRow` |

### Since 1.5 (GNOME 46) -- `v1_5`

| Widget |
|--------|
| `AdwDialog` |
| `AdwAlertDialog` |
| `AdwPreferencesDialog` |
| `AdwAboutDialog` |

### Since 1.6 (GNOME 47) -- `v1_6`

| Widget |
|--------|
| `AdwBottomSheet` |
| `AdwSpinner`, `AdwSpinnerPaintable` |
| `AdwButtonRow` |
| `AdwMultiLayoutView`, `AdwLayout`, `AdwLayoutSlot` |
| `AdwInlineViewSwitcher` |
| `AdwToggle`, `AdwToggleGroup` |

### Since 1.7 (GNOME 48) -- `v1_7`

Bug fixes and `AdwDialog` improvements. No major new widgets.

### Since 1.8 (GNOME 49) -- `v1_8`

| Widget |
|--------|
| `AdwShortcutsDialog`, `AdwShortcutsSection`, `AdwShortcutsItem` |
| `AdwShortcutLabel` |

### Since 1.9 (GNOME 50) -- `v1_9`

| Widget |
|--------|
| `AdwSidebar`, `AdwSidebarItem`, `AdwSidebarSection` |
| `AdwViewSwitcherSidebar` |
| `AdwNoneAnimationTarget` |
| `AdwWrapBox`, `AdwWrapLayout` |

---

## GTK4 vs Libadwaita: When to Replace

Some GTK4 widgets should be replaced with their libadwaita equivalents when
building GNOME apps. Others are fine to use as-is.

### Replace These GTK4 Widgets

| GTK4 Widget | Libadwaita Replacement | Why |
|-------------|----------------------|-----|
| `GtkApplication` | `AdwApplication` | Auto-initializes libadwaita, loads stylesheets. |
| `GtkApplicationWindow` | `AdwApplicationWindow` | Supports breakpoints, no built-in titlebar (use `AdwHeaderBar`). |
| `GtkHeaderBar` | `AdwHeaderBar` | Split title button control, integrates with `AdwNavigationView`. |
| `GtkInfoBar` | `AdwBanner` | Simpler, HIG-compliant. |
| `GtkSpinner` | `AdwSpinner` | Better rendering at all sizes. |
| `GtkShortcutsWindow` | `AdwShortcutsDialog` | Dialog-based, simpler API. |
| `GtkAboutDialog` | `AdwAboutDialog` | GNOME-styled, adaptive. |
| `GtkStackSidebar` | `AdwViewSwitcherSidebar` | Works with `AdwViewStack`, supports sections. |
| `GtkDialog` | `AdwDialog` | Adaptive presentation (floating or bottom sheet). |
| `GtkMessageDialog` | `AdwAlertDialog` | Modern styling, vertical buttons. |

### Keep Using These GTK4 Widgets

| GTK4 Widget | Notes |
|-------------|-------|
| `GtkBox` | No libadwaita replacement. Use `.toolbar` or `.linked` classes as needed. |
| `GtkButton` | No replacement. Apply adwaita style classes (`.suggested-action`, `.flat`, etc). |
| `GtkLabel` | No replacement. Apply typography style classes. |
| `GtkListBox` | No replacement. Use `.boxed-list` class for settings style. |
| `GtkScrolledWindow` | No replacement. |
| `GtkSearchBar` | No replacement. Use `.inline` class when inside `AdwToolbarView`. |
| `GtkActionBar` | No replacement. Compatible with `AdwToolbarView` bottom bar. |
| `GtkMenuButton` | No replacement. |
| `GtkPopover` / `GtkPopoverMenu` | No replacement. |
| `GtkStack` | Use `AdwViewStack` only when you need view switching with `AdwViewSwitcher`. |
| `GtkEntry` / `GtkTextView` | Use `AdwEntryRow` only inside list rows. Standalone text input stays GTK4. |
| `GtkCheckButton` / `GtkToggleButton` | No replacement. |
| `GtkImage` | No replacement. |
| `GtkSeparator` | No replacement. Use `.spacer` class for invisible separators. |

---

## Cargo Feature Flags

Enable the version feature matching your target libadwaita version. Each flag
includes all lower versions.

```toml
[dependencies]
libadwaita = { version = "0.9", rename = "adw", features = ["v1_6"] }
```

| Feature | Libadwaita Version | GNOME Version | Min Fedora |
|---------|-------------------|---------------|------------|
| (none) | 1.0 | 42 | 36 |
| `v1_1` | 1.1 | 42.x | 36 |
| `v1_2` | 1.2 | 43 | 37 |
| `v1_3` | 1.3 | 44 | 38 |
| `v1_4` | 1.4 | 45 | 39 |
| `v1_5` | 1.5 | 46 | 40 |
| `v1_6` | 1.6 | 47 | 41 |
| `v1_7` | 1.7 | 48 | 42 |
| `v1_8` | 1.8 | 49 | 43 |
| `v1_9` | 1.9 | 50 | 44 |
