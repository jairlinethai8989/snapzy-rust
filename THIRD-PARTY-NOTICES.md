# Third-party notices

Rust dependencies are locked in `Cargo.lock`. The license files distributed
with this project are stored under `third-party-licenses/<crate>-<version>/`.
They are copied from the corresponding crate source packages.

## Shared SnapZy Source

`src/scroll.rs` adapts the motion scoring, footer detection and bounded-strip
strategy from the public SnapZy .NET source at
`snapzy-releases/source/snapzy/src/SnapCraft/ScrollStitcher.cs`.
Copyright (c) 2026 jairlinethai, MIT License. The same copyright and complete
MIT permission/warranty notice are retained in this repository's `LICENSE`.
The local reference SHA-256 was
`AD3A0EAEFC788A832D6C690AC207D93E0B13FE71D1CFF4A278A5838915BDE924`.
No private Neo Snap source or .NET runtime dependency is added to this build.

## Lucide Icons

The compact native launcher and editor toolbar use geometry adapted from
Lucide 0.468.0. Lucide is distributed under the ISC License; the complete
notice is included at `third-party-licenses/lucide-0.468.0/LICENSE`.

## Bundled Editor Fonts

The image editor privately loads the Regular faces of Kanit, Prompt, Noto Sans
Thai and Sarabun from the Google Fonts project. These fonts are distributed
under the SIL Open Font License 1.1 and are not installed system-wide. Complete
license texts are included under `third-party-licenses/google-fonts/`.

## Native OCR Runtime and Language Data

The optional, dynamically loaded Windows x64 OCR runtime is Tesseract OCR
5.2.0 (`assets/ocr-runtime/tesseract50.dll`) and Leptonica 1.82.0
(`assets/ocr-runtime/leptonica-1.82.0.dll`). Their complete license texts are
included at `third-party-licenses/tesseract-5.2.0/LICENSE.txt` and
`third-party-licenses/leptonica-1.82.0/LICENSE.txt`. Runtime file SHA-256:

| File | SHA-256 |
| --- | --- |
| `tesseract50.dll` | `DFCB3E6ED0B16BC55BFDBCF53543CFE42A354B87C3E35BD3A95EEBF005D73E76` |
| `leptonica-1.82.0.dll` | `DE4D04EC75095374D98F5DD7A60D14D7E2E0F76589DB693ECCF7AE658BE8CB2B` |

English and Thai `tessdata_fast` models are downloaded only after user consent
from Tesseract revision `87416418657359cb625c412a48b6e1d6d41c29bd`. Their
expected lengths and SHA-256 hashes are pinned in `src/ocr.rs`; full Apache-2.0
terms apply to both models. Model files are kept in the user's SnapZy Rust data
directory and are not included in the portable ZIP. `OCR-NOTICES.txt` is
distributed in `third-party-licenses/`.

## Rust Dependencies

| Crate | Version | Declared license |
| --- | --- | --- |
| adler2 | 2.0.1 | 0BSD OR MIT OR Apache-2.0 |
| arrayref | 0.3.9 | BSD-2-Clause |
| arrayvec | 0.7.8 | MIT OR Apache-2.0 |
| autocfg | 1.5.1 | MIT OR Apache-2.0 |
| base64 | 0.22.1 | MIT OR Apache-2.0 |
| bitflags | 2.13.2 | MIT OR Apache-2.0 |
| bytemuck | 1.25.2 | Zlib OR Apache-2.0 OR MIT |
| byteorder-lite | 0.1.0 | Unlicense OR MIT |
| cfg-if | 1.0.5 | MIT OR Apache-2.0 |
| chrono | 0.4.45 | MIT OR Apache-2.0 |
| crc32fast | 1.5.2 | MIT OR Apache-2.0 |
| equivalent | 1.0.2 | Apache-2.0 OR MIT |
| fdeflate | 0.3.7 | MIT OR Apache-2.0 |
| flate2 | 1.1.10 | MIT OR Apache-2.0 |
| hashbrown | 0.17.1 | MIT OR Apache-2.0 |
| itoa | 1.0.18 | MIT OR Apache-2.0 |
| image | 0.25.10 | MIT OR Apache-2.0 |
| indexmap | 2.14.2 | Apache-2.0 OR MIT |
| log | 0.4.34 | MIT OR Apache-2.0 |
| memchr | 2.8.3 | Unlicense OR MIT |
| miniz_oxide | 0.8.9, 0.9.1 | MIT OR Zlib OR Apache-2.0 |
| moxcms | 0.8.1 | BSD-3-Clause OR Apache-2.0 |
| num-traits | 0.2.19 | MIT OR Apache-2.0 |
| png | 0.18.0 | MIT OR Apache-2.0 |
| proc-macro2 | 1.0.107 | MIT OR Apache-2.0 |
| pxfm | 0.1.30 | BSD-3-Clause OR Apache-2.0 |
| quote | 1.0.47 | MIT OR Apache-2.0 |
| serde | 1.0.229 | MIT OR Apache-2.0 |
| serde_core | 1.0.229 | MIT OR Apache-2.0 |
| serde_derive | 1.0.229 | MIT OR Apache-2.0 |
| serde_json | 1.0.151 | MIT OR Apache-2.0 |
| simd-adler32 | 0.3.10 | MIT |
| strict-num | 0.1.1 | MIT |
| syn | 2.0.119, 3.0.6 | MIT OR Apache-2.0 |
| tiny-skia | 0.12.0 | BSD-3-Clause |
| tiny-skia-path | 0.12.0 | BSD-3-Clause |
| typed-path | 0.12.3 | MIT OR Apache-2.0 |
| unicode-ident | 1.0.26 | (MIT OR Apache-2.0) AND Unicode-3.0 |
| uiautomation | 0.25.1 | Apache-2.0 |
| uiautomation_derive | 0.7.9 | Apache-2.0 |
| windows-link | 0.2.1 | MIT OR Apache-2.0 |
| windows | 0.62.2 | MIT OR Apache-2.0 |
| windows-collections | 0.3.2 | MIT OR Apache-2.0 |
| windows-core | 0.62.2 | MIT OR Apache-2.0 |
| windows-future | 0.3.2 | MIT OR Apache-2.0 |
| windows-implement | 0.60.2 | MIT OR Apache-2.0 |
| windows-interface | 0.59.3 | MIT OR Apache-2.0 |
| windows-numerics | 0.3.1 | MIT OR Apache-2.0 |
| windows-result | 0.4.1 | MIT OR Apache-2.0 |
| windows-strings | 0.5.1 | MIT OR Apache-2.0 |
| windows-threading | 0.2.1 | MIT OR Apache-2.0 |
| windows-sys | 0.61.2 | MIT OR Apache-2.0 |
| zlib-rs | 0.6.8 | Zlib |
| zip | 8.6.0 | MIT |
| zmij | 1.0.23 | MIT |
| zune-core | 0.5.3 | MIT OR Apache-2.0 OR Zlib |
| zune-jpeg | 0.5.15 | MIT OR Apache-2.0 OR Zlib |
