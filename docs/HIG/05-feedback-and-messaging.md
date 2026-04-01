# Feedback and Messaging

Every user action should produce a visible result. This document covers the patterns available for communicating status, results, errors, and ongoing conditions to the user.

**Core principle:** Prefer non-blocking, reversible feedback. Use toasts with undo instead of confirmation dialogs. Reserve modal dialogs for truly destructive or irreversible decisions.

---

## Toasts (AdwToast)

Toasts are brief, temporary messages that appear at the bottom of the window. They inform the user of a completed action or status change without interrupting their workflow.

### When to Use

- Confirming a completed action ("Recording saved")
- Providing an undo opportunity ("Transcription deleted" with "Undo" button)
- Reporting non-critical status updates ("Connected to server")
- Acknowledging a user action that has no other visible effect ("Copied to clipboard")

### Guidelines

- **Duration:** Auto-dismiss after 2-5 seconds. Use shorter durations for simple confirmations, longer for toasts with action buttons.
- **Action button:** Include an optional action button for undo or related follow-up. Only one action per toast.
- **Non-blocking:** Toasts must never block user interaction. The user can continue working while a toast is visible.
- **Stackable:** Multiple toasts queue automatically. Avoid triggering many toasts in rapid succession -- batch related updates.
- **Text:** Keep the message to a single short sentence. No title, just a body string.
- **No icons:** Toasts do not include icons; the text alone should convey meaning.

### Implementation

```python
toast = Adw.Toast(title="Recording saved")
toast.set_timeout(3)
# Optional action
toast.set_button_label("Undo")
toast.connect("button-clicked", on_undo_clicked)
toast_overlay.add_toast(toast)
```

The `AdwToastOverlay` must wrap the main content area of the window to position toasts correctly.

---

## Dialogs (AdwAlertDialog)

Dialogs are modal windows that require the user to make a decision before continuing. They interrupt workflow and should be used sparingly.

### When to Use

- Confirming a destructive action that cannot be undone ("Delete all recordings?")
- Requesting essential information before proceeding
- Presenting a critical error that requires acknowledgment

### When NOT to Use

- For confirmations of safe actions -- use a toast with undo instead.
- For information that could be displayed inline or as a banner.
- For preferences or settings -- use a dedicated preferences window.

### Structure

A dialog consists of:

1. **Title:** A concise heading stating the decision or question ("Delete Recording?")
2. **Body text:** A description of the consequences or additional context ("This will permanently remove the recording and its transcription. This action cannot be undone.")
3. **Buttons:** 2-3 response buttons. Place the primary action on the right; the cancel action on the left.

### Destructive Action Dialogs

When a dialog confirms a destructive action:

- Describe specifically what will happen in the body text.
- Use the `.destructive-action` style class on the confirmation button.
- Label the button with a specific verb ("Delete", "Discard"), not "OK" or "Yes".
- Make the cancel button the default (activated by Enter) so accidental confirmation is less likely.

### Guidelines

- **Limit buttons:** Use 2 buttons for simple yes/no decisions, 3 at most (e.g., "Save", "Discard", "Cancel").
- **Avoid stacking dialogs:** Never open a dialog from within another dialog.
- **Escape to cancel:** Pressing Escape should always dismiss the dialog without taking action.
- **No scrolling:** Dialog content should fit without scrolling. If it doesn't, the content belongs elsewhere.

---

## Banners (AdwBanner)

Banners are persistent, non-blocking strips displayed at the top of the content area. They communicate ongoing conditions that affect the user's experience.

### When to Use

- Offline mode or connectivity issues ("No internet connection")
- Unsaved changes warning
- App update available
- Trial or limited-functionality mode
- Background process status that persists ("Syncing in progress...")

### Guidelines

- **Persistent:** Banners remain visible until the condition resolves or the user dismisses them.
- **Non-blocking:** The user can continue working with the banner visible.
- **Action button:** Include a single action button when the user can resolve the condition ("Reconnect", "Save Now", "Update").
- **Placement:** Always at the top of the content area, below the header bar.
- **Minimal text:** One line describing the condition and, if applicable, what the user can do about it.
- **Revealed property:** Control visibility with the `revealed` property rather than adding/removing the widget.

---

## Progress Indicators

### Determinate Progress (GtkProgressBar)

Use a progress bar when the duration or percentage of completion is known.

- Show a progress bar for operations expected to take longer than 1 second.
- Update the fraction smoothly; avoid large jumps.
- Include descriptive text above or below the bar if the operation benefits from explanation ("Transcribing audio... 45%").
- If embedded in a list row or card, use a thin progress bar at the bottom of the item.

### Indeterminate Activity (GtkSpinner / AdwSpinner)

Use a spinner when the duration is unknown. `AdwSpinner` is the libadwaita variant with improved styling.

- Place spinners inline with the content they relate to, or centered in the area that will display results.
- Replace the spinner with content as soon as results are available.
- For whole-page loading, center the spinner on the page.

### General Progress Guidelines

