use rustybuzz::Face;

/// A Font which can be used for rendering.
#[derive(Clone)]
pub struct Font<'a> {
    pub(crate) font: Face<'a>,
    pub(crate) fallback: bool,
    pub(crate) advance: f32,
    pub(crate) id: u64,
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
                id: 0,
            }
        })
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

    pub(crate) fn ascender(&self) -> f32 {
        self.font.ascender() as f32
    }

    pub(crate) fn em_advance(&self) -> f32 {
        self.advance
    }

    pub(crate) fn scale(&self, height_px: u32) -> f32 {
        height_px as f32 / self.font.height() as f32
    }

    pub(crate) fn char_width(&self, height_px: u32) -> u32 {
        (self.advance * self.scale(height_px)) as u32
    }

    pub(crate) fn underline_metrics(&self, height_px: u32, box_height_px: u32) -> (u32, u32) {
        let scale = self.scale(height_px);

        let ascender = self.font.ascender() as f32;

        let underline_position = self
            .font
            .underline_metrics()
            .map(|m| m.position as f32)
            .unwrap_or(0.0);
        let underline_position = ascender - underline_position;

        let underline_thickness = self
            .font
            .underline_metrics()
            .map(|m| m.thickness as f32)
            .unwrap_or(100.0); /* observed average */
        // default underlines are a bit thin for larger font-sizes.
        let underline_thickness = underline_thickness * 1.3;

        let underline_position = (underline_position * scale) as u32;
        let underline_thickness = ((underline_thickness * scale) as u32).max(1);

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

    pub(crate) fn strikeout_metrics(&self, height_px: u32, _box_height: u32) -> (u32, u32) {
        let scale = self.scale(height_px);

        let ascender = self.font.ascender() as f32;

        let strikeout_position = self
            .font
            .strikeout_metrics()
            .map(|m| m.position as f32)
            .unwrap_or_default();
        let strikeout_position = if strikeout_position > 0.0 {
            ascender - strikeout_position
        } else {
            ascender as f32 * 0.7 /* observed average */
        };

        let strikeout_thickness = self
            .font
            .strikeout_metrics()
            .map(|m| m.thickness as f32)
            .unwrap_or(100.0); /* observed average */
        // default strikeout lines are a bit thin for larger font-sizes.
        let strikeout_thickness = strikeout_thickness * 1.8;

        (
            (strikeout_position * scale) as u32,
            ((strikeout_position + strikeout_thickness) * scale) as u32,
        )
    }
}
