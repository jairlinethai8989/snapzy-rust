pub const IMAGE_LIMIT: usize = 128 * 1024 * 1024;
pub const SOURCE_LIMIT: usize = 256 * 1024 * 1024;
pub const INPUT_FILE_LIMIT: usize = 256 * 1024 * 1024;
pub const SCROLL_WORK_LIMIT: usize = 384 * 1024 * 1024;
pub const MEMORY_RESERVE: u64 = 512 * 1024 * 1024;

pub fn scroll_output_budget(available_physical: u64, workspace_remaining: usize) -> usize {
    (available_physical
        .saturating_sub(MEMORY_RESERVE)
        .min(IMAGE_LIMIT as u64) as usize)
        .min(workspace_remaining)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_budget_obeys_image_workspace_and_system_memory_limits() {
        let plenty = MEMORY_RESERVE + (IMAGE_LIMIT as u64) * 2;
        assert_eq!(scroll_output_budget(plenty, SOURCE_LIMIT), IMAGE_LIMIT);
        assert_eq!(
            scroll_output_budget(plenty, 48 * 1024 * 1024),
            48 * 1024 * 1024
        );
        assert_eq!(
            scroll_output_budget(MEMORY_RESERVE + 64 * 1024 * 1024, SOURCE_LIMIT),
            64 * 1024 * 1024
        );
        assert_eq!(scroll_output_budget(MEMORY_RESERVE, SOURCE_LIMIT), 0);
    }
}
