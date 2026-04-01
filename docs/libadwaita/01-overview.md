# Libadwaita Overview

Libadwaita is a platform library that accompanies GTK 4 and implements the GNOME Human Interface Guidelines. It is the direct successor to Libhandy (which served a similar role for GTK 3). Libadwaita first shipped with GNOME 42 in December 2021.

See also: [Official Documentation](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/)

---

## Purpose

Libadwaita serves several roles in the GNOME ecosystem:

- **Augments GTK 4** with widgets that conform to the GNOME HIG
- **Owns the Adwaita stylesheet** (the theme was migrated out of GTK into libadwaita)
- **Provides adaptive layout widgets** that respond to available screen space
- **Supports runtime recoloring** with named CSS variables
- **Implements cross-desktop preferences** for dark style and accent colors
- **Provides animation APIs** (timed and spring-based)

---

## Relationship to GTK4

When GTK 4 was developed, GNOME-specific widgets were deliberately removed so GTK could remain a general-purpose toolkit. This separation benefits both projects:

- GTK developers can work on general-purpose widgets without being influenced by the GNOME HIG
- Libadwaita can iterate on GNOME-specific patterns at its own pace, with its own release schedule independent of GTK
- Other desktop environments can use GTK 4 without inheriting GNOME design decisions

In practice, GNOME applications use both libraries together. GTK 4 provides foundational widgets (Box, ListBox, Button, Entry) while libadwaita provides GNOME-specific counterparts and patterns (AdwHeaderBar, AdwNavigationView, AdwPreferencesDialog, AdwToolbarView).

---

## Version History

| Version | GNOME Release | Key Additions |
|---------|---------------|---------------|
| 1.0 | GNOME 42 | Initial release, base widgets, animations |
| 1.1 | GNOME 42.x | Incremental improvements |
| 1.2 | GNOME 43 | CSS variables, style refinements |
| 1.3 | GNOME 44 | AdwBanner, tab improvements |
| 1.4 | GNOME 45 | AdwBreakpoint, AdwNavigationView, AdwToolbarView, AdwDialog (preview), AdwSwitchRow, AdwSpinRow |
| 1.5 | GNOME 46 | AdwDialog replaces old dialog patterns, AdwAlertDialog, AdwPreferencesDialog, AdwAboutDialog |
| 1.6 | GNOME 47 | AdwBottomSheet, AdwSpinner, AdwButtonRow, AdwMultiLayoutView, accent colors |
| 1.7 | GNOME 48 | Bug fixes, AdwDialog improvements |
| 1.8 | GNOME 49 | AdwShortcutsDialog, AdwShortcutLabel, typography improvements, CSS media queries |
| 1.9 | GNOME 50 | AdwSidebar, AdwViewSwitcherSidebar, reduced motion support, AdwNoneAnimationTarget |

---

## Rust Crate Version Mapping

The Rust crate `libadwaita` (commonly renamed to `adw` in Cargo.toml) wraps the C library. Crate versions do not match library versions directly.

| Crate Version | C Library Version | Notes |
|---------------|-------------------|-------|
| 0.9.x | libadwaita 1.8 | Current latest stable |
| 0.8.x | libadwaita 1.7 | |
| 0.7.x | libadwaita 1.6 | |
| 0.6.x | libadwaita 1.5 | |

The `gtk4` and `libadwaita` crate versions must be kept in sync. If you use `libadwaita` 0.9, you must use the corresponding `gtk4` crate version. Mixing versions will cause compilation errors due to type mismatches.

### Crate Modules

The libadwaita-rs crate is organized into these modules:

| Module | Purpose |
|--------|---------|
| `prelude` | Commonly used trait imports; re-exports `gtk4::prelude` |
| `subclass` | Subclassing support; re-exports `gtk4::subclass::prelude` |
| `builders` | Builder pattern types for constructing widgets |
| `ffi` | Raw FFI bindings to the C library |

The crate also re-exports `glib`, `gio`, `gdk`, and `gtk` at its root, so you rarely need separate dependencies on those crates.

---

## Feature Flags

Feature flags gate access to APIs introduced in specific library versions. They are hierarchical: enabling `v1_4` automatically enables `v1_1`, `v1_2`, and `v1_3`.

| Feature Flag | Minimum C Library | Notable APIs Unlocked |
|--------------|-------------------|-----------------------|
| `v1_1` | libadwaita 1.1 | Incremental improvements |
| `v1_2` | libadwaita 1.2 | CSS variables |
| `v1_3` | libadwaita 1.3 | AdwBanner |
| `v1_4` | libadwaita 1.4 | AdwBreakpoint, AdwNavigationView, AdwToolbarView, AdwSwitchRow, AdwSpinRow |
| `v1_5` | libadwaita 1.5 | AdwDialog, AdwAlertDialog, AdwPreferencesDialog |
| `v1_6` | libadwaita 1.6 | AdwBottomSheet, AdwSpinner, AdwButtonRow, accent colors |
| `v1_7` | libadwaita 1.7 | Dialog improvements |
| `v1_8` | libadwaita 1.8 | AdwShortcutsDialog, AdwShortcutLabel |
| `v1_9` | libadwaita 1.9 | AdwSidebar, reduced motion support |

