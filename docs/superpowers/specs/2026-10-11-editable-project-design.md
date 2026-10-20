# Editable SnapZy Rust Projects

Date: 2026-10-11 (Asia/Bangkok)
Status: written spec approved by the user; implementation authorized.
Baseline: user-accepted 0.9.0-alpha.13.
Rust only. Preserve existing dirty work and all earlier candidates. No .NET
changes, commit, push, installer or public release in this increment.

## Scope And Selected Approach

Add complete save/open of the Rust editor's current editable state before
attempting compatibility with the .NET `.neosnap` format. Alternatives considered
were restricted .NET import and full two-way interchange. The user selected
Rust-native persistence first because .NET image ownership, source crops and
clipping have no complete Rust equivalents yet. Do not label incompatible files
as compatible or flatten unsupported objects silently.

Use `.snapzy` for one self-contained local project. Preserve the captured source,
inserted images, current object order, geometry, styles, stroke patterns,
opacity, effects, editable text, callout backgrounds/tails, numbered steps,
blur strength, crop and expanded-canvas/source-clip state. Preserve objects
hidden by the crop, including the information needed to resume step numbering.
Opening a project clears selection, starts in Select with the ordinary existing
tool defaults, and starts fresh Undo/Redo history. Subsequent edits remain
undoable; previous-session history is intentionally not stored.

## File Contract

Use a versioned ZIP container through an established archive library, not a
handwritten ZIP parser. Reuse existing structured JSON, PNG and hashing code.
Only ZIP stored/deflate entries are accepted; no encryption, ZIP64 or multipart
archives in schema 1. PNG assets may be stored without extra deflate work.

- `manifest.json`: UTF-8 JSON, format identifier `snapzy-rust-project`, schema
  version 1, writer version, base-frame asset reference, crop/source-clip,
  asset/mask descriptors and ordered annotation records.
- `images/<id>.png`: lossless source images with original dimensions, pixels
  and alpha. Shared Arc sources are encoded once and referenced by stable IDs.
- `masks/<id>.png`: lossless 8-bit grayscale glyph coverage for Text, Callout
  and Step, shared by reference where appropriate. Keep text, font size,
  wrap width and step number alongside masks so editing remains possible.
- Manifest descriptors include dimensions, encoded length and SHA-256 for
  referenced payloads. Validate them before publishing a loaded document.

Asset and mask IDs are unsigned 64-bit decimal integers with canonical spelling
(no sign or leading zeros except `0`), unique within their respective namespaces.

Persist glyph coverage to preserve rendering exactly on reopen without relying
on another machine's text rasterization. Text edits/font changes still regenerate
coverage with the existing native text implementation. Do not store derived
shadow/outline masks, flattened output, composition caches, Undo/Redo, clipboard
data, hotkeys, UI language or drawing defaults. Assets contain no external paths
or URLs; loading never downloads content or executes anything.

Keep original BGRA channels/alpha exactly through PNG conversion, including RGB
under transparent pixels. Serialize finite source-pixel coordinates without
rounding curves, tail offsets or image placement. Preserve unique annotation
IDs; initialize the next edit/object identity safely above loaded IDs, rejecting
overflow. Do not trust or restore process/window identities or revision tokens.

## Validation And Budgets

Validate before changing application state or allocating large decoded buffers.
Use bounded readers and checked arithmetic rather than reading/decompressing an
unlimited archive first. Treat every project as untrusted input.

- Container and total uncompressed entries: at most 96 MiB each; manifest at
  most 32 MiB. At most 4,101 entries, including the single manifest. Reject
  duplicate names, unreferenced payloads, missing references and repeated IDs.
- Never extract entries to disk. Accept only the canonical manifest and numeric
  image/mask names; reject traversal, absolute paths, backslashes, symlinks,
  unsupported compression and inconsistent central-directory/local metadata.
- Each edge is 1-32,767 px. Canvas output and total distinct source-frame BGRA
  bytes are separately limited to 32 MiB, retaining existing editor limits.
  The application's existing 32 MiB cross-window source allowance and 16-editor
  limit still apply, including reservations for pending loads.
- At most 2,000 annotations, 100,000 points per freehand stroke and 1,000,000
  points total. At most 4,096 Unicode characters per text object. Total distinct
  glyph coverage is at most 1 MiB, matching the existing retained-mask budget.
- Enforce legal tool/content pairings and point counts, nonempty shapes/images,
  finite coordinates within the existing +/-32,767 range, valid text/mask
  dimensions and existing editable ranges for style, opacity, effects, font,
  wrap width, step diameter/number and blur strength.
- Crop/source-clip must follow the renderer's actual expanded-canvas semantics;
  do not require an expanded crop to fit the original frame. Never permit
  invalid bounds/overflow or data that the current editor cannot safely edit.
- Unknown format/schema, unsupported document/object properties, tools or enum
  variants are rejected with localized feedback, not ignored or flattened.
  Only defined optional writer metadata may be ignored; it is non-executable
  and does not change load semantics.

The same validation contract applies to writing. If an existing in-memory work
exceeds a project-specific bound, explain the limit and leave it open and dirty;
never truncate points, discard objects, silently resize images or overwrite an
older valid file. Archive checks must constrain allocations, not just final size.

## Open, Save And Save As

Add compact folder-open and save icon commands to the editor. Save As is an
explicit command in the save menu. Keep Copy and Save PNG distinct. Expose
Open project from the launcher so a saved project can be reopened after restart.
Use EN/TH labels, tooltips, status and Help. Ctrl+O, Ctrl+S and Ctrl+Shift+S apply
when editor focus is not inside a text/numeric entry dialog. No broad UI redesign.

