# Libadwaita Reference Documentation

This directory contains practical reference documentation for building GNOME applications with [libadwaita](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/) and Rust. It covers widget APIs, application setup patterns, and idiomatic usage of the libadwaita-rs crate. These documents are intended to be used alongside the official upstream documentation, not as a replacement.

---

## Component Quick Reference

| Application Component | Relevant Files |
|---|---|
| **Project setup** (Cargo.toml, dependencies, platform packages) | [02 Application Setup](02-application-setup.md) |
| **Main window** (AdwApplicationWindow, header bar, toolbars) | [03 Window & Toolbar](03-window-and-toolbar.md) |
| **Widget overview** (full widget list, version history, feature flags) | [01 Overview](01-overview.md) |
| **Application lifecycle** (AdwApplication, init, prelude imports) | [02 Application Setup](02-application-setup.md) |
| **Subclassing** (CompositeTemplate, ObjectSubclass) | [02 Application Setup](02-application-setup.md) |
| **Navigation** (NavigationView, split views, drill-down) | [04 Navigation](04-navigation.md) |
| **View switching** (ViewStack, tabs, switchers) | [05 View Switching and Tabs](05-view-switching-and-tabs.md) |
| **Lists and rows** (action rows, switch rows, boxed lists) | [06 Lists & Rows](06-lists-and-rows.md) |
| **Preferences and dialogs** (preferences, alerts, shortcuts) | [07 Preferences & Dialogs](07-preferences-and-dialogs.md) |
| **Toasts, banners, status pages** | [08 Feedback and Status](08-feedback-and-status.md) |
| **Breakpoints and responsive layout** | [09 Adaptive Design](09-adaptive-design.md) |
| **Clamp, carousel, wrap box, layout helpers** | [10 Layout Widgets](10-layout-widgets.md) |
| **Color scheme, accent colors, CSS variables** | [11 Styling and Theming](11-styling-and-theming.md) |
| **CSS classes and styling** | [12 Style Classes](12-style-classes.md) |
| **Timed and spring animations** | [13 Animations](13-animations.md) |
| **Complete UI recipes** (app lifecycle, preferences, adaptive sidebar) | [14 Common Patterns](14-common-patterns.md) |
| **Widget selection help** (deprecated widgets, migration) | [15 Widget Selection & Migration](15-widget-selection-and-migration.md) |

---

## Full Table of Contents

| # | File | Description |
|---|---|---|
| 01 | [Overview](01-overview.md) | What libadwaita is, its relationship to GTK4, version history, Rust crate mapping, feature flags, and the complete widget list. |
| 02 | [Application Setup](02-application-setup.md) | Cargo.toml configuration, platform dependencies, AdwApplication vs GtkApplication, prelude imports, subclassing with CompositeTemplate, and Blueprint integration. |
| 03 | [Window & Toolbar](03-window-and-toolbar.md) | AdwWindow, AdwApplicationWindow, AdwToolbarView, AdwHeaderBar, AdwWindowTitle, AdwBottomSheet, and practical window/toolbar skeleton code. |
| 04 | [Navigation](04-navigation.md) | AdwNavigationView, AdwNavigationSplitView, AdwOverlaySplitView, and drill-down navigation patterns. |
| 05 | [View Switching and Tabs](05-view-switching-and-tabs.md) | AdwViewStack, AdwViewSwitcher, AdwTabView, AdwTabBar, and desktop/mobile switching patterns. |
| 06 | [Lists & Rows](06-lists-and-rows.md) | AdwActionRow, AdwSwitchRow, AdwSpinRow, AdwComboRow, AdwEntryRow, AdwExpanderRow, AdwButtonRow, and the boxed list pattern. |
| 07 | [Preferences & Dialogs](07-preferences-and-dialogs.md) | AdwPreferencesDialog, AdwDialog, AdwAlertDialog, AdwShortcutsDialog, and adaptive dialog patterns. |
| 08 | [Feedback and Status](08-feedback-and-status.md) | AdwToast, AdwToastOverlay, AdwBanner, AdwStatusPage, and notification patterns. |
| 09 | [Adaptive Design](09-adaptive-design.md) | AdwBreakpoint, scalable pixels, responsive layout patterns, and multi-layout views. |
| 10 | [Layout Widgets](10-layout-widgets.md) | AdwClamp, AdwWrapBox, AdwCarousel, AdwAvatar, AdwSplitButton, AdwButtonContent, and AdwBin. |
| 11 | [Styling and Theming](11-styling-and-theming.md) | AdwStyleManager, accent colors, CSS variables reference, GNOME palette, media queries, and custom stylesheets. |
| 12 | [Style Classes](12-style-classes.md) | CSS class reference for buttons, typography, containers, and visual styling. |
| 13 | [Animations](13-animations.md) | AdwTimedAnimation, AdwSpringAnimation, animation targets, and easing functions. |
| 14 | [Common Patterns](14-common-patterns.md) | Complete recipes: app lifecycle, window+toolbar, preferences dialog, adaptive navigation, boxed lists, toast with undo, shortcuts. |
| 15 | [Widget Selection & Migration](15-widget-selection-and-migration.md) | Decision table for choosing widgets, deprecated widgets and replacements, version availability by feature flag. |

---

## External Resources

- [Libadwaita Official Documentation](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/)
- [Libadwaita Widget Gallery](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/widget-gallery.html)
- [libadwaita crate on crates.io](https://crates.io/crates/libadwaita)
- [libadwaita-rs API Docs (docs.rs)](https://docs.rs/libadwaita/latest/libadwaita/)
- [GUI Development with Rust and GTK 4](https://gtk-rs.org/gtk4-rs/stable/latest/book/)
- [GNOME Human Interface Guidelines](https://developer.gnome.org/hig/)
- [Libadwaita CSS Variables Reference](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/css-variables.html)
- [Libadwaita Style Classes Reference](https://gnome.pages.gitlab.gnome.org/libadwaita/doc/main/style-classes.html)

---

## Related Documentation

- [HIG Reference](../HIG/00-INDEX.md) - GNOME Human Interface Guidelines reference for design decisions
