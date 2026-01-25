use rustybuzz::Face;
use rustybuzz::ttf_parser::GlyphId;

/// A Font which can be used for rendering.
#[derive(Clone)]
pub struct Font<'a> {
    font: Face<'a>,
    fallback: bool,
    advance: f32,
    height_px: u32,
    width_px: u32,
    id: u64,
}

impl<'a> Font<'a> {
    /// Create a new Font from data. Returns [`None`] if the font cannot
    /// be parsed.
    pub fn new(data: &'a [u8]) -> Option<Self> {
        Face::from_slice(data, 0).map(|font| {
            let em_idx;
            let advance;
            if font.is_monospaced() {
                em_idx = font.glyph_index('m').unwrap_or_default();
                advance = font.glyph_hor_advance(em_idx).unwrap_or_default() as f32;
            } else {
                em_idx = font.glyph_index('n').unwrap_or_default();
                advance = font.glyph_hor_advance(em_idx).unwrap_or_default() as f32;
            }

            Self {
                font,
                fallback: false,
                advance,
                height_px: 0,
                width_px: 0,
                id: 0,
            }
        })
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn set_id(&mut self, id: u64) {
        self.id = id;
    }

    pub fn face(&'_ self) -> &'_ Face<'_> {
        &self.font
    }

    pub fn into_face(self) -> Face<'a> {
        self.font
    }

    pub(crate) fn is_fallback(&self) -> bool {
        self.fallback
    }

    pub(crate) fn set_fallback(&mut self, fallback: bool) {
        self.fallback = fallback;
    }

    pub(crate) fn ascender(&self) -> u32 {
        (self.font.ascender() as f32 * self.height_px as f32 / self.font.height() as f32) as u32
    }

    // Base advance. Or em (or en).
    pub(crate) fn em_advance(&self) -> f32 {
        self.advance
    }

    // Active font height.
    pub(crate) fn set_height_px(&mut self, height_px: u32) {
        self.height_px = height_px;
    }

    // Active font width.
    pub(crate) fn set_width_px(&mut self, width_px: u32) {
        self.width_px = width_px;
    }

    // Base width, preserving the aspect ratio of the font.
    pub(crate) fn base_width_px(&self) -> u32 {
        (self.advance * self.height_px as f32 / self.font.height() as f32) as u32
    }

    pub(crate) fn scale_y(&self, _glyph_id: u16, block_char: bool) -> f32 {
        if self.fallback && block_char {
            self.height_px as f32 / self.font.height() as f32
        } else if self.fallback {
            self.height_px as f32 / self.font.height() as f32
        } else if !self.font.is_monospaced() {
            self.height_px as f32 / self.font.height() as f32
        } else {
            self.height_px as f32 / self.font.height() as f32
        }
    }

    pub(crate) fn scale_x(&self, glyph_id: u16, block_char: bool, char_width: u32) -> f32 {
        if self.fallback && block_char {
            // fit vertically
            self.height_px as f32 / self.font.height() as f32
        } else if self.fallback {
            let actual_width = self
                .font
                .glyph_hor_advance(GlyphId(glyph_id))
                .unwrap_or_default();

            // fit either horizontally or vertically.
            // preserve aspect.
            let scale_x = (self.width_px * char_width) as f32 / actual_width as f32;
            let scale_y = self.height_px as f32 / self.font.height() as f32;

            if scale_x / scale_y > 1.0 {
                scale_y
            } else {
                scale_x
            }
        } else if !self.font.is_monospaced() {
            let actual_width = self
                .font
                .glyph_hor_advance(GlyphId(glyph_id))
                .unwrap_or_default();
            let scale_x = (self.width_px * char_width) as f32 / actual_width as f32;
            let scale_y = self.height_px as f32 / self.font.height() as f32;

            if scale_x / scale_y > 1.0 {
                scale_y
            } else {
                scale_x
            }
        } else {
            // regular fonts will probably be from one font family and therefore have
            // more regular properties.

            self.height_px as f32 / self.font.height() as f32
        }
    }

    pub(crate) fn underline_metrics(&self, ascender: u32, box_height_px: u32) -> (u32, u32) {
        let font_ascender = self.font.ascender() as f32;
        let underline_position = self
            .font
            .underline_metrics()
            .map(|m| m.position as f32)
            .unwrap_or(0.0);
        let mut underline_percent = underline_position / font_ascender;
        // hack 9902
        if underline_percent > 0.0 {
            underline_percent = -0.05;
        }

        let underline_thickness = self
            .font
            .underline_metrics()
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

    pub(crate) fn strikeout_metrics(&self, ascender: u32) -> (u32, u32) {
        let font_ascender = self.font.ascender() as f32;

        let strikeout_position = self
            .font
            .strikeout_metrics()
            .map(|m| m.position as f32)
            .unwrap_or_default();
        let strikeout_position = if strikeout_position > 0.0 {
            strikeout_position
        } else {
            font_ascender * 0.3 /* observed average */
        };
        let strikeout_percent = strikeout_position / font_ascender;

        let strikeout_thickness = self
            .font
            .strikeout_metrics()
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
