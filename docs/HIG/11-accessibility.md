# Accessibility

> "Good design and accessibility are mutually reinforcing."

Accessibility is not a feature to be added later -- it is a fundamental quality of well-designed software. An application that is accessible is better for everyone: users with disabilities, users in challenging environments, users with temporary impairments, and users who simply prefer alternative input methods.

---

## Accessibility Testing Checklist

Before releasing any application or significant update, verify accessibility across all of these dimensions.

| Test | What to Verify |
|---|---|
| High-contrast mode | All UI elements render properly; text is readable; icons are distinguishable |
| Large text mode | Labels remain readable; layouts do not break, overlap, or truncate |
| Keyboard navigation | Every feature is accessible without a mouse or touchscreen |
| Screen reader (Orca) | UI elements are read accurately with correct names, roles, and states |
| On-screen keyboard | All text input fields accept input reliably; keyboard appears when expected |
| Reduced motion | Animations are suppressed or minimized; no functionality is lost |
| Right-to-left layouts | Mirrored layout is correct for RTL languages (if applicable) |

---

## Accessible Names and Descriptions

Every interactive element must have a name that assistive technologies can present to the user. Many elements derive their accessible name automatically from their visible label, but icon-only buttons, images, and custom widgets require explicit annotation.

### Properties

| Property | Purpose | When to Use |
|---|---|---|
| `accessible-name` | Short, descriptive name read by screen readers | Icon-only buttons, images, custom widgets |
| `accessible-description` | Additional context beyond the name | When the name alone is ambiguous |
| `accessible-role` | Semantic role of the element | Custom widgets that do not inherit a standard role |

### Blueprint Syntax

```blueprint
Button {
  icon-name: "edit-copy-symbolic";

  accessibility {
    label: _("Copy to clipboard");
  }
}

Image {
  file: "chart.png";

  accessibility {
    label: _("Monthly revenue chart showing 15% growth");
  }
}
```

### Guidelines

- **Names should be concise yet informative.** "Copy" is better than "Button" but worse than "Copy to clipboard" when there are multiple copy actions.
- **Customize GTK defaults** when the automatic name is generic or unclear. A `GtkButton` with only an icon has no useful default name.
- **Keep descriptions supplementary.** The name should be sufficient for basic understanding. The description provides additional detail when needed.
- **Dynamic content changes should be announced.** Use live regions (ATK live region properties) to notify screen readers of content updates that occur without user action (e.g., a new message arriving, a progress bar completing).
- **Translatable strings.** Always mark accessible names and descriptions for translation using `_()` or your project's translation function.

---

## Keyboard Accessibility

Every action that can be performed with a pointer must also be possible with the keyboard alone. This is a hard requirement, not a goal.

### Requirements

| Requirement | Details |
|---|---|
| Full keyboard operability | Every interactive element can be reached and activated via keyboard |
| Logical tab order | Tab order follows the visual layout (left-to-right, top-to-bottom in LTR locales) |
| Visible focus at all times | The currently focused element always has a visible focus indicator |
| No keyboard traps | The user can always navigate away from any element using Tab, Escape, or arrow keys |
| Shortcuts for frequent actions | Provide keyboard shortcuts for actions that users perform often |

### Focus Visibility

- Use the default Adwaita focus ring. Do not replace it with a custom style unless the custom style provides equal or better visibility.
- Focus indicators must have a minimum contrast ratio of **3:1** against the surrounding background.
- Never suppress focus styles (e.g., `outline: none`) for aesthetic reasons.

See [Keyboard Shortcuts](10-keyboard-shortcuts.md) for the full shortcut reference and design conventions.

---

## Visual Accessibility

### Color

- **Never use color as the only means to distinguish information.** Always supplement color with text labels, icons, patterns, or positional cues. For example, an error state should not be indicated solely by turning text red -- add an error icon and descriptive text.
- Use the system color palette, which is designed with accessibility in mind. See [Color Palette](07-color-palette.md).

### Contrast Ratios

Maintain minimum contrast ratios as defined by WCAG 2.1 Level AA.

| Element | Minimum Contrast Ratio |
|---|---|
| Normal text (below 18pt / 14pt bold) | 4.5:1 |
| Large text (18pt+ / 14pt+ bold) | 3:1 |
| UI components and graphical objects | 3:1 |
| Focus indicators | 3:1 |
| Disabled elements | No minimum (but should be visually distinguishable from enabled) |

### Flashing and Blinking

- **No flashing or blinking elements.** Content that flashes more than three times per second can trigger seizures in people with photosensitive epilepsy.
- Avoid rapid visual transitions, strobing effects, or high-contrast alternating patterns.

### Text Scaling

- **Respect the system text scaling setting.** GNOME allows users to increase text size globally. Applications must respond to this setting without breaking layouts.
- Test your application at the maximum system text scale (typically 200%).
- Use relative sizing (em, rem) rather than fixed pixel sizes for text where possible.

### High-Contrast Theme

- Test with the GNOME high-contrast theme enabled (System Settings > Accessibility > High Contrast).
- Ensure all icons, borders, separators, and interactive elements remain visible and distinguishable.
- Symbolic icons adapt to high-contrast mode automatically when using the standard icon system.

