# Interaction

GNOME applications must support pointer, touch, and keyboard interaction equally. Design input-agnostically: the interface should work well regardless of how the user interacts with it.

---

## Core Principles

- **Design input-device agnostically.** Do not assume a mouse, trackpad, or touchscreen. All three must work.
- **Never reference specific devices in UI text.** Say "select" or "activate," not "click" or "tap."
- **Hover must not reveal essential actions or information.** Touchscreens have no hover state. Any content or action gated behind hover is invisible to touch users.
- **Every pointer action needs a keyboard alternative.** See [Keyboard Shortcuts](10-keyboard-shortcuts.md) for the full reference.
- **Pressing Esc cancels pointer operations.** Drags, selections, and other in-progress pointer actions must be cancellable with the Escape key.

---

## Primary Actions (Left Click / Tap)

The primary action is the default interaction for activating elements. It corresponds to a left mouse click, a single tap on a touchscreen, or pressing Enter/Space on a focused element.

| Action | Examples |
|---|---|
| Activate a control | Press a button, toggle a switch, check a checkbox |
| Follow a link | Open a URL or navigate to a linked view |
| Open an item | Launch a file, expand a row, open a document |
| Select an item | Choose an item in a list or grid |
| Place the text cursor | Position the insertion point in an editable text field |

---

## Secondary Actions (Right Click / Long Press)

The secondary action opens a **context menu** presenting additional actions relevant to the target element.

| Input Method | Trigger |
|---|---|
| Mouse | Right click |
| Touchscreen | Long press (press and hold) |
| Keyboard | Menu key, or Shift+F10 |

### Guidelines

- Context menus provide **supplementary** actions. The most important actions should always be directly accessible in the UI.
- Context menus **should not be the sole path to destructive actions.** If a destructive action is available in a context menu, it must also be accessible through a more deliberate path (toolbar, menu bar, etc.).
- Keep context menus concise. Group related items with separators.

---

## Scrolling

Scrolling moves content along a single axis (usually vertical).

| Device | Method |
|---|---|
| Mouse | Scroll wheel (vertical) |
| Touchpad | Two-finger drag |
| Touchscreen | One-finger drag |

### Guidelines

- Scrolling should feel smooth and responsive on all devices.
- Support kinetic (momentum) scrolling on touch and touchpad inputs.
- Provide visible scrollbars that appear on interaction and fade when idle.
- Horizontal scrolling should only be used when the content model requires it (e.g., a timeline, a code editor with long lines).

---

## Panning (Multi-Axis)

Panning moves content freely across both axes, typically in canvas or map-style views.

| Device | Method |
|---|---|
| Mouse | Click and drag (middle button or designated modifier) |
| Touchscreen | One-finger drag |

### Guidelines

- Reserve panning for views where multi-axis movement is natural (maps, canvases, large diagrams).
- Provide visual indicators of the current viewport position (minimap, scrollbar crosshairs).

---

## Zooming

Zooming changes the scale of content within a view.

| Device | Method |
|---|---|
| Mouse | Ctrl + scroll wheel |
| Touchpad | Pinch gesture |
| Touchscreen | Pinch gesture |

### Guidelines

- Zoom toward the pointer or pinch center, not the viewport center.
- Provide discrete zoom controls (buttons or menu items) as keyboard-accessible alternatives.
- Display the current zoom level when it is not 100%.
- Respect sensible minimum and maximum zoom bounds.

---

## Drag and Drop

Drag and drop allows direct manipulation of objects by moving them from one location to another.

### Interaction Flow

1. **Initiate:** Press and hold on the source item, then begin moving.
2. **Drag:** The item (or a visual proxy) follows the pointer. Provide continuous visual feedback.
3. **Hover over targets:** Valid drop targets highlight to indicate they will accept the item.
4. **Drop:** Release to complete the operation.
5. **Cancel:** Press Esc at any point to abort the drag and return the item to its origin.

### Guidelines

- Provide clear visual feedback throughout the entire drag operation (cursor change, drag icon, drop target highlighting).
- Show invalid drop targets as distinct from valid ones (e.g., no highlight, or a "not allowed" cursor).
- Support drag and drop between applications where semantically meaningful (e.g., dragging files).
- On touchscreens, use a longer press-and-hold threshold to distinguish drag initiation from scrolling.

---

## Touch Gesture Restrictions

Several gestures are reserved by the GNOME system shell and must not be used by applications.

### Reserved Gestures (Do Not Use)

| Gesture | System Function |
|---|---|
| Three-finger gestures | Workspace switching |
| Four-finger gestures | System-level actions |
| Drag from top screen edge | System notifications / quick settings |
| Drag from bottom screen edge | Application overview |

### Available for Applications

| Gesture | Typical Use |
|---|---|
| Two-finger gestures | Pinch to zoom, two-finger rotate, two-finger swipe |
| Drag from left screen edge | Application-defined (e.g., open sidebar) |
| Drag from right screen edge | Application-defined (e.g., open panel) |

---

## Click and Touch Target Size

Interactive elements must be large enough for comfortable and accurate activation.

- **Minimum target size: 44 x 44 px.** This applies to buttons, list rows, toggle controls, and all other interactive areas.
- Larger targets improve usability for **all** users, not just touch users. Motor impairments, shaky environments (e.g., public transit), and even simple convenience all benefit from generous target sizes.
- If visual design requires a smaller element (e.g., a small icon button), extend the **hit area** beyond the visible bounds to meet the 44 px minimum.
- Maintain adequate spacing between adjacent targets to prevent accidental activation of the wrong element.

---

## Hover States

Hover states provide visual feedback when the pointer is over an interactive element.

### Guidelines

- **Never hide essential functionality behind hover.** Hover is unavailable on touchscreens and is unreliable for keyboard users. Any action or information revealed on hover must also be accessible through other means.
- Hover effects should be **subtle**: a slight background color change, an underline appearing on a link, or a tooltip after a brief delay.
- Use hover to communicate interactivity. If an element responds to clicks, a hover state confirms that to the user.
- Tooltips (shown on hover) are supplementary. They may provide additional context, but the UI must be understandable without them.

---

## Focus Indicators

Focus indicators show which element will receive keyboard input. They are essential for keyboard and assistive technology users.

### Guidelines

- **Every interactive element must show a visible focus indicator** when focused via keyboard navigation.
- Follow the system focus style. Adwaita provides default focus ring styling that is consistent across GNOME applications.
- **Never remove or hide focus indicators.** Removing `outline` or `box-shadow` for aesthetic reasons breaks keyboard accessibility.
- Focus indicators should have sufficient contrast against the background (minimum 3:1 ratio per WCAG guidelines).
- When focus moves programmatically (e.g., after closing a dialog), place it on the most logical element. See [Accessibility](11-accessibility.md) for focus management guidelines.

---

## Cross-References

- [Keyboard Shortcuts](10-keyboard-shortcuts.md) -- full keyboard shortcut reference and design conventions
- [Accessibility](11-accessibility.md) -- accessibility requirements including keyboard and screen reader support
- [Controls and Inputs](04-controls-and-inputs.md) -- individual control behavior and interaction patterns