- **Show progress for operations > 1 second.** Anything shorter should appear instant.
- **Provide a cancel option** for operations that take more than a few seconds. Use a "Cancel" button or the Escape key.
- **Avoid progress dialogs.** Show progress inline within the window rather than opening a modal progress dialog.
- **Label the operation** so the user knows what is happening, especially if multiple operations could be in progress.

---

## Status Pages (AdwStatusPage)

Status pages fill an entire view or content area with a centered message. They communicate significant states to the user.

### Empty States

Shown when there is no content to display:

- **Icon:** A relevant symbolic icon (e.g., `document-symbolic` for an empty document list).
- **Title:** A descriptive statement ("No Transcriptions Yet").
- **Description:** Guidance on how to get started ("Record or import audio to begin transcribing.").
- **Action button (optional):** A suggested action ("Start Recording").

Use a welcoming, instructive tone. Empty states are an opportunity to guide new users.

### Error States

Shown when something has gone wrong:

- **Icon:** `dialog-error-symbolic` or a relevant contextual icon.
- **Title:** A clear statement of the problem ("Unable to Load Recordings").
- **Description:** What happened and what the user can do ("Check your internet connection and try again.").
- **Action button:** A recovery action ("Retry", "Go Back").

Avoid technical jargon. Focus on what the user can do, not what failed internally.

### Other Uses

- Onboarding or welcome screens
- Permission requests ("Microphone Access Required")
- Completion states ("Transcription Complete")

---

## Placeholder Pages

Placeholder pages are a subset of status pages shown when a view is empty because the user has not yet added content. They differ from error states in tone: placeholders should be inviting and helpful, not alarming.

### Guidelines

- Use a large, friendly illustration or icon.
- Write a short, encouraging title ("Welcome to Transcriber").
- Provide a brief description of what the user can do.
- Include a prominent action button if there is a clear first step.
- Avoid making the user feel like something is wrong.

---

## Notifications (Gio.Notification)

Desktop notifications alert the user to events that occur when the application is not focused or is running in the background.

### When to Use

- A long-running background task has completed ("Transcription complete").
- An event requires the user's attention ("Recording stopped due to low disk space").
- A message or update has arrived (in communication apps).

### When NOT to Use

- For events the user is actively watching (they can already see the result).
- For frequent, low-importance updates (these become noise).
- For advertising or promotional purposes.

### Guidelines

- **Title and body:** Keep both short. The title is the headline; the body provides one line of detail.
- **Default action:** Clicking the notification should bring the relevant window or view into focus.
- **Action buttons:** Add up to 2 action buttons for quick responses ("Open", "Dismiss").
- **Priority:** Use `Gio.NotificationPriority` to set urgency. Reserve `URGENT` for truly critical events.
- **Withdraw:** Remove outdated notifications using `app.withdraw_notification()` when they are no longer relevant.
- **Respect user settings:** The system notification settings take precedence. Do not attempt to bypass Do Not Disturb.

### Implementation

```python
notification = Gio.Notification.new("Transcription Complete")
notification.set_body("interview_2026-03-15.wav has been transcribed.")
notification.set_default_action("app.show-transcription")
app.send_notification("transcription-complete", notification)
```

---

## Tooltips

Tooltips provide brief contextual information when the user hovers over or focuses a widget.

### Guidelines

- **Keep them short:** One line, ideally under 60 characters.
- **Required for icon-only buttons:** Every button without a text label must have a tooltip describing its action.
- **Include keyboard shortcut:** If the control has a keyboard shortcut, append it to the tooltip (e.g., "Paste (Ctrl+V)").
- **Do not duplicate visible labels:** If a button already has a text label that fully describes its action, a tooltip is unnecessary.
- **Accessible:** Tooltips are read by screen readers, making them essential for accessibility of icon-only controls.
- **No interaction:** Tooltips cannot contain interactive elements (links, buttons). Use a popover if interaction is needed.
- **Delay:** Tooltips appear after a brief hover delay (handled by GTK automatically). Do not customize the delay.

---

## Choosing the Right Feedback Pattern

| Situation | Pattern | Rationale |
|-----------|---------|-----------|
| Action completed successfully | Toast | Non-blocking confirmation |
| Action completed, undo available | Toast with action button | Allows reversal without a dialog |
| Destructive action confirmation | AdwAlertDialog | Prevents accidental data loss |
| Ongoing condition (e.g., offline) | Banner | Persistent, non-blocking awareness |
| Long operation in progress (known %) | Progress bar | Shows completion status |
| Long operation in progress (unknown) | Spinner | Indicates activity |
| Empty content area | Status page (empty state) | Guides user to first action |
| Error loading content | Status page (error state) | Explains problem, offers recovery |
| Background task completed | Desktop notification | Alerts user who may be elsewhere |
| Contextual info on hover | Tooltip | Quick, non-intrusive help |

---

## Cross-References

- [Controls and Inputs](04-controls-and-inputs.md) -- the interactive elements that produce feedback.
- [Writing Style](12-writing-style.md) -- how to write clear, helpful messages.