---

## Touch Target Size

- **Minimum interactive target size: 44 x 44 px.** This applies to all buttons, list rows, checkboxes, switches, and other interactive elements.
- Maintain **adequate spacing** between adjacent targets. Targets that are technically 44 px but flush against each other still cause mis-taps.
- If a visual element is smaller than 44 px (e.g., a small icon button), extend the invisible **hit area** to meet the minimum.
- This requirement benefits pointer users as well -- smaller targets are harder to click for users with motor impairments.

---

## Screen Reader Compatibility

GNOME's screen reader is **Orca**. All applications should work correctly with Orca out of the box.

### Requirements

| Requirement | Details |
|---|---|
| Accessible names for all elements | Every interactive and informative element has a meaningful name |
| Alt text for all images | Decorative images may use an empty alt text to be skipped |
| Semantic widget roles | Use standard GTK widgets; custom widgets must declare proper roles |
| Logical grouping | Related elements are grouped with appropriate containers (e.g., `GtkBox`, `GtkListBox`) |
| State change announcements | Dynamic content updates are announced via live regions |
| Correct reading order | The accessibility tree order matches the visual reading order |

### Testing with Orca

1. Enable Orca: Settings > Accessibility > Screen Reader, or run `orca` from a terminal.
2. Navigate your application using only the keyboard.
3. Verify that Orca reads each element's **name**, **role**, and **state** correctly.
4. Verify that dynamic content changes (notifications, progress updates, list additions) are announced.
5. Verify that decorative elements are not read aloud.

---

## Focus Management

Proper focus management ensures that keyboard and screen reader users always know where they are in the interface and are never disoriented by unexpected focus changes.

### Guidelines

| Scenario | Correct Focus Behavior |
|---|---|
| Application launch | Focus on the most relevant interactive element (e.g., main content area, primary input) |
| Dialog opens | Focus on the first interactive element in the dialog (or the dialog title for informational dialogs) |
| Dialog closes | Return focus to the element that triggered the dialog |
| Navigation (page change) | Move focus to the primary content of the new page |
| Item deletion | Move focus to the next item, or the previous item if the last item was deleted |
| Inline content update | Do not move focus; announce the change via a live region if important |
| Error occurs | Move focus to the error message or the offending input field |

### Anti-Patterns

- **Never move focus unexpectedly.** If the user is typing in a text field and a notification arrives, focus must remain in the text field.
- **Never trap focus.** The user must always be able to navigate away from any element.
- **Never remove the focused element** without moving focus to a logical successor.

---

## Motion and Animation

### Respecting User Preferences

- **Check the `prefers-reduced-motion` setting.** When the user has enabled reduced motion in system accessibility settings, suppress or minimize animations.
- In GTK, use `gtk_settings_get_enable_animations()` or the `GtkSettings:gtk-enable-animations` property.
- libadwaita respects this setting automatically for its built-in transitions.

### Guidelines

| Guideline | Details |
|---|---|
| Provide alternatives | If an interaction relies on motion (e.g., swipe-to-dismiss), provide a non-motion alternative (e.g., a delete button) |
| No auto-playing animations | Moving, blinking, or auto-scrolling content should not start automatically |
| Keep transitions brief | Transitions should be under 300ms; prefer 150-200ms |
| Purpose-driven motion | Every animation should serve a functional purpose (orientation, feedback, continuity) |
| Avoid parallax and complex motion | These can cause discomfort for users with vestibular disorders |

---

## Testing Tools

| Tool | Purpose | How to Use |
|---|---|---|
| **Orca** | GNOME screen reader | Enable in Settings > Accessibility, or run `orca` |
| **Accerciser** | AT-SPI accessibility tree browser and tester | Install from your distribution's packages; inspect the accessibility tree of running applications |
| **GTK Inspector** | Inspect accessible properties of widgets | Ctrl+Shift+D in a running GTK application (debug builds) |
| **High-contrast theme** | Verify visual accessibility | Settings > Accessibility > High Contrast |
| **Large text** | Verify text scaling | Settings > Accessibility > Large Text |
| **Contrast checker** | Verify color contrast ratios | Use any WCAG contrast ratio tool (e.g., WebAIM Contrast Checker) with sampled colors |

---

## Summary of Key Numbers

| Metric | Value |
|---|---|
| Minimum touch target size | 44 x 44 px |
| Normal text contrast ratio | 4.5:1 (WCAG AA) |
| Large text contrast ratio | 3:1 (WCAG AA) |
| UI component contrast ratio | 3:1 (WCAG AA) |
| Maximum flash frequency | 3 per second |
| Recommended transition duration | 150-200 ms |
| Maximum transition duration | 300 ms |

---

## Cross-References

- [Visual Design](06-visual-design.md) -- spacing, typography, and visual consistency
- [Color Palette](07-color-palette.md) -- system colors and their accessibility characteristics
- [Interaction](09-interaction.md) -- pointer, touch, and gesture interaction patterns
- [Keyboard Shortcuts](10-keyboard-shortcuts.md) -- keyboard shortcut reference and navigation conventions
