# Application Setup

This document covers everything needed to start a GNOME application with libadwaita in Rust: dependency configuration, platform packages, application lifecycle, prelude imports, subclassing, and Blueprint integration.

See also: [gtk-rs Book - Libadwaita](https://gtk-rs.org/gtk4-rs/stable/latest/book/libadwaita.html)

---

## Cargo.toml Setup

Add libadwaita with the `--rename` flag so you can use `adw` throughout your code instead of the longer `libadwaita` name:

```bash
cargo add libadwaita --rename adw --features v1_8
```

This produces the following in `Cargo.toml`:

```toml
[dependencies]
adw = { package = "libadwaita", version = "0.9", features = ["v1_8"] }
```

### Syncing gtk4 and libadwaita Versions

The `gtk4` and `libadwaita` crate versions must match. If you use `libadwaita` 0.9, you need the corresponding `gtk4` crate version. Mixing versions causes compilation errors because the types from one version are incompatible with the other.

In most cases, you do not need a separate `gtk4` dependency at all. The `libadwaita` crate re-exports `gtk` (along with `glib`, `gio`, and `gdk`), so you can access GTK types through the `adw` crate:

```rust
use adw::gtk;  // Re-exported gtk4 crate
use adw::glib;
use adw::gio;
use adw::gdk;
```

If you do need `gtk4` as a direct dependency (for example, to enable its feature flags), ensure the version matches what `libadwaita` expects.

### Feature Flags

Feature flags are hierarchical. Enabling `v1_8` automatically enables `v1_1` through `v1_7`. Choose the flag matching the libadwaita version on your target platform:

| Target Platform | Recommended Feature |
|-----------------|---------------------|
| Fedora 41 / GNOME 47 | `v1_6` |
| Fedora 42 / GNOME 48 | `v1_7` |
| Fedora 43 / GNOME 49 | `v1_8` |
| Ubuntu 24.04 LTS | `v1_4` |
| Ubuntu 24.10 | `v1_6` |
| Flathub (GNOME 48 runtime) | `v1_7` |

---

## Platform Installation

The libadwaita C library and its development headers must be installed on the build system.

### Fedora / RHEL

```bash
sudo dnf install libadwaita-devel
```

### Ubuntu / Debian

```bash
sudo apt install libadwaita-1-dev
```

### Windows (gvsbuild)

```bash
gvsbuild build libadwaita librsvg
```

### Flatpak

When building with Flatpak, libadwaita is included in the GNOME SDK runtime. No additional installation is needed beyond specifying the runtime in your Flatpak manifest:

```yaml
runtime: org.gnome.Platform
runtime-version: '48'
sdk: org.gnome.Sdk
```

---

## AdwApplication vs GtkApplication

`adw::Application` is a subclass of `gtk::Application`. It adds automatic initialization that you would otherwise need to do manually:

| Responsibility | GtkApplication | AdwApplication |
|----------------|----------------|----------------|
| Call `adw::init()` | You must call it yourself | Done automatically |
| Load translations | Manual | Automatic |
| Register types | Manual | Automatic |
| Load `style.css` from resources | Manual | Automatic |
| Load icon resources | Manual | Automatic |
| Set up Adwaita stylesheet | Manual | Automatic |

Always use `adw::Application` for GNOME apps. Using `gtk::Application` and calling `adw::init()` manually works but provides fewer features.

### Automatic Style Loading

`AdwApplication` loads stylesheets from your application's GResource bundle automatically:

| Resource Path | When Loaded |
|---------------|-------------|
| `style.css` | Always |
| `style-dark.css` | When dark mode is active *(deprecated in 1.9)* |
| `style-hc.css` | When high contrast is active *(deprecated in 1.9)* |
| `style-hc-dark.css` | When both are active *(deprecated in 1.9)* |

Since libadwaita 1.9, the recommended approach is a single `style.css` with CSS media queries:

```css
/* style.css */
.my-custom-widget {
    background-color: var(--view-bg-color);
    color: var(--view-fg-color);
}

@media (prefers-color-scheme: dark) {
    .my-custom-widget {
        border-color: rgba(255, 255, 255, 0.1);
    }
}

@media (prefers-contrast: more) {
    .my-custom-widget {
        border: 2px solid var(--border-color);
    }
}
```

---

## Basic Application Example

A minimal libadwaita application in Rust:

```rust
use adw::prelude::*;
use adw::{Application, ApplicationWindow, HeaderBar};
use gtk::{Box, Orientation};

fn main() -> glib::ExitCode {
    let app = Application::builder()
        .application_id("com.example.MyApp")
        .build();

    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &adw::Application) {
    let content = Box::new(Orientation::Vertical, 0);

    let header_bar = HeaderBar::new();
    content.append(&header_bar);

    let window = ApplicationWindow::builder()
        .application(app)
        .title("My App")
        .content(&content)
        .build();

    window.present();
}
```

The application lifecycle follows this sequence:

1. Create `adw::Application` with an application ID
2. Connect to the `activate` signal
3. Build the UI inside the activate handler
4. Call `app.run()` to start the main loop

---

## Prelude Imports

The `adw::prelude` module re-exports the `gtk4::prelude`, so you only need one import for both:

```rust
use adw::prelude::*;
```

This gives you access to all trait methods from both libadwaita and GTK 4 (WidgetExt, BoxExt, ButtonExt, ApplicationWindowExt, and so on).

Similarly, `adw::subclass::prelude` re-exports `gtk4::subclass::prelude`:

```rust
use adw::subclass::prelude::*;
```

There is no need to import `gtk::prelude::*` or `gtk::subclass::prelude::*` separately when using the adw versions.

### Re-exports from the Crate

The `libadwaita` crate re-exports several foundational crates so you do not need to add them as direct dependencies:

| Re-export | Access via |
|-----------|-----------|
| gtk4 | `adw::gtk` or `use gtk` (if re-exported at crate root) |
| glib | `adw::glib` |
| gio | `adw::gio` |
| gdk4 | `adw::gdk` |
| libadwaita-sys (FFI) | `adw::ffi` |

---

## Subclassing with CompositeTemplate

Most non-trivial applications define custom window and widget types using the GObject subclassing pattern. This connects Rust code to UI definitions in XML or Blueprint files.

### Window Implementation (imp.rs)

```rust
use adw::subclass::prelude::*;
use gtk::CompositeTemplate;

#[derive(CompositeTemplate, Default)]
#[template(resource = "/com/example/myapp/window.ui")]
pub struct Window {
    #[template_child]
    pub header_bar: TemplateChild<adw::HeaderBar>,
    #[template_child]
    pub content_box: TemplateChild<gtk::Box>,
}

#[glib::object_subclass]
impl ObjectSubclass for Window {
    const NAME: &'static str = "MyAppWindow";
    type Type = super::Window;
    type ParentType = adw::ApplicationWindow;

    fn class_init(klass: &mut Self::Class) {
        klass.bind_template();
    }

    fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for Window {}
impl WidgetImpl for Window {}
impl WindowImpl for Window {}
impl ApplicationWindowImpl for Window {}
impl AdwApplicationWindowImpl for Window {}
```

### Window Wrapper (mod.rs)

```rust
mod imp;

use adw::subclass::prelude::*;
use gtk::gio;

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends adw::ApplicationWindow, gtk::ApplicationWindow,
                 gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap;
}

impl Window {
    pub fn new(app: &adw::Application) -> Self {
        glib::Object::builder()
            .property("application", app)
            .build()
    }
}
```

### Key Points

- The `ParentType` must be an adw type (e.g., `adw::ApplicationWindow`) rather than a GTK type when subclassing libadwaita widgets.
- You must implement the full trait chain: `ObjectImpl`, `WidgetImpl`, `WindowImpl`, `ApplicationWindowImpl`, `AdwApplicationWindowImpl`. Missing any of these causes a compilation error.
- `#[template(resource = "...")]` loads from GResource. Use `#[template(file = "...")]` during development to load from the filesystem directly (no resource compilation needed).

---

## Blueprint Integration

[Blueprint](https://jwestman.pages.gitlab.gnome.org/blueprint-compiler/) is a modern markup language for defining GTK/libadwaita UIs. It compiles to the same XML format that `CompositeTemplate` expects.

### Blueprint Basics

A Blueprint file begins with `using` declarations for the libraries it references:

```blueprint
using Gtk 4.0;
using Adw 1;
```

### Example: Window Template

```blueprint
using Gtk 4.0;
using Adw 1;

template $MyAppWindow: Adw.ApplicationWindow {
  title: "My App";
  default-width: 800;
  default-height: 600;

  content: Adw.ToolbarView {
    [top]
    Adw.HeaderBar {}

    content: Gtk.Box {
      orientation: vertical;
      spacing: 12;

      Gtk.Label {
        label: "Hello, World!";
        styles ["title-1"]
      }
    };
  };
}
```

### Compiling Blueprints

Blueprint files (`.blp`) are compiled to XML (`.ui`) using `blueprint-compiler`:

```bash
blueprint-compiler compile window.blp --output window.ui
```

In a Meson build system, use the `blueprints` option in `gnome.compile_resources()`:

```meson
gnome.compile_resources(
  'myapp',
  'myapp.gresource.xml',
  gresource_bundle: true,
  install: true,
  install_dir: pkgdatadir,
  dependencies: blueprints,
)
```

For Cargo-only projects without Meson, pre-compile `.blp` files to `.ui` and reference the `.ui` files directly with `#[template(file = "...")]`.

---

## Further Reading

- [Overview](01-overview.md) - Version history, widget list, and feature flags
- [Window & Toolbar](03-window-and-toolbar.md) - Window types, header bars, and toolbar patterns
- [HIG Layout & Navigation](../HIG/03-layout-and-navigation.md) - Design guidance for window structure
