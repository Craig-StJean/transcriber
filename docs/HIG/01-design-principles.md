# Design Principles

The GNOME Human Interface Guidelines are built on four core design principles. These principles guide every decision, from high-level architecture down to individual widget placement. They are not abstract ideals; they are practical constraints that shape how software should behave.

See also: [Official GNOME HIG - Design Principles](https://developer.gnome.org/hig/)

---

## 1. Design for People

Software exists to serve the people who use it. This means accommodating a wide range of abilities, cultures, devices, and levels of experience.

### Key Practices

- **Assume diverse users.** People differ in vision, motor skills, cognitive ability, language, cultural context, and technical background. Design for this diversity from the start, not as an afterthought.
- **Minimize specialist knowledge.** Users should not need to understand technical implementation details. A transcription app should not require users to know about audio codecs or sampling rates to get started.
- **Use familiar metaphors.** Draw on concepts people already understand. A "trash" is easier to grasp than a "soft-delete queue."
- **Support assistive technologies.** Every interactive element must be reachable by keyboard and announced by screen readers. See [Accessibility](09-accessibility.md).
- **Respect internationalization.** Layouts must accommodate text expansion (German strings are often 30% longer than English). Icons should not rely on culturally specific symbols.

### Example

A settings panel for audio input should present device names in plain language ("Built-in Microphone", "USB Headset") rather than ALSA device identifiers (`hw:0,0`). The user should never need to consult external documentation to complete a basic task.

### Anti-Patterns

| Anti-Pattern | Problem |
|---|---|
| Exposing raw configuration file paths | Requires filesystem knowledge |
| Using jargon in labels ("Quantization", "Bitrate") without explanation | Excludes non-specialist users |
| Requiring mouse-only interactions (drag to reorder with no keyboard alternative) | Excludes users with motor impairments |
| Hard-coding locale-specific date formats | Breaks for international users |

---

## 2. Make it Simple

Each application should do one thing well. Complexity is the enemy of usability.

### Key Practices

- **One primary function.** An app's purpose should be expressible in a single sentence. If it takes a paragraph, the scope is too broad.
- **Progressive disclosure.** Show the most important controls first. Place secondary options in expandable sections, popovers, or preference pages. Do not overwhelm users on first launch.
- **Prominent frequent actions, hidden rare ones.** The action a user performs 90% of the time should be the most visible element. Configuration toggles used once per month belong in a preferences window, not the header bar.
- **Reduce visual noise.** Every element on screen competes for attention. Remove decorative elements that do not aid comprehension. Use whitespace deliberately.
- **Flat navigation.** Prefer shallow hierarchies. Users should reach any part of the app in two clicks or fewer when possible.

### Progressive Disclosure in Practice

Consider a transcription application:

| Layer | What to Show | Where |
|---|---|---|
| **Primary** | Record button, transcript output | Main window, always visible |
| **Secondary** | Language selection, model choice | Header bar or inline controls |
| **Tertiary** | Audio format settings, export options | Preferences window or popover |
| **Advanced** | Debug logging, raw API parameters | Hidden unless explicitly enabled |

### Example

A text editor's main window should show the document and basic formatting. Font management, plugin configuration, and file encoding settings belong in a preferences window (see [Layout & Navigation](03-layout-and-navigation.md)).

### Anti-Patterns

| Anti-Pattern | Problem |
|---|---|
| Toolbar with 20+ buttons | Overwhelms users, slows task completion |
| Deeply nested preference categories | Users cannot find settings |
| Showing all configuration on a single page | No hierarchy of importance |
| Feature-flag UI visible to end users | Leaks developer concerns into user space |

---

## 3. Reduce User Effort

Every click, keystroke, and decision costs the user time and cognitive energy. Minimize these costs.

### Key Practices

- **Automate where possible.** If the system can determine the correct value, do not ask the user. Auto-detect audio devices, infer file types, remember window positions.
- **Streamline common workflows.** Count the steps required for the most frequent task. Then reduce that number. If recording audio takes five clicks, find a way to make it two.
- **Provide smart defaults.** Choose defaults that work for the majority of users. A transcription app should default to the system language and the default microphone without requiring configuration.
- **Remember context.** Preserve recent files, last-used settings, scroll positions, and window geometry across sessions. Users should not have to re-establish context after restarting the app.
- **Use concise language.** Labels and descriptions should use the fewest words necessary. "Save" is better than "Save the current document to disk." See [Typography & Labels](05-typography-and-labels.md).
- **Offer auto-suggestions.** When users type into a search field or entry, suggest completions based on history or likely values.

### Measuring Effort

For any workflow, count:

1. **Clicks/taps** required
2. **Decisions** the user must make (each dropdown or choice is a decision)
3. **Context switches** (moving between windows, tabs, or apps)
4. **Wait time** (loading, processing)

A well-designed workflow minimizes all four.

### Example

Instead of asking users to manually configure an output directory for saved transcripts, default to `XDG_DOCUMENTS_DIR/Transcripts/` and create the directory automatically. Offer a "Change" button for the minority who want a different location.

### Anti-Patterns

| Anti-Pattern | Problem |
|---|---|
| Requiring initial setup wizard with 6+ steps | Delays time to first use |
| Forgetting user preferences between sessions | Forces repeated configuration |
| Asking "Are you sure?" before every action | Adds a decision to every workflow |
| Requiring manual entry of values the system already knows | Wastes time, invites errors |

---

## 4. Be Considerate

Software should anticipate user needs, prevent errors, and respect the user's time and attention.

### Key Practices

- **Prevent errors proactively.** Disable or hide controls that cannot be used in the current context. Grey out a "Stop Recording" button when no recording is active, rather than showing an error when it is clicked.
- **Enable undo instead of confirmation dialogs.** Confirmation dialogs ("Are you sure you want to delete this?") interrupt flow and train users to click "Yes" without reading. Instead, perform the action immediately and offer an undo toast. See [Error Handling & Undo](12-error-handling-and-undo.md).
- **Respect attention.** Notifications and alerts should be used sparingly. A background transcription completing is worth a notification; a routine auto-save is not. See [Dialogs & Notifications](07-dialogs-and-notifications.md).
- **Avoid interruptions.** Do not steal focus. Do not pop up windows unexpectedly. Do not play sounds without the user's consent.
- **Graceful degradation.** When something goes wrong, explain what happened in plain language, suggest what the user can do, and preserve their work. Never show a stack trace or crash without saving state.
- **Be transparent about limitations.** If an operation will take a long time, show a progress indicator. If a feature requires network access, say so before the user starts. See [Feedback & Status](06-feedback-and-status.md).

### Undo vs. Confirmation

| Approach | User Experience |
|---|---|
| **Confirmation dialog** | Interrupts flow. Users develop "dialog blindness" and click through without reading. Does not help if the user clicks "Yes" by mistake. |
| **Undo (toast/action)** | Action happens immediately. User can reverse it within a time window. No interruption for the common case. Safer because the user actively chooses to undo only when they notice a mistake. |

Prefer undo for all reversible destructive actions. Reserve confirmation dialogs for truly irreversible operations with significant consequences (e.g., permanently deleting an account).

### Example

When a user deletes a transcript from history, remove it from the list immediately and show a toast: "Transcript deleted. Undo." The transcript is only permanently removed after the undo window expires (typically 5-10 seconds). This is faster, less intrusive, and actually safer than a confirmation dialog.

### Error Messages

Good error messages follow this structure:

1. **What happened** (in plain language)
2. **Why it happened** (if known and useful)
3. **What the user can do** (actionable next step)

| Bad | Good |
|---|---|
| "Error: ECONNREFUSED" | "Could not connect to the transcription service. Check your internet connection and try again." |
| "Operation failed" | "The recording could not be saved because the disk is full. Free up space and try again." |
| "Invalid input" | "The file format is not supported. Use WAV, MP3, or OGG files." |

---

## Applying the Principles Together

The four principles are not independent; they reinforce each other. A design decision that satisfies one principle should not violate another.

| Scenario | Design for People | Make it Simple | Reduce Effort | Be Considerate |
|---|---|---|---|---|
| Adding a new setting | Is the label understandable to non-technical users? | Does it belong in the main UI or preferences? | Can the system infer the value automatically? | What happens if the user picks the wrong value? |
| Handling an error | Is the message readable by all users? | Is the error presented without technical clutter? | Can the system recover automatically? | Does it preserve the user's work? |
| Designing a workflow | Can it be completed with assistive technology? | Are the steps minimal and linear? | Are there unnecessary decisions? | Does it interrupt the user needlessly? |

---

## Further Reading

- [App Identity](02-app-identity.md) - Applying principles to naming and icon design
- [Layout & Navigation](03-layout-and-navigation.md) - Structural patterns that embody simplicity
- [Accessibility](09-accessibility.md) - Designing for people in depth
- [Error Handling & Undo](12-error-handling-and-undo.md) - Considerate error recovery
