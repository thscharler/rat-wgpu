//! Fonts for rat-wgpu.
//!
//! Use font_db() to get an instance of a fontdb::Database.
//! Use this to query for installed fonts and create the
//! font with the fontdb::ID for the font.

mod font;
mod fonts;
pub(crate) mod outline;
pub(crate) mod rasterize;

pub use font::{
    Font, fallback_emoji_font, fallback_family, fallback_font, fallback_symbol_font, font_db,
    installed_mono_fonts, load_font_by_family, load_font_by_name,
};
pub(crate) use fonts::Fonts;
