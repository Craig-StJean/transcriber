# Adaptive Design

GNOME applications should gracefully adapt to different window sizes and form factors, from phones to large desktop displays. Rather than designing separate layouts for each size, use adaptive patterns that fluidly adjust to available space.

---

## Core Approach

- **Design from the most constrained size upward.** Start with the smallest supported size and progressively enhance for larger windows. This ensures that the essential experience is never lost.
- **Maintain full functionality at every size.** Adapting the layout should never remove features -- only reorganize how they are presented.
- **Use established adaptive widgets.** libadwaita provides purpose-built containers and patterns (`AdwBreakpoint`, `AdwClamp`, `AdwNavigationSplitView`, etc.) that handle the details of responsive behavior. Prefer these over custom solutions.
- **Favor list-based patterns.** Lists scale naturally across narrow and wide layouts, making them an ideal default for content presentation.

---

## Size Specifications

Different contexts impose different minimum size requirements. Applications must remain fully usable at these minimums.

| Context | Minimum Size |
|---|---|
| Desktop applications | 1024 x 600 px |
| Phone form factor | 360 x 294 px |
| Window snapping (half-display) | Maximum 640 px minimum width |

> **Note:** If your application supports window snapping (tiling to half the display), its minimum width must not exceed 640 px. Otherwise, snapping will fail on common display resolutions.

---

## Breakpoints with AdwBreakpoint

`AdwBreakpoint` allows you to define discrete layout changes that occur at specific width (or height) thresholds. When the window crosses a breakpoint, the layout transitions to a configuration better suited to the new size.

### Common Breakpoint Thresholds

| Threshold | Classification | Typical Behavior |
|---|---|---|
| ~450 px | Narrow | Single-column layout; controls collapse into menus or toolbars |
| ~700 px | Medium | Two-pane layout becomes available; utility panels appear inline |
| ~900 px | Wide | Full multi-pane layout; all controls visible simultaneously |

### Guidelines

- **Switch layouts smoothly.** Breakpoint transitions should feel natural, not jarring. Avoid large jumps in element position or sudden content reflow.
- **Keep breakpoint count low.** Two or three breakpoints are usually sufficient. Excessive breakpoints add complexity without proportional benefit.
- **Test at the boundaries.** Verify behavior at exactly the breakpoint width, not just well above or below it.

### Common Breakpoint Patterns

| Pattern | Wide Behavior | Narrow Behavior |
|---|---|---|
| Utility pane | Side panel beside main content | Bottom panel below main content |
| Toolbar controls | All controls visible in toolbar | Overflow controls move to a menu |
| Multi-column content | Content in multiple columns | Single-column stacked layout |
| Header bar actions | Actions shown as buttons | Actions collapse into a menu button |

---

## AdwClamp: Width-Constrained Content

`AdwClamp` is a container that limits content width to a comfortable maximum, centering it within the available space. This is essential for preventing layout degradation on large displays.

### Why Use AdwClamp

Without width constraints, several problems arise on wide windows:

| Problem | Description |
|---|---|
| Controls become separated | Buttons and inputs drift apart, breaking visual grouping |
| Text lines become too long | Reading comprehension drops significantly beyond ~80 characters per line |
| Visual structure deteriorates | Layouts lose cohesion when elements stretch to fill excessive width |

### Usage

- Apply `AdwClamp` to content areas that contain text, forms, or vertically-oriented lists.
- The default maximum width (typically around 600 px) is suitable for most content. Adjust only when you have a specific reason.
- `AdwClamp` handles its own internal padding and margin transitions, so manual responsive padding is usually unnecessary.

---

## Folding Patterns

Folding patterns present two related panes side-by-side on wide windows and collapse them into a single navigable view on narrow windows. libadwaita provides two primary widgets for this.

### AdwNavigationSplitView

- Displays a **sidebar** and **content pane** side-by-side on wide windows.
- On narrow windows, collapses to a single view with animated navigation between the sidebar and content.
- Ideal for **master-detail** patterns (e.g., a list of conversations on the left, selected conversation on the right).
- The sidebar acts as a navigation list; selecting an item pushes the detail view.

### AdwOverlaySplitView

- Displays a **sidebar** and **content pane** side-by-side on wide windows.
- On narrow windows, the sidebar **overlays** the content as a slide-out panel rather than replacing it.
- Ideal for sidebars that the user opens and closes frequently (e.g., a file browser panel in an editor).
- The content pane remains visible (dimmed) beneath the overlay sidebar.

### Choosing Between Them

| Use Case | Recommended Widget |
|---|---|
| Selecting items from a list to view details | `AdwNavigationSplitView` |
| Optional sidebar that supplements main content | `AdwOverlaySplitView` |
| File manager / email client style navigation | `AdwNavigationSplitView` |
| Properties or inspector panel | `AdwOverlaySplitView` |

---

## Container Proportions

When using split views or resizable panes, pay attention to proportional sizing:

- **Sidebars should never appear excessively wide or narrow** relative to the main window. A sidebar consuming half the window width, or one that is barely visible, both indicate a proportion problem.
- **Resizing should be smooth and glitch-free.** Dragging a separator or resizing the window must not cause flickering, content jumps, or layout thrashing.
- **Set sensible minimum and maximum widths** for each pane. This prevents degenerate states where one pane collapses to zero or dominates the window.
- **Respect the user's pane size preference** across sessions when practical.

---

## Testing Adaptive Layouts

Thorough testing across a range of sizes is essential. libadwaita provides tooling to make this practical.

### Using Adaptive Preview (GTK Inspector)

1. Open GTK Inspector (Ctrl+Shift+D or via the application menu in a development build).
2. Use the **adaptive preview** mode, which functions similarly to the responsive design mode in Firefox.
3. Set the preview to specific sizes (phone, tablet, desktop) or drag to arbitrary dimensions.

### Testing Checklist

| Test | What to Verify |
|---|---|
| Minimum phone size (360 x 294 px) | All content accessible; no truncation of essential text |
| Minimum desktop size (1024 x 600 px) | Full layout renders correctly |
| Breakpoint transitions | Layout switches smoothly at each threshold |
| Intermediate sizes | No awkward gaps, overlaps, or disproportionate elements |
| Rapid resizing | No flickering, layout thrashing, or rendering glitches |
| Snapped half-display (640 px width) | Application remains fully functional |
| Very large window (e.g., 2560 px wide) | Content constrained by AdwClamp; no excessive stretching |

---

## Cross-References

- [Layout and Navigation](03-layout-and-navigation.md) -- structural layout patterns and navigation models
- [Visual Design](06-visual-design.md) -- spacing, alignment, and visual consistency across sizes
