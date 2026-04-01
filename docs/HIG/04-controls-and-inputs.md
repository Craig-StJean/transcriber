# Controls and Inputs

This document covers all interactive controls available in GTK 4 and libadwaita, along with guidelines for choosing the right widget for each use case.

---

## Buttons

Buttons are the primary means of initiating actions. Their visual styling communicates the nature and importance of the action they perform.

### Standard Buttons

Standard buttons use the default Adwaita styling and are appropriate for most actions. They have a subtle background that distinguishes them from flat buttons while remaining visually unobtrusive.

### Suggested Buttons

Apply the `.suggested-action` CSS class to highlight the primary or recommended action in a dialog or view. These buttons use the accent color to draw attention.

- Use exactly **one** suggested button per context (dialog, toolbar group, etc.).
- The suggested action should be the safe, expected choice.

### Destructive Buttons

Apply the `.destructive-action` CSS class to actions that are irreversible or cause data loss (delete, remove, discard). These buttons are styled in red to signal danger.

- Always pair with a confirmation dialog for significant destructive actions.
- Label destructive buttons with specific verbs ("Delete Recording", not "OK").

### Flat Buttons

Apply the `.flat` CSS class for buttons embedded in toolbars, header bars, or inline with content. Flat buttons have no visible background until hovered or pressed.

- Use in header bars and toolbars where a raised button would look heavy.
- Appropriate for secondary or supplementary actions alongside content.

### Toggle Buttons

Toggle buttons maintain a pressed/active state and are suitable for binary options that apply immediately (e.g., bold text, mute audio). Use `GtkToggleButton` and bind its `active` property.

- The pressed state must be visually distinct.
- Provide a tooltip describing what the toggle controls.

### Linked Button Groups

Group related buttons together using the `.linked` CSS class on their parent `GtkBox`. Linked buttons share borders, creating a unified control.

- Suitable for mutually exclusive options (e.g., view mode: list/grid) or related sequential actions (zoom in/out).
- Limit linked groups to 2-4 buttons.

### Icon-Only Buttons

Buttons that display only an icon (no label) **must** have a tooltip set via `set_tooltip_text()`. Without a tooltip, the button's purpose is inaccessible to users who rely on screen readers or are unfamiliar with the icon.

- Prefer symbolic icons from the GNOME icon set.
- If an action is ambiguous as an icon alone, add a label instead.

---

## Text Input

### Preference-Style Rows

Use `AdwEntryRow` and `AdwPasswordEntryRow` within `AdwPreferencesGroup` containers for settings and form fields. These provide a consistent look with a title label integrated into the row.

```xml
<object class="AdwEntryRow">
  <property name="title">Server Address</property>
</object>
```

- `AdwEntryRow` is the standard text input row.
- `AdwPasswordEntryRow` adds a visibility toggle for password fields.
- Add suffix widgets (buttons, icons) for inline actions like "clear" or "reveal".

### Standalone Text Fields

Use `GtkEntry` for single-line standalone text fields outside of preference groups (e.g., search bars, quick input areas).

Use `GtkTextView` for multi-line text editing. Wrap it in a `GtkScrolledWindow` and apply the `.card` style class for a bordered appearance.

### Placeholder Text

- Use placeholder text to show the expected format or an example value ("https://example.com").
- Do **not** use placeholder text as a replacement for a label -- placeholders disappear once the user starts typing.
- Keep placeholder text short and descriptive.

### Validation and Error States

- Apply the `.error` CSS class to the entry widget to indicate invalid input.
- Display an error message below the field or as an inline label describing what is wrong.
- Validate on focus-out or on submission, not on every keystroke (unless providing real-time search).
- Provide clear guidance on how to fix the error ("Enter a valid URL starting with https://").

---

## Selection Controls

### AdwSwitchRow

A row containing a label and a toggle switch. Use for binary on/off settings that take effect immediately.

- Place within `AdwPreferencesGroup`.
- The title should describe the enabled state ("Show Notifications", not "Notification Toggle").
- Add a subtitle for additional context when needed.

### AdwComboRow

A dropdown row for selecting one option from a list. Use when there are 3 or more mutually exclusive options.

- Provide a clear title describing what is being selected.
- List options in a logical order (alphabetical, frequency of use, or severity).
- For two options only, consider using an `AdwSwitchRow` or linked toggle buttons instead.

### Checkboxes (GtkCheckButton)

Use checkboxes for selecting zero or more options from a set, or for a single standalone boolean option.

- Each checkbox operates independently.
- Label each checkbox clearly.
- Group related checkboxes visually using spacing or a preferences group.

### Radio Buttons (GtkCheckButton in group)

Use radio buttons for selecting exactly one option from a small set (2-5 options). Implemented with `GtkCheckButton` by setting the `group` property.

