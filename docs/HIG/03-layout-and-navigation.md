# Layout and Navigation

Layout and navigation patterns determine how content is structured and how users move between different parts of an application. GNOME provides a set of standard containers and navigation models that ensure consistency across the desktop.

See also: [Official GNOME HIG - Patterns](https://developer.gnome.org/hig/)

---

## Containers

Containers are the structural building blocks of a GNOME application. Each container type serves a specific purpose and carries specific behavioral expectations.

### Windows

GNOME applications use two types of windows:

#### Primary Windows

The main application window. Most apps have exactly one.

| Property | Guideline |
|---|---|
| **Resizability** | Resizable by default. Set minimum size to prevent layout breakage. |
| **Default size** | Should match the content's natural dimensions. A text editor should be wide; a calculator should be compact. |
| **State persistence** | Remember size and position across sessions via GSettings. |
| **Title** | Use the content title (document name) or the app name. Do not combine both. |
| **Close behavior** | Closing the last primary window quits the application (unless background operation is active; see [Background Services](13-background-services-and-portals.md)). |

Use `AdwApplicationWindow` as the base class. It provides built-in support for adaptive layout, header bar integration, and libadwaita styling.

#### Secondary Windows

Supplemental windows for tasks that relate to but are separate from the primary window.

| Property | Guideline |
|---|---|
| **Modality** | Typically modal (blocks interaction with the parent window). |
| **Size** | Smaller than the primary window. Often not resizable. |
| **Purpose** | Preferences, about dialogs, properties, export dialogs. |
| **Lifetime** | Transient. Should not outlive the parent window. |

Use `AdwPreferencesWindow` for settings windows and `AdwAboutWindow` for about dialogs.

---

### Header Bars

The header bar spans the top of the window and serves as both the title bar and the primary toolbar.

**Widget:** `AdwHeaderBar` (inside `AdwToolbarView`)

#### Layout Zones

The header bar is divided into three alignment zones:

```
┌──────────────────────────────────────────────────────┐
│  [Back] [Action]     App Title / View Title    [Menu]│
│  ← Left zone →      ← Center zone →    ← Right zone→│
└──────────────────────────────────────────────────────┘
```

| Zone | Purpose | Examples |
|---|---|---|
| **Left** | Navigation and primary actions | Back button, new/add button |
| **Center** | Title, view switcher, or search entry | App name, `AdwInlineViewSwitcher`, search bar |
| **Right** | Secondary actions and app menu | Toggle buttons, primary menu button |

#### Header Bar Rules

- **Small number of controls.** Aim for 3-5 buttons maximum. More than that signals design problems; move actions elsewhere.
- **Draggable space.** Leave enough empty space in the header bar for window dragging. Do not fill the entire bar with controls.
- **No redundant labels.** If the action is clear from its icon, do not add a label. See [App Identity - Icon vs. Label](02-app-identity.md).
- **Consistent menu button.** Place the primary app menu (`open-menu-symbolic`) at the far right of the header bar. It should contain preferences, keyboard shortcuts, about, and quit.

---

### Popovers

Floating contextual panels that appear attached to a parent widget. They overlay the current content without opening a new window.

**Widget:** `GtkPopover`, `GtkPopoverMenu`

| Property | Guideline |
|---|---|
| **Trigger** | Appears on click/tap of a button or menu control |
| **Position** | Anchored to the triggering widget, pointing toward it |
| **Dismissal** | Clicking outside the popover closes it |
| **Content** | Contextual options, settings subsets, small forms |
| **Size** | Keep compact. If the content needs scrolling, consider a dialog instead. |

Use popovers for contextual actions and small option panels. Do not use them for primary content or long forms.

---

### Utility Panes

Side panels that provide tools, options, or supplementary information alongside the main content.

| Property | Guideline |
|---|---|
| **Position** | Left or right side of the main content area |
| **Visibility** | Togglable via a button or keyboard shortcut |
| **Width** | Fixed or user-resizable, depending on content |
| **Content** | Tool palettes, property panels, file trees |

Utility panes are appropriate for applications where users frequently switch between content and tool panels (image editors, IDEs). For simpler apps, prefer popovers or preference windows.

---

### Boxed Lists

Grouped content items presented in bordered, rounded sections. This is the standard pattern for settings, preferences, and structured options.

**Widget:** `AdwPreferencesGroup` (containing `AdwActionRow`, `AdwSwitchRow`, `AdwComboRow`, `AdwSpinRow`, etc.)

```
┌─ Group Title ──────────────────────────────────────────┐
│  Description text explaining this group               │
│                                                        │
│  ┌──────────────────────────────────────────────────┐  │
│  │  Row Title                          [Control]    │  │
│  ├──────────────────────────────────────────────────┤  │
│  │  Row Title                          [Control]    │  │
│  │  Subtitle with additional details                │  │
│  ├──────────────────────────────────────────────────┤  │
│  │  Row Title                          [Control]    │  │
│  └──────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────┘
```

#### Boxed List Guidelines

- Group related settings together under a descriptive title.
- Add a group description when the title alone is not sufficient.
- Each row contains a title (and optional subtitle) on the left and a control on the right.
- Rows are separated by subtle horizontal dividers.
- Use `AdwPreferencesPage` to organize multiple groups into a scrollable page with optional section headers.

---

### Grid Views

Multi-column layouts for displaying collections of items such as images, files, or cards.

**Widget:** `GtkGridView`

| Property | Guideline |
|---|---|
| **Item sizing** | Uniform item sizes within a grid |
| **Columns** | Adapt column count to available width |
| **Selection** | Support single and multi-select with checkmarks |
| **Spacing** | Consistent gaps between items |

---

### List and Column Views

Linear layouts for displaying sequential or hierarchical data.

**Widgets:** `GtkListView`, `GtkColumnView`

| Pattern | Use For | Widget |
|---|---|---|
| **Simple list** | Linear sequences of items | `GtkListView` |
| **Column view** | Tabular data with sortable columns | `GtkColumnView` |
| **Tree list** | Hierarchical/nested items | `GtkTreeListModel` with `GtkListView` |

Use `GtkListView` with `GtkSingleSelection` or `GtkMultiSelection` for selection behavior. Back all list views with `GListModel` implementations for efficient rendering of large data sets.

---

### Selection and Edit Modes

Interactive modes where users select items for batch operations (delete, move, share).

| Property | Guideline |
|---|---|
| **Activation** | Long press on an item, or a dedicated "Select" button |
| **Visual feedback** | Checkmarks on selected items, header bar changes color/content |
| **Actions** | Context-appropriate batch actions appear in the header bar |
| **Exit** | "Cancel" button or pressing Escape returns to normal mode |

During selection mode, the header bar typically replaces its normal content with a selection count and action buttons (e.g., "3 selected — Delete — Share").

---

## Navigation Patterns

Navigation patterns determine how users move between different views and content within an application. Choose the simplest pattern that fits the app's structure.

### Navigation Pattern Decision Guide

Use this table to select the appropriate navigation pattern:

| Number of Views | View Relationship | Pattern | Widget |
|---|---|---|---|
| 2-5 equivalent views | Peer (same level) | **View Switcher** | `AdwViewStack` + `AdwInlineViewSwitcher` |
| Many content sections | Independent labeled groups | **Tabs** | `AdwTabView` + `AdwTabBar` |
| 6+ navigation targets | Hierarchical or categorized | **Sidebar** | `AdwNavigationSplitView` |
| Content exploration | Parent-child relationships | **Browsing** | `AdwNavigationView` |
| Finding specific items | Any structure | **Search** | `GtkSearchBar` + `GtkSearchEntry` |

---

### View Switchers

The simplest navigation pattern. Toggle between a small number of equivalent, peer-level views.

**Widgets:** `AdwViewStack` + `AdwInlineViewSwitcher`

| Property | Guideline |
|---|---|
| **View count** | 2-5 views. More than 5 signals that a sidebar is more appropriate. |
| **View relationship** | All views must be equivalent in importance and at the same hierarchy level. |
| **Placement** | In the center of the header bar. |
| **Labels** | Short, clear labels. Optionally accompanied by icons. |
| **Persistence** | The selected view should persist across app restarts. |

```
┌───────────────────────────────────────────┐
│  [←]   [View A] [View B] [View C]  [≡]   │  ← Header bar with view switcher
├───────────────────────────────────────────┤
│                                           │
│         Content for selected view         │
│                                           │
└───────────────────────────────────────────┘
```

For adaptive layouts, use `AdwViewSwitcherBar` at the bottom of the window when the header bar is too narrow. See [Adaptive Design](08-adaptive-design.md).

---

### Tabs

Labeled content sections for managing multiple independent documents or sessions.

**Widgets:** `AdwTabView` + `AdwTabBar` + `AdwTabOverview`

| Property | Guideline |
|---|---|
| **Purpose** | Multiple independent documents, sessions, or workspaces |
| **Tab count** | Unlimited. Tabs should scroll when they overflow. |
| **Closable** | Individual tabs should be closable |
| **Reorderable** | Tabs should support drag-to-reorder |
| **Overview** | Use `AdwTabOverview` for a grid overview of all tabs (especially useful on narrow screens) |

Tabs differ from view switchers in that tabs represent user-created instances (open documents, terminal sessions) while view switchers represent app-defined categories.

---

### Sidebars

Vertical navigation lists for applications with many navigation targets or hierarchical organization.

**Widget:** `AdwNavigationSplitView`

| Property | Guideline |
|---|---|
| **Use when** | Too many views for a view switcher (6+), or views are categorized/hierarchical. |
| **Position** | Left side of the window. |
| **Width** | Typically 200-280px. May be resizable. |
| **Content** | Navigation items with icons and labels. May include sections/headers. |
| **Collapse behavior** | On narrow windows, the sidebar collapses and the app uses push navigation. See [Adaptive Design](08-adaptive-design.md). |

#### When NOT to Use Sidebars

- **Rich or immersive content apps.** If the main content area needs maximum space (photo editors, video players, games), a sidebar wastes screen real estate.
- **Simple apps with few views.** Use a view switcher instead.
- **Mobile-first apps.** Sidebars do not work well on narrow screens unless they collapse into push navigation.

```
┌────────────┬─────────────────────────────┐
│ Sidebar    │  Content Area               │
│            │                             │
│ > Item A   │  Content for selected item  │
│   Item B   │                             │
│   Item C   │                             │
│            │                             │
│ Section    │                             │
│   Item D   │                             │
│   Item E   │                             │
└────────────┴─────────────────────────────┘
```

---

### Browsing

Navigating through a hierarchy of content by moving forward into child views and backward to parent views.

**Widget:** `AdwNavigationView`

| Property | Guideline |
|---|---|
| **Use when** | Content has parent-child relationships (folders, categories, detail views). |
| **Navigation** | Push new views onto a stack. Provide a back button to pop back. |
| **Back button** | Always present in the header bar when a parent view exists. |
| **Transitions** | Use slide transitions to reinforce the spatial model. |

```
┌──────────────┐      ┌──────────────────┐      ┌──────────────────┐
│  Categories  │ ──→  │  Category Items   │ ──→  │  Item Detail     │
│              │      │                    │      │                  │
│  > Audio     │      │  > Recording 1     │      │  Title: ...      │
│    Video     │      │    Recording 2     │      │  Date: ...       │
│    Text      │      │    Recording 3     │      │  Content: ...    │
│              │  ←── │                    │  ←── │                  │
└──────────────┘      └──────────────────┘      └──────────────────┘
         Back                  Back
```

Use `AdwNavigationView` to manage the view stack. Each pushed view gets its own `AdwHeaderBar` with a back button automatically provided.

---

### Search

Finding specific content within the application.

**Widgets:** `GtkSearchBar` + `GtkSearchEntry`

| Property | Guideline |
|---|---|
| **Activation** | Clicking a search button or pressing `Ctrl+F` |
| **Placement** | Search bar slides in below the header bar, or inline within content |
| **Behavior** | Filter results in real time as the user types |
| **Empty state** | Show a helpful message when no results match |
| **Dismissal** | Pressing `Escape` or clicking the search button again |

Search is not a standalone navigation pattern; it augments other patterns. Any app with more than a handful of items should support search.

---

## Container Nesting and Composition

A typical GNOME application composes these containers as follows:

```
AdwApplicationWindow
└── AdwToolbarView
    ├── AdwHeaderBar (top bar)
    │   ├── Left zone: navigation buttons
    │   ├── Center zone: AdwInlineViewSwitcher or title
    │   └── Right zone: menu button
    └── Content area (one of):
        ├── AdwViewStack (for view switcher navigation)
        ├── AdwNavigationSplitView (for sidebar navigation)
        │   ├── Sidebar: GtkListView
        │   └── Content: AdwNavigationView
        ├── AdwNavigationView (for browsing navigation)
        └── AdwTabView (for tabbed navigation)
```

For preferences windows:

```
AdwPreferencesWindow
└── AdwPreferencesPage
    ├── AdwPreferencesGroup ("General")
    │   ├── AdwSwitchRow
    │   ├── AdwComboRow
    │   └── AdwActionRow
    └── AdwPreferencesGroup ("Advanced")
        ├── AdwSpinRow
        └── AdwEntryRow
```

Reference: `AdwToolbarView` manages the vertical stacking of toolbars and content. It handles toolbar reveal animations and ensures correct spacing.

---

## Further Reading

- [Controls & Inputs](04-controls-and-inputs.md) - Widgets used within these containers
- [Adaptive Design](08-adaptive-design.md) - How these layouts respond to different screen sizes
- [App Identity](02-app-identity.md) - Header bar content and icon placement
- [Design Principles](01-design-principles.md) - Why simplicity and flat navigation matter
- [Keyboard & Shortcuts](15-keyboard-and-shortcuts.md) - Keyboard navigation between views
