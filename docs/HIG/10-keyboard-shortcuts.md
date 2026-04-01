# Keyboard Shortcuts

Keyboard shortcuts provide fast access to actions for experienced users and are essential for users who cannot use a pointer device. GNOME applications should support both standard platform shortcuts and application-specific shortcuts.

---

## Standard GNOME Keyboard Shortcuts

These shortcuts are expected by users across all GNOME applications. Do not reassign them to different actions.

### Window Management

| Shortcut | Action |
|---|---|
| Ctrl+W | Close window or tab |
| Ctrl+Q | Quit application |
| F1 | Open help |
| F9 | Toggle utility panes |
| F10 | Open primary menu |
| F11 | Toggle fullscreen |
| Ctrl+? | Open keyboard shortcuts dialog |
| Ctrl+, | Open preferences |

### File Operations

| Shortcut | Action |
|---|---|
| Ctrl+N | New document or window |
| Ctrl+O | Open file |
| Ctrl+S | Save |
| Shift+Ctrl+S | Save as |
| Ctrl+P | Print |
| Shift+Ctrl+P | Print preview |

### Editing

| Shortcut | Action |
|---|---|
| Ctrl+Z | Undo |
| Shift+Ctrl+Z | Redo |
| Ctrl+X | Cut |
| Ctrl+C | Copy |
| Ctrl+V | Paste |
| Ctrl+A | Select all |

### Find and Replace

| Shortcut | Action |
|---|---|
| Ctrl+F | Find |
| Ctrl+G | Find next |
| Shift+Ctrl+G | Find previous |
| Ctrl+H | Find and replace |

### View

| Shortcut | Action |
|---|---|
| Ctrl++ | Zoom in |
| Ctrl+- | Zoom out |
| Ctrl+0 | Reset zoom to 100% |
| Ctrl+R | Reload |

### Text Formatting

| Shortcut | Action |
|---|---|
| Ctrl+B | Bold |
| Ctrl+I | Italic |
| Ctrl+U | Underline |

### Navigation

| Shortcut | Action |
|---|---|
| Alt+Left | Go back |
| Alt+Right | Go forward |
| Alt+Home | Go to home / start |
| Ctrl+D | Add bookmark |

---

## Shortcut Key Design Conventions

When defining application-specific shortcuts, follow these conventions to maintain consistency and discoverability.

### Modifier Key Usage

| Modifier | Usage | Notes |
|---|---|---|
| Ctrl | Primary modifier for shortcuts | Most shortcuts should use Ctrl |
| Shift+Ctrl | Reverse or extended variant | e.g., Ctrl+Z (undo) / Shift+Ctrl+Z (redo) |
| Alt | **Avoid** | Conflicts with access keys (mnemonics) |
| Super | **Never use** | Reserved for system and desktop shell |

### Design Principles

- **Make shortcuts mnemonic.** Ctrl+S for Save, Ctrl+F for Find, Ctrl+P for Print. Users remember these because the letter corresponds to the action.
- **Use Shift to reverse or extend.** If Ctrl+G finds the next match, Shift+Ctrl+G finds the previous. If Ctrl+Z undoes, Shift+Ctrl+Z redoes.
- **Design for one-handed operation** when possible. Ctrl+S is easy to press with the left hand alone. Ctrl+Shift+Alt+K is not.
- **Avoid multi-modifier combinations.** Two modifiers (Shift+Ctrl) is acceptable. Three modifiers is excessive and error-prone.
- **Do not shadow standard shortcuts.** If the standard table above defines a shortcut, your application must not reassign it.

---

## Access Keys (Mnemonics)

Access keys allow users to activate controls by pressing Alt plus an underlined letter in the control's label. They are distinct from keyboard shortcuts.

### How They Work

- In menus and dialogs, one letter of each label is underlined (e.g., **S**ave, **O**pen).
- Pressing Alt+S activates the "Save" button; pressing Alt+O activates "Open."
- Access keys are shown when the user presses and holds Alt.

### Choosing Access Keys

