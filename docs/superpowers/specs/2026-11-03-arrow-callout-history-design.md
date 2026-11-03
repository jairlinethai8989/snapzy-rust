# Arrow, callout, and history controls

## Scope

- Arrow objects support an end head or heads at both endpoints. The selected choice remains the drawing default and is saved with each object.
- Speech bubbles support rounded, square, and oval frames. The tail remains editable and the text remains an editable object.
- `Ctrl+Z` undoes the active editor document. `Ctrl+Shift+Z` and `Ctrl+Y` redo. Text fields keep their normal native editing behavior.

## Compatibility

The editable `.snapzy` manifest advances to schema 7. Schemas 1 through 6 remain readable. Older imported objects use a single end arrow and rounded callout frame.

## Verification

Rust tests cover arrow and callout rendering properties plus schema round trips. UI tests cover preset menus and the history shortcut contract.
