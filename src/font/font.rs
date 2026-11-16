#[cfg(any(feature = "fallback_font", feature = "fallback_symbol_font", feature = "fallback_emoji_font"))]
use fontdb::Source;
use rustybuzz::Face;
use rustybuzz::ttf_parser::{GlyphId, LineMetrics};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::sync::{Arc, OnceLock, RwLock};

/// Font data.
pub struct Font {
    font: fontdb::ID,
    face_data: Arc<dyn AsRef<[u8]>>,
    face_idx: u32,
    face_name: String,

    fallback: bool,

    // font metrics
    height: i16,
    ascender: i16,
    em_advance: u16,
    underline_metrics: Option<LineMetrics>,
    strikeout_metrics: Option<LineMetrics>,
    monospaced: bool,
    bold: bool,
    italic: bool,
    oblique: bool,

    // glyph caches
    glyph_id: RefCell<HashMap<char, Option<GlyphId>>>,
    glyph_hor_advance: RefCell<HashMap<GlyphId, Option<u16>>>,

    // rendered size
    height_px: u32,
    width_px: u32,
}

impl Debug for Font {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Font")
            .field("font", &self.font)
            .field("face_name", &self.face_name)
            .field("face_idx", &self.face_idx)
            .field("fallback", &self.fallback)
            .field("advance", &self.em_advance)
            .field("height", &self.height)
            .field("ascender", &self.ascender)
            .field("underline_metrics", &self.underline_metrics)
            .field("strikeout_metrics", &self.strikeout_metrics)
            .field("monospaced", &self.monospaced)
            .field("bold", &self.bold)
            .field("italic", &self.italic)
            .field("oblique", &self.oblique)
            .field("height_px", &self.height_px)
            .field("width_px", &self.width_px)
            .finish()
    }
}

impl Font {
    /// Create a new Font from data. Returns [`None`] if the font cannot
    /// be parsed.
    ///
    /// __Caution__: This requires a write lock for the `font_db()`. If you
    /// run a query at the same time this will deadlock.
    pub fn new(font: fontdb::ID) -> Option<Self> {
        let mut font_db = font_db().write().expect("font-db");
        let (face_data, face_idx) = unsafe { font_db.make_shared_face_data(font)? };

        let face = Face::from_slice((*face_data).as_ref(), face_idx)?;

        let face_name = face
            .names()
            .into_iter()
            .find_map(|v| {
                if v.name_id == ttf_parser::name_id::FULL_NAME {
                    let n = if v.is_unicode() {
                        v.to_string().unwrap_or_default()
                    } else {
                        String::from_utf8_lossy_owned(Vec::from(v.name))
                    };
                    Some(n)
                } else {
                    None
                }
            })
            .unwrap_or_default();

        let height = face.height();
        let ascender = face.ascender();
        let monospaced = face.is_monospaced();
        let bold = face.is_bold();
        let italic = face.is_italic();
        let oblique = face.is_oblique();
        let underline_metrics = face.underline_metrics();
        let strikeout_metrics = face.strikeout_metrics();

        let em_idx;
        let em_advance;
        if face.is_monospaced() {
            em_idx = face.glyph_index('m').unwrap_or_default();
            em_advance = face.glyph_hor_advance(em_idx).unwrap_or_default();
        } else {
            em_idx = face.glyph_index('n').unwrap_or_default();
            em_advance = face.glyph_hor_advance(em_idx).unwrap_or_default();
        }

        Some(Self {
            font,
            face_data,
            face_idx,
            face_name,
            fallback: false,
            em_advance,
            height,
            ascender,
            underline_metrics,
            strikeout_metrics,
            monospaced,
            bold,
            italic,
            oblique,
            glyph_id: Default::default(),
            glyph_hor_advance: Default::default(),
            height_px: 0,
            width_px: 0,
        })
    }

    /// Font ID
    pub fn id(&self) -> fontdb::ID {
        self.font
    }

    /// Font name
    pub fn face_name(&self) -> &str {
        self.face_name.as_ref()
    }

