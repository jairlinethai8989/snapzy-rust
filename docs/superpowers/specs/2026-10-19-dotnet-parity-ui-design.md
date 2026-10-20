# SnapZy Rust .NET-Parity UI and Icon Refresh

## Goal

Bring the Rust preview interface closer to the proven .NET SnapZy workflow without changing capture, editor, hotkey, delay, or file behavior.

## Scope

- Keep the launcher compact and organize the primary capture actions as Area, Window, Scrolling, and Video. Window/Desktop remains a single grouped action with a visible dropdown affordance.
- Keep Delay, scrolling mode, and open destination in a compact secondary row.
- Use a consistent SnapZy palette: cool neutral surfaces, blue primary action, clear hover/focus/pressed states, and system colors in High Contrast mode.
- Replace ambiguous launcher/editor glyphs with dedicated semantic icons for scrolling capture, filled shapes, and stroke patterns. Preserve the existing icon cache, DPI scaling, tooltip text, and accessible names.
- Keep the editor tool order aligned with the .NET workflow: select, crop, draw/highlight, line/arrow, shape, blur/step/text/callout, layer, delete, undo/redo. Secondary project/image/effect commands remain under More or their grouped menus.
- Keep English as the default UI language with Thai switching, and retain tooltips for every icon button.

## Non-goals

- No changes to capture algorithms, recording, scrolling detection, OCR, project formats, or hotkey registration behavior.
- No installer, package, or version change in this UI pass.

## Validation

- Unit tests assert launcher grouping and semantic icon mapping.
- `cargo test --locked --quiet` remains green.
- Windows target check/build pass.
- Native UI smoke check verifies launcher/editor layout, tooltips, DPI-safe bounds, and High Contrast palette fallback.
