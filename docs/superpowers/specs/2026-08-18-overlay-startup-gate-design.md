# Overlay startup gate

## Goal

Prevent any unstyled or partially sized overlay surface from appearing when Wisper starts.

## Design

The native Windows overlay starts hidden. Rust applies its idle size, pill-shaped native region, opacity, and taskbar-relative position before it may be displayed. The overlay renderer marks its document as an overlay before React paints, renders the idle black pill, and then invokes an idempotent `overlay_ready` command. That command shows the already-prepared native window.

If the renderer does not load, the overlay remains hidden rather than showing a broken temporary surface. Recording state changes still resize and position the same window after it has been revealed.

## Verification

Build the frontend and run `cargo fmt --check` plus `cargo check`. On the next app launch, no gray, white, or rectangular startup flash should be visible; only the final idle pill should appear.
