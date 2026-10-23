# SnapZy Editor Palette, Line Style, and Font Design

**Date:** 2026-10-23  
**Target:** SnapZy Rust `0.15.0-alpha.3`  
**Status:** Approved design

## Goal

Make visual formatting faster to understand and more consistent across the image editor. Replace raw color inputs with one shared palette, replace text-only line-style choices with visual samples, and add a compact curated font selector that renders Thai and English text faithfully.

## Scope

This release covers:

- one reusable color palette for every editable color exposed by the Tauri editor;
- visual solid, dashed, and dotted line-style choices;
- font selection for new and existing text and speech bubbles;
- editable-project persistence, Undo/Redo, export, and compatibility;
- bundled open-font resources and notices; and
- EN/TH labels, tooltips, keyboard access, DPI, and High Contrast behavior.

It does not add mixed fonts within one text object, bold/italic controls, font downloading, arbitrary system-font enumeration, or changes to the .NET application.

## Shared Color Palette

All color controls use one compact popover component instead of visible native color inputs. It is used for:

- drawing and object foreground color;
- text foreground color;
- text and speech-bubble background color;
- shadow color; and
- text-outline color.

The popover contains 20 stable preset swatches, up to six most-recently-used colors, and an additional-color command that opens the operating-system color picker. The active color has a visible selected state and accessible name. Transparent is a separate explicit setting where the feature supports it; it is not represented by a fake color.

Choosing a color updates the relevant draft/default value. If an editable object is selected, the existing editor rules apply the change to that object as one Undo/Redo step. Recent colors are UI preferences, not project content, and are bounded to six unique values.

## Visual Line Styles

The context bar replaces the text `<select>` with a compact button showing the current line sample and a chevron. Its popover shows three rows rendered as actual horizontal samples:

- solid;
- dashed; and
- dotted.

Visible explanatory text is not required inside the compact control. Each option retains EN/TH accessible naming and a tooltip. The control continues to affect only tools that support patterns: pen, line, arrow, and outline shapes. Selection and transform guides always remain the editor's fixed dashed guide and never inherit the drawing pattern.

## Font Set and Selection

The first curated font set is:

- bundled: Kanit Regular, Prompt Regular, Noto Sans Thai Regular, and Sarabun Regular;
- system: Segoe UI, Arial, Calibri, and Tahoma.

The text dialog gains a compact font selector that previews each family with its own typeface. The selected family applies to both text and speech bubbles. New objects use the current text-font default. Selecting an existing text object exposes its family and changing it rerasterizes the text as one Undo/Redo operation while preserving text, size, wrap width, position, foreground/background colors, opacity, tail geometry, layer, shadow, and outline.

Bundled font files are loaded privately by the Rust editor engine and are not installed globally into Windows. Their OFL license files and attribution are included in the distribution. A missing or unusable font falls back to Segoe UI without blocking the document, while the requested family remains recorded so it can recover when available.

## Data Model and Commands

Add a validated font-family identifier to native text content. The identifier is an enum-like bounded value rather than an arbitrary operating-system font name. Existing text created before this schema change defaults to Segoe UI.

The editor service adds commands and view state for:

- selecting a text font;
- exposing the selected object's effective/requested font;
- setting text or callout background color and transparency; and
- preserving the existing foreground-color command semantics.

Font changes regenerate only the affected glyph mask. Invalid identifiers, missing resources, excessive glyph allocation, and non-text targets fail without partially changing history or project state.

## Project Compatibility

The `.snapzy` schema is extended without changing source image pixels or flattening annotations. The reader accepts older projects that omit the font field and assigns Segoe UI. New projects retain font family through save/open and remain editable.

Imported `.neosnap` text keeps its current compatibility behavior and maps to Segoe UI unless a supported native family is explicitly available in the source format. PNG and clipboard output always use the same rasterized result shown in the editor.

## Interaction and Layout

Palette and line-style popovers use the existing editor popup layer so only one popup is open at a time and it stays inside the viewport. They support mouse, keyboard arrows, Home/End, Enter/Space, and Escape. Clicking outside closes the popup without changing the current value.

The controls remain usable at the minimum editor size, common Windows DPI settings, and forced-color mode. Swatches use borders and selection marks so meaning is not conveyed by color alone. Font names are never clipped; the compact selector truncates safely and the popup provides the full name.

## Error Handling

- An unavailable bundled font resource reports a bounded localized error and uses Segoe UI.
- Invalid palette values and font identifiers are rejected at the service boundary.
- A failed text rerasterization leaves the object, selection, history, and saved state unchanged.
- Color picker cancellation leaves the previous color selected.
- Project load never silently drops text or converts it to a flat image.

## Verification

Automated coverage includes:

- palette validation, active/recent states, transparency, and every color target;
- visual line-style choices and fixed dashed transform guides;
- Thai, English, and mixed text masks for all bundled fonts;
- changing an existing text/callout font with Undo/Redo;
- font preservation through `.snapzy` save/open and old-project defaulting;
- missing-font fallback and bounded failure behavior;
- copy/PNG pixel agreement;
- EN/TH, keyboard access, tooltips, DPI, compact layout, and High Contrast;
- full editor, OCR, scrolling capture, and video regression suites; and
- an installed `0.15.0-alpha.3` Windows build.

## Delivery

The implementation is delivered first as a local Alpha 3 installer for user testing. It is not pushed or published until the installed build is approved.