- One option must always be selected.
- If there are more than 5 options, use `AdwComboRow` instead.
- Order options logically (most common first, or in sequence).

### AdwSpinRow

A numeric input row with increment and decrement buttons. Use for bounded numeric values where the user may want fine-grained control.

- Set sensible `lower`, `upper`, and `step_increment` values via the `GtkAdjustment`.
- The title should describe the value being configured ("Timeout (seconds)").
- Use for values where typing a number is common; for continuous ranges, prefer a slider.

---

## Menus

### Primary Menu (Hamburger Menu)

The primary menu is accessed via the hamburger icon (three horizontal lines) in the header bar. It contains app-wide actions and navigation.

- Place infrequently used actions here (Preferences, Keyboard Shortcuts, About).
- Use `GtkMenuButton` with a `GMenu` model.
- Keep the menu short; group items with separators.
- Include "Keyboard Shortcuts" and "About {App Name}" as the last two items.

### Context Menus

Context menus appear on right-click (or long-press on touch) and provide actions relevant to the item under the pointer.

- Only include actions that apply to the specific item or selection.
- Mirror relevant actions from toolbars or menus for discoverability.
- Use `GtkPopoverMenu` with a `GMenu` model attached via `GtkGestureClick`.

### Popovers

Popovers (`GtkPopover`, `GtkPopoverMenu`) are floating panels anchored to a button or widget. Use for inline options, filters, or small forms that don't warrant a full dialog.

- Dismiss on click-outside or Escape.
- Keep content minimal; move complex forms to a separate dialog or page.
- Anchor to the control that triggered them.

---

## Sliders

Use `GtkScale` for selecting a value from a continuous range (volume, brightness, opacity).

### Continuous Sliders

- Suitable when the exact numeric value is less important than the relative position.
- Provide a label or indicator showing the current value if precision matters.

### Sliders with Scale Marks

Use `gtk_scale_add_mark()` to add reference points along the slider. Marks help users target specific values (e.g., 25%, 50%, 75%, 100%).

- Add marks at meaningful intervals, not arbitrary ones.
- Too many marks create visual clutter; limit to 5-7 marks.

### General Slider Guidelines

- Always set an accessible label for the slider.
- Consider adding a numeric entry alongside the slider for precise input.
- Horizontal orientation is standard; use vertical only when it maps to a spatial concept (e.g., a mixer fader).

---

## Overlaid Controls

Overlaid controls appear on top of content when the user hovers over an area. Common uses include media playback controls and image action buttons.

- Use `GtkOverlay` to position controls over content.
- Reveal controls on hover or focus; hide on mouse-out after a delay.
- Ensure controls are accessible via keyboard -- do not rely solely on hover.
- Apply a semi-transparent background behind controls to ensure legibility over varied content.
- Keep overlaid controls simple: a few icon buttons at most.

---

## Which Control to Use?

The following decision table maps common use cases to the recommended widget.

| Use Case | Widget | Notes |
|----------|--------|-------|
| Trigger an action | `GtkButton` | Use `.suggested-action` for primary, `.destructive-action` for dangerous |
| Toggle a setting on/off | `AdwSwitchRow` | In preferences; immediate effect |
| Toggle a mode in a toolbar | `GtkToggleButton` | E.g., bold, mute, show/hide |
| Choose one from 2 options | `AdwSwitchRow` or linked `GtkToggleButton` | Switch for settings, toggles for modes |
| Choose one from 3-5 options | Radio buttons (`GtkCheckButton` with group) | Visible options, no dropdown needed |
| Choose one from 6+ options | `AdwComboRow` | Dropdown to save space |
| Choose multiple from a set | `GtkCheckButton` (checkboxes) | Independent toggles |
| Enter short text | `AdwEntryRow` or `GtkEntry` | Row for preferences, standalone elsewhere |
| Enter a password | `AdwPasswordEntryRow` | Includes visibility toggle |
| Enter long text | `GtkTextView` | Wrap in `GtkScrolledWindow` |
| Select a numeric value (bounded) | `AdwSpinRow` | With increment/decrement buttons |
| Select a value from a range | `GtkScale` | Continuous slider |
| Select a file | `GtkFileDialog` | Native file chooser |
| Show a list of actions | `GtkPopoverMenu` with `GMenu` | For context or overflow menus |
| Inline options panel | `GtkPopover` | Anchored floating panel |
| App-wide actions | Primary menu (hamburger) | In the header bar |

---

## Cross-References

- [Layout and Navigation](03-layout-and-navigation.md) -- how to arrange controls within views and dialogs.
- [Feedback and Messaging](05-feedback-and-messaging.md) -- how to communicate the results of user actions.
