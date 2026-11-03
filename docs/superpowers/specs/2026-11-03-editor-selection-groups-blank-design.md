# Editor selection, groups, blank canvas, and copy icons

Approved by the user on 2026-11-03.

- A successful draw or insertion selects the new object and returns to Select. Failed or cancelled gestures keep the chosen tool.
- Dragging from empty canvas in Select creates a marquee. Fully enclosed editable objects become the selection. A click on a member of a saved group selects its members.
- Group and Ungroup are single Undo/Redo edits. Group members stay independent editable objects in `.snapzy`; dragging a group moves its members together. Ungroup restores individual selection. Existing image combination ownership is preserved.
- New blank tab starts at 1024 × 768 with a white background. The existing canvas size command changes its dimensions.
- Copy Image and Duplicate Object use visually distinct icons and EN/TH tooltips.
- Keep existing `.snapzy` files readable. Project files with groups use a new schema version; unsupported group metadata must be rejected rather than silently dropped.
- Verify editor gestures, grouping, project round trip, blank PNG, and UI controls. No Git push or public release in this change.
