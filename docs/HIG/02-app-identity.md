# App Identity

An application's identity is defined by three elements: its name, its icon, and its desktop integration metadata. These must work together to create a recognizable, professional presence across the GNOME desktop.

See also: [Official GNOME HIG - App Icons](https://developer.gnome.org/hig/guidelines/app-icons.html)

---

## App Naming

The app name is the user's primary identifier for the application. It appears in the Activities overview, the header bar, the app grid, and the About dialog.

### Good Name Characteristics

| Criterion | Guideline | Example |
|---|---|---|
| **Length** | 1-2 simple nouns, under 15 characters | "Transcriber", "Text Editor", "Files" |
| **Domain relevance** | Related to the app's function or content area | "Podcasts" for a podcast player |
| **Pronounceability** | Easy to say aloud and remember | "Clocks" not "ChronoSync" |
| **Capitalization** | Header capitalization (capitalize major words) | "Sound Recorder", not "sound recorder" |
| **Searchability** | Distinctive enough to find in a search | "Transcriber" not "Tool" |

### Names to Avoid

| Pattern | Why It Fails | Bad Example | Better Alternative |
|---|---|---|---|
| Reusing trademarks | Legal risk, user confusion | "Google Docs Editor" | "Documents" |
| "G" or "Gnome" prefix | Outdated convention, clutters name | "GTranscribe" | "Transcriber" |
| Complex terminology | Excludes non-specialist users | "Acoustic Diarizer" | "Voice Notes" |
| Humor or puns | May not translate across cultures | "Ctrl+Zhift" | "Undo" |
| Non-standard punctuation | Causes sorting/display issues | "trans.cr" | "Transcriber" |
| Generic single words | Impossible to search for | "App", "Tool" | "Sound Recorder" |
| Acronyms | Opaque to new users | "ASR Suite" | "Transcriber" |

### Name Selection Process

1. **Brainstorm.** List 15-20 candidate names. Include simple nouns, compound nouns, and short descriptive phrases.
2. **Shortlist.** Narrow to 3-5 names based on the criteria above.
3. **Check availability.** Search Flathub, GNOME GitLab, and package repositories. Verify the app ID namespace is available.
4. **Evaluate.** Test each finalist for pronunciation, memorability, searchability, and cultural sensitivity. Ask someone unfamiliar with the project to guess what the app does from the name alone.

---

## App Icons

Every GNOME application must have a unique, recognizable icon. The icon is the app's visual identity and appears in the app grid, the Activities overview, the taskbar, and the About dialog.

### Design Requirements

#### Metaphor and Recognition

- Choose a simple, recognizable metaphor connected to the app's name or primary function.
- The metaphor should be identifiable at a glance. A microphone for a recorder. A globe for a browser. A pencil for an editor.
- Avoid abstract or overly clever imagery. If users cannot guess the app's purpose from the icon, the metaphor is wrong.

#### Canvas and Geometry

| Property | Value |
|---|---|
| Canvas size | 128 x 128 px |
| Grid | 2px grid template |
| Minimum legibility | Must be recognizable at 32 x 32 px |
| Shape basis | Simple geometric forms (circles, rounded rectangles, basic polygons) |

- Use basic geometric shapes as the foundation. Avoid complex outlines and fine details.
- Maintain balanced visual weight. The icon should feel stable and centered on the canvas.
- Follow the standard 2px grid template provided by the GNOME icon design resources.

#### Perspective and Depth

- Use a combined **top + front view** (slightly tilted forward, as if viewed from above at a shallow angle).
- The **front profile should be darker** than the top surface, with a maximum height of 4 pixels for the front face.
- This creates a subtle 3D effect without full perspective rendering.

```
  ┌──────────────────┐  ← Top surface (lighter)
  │                  │
  │    Icon motif    │     Height: main area
  │                  │
  ├──────────────────┤
  │  Front profile   │     Height: max 4px (darker)
  └──────────────────┘
```

#### Color and Shading

| Surface Type | Color Treatment |
|---|---|
| Flat / straight surfaces | Flat, solid colors |
| Curved surfaces | Subtle gradients following the curve direction |
| Shadows | Minimal. Light source from above. Soft drop shadow only. |

- Use a limited, deliberate color palette. Two to four dominant colors is typical.
- Avoid harsh outlines. Let color contrast and subtle shading define edges.
- The light source is always from directly above, creating consistent shadow direction across all GNOME app icons.

#### Variants

| Variant | Purpose | Visual Treatment |
|---|---|---|
| **Stable / Release** | Production icon | Standard colors and design |
| **Nightly** | Development builds | Add a hardhat, construction theme, or desaturated palette |
| **Beta** | Pre-release testing | Subtle badge or color shift to distinguish from stable |

Each variant must be visually distinct so users can tell at a glance which version they are running, especially when multiple versions are installed simultaneously.

### Icon Design Checklist

- [ ] Simple geometric shapes, no fine detail
- [ ] Recognizable metaphor connected to app name/function
- [ ] 128 x 128 px canvas, follows 2px grid
- [ ] Legible at 32 x 32 px
- [ ] Top + front perspective, front face max 4px and darker
- [ ] Flat colors on straight surfaces, gradients on curves
- [ ] Light from above, minimal shadows
- [ ] Nightly and beta variants created
- [ ] Tested on both light and dark desktop backgrounds

---

## Symbolic and UI Icons

Symbolic icons are monochrome, minimalist icons used within the application UI for buttons, menus, and status indicators. They are distinct from the full-color app icon.

### Design Specifications

| Property | Value |
|---|---|
| Format | SVG |
| Canvas size | 16 x 16 px |
| Color | Monochrome (single fill color) |
| Stroke width | 2px |
| Perspective | Orthogonal (flat, no depth) |
| Recoloring | Must support programmatic recoloring via CSS/GTK |

### Design Principles for Symbolic Icons

- **Minimalist.** Use the fewest possible strokes to convey meaning. Remove any detail that does not aid recognition.
- **Orthogonal perspective.** No 3D effects, no gradients, no shadows. Pure flat design.
- **2px strokes.** Consistent stroke width across all icons for visual harmony.
- **Programmatic recoloring.** Symbolic icons are recolored by the system to match the current theme (light or dark). They must be authored as a single color that the system replaces. Do not embed specific colors.
- **Pixel-aligned.** Strokes and shapes should align to the pixel grid to avoid blurring at small sizes.

### Icon vs. Label Usage

| Context | Use Icon Only | Use Label Only | Use Both |
|---|---|---|---|
| Header bar buttons | Yes | No | No |
| Toolbar actions | Yes | No | No |
| Menu items | No | Yes | No |
| Sidebar navigation | No | No | Yes (icon + label) |
| View switchers | No | No | Yes (icon + label) |
| Primary action buttons | No | Yes | No |

The general rule: use an icon **or** a label, not both. The exceptions are sidebars and view switchers, where the combination of icon and label aids scanning and recognition in a list context.

### Standard Symbolic Icons

Before creating custom symbolic icons, check the [GNOME icon library](https://gitlab.gnome.org/World/design/icon-library). Many common actions already have standard icons:

| Action | Icon Name |
|---|---|
| Create / Add | `list-add-symbolic` |
| Delete / Remove | `list-remove-symbolic` |
| Edit | `document-edit-symbolic` |
| Settings / Preferences | `preferences-system-symbolic` or `emblem-system-symbolic` |
| Search | `system-search-symbolic` |
| Menu | `open-menu-symbolic` |
| Back / Previous | `go-previous-symbolic` |
| Forward / Next | `go-next-symbolic` |
| Refresh | `view-refresh-symbolic` |
| Copy | `edit-copy-symbolic` |
| Paste | `edit-paste-symbolic` |

---

## Desktop Files

The `.desktop` file registers the application with the desktop environment. It controls how the app appears in the app grid, Activities search, and application menus.

### App ID Format

The app ID uses reverse domain name notation:

```
org.example.AppName
```

| Segment | Convention |
|---|---|
| TLD | Lowercase (`org`, `com`, `io`) |
| Domain | Lowercase, matching the project's actual domain |
| App name | PascalCase, matching the app's display name |

Examples:
- `org.gnome.TextEditor`
- `com.example.Transcriber`
- `io.github.username.MyApp`

The app ID is used for:
- The `.desktop` file name (`com.example.Transcriber.desktop`)
- The GApplication ID
- The D-Bus name
- The GSettings schema path
- The Flatpak bundle ID

All of these **must match exactly**.

### Required Desktop File Fields

| Field | Purpose | Example |
|---|---|---|
| `Name` | Display name shown to users | `Transcriber` |
| `GenericName` | Generic description of the app type | `Audio Transcription` |
| `Comment` | One-line description for tooltips | `Transcribe audio to text` |
| `Exec` | Command to launch the app | `transcriber %U` |
| `Icon` | App icon name (matches app ID) | `com.example.Transcriber` |
| `Type` | Always `Application` | `Application` |
| `Categories` | Freedesktop.org category list | `Audio;Utility;` |
| `MimeType` | Supported file types (if applicable) | `audio/wav;audio/mpeg;` |
| `Keywords` | Search keywords (semicolon-separated) | `audio;speech;voice;transcription;` |
| `StartupNotify` | Whether to show a launching indicator | `true` |

### Desktop File Example

```ini
[Desktop Entry]
Name=Transcriber
GenericName=Audio Transcription
Comment=Transcribe audio to text
Exec=transcriber %U
Icon=com.example.Transcriber
Type=Application
Categories=Audio;Utility;
MimeType=audio/wav;audio/mpeg;audio/ogg;
Keywords=audio;speech;voice;transcription;STT;
StartupNotify=true
```

### Categories

Use the [Freedesktop.org menu specification categories](https://specifications.freedesktop.org/menu-spec/latest/apa.html). Common categories:

| Category | Use For |
|---|---|
| `Audio` | Audio playback, recording, editing |
| `Video` | Video playback, recording, editing |
| `Graphics` | Image editing, drawing, photography |
| `Office` | Documents, spreadsheets, presentations |
| `Utility` | Tools, calculators, file managers |
| `Development` | IDEs, debuggers, version control |
| `Network` | Browsers, email, chat |
| `System` | System tools, monitors, configuration |

An app may belong to multiple categories. List the most specific category first.

---

## Further Reading

- [Design Principles](01-design-principles.md) - Why these identity guidelines exist
- [Layout & Navigation](03-layout-and-navigation.md) - Where icons and labels appear in the UI
- [Typography & Labels](05-typography-and-labels.md) - Text formatting for app names and labels
- [Theming & Color](11-theming-and-color.md) - Color usage in icons and UI
