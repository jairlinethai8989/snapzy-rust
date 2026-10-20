# Native Thai/English OCR

Status: approved; implemented in the local 0.9.0-alpha.1 candidate, pending tester validation.
Target: SnapZy Rust Preview 0.9.0-alpha.1. Rust only; no commit or Git push.
The user reported acceptance of multi-image 0.8.0-alpha.1 on 2026-10-10.

## Purpose

Continue the experiment roadmap with local OCR after the accepted image editor,
scrolling capture, ordering and multi-image increments. Reuse the established
Thai/English recognition approach from the .NET source without introducing a
.NET dependency or changing either .NET application.

## Alternatives

1. Recommended: distribute the native OCR runtime and its notices with the
   portable candidate, load it only on demand, and download pinned Thai/English
   models after explicit first-use consent. Keeps language data out of the ZIP;
   recognition works offline once setup has completed.
2. Include runtime and both models in the ZIP. First-use offline OCR is simpler,
   but every download includes the additional language data.

Runtime files increase the portable package size in either approach. Do not
promise the current approximately 1 MB ZIP size or a measured startup cost
before producing and benchmarking the candidate. No cloud OCR is proposed.

## Interaction

- Add an OCR scan icon to the editor's compact command row, with EN/TH tooltip
  and Help. English remains the interface default.
- Open one compact OCR window per editor, with language selection, Read text,
  an editable multiline result, Copy and Close. Default recognition language
  is Thai + English, independent of the interface language. Also offer English.
- Read the current canvas at its actual pixel dimensions, including placed
  images and annotations, excluding selection handles and editor chrome.
  Capture a consistent revision; disable Read while composition is unfinished.
  The existing Crop tool can narrow the image before OCR. A dedicated OCR
  region selector and automatic table/TSV reconstruction are deferred.
- Do not start recognition just by opening the OCR window. Clearly show Ready,
  Preparing models, Reading, Completed, No text found or an actionable error.
  Do not claim perfect transcription; results can be corrected before copying.
- Closing OCR or its owning editor invalidates pending results. Late results
  cannot overwrite another editor or a newer OCR request. Existing document,
  annotations, Undo history and clipboard remain unchanged by recognition.
- Copy writes only the reviewed text when the user requests it. Errors retain
  the last successful or manually edited result instead of clearing it.

## Data And Performance

- Use native Tesseract through its C API, not the .NET wrapper. Pin the tested
  runtime/model versions and include all applicable Tesseract/Leptonica notices.
  Validate the actual runtime architecture/dependencies before distribution.
- Reuse the existing verified tessdata_fast Thai/English revision and expected
  lengths/hashes as initial candidates; independently verify downloaded files.
  Keep models in the Rust Preview data directory, separate from the .NET apps.
- Ask before any model download. Explain approximate model size (currently
  about 5 MB for the pair), source, and local image processing. Cancel/refusal
  leaves capture and editing usable. No silent background download or upload.
- Download over HTTPS on a worker with bounded size, timeout and integrity
  validation, then atomic replacement. Offline or interrupted setup must allow
  a later retry without treating incomplete files as usable models.
- Keep engine initialization and recognition off the UI thread. Load neither
  OCR libraries nor models at launcher startup, capture or ordinary editor open.
- Permit one OCR job globally at a time. Bound input to 8 million pixels and
  decoded image storage to 32 MiB; fail with a crop suggestion before OCR work.
  These are input limits, not a promise that the engine's total RAM is 32 MiB.
- Closing the result window suppresses delivery; it need not forcibly interrupt
  a native recognition call. Keep the job slot until its worker actually exits.
  On completion release engine resources rather than retaining an idle engine.

## Verification And Delivery

- Unit tests for state/revision identity, model validation and atomic install,
  size limits, Unicode text/clipboard output and failure preservation.
- Real engine tests using controlled Thai, English and mixed-language bitmaps;
  compare against known phrases, not merely nonempty output. Add blank-image,
  missing/corrupt-model and oversize cases. Fix Thai joining without inserting
  spaces between every character; retain meaningful Latin word separation.
- Native UI checks for EN/TH, consent/refusal/retry, Read/Copy states, reviewed
  text, multiple-editor isolation, close-during-work and compact layout.
- Run all existing capture/editor/multi-image/scrolling regression checks.
  Compare startup and preview against the retained accepted alpha8 using the
  same workload, separately report OCR latency and peak memory.
- Deliver a verified portable 0.9.0-alpha.1 candidate, updated bilingual docs,
  licenses and tester checklist. Do not push until separately authorized.

## Deferred

Automatic table structure, handwriting promises, cloud recognition, searchable
PDF, translation, video, editable projects, automatic image layouts, full layer
panel and .NET changes are not included in this OCR milestone.

## Primary References

- https://tesseract-ocr.github.io/tessdoc/APIExample.html
- https://tesseract-ocr.github.io/tessdoc/Data-Files-in-tessdata_fast.html
- Existing local SnapZy OcrModels.cs, TesseractOcr.cs and OCR-NOTICES.txt.