Open always creates a new independent editor, leaving all existing editors
unchanged. Native file filters clearly separate `.snapzy` from PNG and `.neosnap`.
Opening `.neosnap` gives an explicit unsupported-format message for this phase.
Cancel/open error creates no editor and does not alter existing work or budgets.
Decode, validate and reserve resources before presenting the new editor. Opening
a valid file associates its path and marks its loaded revision project-saved.

Save uses the associated path or asks for a path the first time. Save As always
asks for a new destination and updates the association only after success.
Confirm overwrite through the native chooser. A project privacy warning appears
before the first Save and each Save As: original images and information beneath
blur/covering shapes remain recoverable; share exported PNG instead. Warn in
both EN/TH and Help. Do not claim blur or the project file is secure redaction.
Cancelling this warning cancels the save without writing or changing the path
association, saved revision or pending close decision.

Capture an immutable source/document snapshot after cancelling unfinished
geometry using existing cancellation rules. Encode/write in the background,
without awaiting flattened composition. Keep the app responsive; prevent
duplicate save/open requests and do not hold STATE/RefCell borrows across a
native modal, message pump, disk operation or worker completion.

Write to an exclusively created temporary sibling, flush/sync, then atomically
replace/rename to the chosen destination using the appropriate Windows API.
Do not delete the existing destination before replacement. Failure leaves the
old file unchanged and the document dirty; clean up only the operation's own
temporary file. Do not claim power-loss durability beyond OS/filesystem guarantees.

Successful completion marks only the exact saved revision, with preview identity
and operation generation guards. Later edits remain dirty. Completion for a
closed/reused native window or stale operation cannot mark another work saved.
Associate the path only with the corresponding successful operation.

## Saved State And Closing

Project-saved state is separate from PNG/clipboard kept state. A new capture or
imported image starts as an unsaved project. Copy or PNG export never counts as
project save and never suppresses the editable-project close warning. Undo back
to the project-saved revision is clean; Redo/new edits away from it are dirty.

Closing a dirty editor offers Save project / Discard / Cancel, with Cancel the
safe default. Save closes only after successful completion of that exact current
revision. Cancelling a chooser, save failure or an edit made while saving leaves
the editor open. Discard explicitly closes without persistence. Closing a clean
loaded/saved project does not prompt. While writes are pending, closing/exiting
must not terminate workers or report success prematurely.

Application Exit resolves dirty editors individually. Stage discard decisions
until all required decisions/saves succeed; Cancel or failure keeps remaining
open editors intact. Once resolved, destroy editors/owned OCR windows through
existing cleanup and exit. Do not add bulk-window management beyond what this
safe close workflow needs. Update test-owned close helpers for the new prompt;
do not add an application test flag that bypasses user data-loss protection.

## Responsibilities And Performance

Keep a focused project-format module responsible for DTOs, validation,
bounded archive/PNG I/O and atomic writing. Editor owns validated document
rehydration, identity/history and project-saved revision state. A small UI
integration module may own chooser/close intent state to avoid growing unrelated
code in ui.rs; keep native routing and the existing worker channel in place.

Snapshots share existing Arc pixels/points/masks. Do not clone full original
frames per object or encode the same shared asset repeatedly. Account for
pending project-open/save buffers and release reservations on every exit path.
No project scan, PNG encode, archive parsing or large disk I/O at launcher startup
or on the capture/hotkey path. Rendering effects and saved work use the same
existing renderer; no alternative project preview engine.

## Verification And Delivery

1. TDD round-trip every object, negative/curved coordinates, Thai/English
   multiline text, wrap/background/tail, numbering, opacity, effects, alpha,
   cropped-hidden objects and expanded source-clip. Match flattened pixels
   before save/after load exactly, preserving source bytes and shared references.
2. Test clear reopened history, selection/default reset, safe next identities,
   further edit/Undo/Redo, project vs PNG/clipboard saved-state isolation,
   late/stale writes, Undo-to-saved, Save As and no-op saves.
3. Corrupt ZIP/CRC/hash/PNG/JSON, truncated/missing/duplicate assets, traversal,
   encrypted/unsupported archive features, malformed objects, oversized/zip-bomb
   inputs, unknown versions and constrained budgets must reject atomically.
4. Inject encode/write/flush/replace failures; verify old destination bytes,
   dirty state, path association and existing editors remain intact. Use isolated
   temporary fixtures, never the user's real project files for fault injection.
5. Run full unit tests, format, warning-free release build and bounded performance
   measurements. Compare save/open on plain, annotated and large scrolling work;
   do not confuse native-window detection with completed decoding/painting.
6. After fresh input-free consent, test native Open/Save/Save As, EN/TH controls,
   Unicode paths/overwrite/cancel, privacy warning, reopen/edit/Copy/PNG, all three
   close decisions and multi-editor Exit. Recheck OCR and scrolling Stop/Escape
   in test-owned windows. Live Lark, hardware inputs and chat apps remain distinct.
7. Preserve Alpha 13 and produce a separate local candidate with accurate EN/TH
   notes and ZIP/staging hashes. Obtain user testing acceptance; no commit/push.

## Deferred

.NET `.neosnap` import/export, automatic recovery/autosave, persisted Undo/Redo,
recent-project lists, thumbnails, file associations/double-click installation,
cloud synchronization/encryption, automatic image arrangement, video, broad
UI parity and installer work are outside this focused increment.
