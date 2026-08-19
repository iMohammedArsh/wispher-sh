# Wisper calm-utility refinement design

## Objective

Make the existing warm Wisper desktop interface feel easier to read, more responsive, and more alive without changing its professional Apple × Linear visual direction or dictation behavior.

## Design decisions

- Increase the base reading scale and enlarge navigation, labels, controls, and empty-state text. Desktop controls use a comfortable 13–15px range; critical status text and primary actions are larger.
- Let the main content use the available window width rather than a fixed, small central column. Use a fluid content measure with responsive horizontal padding and a two-column desktop composition where it improves scanning.
- Turn static cards into a clear hierarchy: one primary recording surface, a supporting transcript panel, then secondary settings/history content. Use low-contrast mineral surfaces, thin dividers, and controlled shadows.
- Give interactions subtle feedback: 160–220ms hover, press, tab, and state transitions. Respect reduced-motion preferences. There are no gradients, decorative AI motifs, or ambient animation.
- Polish the custom Windows title bar: larger hit areas, a stronger active/hover state, recognizable minimize/maximize/close icons, and the conventional red close affordance only on hover.
- On narrower resized windows, collapse the sidebar into a compact navigation strip rather than hiding the only route to secondary pages. Content remains readable without horizontal scrolling.

## Component scope

### Application shell

The title bar grows slightly to improve balance and touch/mouse target size. The sidebar gains larger row height, labels, and active state. The content area adopts fluid spacing using CSS `clamp()` so it is neither cramped at small widths nor sparse on wide displays.

### Write screen

The page gets a larger display heading and stronger status hierarchy. The recording surface becomes the visual anchor, with a comfortable minimum height and responsive width. Its real audio indicator retains the existing behavior while transitions make recording, processing, and delivered states legible at a glance. The transcript card appears alongside or below the recording surface depending on available width.

### Secondary screens

History, Vocabulary, Settings, and Help share the same readable content scale. Existing controls retain their command/data contracts; only spacing, typography, grouping, and responsive layout change.

## Technical boundaries

- CSS and presentational React markup only; no changes to Rust dictation, hotkeys, overlay logic, clipboard delivery, or persistence.
- Preserve all Tauri command names and current state listeners.
- Add a compact responsive navigation control for narrow widths so no existing screen becomes unreachable.
- Use semantic warm-color tokens and avoid introducing gradients or pure-white canvases.

## Accessibility and verification

- Maintain at least 4.5:1 contrast for active text, meaningful icons, borders, and focus states.
- Keep all controls keyboard accessible with visible focus.
- Check 900px desktop, 700px resized desktop, and 320px minimum layouts for clipping and horizontal overflow.
- Run `npm run build`, then manually verify title-bar controls, navigation, recording states, copy/paste, settings save, and reduced motion.
