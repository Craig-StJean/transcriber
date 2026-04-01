# Writing Style

Guidelines for all user-facing text in GNOME applications and extensions. Well-written UI text
improves comprehension speed, reduces translation overhead, and helps users accomplish tasks
without friction.

## Core Principles

- **Be concise.** Use the minimum number of words that maintain clarity. Every extra word slows
  the reader down and increases the translation burden.
- **Improve comprehension speed.** Users scan UI text — they do not read it. Front-load the most
  important information.
- **Account for translation expansion.** Translations into languages such as German, Finnish, or
  Portuguese are often 30--50% longer than the English source. Short source strings leave room
  for expansion without breaking layouts.
- **Prefer familiar words.** Use task-domain language the user already knows. Avoid system
  jargon, abbreviations, and internal terminology.

## Tone and Voice

- Maintain a **neutral, professional** tone. The interface should feel helpful, not chatty.
- **Avoid first- and second-person pronouns** in labels and headings. Write "Files", not
  "My Files". Write "Settings", not "Your Settings".
- Use **"your"** only when referring to something the user owns or has created (for example,
  "Your recording has been saved").
- Use **inclusive, welcoming language**. Avoid idioms, slang, or culturally specific references
  that may not translate well.
- Never blame the user. Frame problems in terms of what the system could not do, not what the
  user did wrong.

## Capitalization Rules

Two capitalization styles are used throughout the GNOME desktop. Apply them consistently.

| Style | When to Use | Rule | Example |
|---|---|---|---|
| Header capitalization | Headings, buttons, menu items, tab labels, window titles | Capitalize all words of four or more letters, all verbs and nouns regardless of length, and the first and last word | "Save As", "Find and Replace", "Open in New Window" |
| Sentence capitalization | Descriptions, checkboxes, radio buttons, tooltips, body text, placeholder text | Capitalize only the first word and proper nouns | "Show line numbers", "Enable dark mode", "Use system font" |

### Quick test

If the text is a **heading or an interactive control label** (button, menu item, tab), use
header capitalization. If it is a **description, explanation, or inline option**, use sentence
capitalization.

## Button Labels

- Use **specific verbs** that name the action: "Save", "Delete", "Send", "Discard", "Cancel".
- Avoid generic labels such as "OK", "Yes", or "No". These force the user to re-read the
  surrounding context to understand what will happen.
- **Destructive buttons must name the destructive action.** Write "Delete File", not "OK".
  Write "Discard Changes", not "Yes".
- The button label should **match the action described in the dialog text**. If the dialog asks
  "Save changes to this document?", the confirming button should read "Save", not "Continue".

### Labels for Common Actions

| Action | Preferred Label | Avoid |
|---|---|---|
| Confirm creation | "Create" or "Add" | "OK" |
| Save changes | "Save" | "OK", "Done" |
| Cancel operation | "Cancel" | "No", "Back" |
| Delete item | "Delete" or "Remove" | "OK", "Yes" |
| Close dialog | "Close" | "OK", "Done" |
| Apply settings | "Apply" | "OK" |

## Punctuation

- **Ellipsis (…)**: Use only when the action requires further input before completing. For
  example, "Save As…" opens a file dialog; "Save" does not. Always use the Unicode ellipsis
  character (U+2026), not three consecutive periods.
- **Periods**: Do not use periods in headings, labels, tooltips, or single-sentence
  descriptions. Use periods only when text contains multiple sentences or paragraph breaks.
- **Unicode characters**: Use proper typographic characters throughout. Ellipsis (…, U+2026),
  en dash (–, U+2013), and em dash (—, U+2014) are preferred over ASCII approximations.

## Error Messages

Good error messages tell the user what went wrong and what to do about it.

- **State the problem clearly.** Describe what happened in plain language.
- **Suggest a remedy.** Tell the user what they can do to fix or work around the problem.
- **Never blame the user.** Frame the message around the system's inability to complete a task.

| Bad | Good |
|---|---|
| "Error: Connection failed" | "Could not connect to the server. Check your internet connection." |
| "Invalid input" | "The API key must be at least 32 characters." |
| "Permission denied" | "Could not save the file. Check that you have write access to this folder." |
| "Unknown error" | "Something went wrong. Try again, or restart the application if the problem persists." |

## Placeholder Text

Placeholder text appears inside empty input fields and should provide a **helpful hint**, not
a repetition of the field label.

| Label | Bad Placeholder | Good Placeholder |
|---|---|---|
| API URL | "Enter API URL" | "https://api.example.com/v1" |
| Name | "Name" | "Grace Hopper" |
| Search | "Search" | "Search files, folders, and settings" |

Placeholder text uses sentence capitalization and does not end with a period.

## Translation Considerations

GNOME applications are translated into dozens of languages. Follow these rules to make
translation easier and more accurate.

- **Wrap all user-visible strings** with `_()` (gettext) so translators can find them.
- **Do not split sentences across multiple UI controls.** A sentence that spans a label, a
  dropdown, and another label cannot be translated correctly because word order varies between
  languages.
- **Replace Latin abbreviations** with full English equivalents:
  - "i.e." becomes "that is"
  - "e.g." becomes "for example"
  - "etc." becomes "and so on"
  - "vs." becomes "compared to" or "versus"
- **Leave room for text expansion.** Design layouts that can accommodate strings 30--50% longer
  than the English original. Use flexible containers and avoid fixed-width labels wherever
  possible.
- **Do not embed numbers or technical strings in translatable text.** Use format substitution
  instead:

  ```python
  # Bad — translator cannot reorder the sentence
  _("Downloading " + filename + " (" + str(percent) + "%)")

  # Good — translator can reorder freely
  _("Downloading {filename} ({percent}%%)").format(filename=filename, percent=percent)
  ```

- **Provide translator comments** for ambiguous strings. Use `# Translators:` comments
  immediately before the `_()` call.

## See Also

- [Feedback and Messaging](05-feedback-and-messaging.md) — dialog patterns and notification
  guidelines
- [Accessibility](11-accessibility.md) — accessible label requirements and screen reader
  considerations
