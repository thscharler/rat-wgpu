//! Fonts.
//!
//! Use [font_db] to get an instance of a [fontdb::Database].
//! Use that to query for installed fonts and then create the
//! [Font] with the [fontdb::ID] for the font.
//!
//! __Helpers__:
//! * [installed_mono_fonts]
//! * [load_font_by_family]
//! * [load_font_by_name]
//!
//! __Fallback__:
//! There are fallback fonts that can be embedded in your binary.
//! They are only added with the matching feature flag.
//! * [fallback_font], [fallback_family]
//! * [fallback_symbol_font]
//! * [fallback_emoji_font]

mod font;
mod fonts;
pub(crate) mod outline;
pub(crate) mod rasterize;

pub use font::{
    Font, fallback_emoji_font, fallback_family, fallback_font, fallback_symbol_font, font_db,
    installed_mono_fonts, load_font_by_family, load_font_by_name,
};
pub(crate) use fonts::Fonts;

/// Describes the dimensions of a text cell.
#[derive(Debug, Default, Clone, Copy)]
pub struct CellBox {
    /// Width in px.
    pub width: u32,
    /// Height in px.
    pub height: u32,
    /// Baseline for glyphs. Measured from the top of the box.
    pub ascender: u32,
}

impl CellBox {
    /// Pixel to cell-position. Rounded down.
    /// Out-of-bounds are clamped to 0..width/height - 1.
    pub(crate) fn cell_pos(
        &self,
        x: i32,
        y: i32,
        bounds: ratatui_core::layout::Size,
    ) -> ratatui_core::layout::Position {
        let cx = (x / self.width as i32).clamp(0, bounds.width.saturating_sub(1) as i32) as u16;
        let cy = (y / self.height as i32).clamp(0, bounds.height.saturating_sub(1) as i32) as u16;
        ratatui_core::layout::Position::new(cx, cy)
    }
}