| Guideline | Rationale |
|---|---|
| Use the first letter of the label when available | Most memorable and discoverable |
| Avoid thin letters: i, l, I | Underline is difficult to see |
| Avoid letters with descenders: g, j, p, q, y | Underline may collide with the descender |
| Ensure uniqueness within the same scope | Two controls in the same dialog must not share an access key |
| Account for translations | A mnemonic that works in English may conflict or become nonsensical in another language. Translators should be empowered to reassign mnemonics per locale. |

---

## Keyboard Navigation

Every element in the user interface must be reachable and operable via the keyboard.

### Navigation Keys

| Key | Action |
|---|---|
| Tab | Move focus to the next control |
| Shift+Tab | Move focus to the previous control |
| Arrow keys | Move within a control group (list items, radio buttons, tabs) |
| Enter / Space | Activate the focused element |
| Escape | Cancel current operation, close popover or dialog |
| F10 | Open the primary menu |
| Menu key / Shift+F10 | Open context menu for focused element |
| Home / End | Move to first / last item in a list or text field |
| Page Up / Page Down | Scroll by one page in a scrollable area |

### Navigation Order

- **Tab order should follow the visual layout:** left to right, top to bottom in LTR locales.
- **Group related controls** so that arrow keys navigate within the group while Tab moves between groups.
- **Skip decorative or non-interactive elements.** Only focusable elements should appear in the tab order.

---

## GtkShortcutsWindow

Every GNOME application should provide a keyboard shortcuts reference accessible via Ctrl+?.

### Requirements

- **List all keyboard shortcuts** supported by the application, both standard and application-specific.
- **Group shortcuts by category** (e.g., General, Editing, Navigation, View).
- **Use `GtkShortcutsWindow`** (or `GtkShortcutsOverlay` in GTK 4.14+) for consistent presentation.
- Include a brief description for each shortcut.
- Keep descriptions concise -- one short phrase per shortcut.

### Example Structure

```
General
  Ctrl+?        Keyboard shortcuts
  Ctrl+,        Preferences
  Ctrl+Q        Quit

Editing
  Ctrl+Z        Undo
  Shift+Ctrl+Z  Redo
  Ctrl+X        Cut
  ...
```

---

## GSettings Accelerator Format

When defining shortcuts programmatically or in GSettings schemas, use the GTK accelerator string format.

### Format

Accelerator strings consist of modifier tags followed by the key name:

```
<Control>s
<Shift><Control>z
<Alt>Left
<Super>apostrophe
```

### Modifier Tags

| Tag | Modifier Key |
|---|---|
| `<Control>` | Ctrl |
| `<Shift>` | Shift |
| `<Alt>` | Alt |
| `<Super>` | Super (logo key) |

### Notes

- Modifier tags are concatenated directly: `<Shift><Control>z`, not `<Shift>+<Control>+z`.
- Key names follow GDK key symbol names (e.g., `space`, `Return`, `apostrophe`, `Left`, `F1`).
- Use `null` or an empty string to indicate no shortcut.
- Multiple accelerators for the same action can be specified as a string list in GSettings.

---

## Reserved System Shortcuts

The following shortcuts are handled by the GNOME desktop shell and must never be used by applications. Attempting to capture them will either fail silently or cause confusing behavior.

| Shortcut | System Function |
|---|---|
| Super (alone) | Activities overview |
| Super+L | Lock screen |
| Super+A | Show all applications |
| Alt+Tab | Switch between windows |
| Alt+F2 | Run dialog |
| Ctrl+Alt+Delete | Log out / power options |
| Print Screen | Screenshot |
| Alt+F4 | Close window (fallback) |
| Super+Arrow keys | Window tiling |
| Super+H | Hide (minimize) window |
| Super+D | Show desktop |

> **Important:** Even if your application could technically intercept some of these shortcuts, doing so violates user expectations and breaks platform consistency. Always defer to the system for these bindings.

---

## Cross-References

- [Interaction](09-interaction.md) -- pointer, touch, and gesture interaction patterns
- [Accessibility](11-accessibility.md) -- keyboard accessibility requirements and focus management
- [Platform Integration](13-platform-integration.md) -- integrating with GNOME platform conventions