Choose the feature flag matching the libadwaita version installed on your target platform. For Fedora 42 / GNOME 48, `v1_7` is safe. For Fedora 43 / GNOME 49, `v1_8` is available.

---

## Widget Selection Guide

When choosing between GTK 4 and libadwaita widgets, prefer the libadwaita version when it exists. Common replacements:

| Instead of | Use | Reason |
|------------|-----|--------|
| GtkApplicationWindow | AdwApplicationWindow | No built-in titlebar, supports breakpoints |
| GtkHeaderBar | AdwHeaderBar | Split title-button properties, NavigationView integration |
| GtkDialog | AdwDialog (since 1.5) | Adaptive presentation (floating or bottom sheet) |
| GtkInfoBar | AdwBanner (since 1.3) | Cleaner API, HIG-compliant |
| GtkSpinner | AdwSpinner (since 1.6) | Better rendering at large sizes |
| GtkShortcutsWindow | AdwShortcutsDialog (since 1.8) | Simpler structure, dialog-based |
| AdwLeaflet (deprecated) | AdwNavigationView / AdwNavigationSplitView | Modern navigation API |
| AdwFlap (deprecated) | AdwOverlaySplitView | Overlay sidebar pattern |
| AdwMessageDialog (deprecated) | AdwAlertDialog | Accent-aware, vertical button layout |
| AdwPreferencesWindow (deprecated) | AdwPreferencesDialog | Dialog-based, adaptive |

---

## Complete Widget List

All widgets available in libadwaita as of version 1.9. Deprecated widgets are marked.

### Window & Application
AboutDialog, AboutWindow *(deprecated)*, Application, ApplicationWindow, Window

### Header & Toolbar
HeaderBar, ToolbarView, WindowTitle

### Navigation
NavigationPage, NavigationSplitView, NavigationView, OverlaySplitView

### View Switching
InlineViewSwitcher, ViewStack, ViewStackPage, ViewStackPages, ViewSwitcher, ViewSwitcherBar, ViewSwitcherSidebar, ViewSwitcherTitle *(deprecated)*

### Tabs
TabBar, TabButton, TabOverview, TabPage, TabView

### List & Preference Rows
ActionRow, ButtonRow, ComboRow, EntryRow, ExpanderRow, PasswordEntryRow, PreferencesRow, SpinRow, SwitchRow

### Preferences
PreferencesDialog, PreferencesGroup, PreferencesPage, PreferencesWindow *(deprecated)*

### Dialogs
AlertDialog, Dialog, MessageDialog *(deprecated)*, ShortcutsDialog, ShortcutsItem, ShortcutsSection

### Feedback
Banner, StatusPage, Toast, ToastOverlay

### Layout
Bin, BottomSheet, Breakpoint, BreakpointBin, Clamp, ClampLayout, ClampScrollable, MultiLayoutView, Layout, LayoutSlot, WrapBox, WrapLayout

### Animation
Animation, AnimationTarget, CallbackAnimationTarget, NoneAnimationTarget, PropertyAnimationTarget, SpringAnimation, SpringParams, TimedAnimation

### Other Widgets
Avatar, ButtonContent, Carousel, CarouselIndicatorDots, CarouselIndicatorLines, EnumListItem, EnumListModel, Flap *(deprecated)*, FlapPage *(deprecated)*, Leaflet *(deprecated)*, LeafletPage *(deprecated)*, ShortcutLabel, Sidebar, SidebarItem, SidebarSection, Spinner, SpinnerPaintable, SplitButton, Squeezer *(deprecated)*, SqueezerPage *(deprecated)*, StyleManager, SwipeTracker, Toggle, ToggleGroup

### Enumerations
AccentColor, AnimationState, BannerButtonStyle, CenteringPolicy, ColorScheme, DialogPresentationMode, Easing, LengthUnit, NavigationDirection, ResponseAppearance, SidebarMode, ToastPriority, ToolbarStyle, ViewSwitcherPolicy, WrapPolicy

---

## Deprecated Widgets

Avoid using these widgets in new code. They remain available for backward compatibility but have been replaced by modern alternatives.

| Deprecated Widget | Replacement | Deprecated Since |
|-------------------|-------------|-----------------|
| AdwAboutWindow | AdwAboutDialog | 1.6 |
| AdwFlap | AdwOverlaySplitView | 1.4 |
| AdwLeaflet | AdwNavigationView / AdwNavigationSplitView | 1.4 |
| AdwMessageDialog | AdwAlertDialog | 1.5 |
| AdwPreferencesWindow | AdwPreferencesDialog | 1.6 |
| AdwSqueezer | AdwViewSwitcher with breakpoints | 1.4 |
| AdwViewSwitcherTitle | AdwViewSwitcher + AdwViewSwitcherBar with breakpoint | 1.4 |

---

## Further Reading

- [Application Setup](02-application-setup.md) - Cargo.toml, platform packages, and application lifecycle
- [Window & Toolbar](03-window-and-toolbar.md) - Window types, header bars, and toolbar patterns
- [Navigation](04-navigation.md) - NavigationView, split views, and drill-down patterns
- [Adaptive Design](09-adaptive-design.md) - Breakpoints and responsive layout
- [HIG Design Principles](../HIG/01-design-principles.md) - The design values that libadwaita implements
