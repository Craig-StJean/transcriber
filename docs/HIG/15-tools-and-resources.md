# Tools and Resources

A curated list of design tools, development utilities, and reference materials for building
GNOME-compliant applications and extensions.

## Design Applications

These applications are available on Flathub and help with visual design, icon creation, and
accessibility validation.

| Application | Purpose |
|---|---|
| Icon Library | Browse and search the full set of GNOME symbolic icons by name, category, or keyword |
| Typography | Font selection, character map, and typographic style reference |
| App Icon Preview | Preview and validate application icon designs against the GNOME icon grid |
| Symbolic Preview | Test symbolic icon rendering at various sizes and against different backgrounds |
| Color Palette | Reference the official GNOME color scheme with hex values and named colors |
| Contrast | WCAG contrast ratio checker — verify that text and background combinations meet accessibility thresholds |

Install any of these via Flatpak:

```bash
flatpak install flathub org.gnome.design.IconLibrary
flatpak install flathub org.gnome.design.Contrast
```

## Development Tools

| Tool | Purpose | How to Launch |
|---|---|---|
| Adwaita Demo | Interactive showcase of all libadwaita widgets, patterns, and styling | `adwaita-1-demo` |
| GTK Inspector | Inspect and modify any running GTK application in real time — examine widget hierarchy, CSS, properties, and accessibility tree | `GTK_DEBUG=interactive app-name` |
| GTK Icon Browser | Browse all available GTK and Adwaita icons with search | `gtk4-icon-browser` |
| Looking Glass | Built-in GNOME Shell JavaScript debugger and console | Alt+F2, type `lg`, press Enter |
| D-Feet / D-Bus Monitor | Inspect D-Bus services, interfaces, methods, and signals | `d-feet` or `dbus-monitor` |
| Accerciser | AT-SPI accessibility inspector — explore the accessibility tree of running applications | `accerciser` |

### GTK Inspector Tips

- Press Ctrl+Shift+D inside any GTK4 application to toggle the inspector (if
  `GTK_DEBUG=interactive` is set).
- Use the "Objects" tab to browse the widget tree.
- Use the "CSS" tab to inject custom CSS and test changes live.
- Use the "Accessibility" tab to verify AT-SPI roles and properties.

## Blueprint Compiler

Blueprint is a declarative UI markup language for GTK4 that compiles to GtkBuilder XML. It
offers a cleaner, more readable syntax than raw XML.

### Installation

```bash
# Via Flatpak
flatpak install flathub org.gnome.Blueprint

# Via system package (Fedora)
sudo dnf install blueprint-compiler

# Via pip
pip install blueprint-compiler
```

### Usage

```bash
# Compile a single file
blueprint-compiler compile ui/window.blp

# Batch compile
blueprint-compiler batch-compile output/ ui/*.blp
```

### Example

```
using Gtk 4.0;
using Adw 1;

template $MyAppWindow: Adw.ApplicationWindow {
  default-width: 600;
  default-height: 400;

  content: Adw.ToolbarView {
    [top]
    Adw.HeaderBar {}

    content: Gtk.Label {
      label: "Hello, GNOME";
      styles ["title-1"]
    };
  };
}
```

### Documentation

Full Blueprint documentation is available at:
https://jwestman.pages.gitlab.gnome.org/blueprint-compiler/

## Design Assets

The GNOME Design team maintains shared assets for application mockups and icon design.

- **Color palettes** in Inkscape (`.gpl`) and GIMP (`.gpl`) format
- **SVG templates** for application mockups and wireframes
- **App icon grid template** — 128 x 128 px canvas with a 2 px grid for pixel-aligned icon
  design
- **Symbolic icon template** — 16 x 16 px canvas with standard margins and alignment guides

These assets are available from the GNOME Design team's GitLab repository:
https://gitlab.gnome.org/Teams/Design

## Key Documentation Links

| Resource | URL |
|---|---|
| GNOME Human Interface Guidelines | https://developer.gnome.org/hig/ |
| Libadwaita Documentation | https://gnome.pages.gitlab.gnome.org/libadwaita/doc/ |
| GTK4 Documentation | https://docs.gtk.org/gtk4/ |
| GJS Guide | https://gjs.guide/ |
| GNOME Shell Extension Guide | https://gjs.guide/extensions/ |
| Blueprint Compiler | https://jwestman.pages.gitlab.gnome.org/blueprint-compiler/ |
| GNOME Design GitLab | https://gitlab.gnome.org/Teams/Design |
| Freedesktop Specifications | https://specifications.freedesktop.org/ |
| GNOME Developer Documentation | https://developer.gnome.org/ |

## Recommended Workflow

A practical workflow for building GNOME applications and extensions, from design through
validation.

1. **Reference the HIG** for design decisions — layout, navigation, controls, spacing, and
   typography.
2. **Check Adwaita Demo** (`adwaita-1-demo`) to see which widgets are available and how they
   look and behave before building custom components.
3. **Use Blueprint** for UI layout. Write `.blp` files and compile them to XML. Blueprint
   syntax is easier to read and write than raw GtkBuilder XML.
4. **Use GTK Inspector** (`GTK_DEBUG=interactive`) to debug layout issues, test CSS changes
   live, and inspect widget properties at runtime.
5. **Test with accessibility tools.** Run Orca (screen reader), enable high-contrast mode, and
   switch to large text to verify that the application remains usable.
6. **Validate icons** with App Icon Preview (for application icons) and Symbolic Preview (for
   symbolic/action icons). Ensure icons render clearly at all standard sizes.

## See Also

- [Visual Design](06-visual-design.md) — color, typography, spacing, and iconography
  guidelines
- [App Identity](02-app-identity.md) — application naming, icon design, and branding
- [Accessibility](11-accessibility.md) — accessibility requirements and testing procedures