    /// Creates a rustybuzz::Face for the font-data.
    pub fn as_face<'a>(&'a self) -> Face<'a> {
        Face::from_slice((*self.face_data).as_ref(), self.face_idx).expect("font-face")
    }

    pub(crate) fn is_bold(&self) -> bool {
        self.bold
    }

    pub(crate) fn is_italic(&self) -> bool {
        self.italic
    }

    pub(crate) fn is_oblique(&self) -> bool {
        self.oblique
    }

    pub(crate) fn is_fallback(&self) -> bool {
        self.fallback
    }

    pub(crate) fn set_fallback(&mut self, fallback: bool) {
        self.fallback = fallback;
    }

    /// Is there a glyph for the given character.
    ///
    /// __Note__
    ///
    /// Caches the answer.
    pub(crate) fn have_glyph(&self, ch: char) -> bool {
        {
            let glyph_index = self.glyph_id.borrow();
            if let Some(glyph_id) = glyph_index.get(&ch) {
                return glyph_id.is_some();
            }
        }

        let mut glyph_index = self.glyph_id.borrow_mut();
        if let Some(glyph_id) = self.as_face().glyph_index(ch) {
            glyph_index.insert(ch, Some(glyph_id));
            true
        } else {
            glyph_index.insert(ch, None);
            false
        }
    }

    /// Get the horizontal advance for the given glyph.
    ///
    /// __Note__
    ///
    /// Caches the answer.
    pub(crate) fn glyph_hor_advance(&self, glyph_id: GlyphId) -> Option<u16> {
        {
            let glyph_hor_advance = self.glyph_hor_advance.borrow();
            if let Some(advance) = glyph_hor_advance.get(&glyph_id) {
                return *advance;
            }
        }

        let mut glyph_advance = self.glyph_hor_advance.borrow_mut();
        if let Some(advance) = self.as_face().glyph_hor_advance(glyph_id) {
            glyph_advance.insert(glyph_id, Some(advance));
            Some(advance)
        } else {
            glyph_advance.insert(glyph_id, None);
            None
        }
    }

    /// Active font height(px).
    pub(crate) fn set_height_px(&mut self, height_px: u32) {
        self.height_px = height_px;
    }

    /// Active font-width(px).
    pub(crate) fn set_width_px(&mut self, width_px: u32) {
        self.width_px = width_px;
    }

    /// Standard em width for the font for the active font-height(px).
    pub(crate) fn em_advance_px(&self) -> u32 {
        (self.em_advance as f32 * self.height_px as f32 / self.height as f32) as u32
    }

    /// Ascender for the font for the active font-height(px).
    pub(crate) fn ascender_px(&self) -> u32 {
        (self.ascender as f32 * self.height_px as f32 / self.height as f32) as u32
    }

    /// Font scale factor for the active font-height(px)
    /// Scales this font to fit in the overall cell-box.
    pub(crate) fn scale_y(&self, _glyph_id: u16, block_char: bool) -> f32 {
        if self.fallback && block_char {
            self.height_px as f32 / self.height as f32
        } else if self.fallback {
            self.height_px as f32 / self.height as f32
        } else if !self.monospaced {
            self.height_px as f32 / self.height as f32
        } else {
            self.height_px as f32 / self.height as f32
        }
    }

    /// Font scale factor for the active font-height(px) and font-width(px).
    /// Scales this font to fit in the overall cell-box.
    pub(crate) fn scale_x(
        &self,
        glyph_id: GlyphId,
        block_char: bool,
        char_width: u32,
    ) -> Option<f32> {
        let scale = if self.fallback && block_char {
            // fit vertically
            self.height_px as f32 / self.height as f32
        } else if self.fallback {
            let actual_width = self.glyph_hor_advance(glyph_id)?;

            // fit either horizontally or vertically.
            // preserve aspect.
            let scale_x = (self.width_px * char_width) as f32 / actual_width as f32;
            let scale_y = self.height_px as f32 / self.height as f32;

            if scale_x / scale_y > 1.0 {
                scale_y
            } else {
                scale_x
            }
        } else if !self.monospaced {
            let actual_width = self.glyph_hor_advance(glyph_id)?;

            let scale_x = (self.width_px * char_width) as f32 / actual_width as f32;
            let scale_y = self.height_px as f32 / self.height as f32;

            if scale_x / scale_y > 1.0 {
                scale_y
            } else {
                scale_x
            }
        } else {
            // regular fonts will probably be from one font family and therefore have
            // more regular properties.

            self.height_px as f32 / self.height as f32
        };

        Some(scale)
    }

    /// Returns the underline metrics for the parameters.
    /// Gives to y coordinates for start and end of the underline.
    pub(crate) fn underline_metrics(&self, ascender: u32, box_height_px: u32) -> (u32, u32) {
        let font_ascender = self.ascender as f32;
        let underline_position = self
            .underline_metrics
            .map(|m| m.position as f32)
            .unwrap_or(0.0);
        let mut underline_percent = underline_position / font_ascender;
        // hack 9902
        if underline_percent > 0.0 {
            underline_percent = -0.05;
        }

        let underline_thickness = self
            .underline_metrics
            .map(|m| m.thickness as f32)
            .unwrap_or(100.0); /* observed average */
        // default underlines are a bit thin for larger font-sizes.
        let underline_thickness = underline_thickness * 1.3;
        let underline_thickness_percent = underline_thickness / font_ascender;

        let underline_position = ascender - (ascender as f32 * underline_percent) as u32;
        let underline_thickness = ((ascender as f32 * underline_thickness_percent) as u32).max(1);

        // might overflow the box
        if underline_position + underline_thickness < box_height_px {
            (underline_position, underline_position + underline_thickness)
        } else {
            (
                box_height_px.saturating_sub(underline_thickness),
                box_height_px,
            )
        }
    }

    /// Returns the strikeout metrics for the parameters.
    /// Gives to y coordinates for start and end of the strikeout.
    pub(crate) fn strikeout_metrics(&self, ascender: u32) -> (u32, u32) {
        let font_ascender = self.ascender as f32;

        let strikeout_position = self
            .strikeout_metrics
            .map(|m| m.position as f32)
            .unwrap_or_default();
        let strikeout_position = if strikeout_position > 0.0 {
            strikeout_position
        } else {
            font_ascender * 0.3 /* observed average */
        };
        let strikeout_percent = strikeout_position / font_ascender;

        let strikeout_thickness = self
            .strikeout_metrics
            .map(|m| m.thickness as f32)
            .unwrap_or(100.0); /* observed average */
        // default strikeout lines are a bit thin for larger font-sizes.
        let strikeout_thickness = strikeout_thickness * 1.8;
        let strikeout_thickness_percent = strikeout_thickness / font_ascender;

        let strikeout_position = ascender - (ascender as f32 * strikeout_percent) as u32;
        let strikeout_thickness = ((ascender as f32 * strikeout_thickness_percent) as u32).max(1);

        (strikeout_position, strikeout_position + strikeout_thickness)
    }
}

