# Feedback and Status

This document covers libadwaita widgets for providing feedback to users: toast notifications, banners, status pages, and spinners. Each serves a distinct purpose and choosing the right widget for the right context is important for a polished GNOME application.

---

## AdwToastOverlay

`AdwToastOverlay` is a container widget that displays `AdwToast` notifications overlaid on its content. It acts as the anchor point for toast messages and must wrap the content area where toasts should appear.

### Key Properties

| Property | Type | Description |
|----------|------|-------------|
| `child` | `GtkWidget` | The main content widget displayed beneath toasts |

### Usage

- Add a single main child widget (your content area)
- Call `add_toast()` to display a toast notification
- Multiple toasts queue automatically; only one is shown at a time
- Typically wraps the main content of a window or page

### Rust Example

```rust
use adw::prelude::*;

let toast_overlay = adw::ToastOverlay::new();
toast_overlay.set_child(Some(&content_widget));

// Show a simple toast
let toast = adw::Toast::new("File saved successfully");
toast_overlay.add_toast(toast);
```

---

## AdwToast

`AdwToast` represents a single toast notification. Toasts are transient messages that appear briefly and then dismiss themselves. They are ideal for confirming completed actions or providing non-critical information.

### Key Properties

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `title` | `String` | Required | The message text |
| `button-label` | `String` | `None` | Optional action button text |
| `action-name` | `String` | `None` | Action triggered by the button |
| `use-markup` | `bool` | `false` | Whether title contains Pango markup |
| `timeout` | `u32` | `5` | Seconds before auto-dismiss (0 = no timeout) |
| `priority` | `ToastPriority` | `Normal` | `Normal` or `High` |

### Priority Levels

| Priority | Behavior |
|----------|----------|
| `Normal` | Queued behind other toasts; replaced by newer toasts if queue is full |
| `High` | Displayed immediately, replacing the current toast |

### Undo Pattern

The most common use of the toast button is providing an undo action for destructive operations. This gives users a brief window to reverse an action before it becomes permanent.

```rust
use adw::prelude::*;

fn delete_item(toast_overlay: &adw::ToastOverlay, item_name: &str) {
    // Mark the item as deleted (but don't destroy it yet)
    let toast = adw::Toast::new(&format!("{item_name} deleted"));
    toast.set_button_label(Some("Undo"));
    toast.set_action_name(Some("app.undo-delete"));
    toast.set_timeout(5);
    toast.set_priority(adw::ToastPriority::High);

    // Connect to the dismissed signal to finalize the deletion
    toast.connect_dismissed(move |_| {
        // Permanently delete the item here
    });

    toast_overlay.add_toast(toast);
}
```

### Markup Example

```rust
let toast = adw::Toast::new("");
toast.set_title(Some("<b>3 files</b> moved to Trash"));
toast.set_use_markup(true);
```

---

## AdwBanner

*Since 1.3.* `AdwBanner` replaces `GtkInfoBar` and displays a persistent message at the top of the content area. Use banners for important, ongoing information that the user should be aware of but that does not block interaction.

### Key Properties

| Property | Type | Default | Description |
|----------|------|---------|-------------|
| `title` | `String` | Required | The message text (supports markup) |
| `button-label` | `String` | `None` | Optional action button text |
| `revealed` | `bool` | `false` | Whether the banner is visible |
| `use-markup` | `bool` | `false` | Whether title contains Pango markup |

### Usage Notes

- Place the banner at the top of your content area, above the main content but below the header bar
- Control visibility with the `revealed` property (animates in/out)
- Supports at most one action button
- The banner stretches the full width of its container

### Rust Example

```rust
use adw::prelude::*;

let banner = adw::Banner::new("No internet connection");
banner.set_button_label(Some("Retry"));
banner.set_revealed(true);

// In a toolbar view, add the banner above content
let toolbar_view = adw::ToolbarView::new();
toolbar_view.add_top_bar(&adw::HeaderBar::new());
toolbar_view.add_top_bar(&banner);
toolbar_view.set_content(Some(&main_content));

// Connect the button click
banner.connect_button_clicked(move |_banner| {
    // Handle retry logic
});
```

### Blueprint

