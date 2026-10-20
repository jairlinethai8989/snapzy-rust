# SnapZy Rust Hotkey Coverage Design

## Goal

Expand the launcher hotkey settings so every primary capture workflow is
configurable and the delay preference can be changed without opening the main
window.

## Scope

The Hotkey window will expose six independent command rows:

1. Custom/Area capture
2. Window capture
3. Desktop/All capture
4. Scrolling capture
5. Video capture
6. Delay selector

Each row has an enable checkbox, a modifier selector, and a key selector. The
existing modifier and key choices remain available for compatibility. A
shortcut may be assigned to only one SnapZy command, and registration failures
from Windows are reported without replacing the active configuration.

The current single-hotkey settings migrate to the Custom/Area row. Existing
users therefore keep the same behavior after upgrading. New rows are disabled
by default unless a prior setting exists for them.

## Delay behavior

The Delay hotkey opens a compact menu with `0`, `3`, `5`, and `10` seconds.
Selecting a value persists it and applies it to the next still or scrolling
capture. The value is consumed once, then returns to `0` so an accidental
future capture is not silently delayed. The launcher displays the current
delay and keeps the existing scrolling-mode control separate.

Video keeps its existing mandatory 3-2-1 countdown and is not given a second
delay countdown by the Delay preference.

## Registration and lifecycle

- Register each enabled command with a stable internal `WM_HOTKEY` id.
- Apply all changes transactionally: register the new set first, save the
  settings, then unregister the old set. On any failure, restore the old
  registrations and settings.
- Reject duplicate shortcuts inside SnapZy before calling Windows.
- Unregister every command id during shutdown.
- A hotkey event dispatches directly to its command and is ignored while the
  app is busy with a modal workflow, recording, or an active capture.
- Delay selection is handled by the launcher menu and does not steal the
  focus of the source application beyond the normal hotkey event.

## UI and localization

The settings window becomes a compact scrollable form that fits the six rows
at normal DPI and remains usable at higher DPI. Labels and conflict/success
messages are available in English and Thai. The existing Apply action remains
the only commit action, and a successful save is shown in a modal confirmation
as it is today.

## Compatibility and safety

- Keep the existing JSON settings file readable.
- Missing new fields receive disabled/default values through serde defaults.
- Preserve the existing `hotkey_enabled`, `hotkey_modifiers`, and
  `hotkey_key` fields as the migration source for Custom/Area.
- Do not change the .NET implementation.
- Do not change video recording's countdown or capture source selection.

## Verification

Add unit coverage for settings migration, shortcut uniqueness, command-to-mode
dispatch, delay choices and one-shot consumption. Add native smoke coverage for
opening the expanded settings form, enabling each command, applying the set,
opening Delay, selecting 3 seconds, and triggering Custom/Area plus Desktop.
Run formatting, locked check/tests, release build, and installer/package
verification before reporting completion.
