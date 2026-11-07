use relm4::gtk;

pub const WIDGET_NONE: Option<gtk::Widget> = None;

pub const PAGE_MARGIN: i32 = 12;

pub const MAX_LIBRARY_WIDTH: i32 = 1280;

// Shared movie/discovery grid geometry. Keep both surfaces in step.
pub const POSTER_WIDTH: i32 = 192;
pub const POSTER_HEIGHT: i32 = 288;
pub const POSTER_COLUMNS: u32 = 6;
pub const POSTER_COLUMN_SPACING: u32 = 16;
pub const POSTER_ROW_SPACING: u32 = 32;
pub const POSTER_TITLE_HEIGHT: i32 = 38;