```xml
<object class="AdwBanner" id="offline_banner">
  <property name="title" translatable="yes">No internet connection</property>
  <property name="button-label" translatable="yes">Retry</property>
  <property name="revealed">false</property>
</object>
```

---

## AdwStatusPage

`AdwStatusPage` displays a prominent status message with an icon, title, description, and optional child widget. Use it for empty states, error pages, loading screens, and onboarding flows.

### Key Properties

| Property | Type | Description |
|----------|------|-------------|
| `icon-name` | `String` | Symbolic icon name |
| `paintable` | `GdkPaintable` | Custom paintable for the icon area |
| `title` | `String` | Primary heading text |
| `description` | `String` | Secondary description text (supports markup) |
| `child` | `GtkWidget` | Optional child widget below the description |

### Style Classes

| Class | Effect |
|-------|--------|
| `.compact` | Reduced spacing, suitable for sidebars, popovers, and smaller containers |

### Common Patterns

**Empty state** -- shown when a list or view has no content:

```rust
use adw::prelude::*;

let status_page = adw::StatusPage::builder()
    .icon_name("folder-documents-symbolic")
    .title("No Documents")
    .description("Documents you create will appear here")
    .build();
```

**Error state** -- shown when content fails to load:

```rust
let retry_button = gtk::Button::builder()
    .label("Try Again")
    .halign(gtk::Align::Center)
    .css_classes(["suggested-action", "pill"])
    .build();

let status_page = adw::StatusPage::builder()
    .icon_name("dialog-error-symbolic")
    .title("Unable to Load")
    .description("Check your internet connection and try again")
    .child(&retry_button)
    .build();
```

**Compact status page** for sidebars:

```rust
let status_page = adw::StatusPage::builder()
    .icon_name("edit-find-symbolic")
    .title("No Results")
    .build();
status_page.add_css_class("compact");
```

---

## AdwSpinner

*Since 1.6.* `AdwSpinner` is a custom-drawn spinner that replaces `GtkSpinner`. It avoids the gradient dithering artifacts that `GtkSpinner` exhibits at larger sizes.

### Key Characteristics

- Caps rendering at 64x64 pixels regardless of allocated size
- No gradient dithering at large sizes (unlike `GtkSpinner`)
- Continues animating even when system animations are disabled (important for indicating ongoing activity)
- No properties beyond standard widget properties; it simply spins when visible

### Rust Example

```rust
let spinner = adw::Spinner::new();

// Use in a status page for loading states
let loading_page = adw::StatusPage::builder()
    .paintable(&adw::SpinnerPaintable::new(None::<&adw::Spinner>))
    .title("Loading")
    .description("Please wait...")
    .build();
```

## AdwSpinnerPaintable

*Since 1.6.* `AdwSpinnerPaintable` is a `GdkPaintable` version of the spinner, suitable for use in `GtkImage`, `AdwStatusPage`, `AdwAvatar`, or anywhere a paintable is accepted.

```rust
// Use in a GtkImage
let paintable = adw::SpinnerPaintable::new(None::<&adw::Spinner>);
let image = gtk::Image::from_paintable(Some(&paintable));
```

---

## Choosing the Right Feedback Widget

| Widget | Duration | Severity | Use When |
|--------|----------|----------|----------|
| `AdwToast` | Transient (auto-dismiss) | Low | Confirming completed actions, undo opportunities |
| `AdwBanner` | Persistent (until dismissed) | Medium | Ongoing conditions (offline, sync issues, updates) |
| `AdwStatusPage` | Permanent (replaces content) | High | Empty states, errors, loading, onboarding |
| `AdwSpinner` | While loading | Neutral | Indicating background activity |

### Guidelines

- **Toasts** should never contain critical information. The user may miss them entirely. Use them for confirmations and undo actions.
- **Banners** are for conditions the user should know about but can continue working despite. They persist until the condition changes.
- **Status pages** replace the entire content area. Use them when there is genuinely nothing else to show (empty list, failed load, first-run experience).
- **Spinners** indicate that something is happening. Pair them with a status page for full-screen loading, or use inline for smaller loading indicators.

---

## Cross-References

- [Visual Design](../HIG/06-visual-design.md) -- typography and spacing guidelines for feedback text
- [Overview](01-overview.md) -- complete widget list and version history
