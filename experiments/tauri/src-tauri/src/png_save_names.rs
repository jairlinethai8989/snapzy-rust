use std::sync::atomic::{AtomicU64, Ordering};

pub struct PngSaveNames {
    run_stamp: String,
    next_number: AtomicU64,
}

impl PngSaveNames {
    pub fn new(run_stamp: String) -> Self {
        Self {
            run_stamp,
            next_number: AtomicU64::new(1),
        }
    }

    pub fn next(&self, stem: &str) -> String {
        // Leave room for the suffix even if the stem contains surrogate pairs.
        let stem = stem
            .char_indices()
            .nth(100)
            .map_or(stem, |(end, _)| &stem[..end]);
        let number = self.next_number.fetch_add(1, Ordering::Relaxed);
        format!("{stem}-{}-{number:03}.png", self.run_stamp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggestions_keep_one_sequence_across_documents_and_reset_for_a_new_run() {
        let first = PngSaveNames::new("20261109-143025-100".into());
        assert_eq!(first.next("จับภาพ"), "จับภาพ-20261109-143025-100-001.png");
        assert_eq!(first.next("Report"), "Report-20261109-143025-100-002.png");
        assert_eq!(first.next("จับภาพ"), "จับภาพ-20261109-143025-100-003.png");

        let reopened = PngSaveNames::new("20261109-143026-200".into());
        assert_eq!(reopened.next("จับภาพ"), "จับภาพ-20261109-143026-200-001.png");
    }

    #[test]
    fn suggested_name_fits_a_windows_filename_with_a_long_unicode_stem() {
        let names = PngSaveNames::new("20261109-143025-100".into());
        let name = names.next(&"📷".repeat(120));
        assert!(name.encode_utf16().count() <= 255);
        assert!(name.ends_with("-001.png"));
    }
}
