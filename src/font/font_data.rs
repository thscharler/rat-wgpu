/// Font finding and loading.
///
/// Keeps the font-data in some static datastructures.
///
use crate::font::font::Font;
use append_only_vec::AppendOnlyVec;
use std::sync::OnceLock;

/// Some fallback font data.
#[cfg(feature = "fallback_font")]
static FALLBACK_DATA: &[u8] = include_bytes!("NotoSansMono-Regular.ttf");
#[cfg(feature = "fallback_font")]
static FALLBACK_FONT: OnceLock<Font<'static>> = OnceLock::new();
#[cfg(feature = "fallback_symbol_font")]
static SYMBOL_DATA: &[u8] = include_bytes!("NotoSansSymbols2-Regular.ttf");
#[cfg(feature = "fallback_symbol_font")]
static SYMBOL_FONT: OnceLock<Font<'static>> = OnceLock::new();
#[cfg(feature = "fallback_emoji_font")]
static EMOJI_DATA: &[u8] = include_bytes!("NotoEmoji-Regular.ttf");
#[cfg(feature = "fallback_emoji_font")]
static EMOJI_FONT: OnceLock<Font<'static>> = OnceLock::new();

static FONTDB: OnceLock<fontdb::Database> = OnceLock::new();
static FONT_DATA: AppendOnlyVec<(fontdb::ID, Box<[u8]>)> = AppendOnlyVec::new();
static FONTS: AppendOnlyVec<(fontdb::ID, Font<'static>)> = AppendOnlyVec::new();
static INSTALLED_MONOSPACED: OnceLock<Vec<String>> = OnceLock::new();

/// Uses fontdb::Database as backend and manages font loading to
/// some static datastructures.
pub struct FontData;

impl FontData {
    /// Returns the embedded fallback font.
    /// This only wokrs with the feature `fallback_font`.
    /// Which is included in the default-features.
    ///
    /// If you want to use your own fallback font deactivate
    /// the feature.
    ///
    /// __Info__
    ///
    /// This adds 560KB to your binary size.
    #[cfg(feature = "fallback_font")]
    pub fn fallback_font(self) -> Option<Font<'static>> {
        Some(
            FALLBACK_FONT
                .get_or_init(|| Font::new(FALLBACK_DATA).expect("valid_font"))
                .clone(),
        )
    }

    /// Returns the embedded fallback font.
    /// This only works with the feature `fallback_font`.
    /// Which is included in the default-features.
    ///
    /// If you want to use your own fallback font deactivate
    /// the feature.
    ///
    /// __Info__
    ///
    /// This adds 560KB to your binary size.
    #[cfg(not(feature = "fallback_font"))]
    pub fn fallback_font(self) -> Option<Font<'static>> {
        None
    }

    /// Returns a fallback font for extra emojis.
    /// This only works with the feature `fallback_emoji_font`.
    ///
    /// __Info__
    ///
    /// This adds 1.4MB to your binary size.
    #[cfg(feature = "fallback_emoji_font")]
    pub fn fallback_emoji_font(self) -> Option<Font<'static>> {
        Some(
            EMOJI_FONT
                .get_or_init(|| Font::new(EMOJI_DATA).expect("valid_font"))
                .clone(),
        )
    }

    /// Returns a fallback font for extra emojis.
    /// This only works with the feature `fallback_emoji_font`.
    ///
    /// __Info__
    ///
    /// This adds 1.4MB to your binary size.
    #[cfg(not(feature = "fallback_emoji_font"))]
    pub fn fallback_emoji_font(self) -> Option<Font<'static>> {
        None
    }

    /// Returns a fallback font for extra symbols.
    /// This only works with the feature `fallback_symbol_font`.
    ///
    /// __Info__
    ///
    /// This adds 1.2MB to your binary size.
    #[cfg(feature = "fallback_symbol_font")]
    pub fn fallback_symbol_font(self) -> Option<Font<'static>> {
        Some(
            SYMBOL_FONT
                .get_or_init(|| Font::new(SYMBOL_DATA).expect("valid_font"))
                .clone(),
        )
    }

    /// Returns a fallback font for extra symbols.
    /// This only works with the feature `fallback_symbol_font`.
    ///
    /// __Info__
    ///
    /// This adds 1.2MB to your binary size.
    #[cfg(not(feature = "fallback_symbol_font"))]
    pub fn fallback_symbol_font(self) -> Option<Font<'static>> {
        None
    }

    /// Gets an instance of the fontdb.
    /// This loads the installed system fonts on init.
    pub fn font_db(self) -> &'static fontdb::Database {
        FONTDB.get_or_init(|| {
            let mut font_db = fontdb::Database::new();
            font_db.load_system_fonts();
            font_db
        })
    }

    /// Gives a list of installed monospaced fonts.
    /// This only returns the font-families.
    pub fn installed_fonts(self) -> &'static Vec<String> {
        INSTALLED_MONOSPACED.get_or_init(|| {
            let mut fonts = self
                .font_db()
                .faces()
                .flat_map(|info| {
                    if info.monospaced {
                        info.families
                            .iter()
                            .map(|(family, _)| family.clone())
                            .collect::<Vec<_>>()
                    } else {
                        Vec::default()
                    }
                })
                .collect::<Vec<_>>();
            fonts.sort();
            fonts.dedup();
            // todo: temporary hack
            if let Some(pos) = fonts.iter().position(|v| v.as_str() == "Lucida Console") {
                fonts.remove(pos);
            }
            if let Some(pos) = fonts.iter().position(|v| v.as_str() == "NSimSun") {
                fonts.remove(pos);
            }
            if let Some(pos) = fonts.iter().position(|v| v.as_str() == "新宋体") {
                fonts.remove(pos);
            }
            if let Some(pos) = fonts.iter().position(|v| v.as_str() == "SimSun-ExtB") {
                fonts.remove(pos);
            }
            if let Some(pos) = fonts.iter().position(|v| v.as_str() == "SimSun-ExtG") {
                fonts.remove(pos);
            }
            fonts
        })
    }

    /// Font already cached?
    pub fn have_font(self, id: fontdb::ID) -> bool {
        for (font_id, _) in FONTS.iter() {
            if id == *font_id {
                return true;
            }
        }
        false
    }

    /// Load a specific font by exact name.
    pub fn load_font_by_name(self, name: &str) -> Option<Font<'static>> {
        if let Some(font) = self
            .font_db()
            .faces()
            .filter(|info| info.post_script_name == name)
            .next()
        {
            Self.load_font(font.id)
        } else {
            None
        }
    }

    /// Create a Font and cache the underlying data.
    pub fn load_font(self, id: fontdb::ID) -> Option<Font<'static>> {
        for (font_id, font) in FONTS.iter() {
            if id == *font_id {
                return Some(font.clone());
            }
        }

        let data = self
            .font_db()
            .with_face_data(id, |d, _| d.to_vec())
            .expect("font_data");
        let idx = FONT_DATA.push((id, data.into_boxed_slice()));
        let (_, data) = &FONT_DATA[idx];

        let font = Font::new(data).expect("valid-font");
        FONTS.push((id, font.clone()));

        Some(font)
    }
}