/// Some fallback font data.
#[cfg(feature = "fallback_font")]
static FALLBACK_DATA: &[u8] = include_bytes!("CascadiaMono-Regular.ttf");
#[cfg(feature = "fallback_symbol_font")]
static SYMBOL_DATA: &[u8] = include_bytes!("NotoSansSymbols2-Regular.ttf");
#[cfg(feature = "fallback_emoji_font")]
static EMOJI_DATA: &[u8] = include_bytes!("NotoEmoji-Regular.ttf");

static FONTDB: OnceLock<RwLock<fontdb::Database>> = OnceLock::new();

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
pub fn fallback_font() -> Option<Font> {
    let ids = {
        let mut font_db = font_db().write().expect("font_db");
        font_db.load_font_source(Source::Binary(Arc::new(FALLBACK_DATA)))
    };
    Font::new(ids[0])
}

/// Returns the font family name for the fallback font.
#[cfg(feature = "fallback_font")]
pub fn fallback_family() -> Option<String> {
    Some("CascadiaMono".to_string())
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
pub fn fallback_font() -> Option<Font> {
    None
}

/// Returns the font family name for the fallback font.
#[cfg(not(feature = "fallback_font"))]
pub fn fallback_family() -> Option<String> {
    None
}

/// Returns a fallback font for extra emojis.
/// This only works with the feature `fallback_emoji_font`.
///
/// __Info__
///
/// This adds 1.4MB to your binary size.
#[cfg(feature = "fallback_emoji_font")]
pub fn fallback_emoji_font() -> Option<Font> {
    let ids = {
        let mut font_db = font_db().write().expect("font_db");
        font_db.load_font_source(Source::Binary(Arc::new(EMOJI_DATA)))
    };
    Font::new(ids[0])
}

/// Returns a fallback font for extra emojis.
/// This only works with the feature `fallback_emoji_font`.
///
/// __Info__
///
/// This adds 1.4MB to your binary size.
#[cfg(not(feature = "fallback_emoji_font"))]
pub fn fallback_emoji_font() -> Option<Font> {
    None
}

/// Returns a fallback font for extra symbols.
/// This only works with the feature `fallback_symbol_font`.
///
/// __Info__
///
/// This adds 1.2MB to your binary size.
#[cfg(feature = "fallback_symbol_font")]
pub fn fallback_symbol_font() -> Option<Font> {
    let ids = {
        let mut font_db = font_db().write().expect("font_db");
        font_db.load_font_source(Source::Binary(Arc::new(SYMBOL_DATA)))
    };
    Font::new(ids[0])
}

/// Returns a fallback font for extra symbols.
/// This only works with the feature `fallback_symbol_font`.
///
/// __Info__
///
/// This adds 1.2MB to your binary size.
#[cfg(not(feature = "fallback_symbol_font"))]
pub fn fallback_symbol_font() -> Option<Font> {
    None
}

/// Gets an instance of the fontdb.
/// This loads the installed system fonts with the first call.
pub fn font_db() -> &'static RwLock<fontdb::Database> {
    FONTDB.get_or_init(|| {
        let mut font_db = fontdb::Database::new();
        font_db.load_system_fonts();
        RwLock::new(font_db)
    })
}

/// Gives a list of installed monospaced fonts.
/// This only returns the font-families.
pub fn installed_mono_fonts() -> Vec<String> {
    let font_db = font_db().read().expect("font-db");

    let mut fonts = font_db
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
}

/// Load the fonts for the given font family.
pub fn load_font_by_family(family: &str) -> Vec<Font> {
    let font_ids = {
        let font_db = font_db().read().expect("font-db");
        font_db
            .faces()
            .filter_map(|info| {
                for (v, _) in &info.families {
                    if v.as_str() == family {
                        return Some(info.id);
                    }
                }
                None
            })
            .collect::<Vec<_>>()
    };

    font_ids.into_iter().filter_map(|v| Font::new(v)).collect()
}

/// Load a specific font by exact name.
pub fn load_font_by_name(name: &str) -> Option<Font> {
    let font_id = {
        let font_db = font_db().read().expect("font_db");
        font_db
            .faces()
            .find(|info| info.post_script_name == name)
            .map(|info| info.id)
    };

    if let Some(font_id) = font_id {
        Font::new(font_id)
    } else {
        None
    }
}
