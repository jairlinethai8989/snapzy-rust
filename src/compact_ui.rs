#![cfg_attr(not(test), allow(dead_code))]

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Size {
    pub width: i32,
    pub height: i32,
}
impl Size {
    pub const fn new(width: i32, height: i32) -> Self {
        Self { width, height }
    }
    pub fn contains(self, rect: Rect) -> bool {
        rect.x >= 0 && rect.y >= 0 && rect.right() <= self.width && rect.bottom() <= self.height
    }
    pub fn to_physical(self, dpi: u32) -> Self {
        Self::new(scale(self.width, dpi), scale(self.height, dpi))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}
impl Rect {
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }
    pub const fn right(self) -> i32 {
        self.x + self.width
    }
    pub const fn bottom(self) -> i32 {
        self.y + self.height
    }
    pub fn to_physical(self, dpi: u32) -> Self {
        let left = scale(self.x, dpi);
        let top = scale(self.y, dpi);
        let right = scale(self.right(), dpi);
        let bottom = scale(self.bottom(), dpi);
        Self::new(left, top, right - left, bottom - top)
    }
}

fn scale(value: i32, dpi: u32) -> i32 {
    ((value as i64 * dpi as i64 + 48) / 96) as i32
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LauncherControl {
    Area,
    Window,
    WindowMenu,
    Scrolling,
    Video,
    Language,
    More,
    OpenImage,
    Delay,
    ScrollingMode,
    OpenMode,
}

#[derive(Debug, PartialEq, Eq)]
pub struct LauncherLayout {
    pub client: Size,
    pub primary: Vec<LauncherControl>,
    controls: Vec<(LauncherControl, Rect)>,
}
impl LauncherLayout {
    pub fn rect(&self, control: LauncherControl) -> Rect {
        self.controls
            .iter()
            .find(|(id, _)| *id == control)
            .map(|(_, rect)| *rect)
            .unwrap_or(Rect::new(0, 0, 0, 0))
    }
    pub fn all_rects(&self) -> impl Iterator<Item = Rect> + '_ {
        self.controls.iter().map(|(_, rect)| *rect)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditorAction {
    Language,
    Help,
    Ocr,
    AddImage,
    Combine,
    OpenProject,
    SaveProject,
    Copy,
    SavePng,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutError {
    TooNarrow,
}

#[derive(Debug, PartialEq, Eq)]
pub struct EditorLayout {
    pub client: Size,
    pub direct_actions: Vec<EditorAction>,
    pub more_actions: Vec<EditorAction>,
    bands: Vec<Rect>,
    controls: Vec<Rect>,
}
impl EditorLayout {
    pub fn total_chrome_height(&self) -> i32 {
        self.bands.iter().map(|r| r.height).sum()
    }
    pub fn all_rects(&self) -> impl Iterator<Item = Rect> + '_ {
        self.controls.iter().copied()
    }
    pub fn band_rects(&self) -> impl Iterator<Item = Rect> + '_ {
        self.bands.iter().copied()
    }
}

pub fn launcher_layout() -> LauncherLayout {
    let controls = vec![
        (LauncherControl::OpenImage, Rect::new(262, 4, 28, 28)),
        (LauncherControl::Language, Rect::new(294, 4, 28, 28)),
        (LauncherControl::More, Rect::new(326, 4, 28, 28)),
        (LauncherControl::Area, Rect::new(6, 38, 82, 48)),
        (LauncherControl::Window, Rect::new(94, 38, 62, 48)),
        (LauncherControl::WindowMenu, Rect::new(156, 38, 20, 48)),
        (LauncherControl::Scrolling, Rect::new(182, 38, 82, 48)),
        (LauncherControl::Video, Rect::new(270, 38, 84, 48)),
        (LauncherControl::Delay, Rect::new(6, 94, 104, 28)),
        (LauncherControl::ScrollingMode, Rect::new(114, 94, 112, 28)),
        (LauncherControl::OpenMode, Rect::new(230, 94, 124, 28)),
    ];
    LauncherLayout {
        client: Size::new(360, 140),
        primary: vec![
            LauncherControl::Area,
            LauncherControl::Window,
            LauncherControl::Scrolling,
            LauncherControl::Video,
        ],
        controls,
    }
}

pub fn editor_layout(width: i32, tabbed: bool) -> Result<EditorLayout, LayoutError> {
    const MIN_WIDTH: i32 = 780;
    const WIDE_WIDTH: i32 = 1000;
    if width < MIN_WIDTH {
        return Err(LayoutError::TooNarrow);
    }

    let mut y = 0;
    let mut bands = Vec::with_capacity(if tabbed { 4 } else { 3 });
    if tabbed {
        bands.push(Rect::new(0, y, width, 25));
        y += 25;
    }
    let work_y = y;
    bands.push(Rect::new(0, y, width, 38));
    y += 38;
    let tools_y = y;
    bands.push(Rect::new(0, y, width, 34));
    y += 34;
    let context_y = y;
    bands.push(Rect::new(0, y, width, 32));
    y += 32;

    let secondary = [
        EditorAction::AddImage,
        EditorAction::Combine,
        EditorAction::OpenProject,
        EditorAction::SaveProject,
    ];
    let mut direct_actions = vec![
        EditorAction::Language,
        EditorAction::Help,
        EditorAction::Ocr,
    ];
    let mut more_actions = Vec::new();
    if width >= WIDE_WIDTH {
        direct_actions.extend(secondary);
    } else {
        more_actions.extend(secondary);
    }
    direct_actions.extend([EditorAction::Copy, EditorAction::SavePng]);

    let mut controls = Vec::new();
    let mut right = width - 6;
    for action in direct_actions.iter().rev() {
        let control_width = match action {
            EditorAction::SavePng => 88,
            EditorAction::Copy => 42,
            _ => 34,
        };
        right -= control_width;
        controls.push(Rect::new(right, work_y + 3, control_width, 32));
        right -= 4;
    }

    // Stable tool cells prevent selected and hover states from moving the toolbar.
    for index in 0..18 {
        controls.push(editor_tool_rect(index, tools_y));
    }
    controls.extend([
        Rect::new(6, context_y + 3, 118, 26),
        Rect::new(130, context_y + 3, 82, 26),
        Rect::new(218, context_y + 3, 104, 26),
        Rect::new(328, context_y + 3, 104, 26),
    ]);

    Ok(EditorLayout {
        client: Size::new(width, y + 451),
        direct_actions,
        more_actions,
        bands,
        controls,
    })
}

pub fn editor_tool_rect(index: usize, tools_y: i32) -> Rect {
    let mut x = 6;
    for previous in 0..index {
        x += if matches!(previous, 7 | 8) { 40 } else { 32 };
        x += if matches!(previous, 2 | 4 | 8 | 12 | 14 | 15) {
            7
        } else {
            3
        };
    }
    Rect::new(
        x,
        tools_y + 3,
        if matches!(index, 7 | 8) { 40 } else { 32 },
        28,
    )
}

pub fn has_overlaps(rects: impl Iterator<Item = Rect>) -> bool {
    let values = rects.collect::<Vec<_>>();
    values.iter().enumerate().any(|(index, a)| {
        values
            .iter()
            .skip(index + 1)
            .any(|b| a.x < b.right() && a.right() > b.x && a.y < b.bottom() && a.bottom() > b.y)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launcher_is_compact_and_every_control_is_visible_without_overlap() {
        let layout = launcher_layout();
        assert_eq!(layout.client, Size::new(360, 140));
        assert_eq!(
            layout.primary,
            vec![
                LauncherControl::Area,
                LauncherControl::Window,
                LauncherControl::Scrolling,
                LauncherControl::Video,
            ]
        );
        assert!(!layout.primary.contains(&LauncherControl::WindowMenu));
        assert_eq!(
            layout.rect(LauncherControl::Window).right(),
            layout.rect(LauncherControl::WindowMenu).x
        );
        assert!(layout.rect(LauncherControl::OpenImage).width > 0);
        assert!(layout.rect(LauncherControl::Delay).width > 0);
        assert!(layout.rect(LauncherControl::ScrollingMode).width > 0);
        assert!(layout.all_rects().all(|rect| layout.client.contains(rect)));
        assert!(!has_overlaps(layout.all_rects()));
    }

    #[test]
    fn editor_keeps_core_actions_direct_and_moves_secondary_actions_at_minimum_width() {
        let layout = editor_layout(780, true).unwrap();
        assert_eq!(layout.total_chrome_height(), 129);
        for action in [
            EditorAction::Language,
            EditorAction::Help,
            EditorAction::Ocr,
            EditorAction::Copy,
            EditorAction::SavePng,
        ] {
            assert!(
                layout.direct_actions.contains(&action),
                "missing {action:?}"
            );
        }
        for action in [
            EditorAction::AddImage,
            EditorAction::Combine,
            EditorAction::OpenProject,
            EditorAction::SaveProject,
        ] {
            assert!(
                layout.more_actions.contains(&action),
                "not moved: {action:?}"
            );
        }
        assert!(layout.all_rects().all(|rect| layout.client.contains(rect)));
        assert!(!has_overlaps(layout.band_rects()));
    }

    #[test]
    fn wide_editor_promotes_secondary_actions_without_changing_band_heights() {
        let narrow = editor_layout(780, false).unwrap();
        let wide = editor_layout(1100, false).unwrap();
        assert_eq!(narrow.total_chrome_height(), 104);
        assert_eq!(wide.total_chrome_height(), 104);
        for action in [
            EditorAction::AddImage,
            EditorAction::Combine,
            EditorAction::OpenProject,
            EditorAction::SaveProject,
        ] {
            assert!(wide.direct_actions.contains(&action), "missing {action:?}");
            assert!(
                !wide.more_actions.contains(&action),
                "duplicated {action:?}"
            );
        }
    }

    #[test]
    fn editor_rejects_width_below_supported_minimum() {
        assert_eq!(editor_layout(779, true), Err(LayoutError::TooNarrow));
    }

    #[test]
    fn dpi_scaling_keeps_every_rect_inside_the_scaled_client() {
        for dpi in [96, 120, 144, 192] {
            let layout = editor_layout(780, true).unwrap();
            let client = layout.client.to_physical(dpi);
            for rect in layout.all_rects() {
                assert!(
                    client.contains(rect.to_physical(dpi)),
                    "dpi={dpi} rect={rect:?}"
                );
            }
        }
    }

    #[test]
    fn tool_groups_leave_room_for_shape_arrows_at_every_supported_dpi() {
        for dpi in [96, 120, 144, 192] {
            let cells: Vec<_> = (0..18)
                .map(|index| editor_tool_rect(index, 38).to_physical(dpi))
                .collect();
            assert!(!has_overlaps(cells.iter().copied()));
            for cell in &cells {
                assert!(Size::new(780, 104).to_physical(dpi).contains(*cell));
            }
            for index in [7, 8] {
                assert!(cells[index].width >= scale(24 + 12, dpi));
            }
        }
    }
}
