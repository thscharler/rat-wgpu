use crate::CellBox;
use crate::font::Font;
use ratatui_core::buffer::Cell;
use ratatui_core::style::Modifier;

/// Collection of fonts for rendering.
#[derive(Debug)]
pub(crate) struct Fonts {
    fallback: Vec<Font>,
    regular: Vec<Font>,
    bold: Vec<Font>,
    italic: Vec<Font>,
    bold_italic: Vec<Font>,

    // derived from all available fonts.
    height_px: u32,
    width_px: u32,
    ascender_px: u32,
}

impl Default for Fonts {
    fn default() -> Self {
        Self {
            fallback: Default::default(),
            regular: Default::default(),
            bold: Default::default(),
            italic: Default::default(),
            bold_italic: Default::default(),
            height_px: 20,
            width_px: 20 / 2,
            ascender_px: 20 * 4 / 5,
        }
    }
}

impl Fonts {
    /// Add the given font as a fallback.
    ///
    /// Fallback fonts are searched in the order
    /// of addition.
    pub fn add_fallback(&mut self, mut font: Font) {
        font.set_fallback(true);
        self.fallback.push(font);
        self.set_height_px(self.height_px);
    }

    /// Is any fallback font set?
    pub fn have_fallback(&self) -> bool {
        !self.fallback.is_empty()
    }

    /// Add a font.
    ///
    /// Typically, you add all fonts of a font-family.
    /// At least one regular, italic, bold and bold+italic Font
    /// is good. At a minimum a regular font will do.
    pub fn add_font(&mut self, font: Font) {
        if (font.is_italic() || font.is_oblique()) && font.is_bold() {
            self.bold_italic.push(font);
        } else if font.is_italic() || font.is_oblique() {
            self.italic.push(font);
        } else if font.is_bold() {
            self.bold.push(font);
        } else {
            self.regular.push(font);
        }
        self.set_height_px(self.height_px);
    }

    /// Remove the non-fallback fonts.
    pub fn clear_fonts(&mut self) {
        self.bold_italic.clear();
        self.italic.clear();
        self.bold.clear();
        self.regular.clear();
        self.set_height_px(self.height_px);
    }

    /// Change the height of all fonts in this collection to the specified
    /// height in pixels.
    ///
    /// __Panic__
    ///
    /// Panics for a font-height of 0.
    ///
    pub fn set_height_px(&mut self, height_px: u32) {
        self.height_px = height_px;

        self.regular
            .iter_mut()
            .chain(self.bold.iter_mut())
            .chain(self.bold_italic.iter_mut())
            .chain(self.italic.iter_mut())
            .chain(self.fallback.iter_mut())
            .for_each(|f| {
                f.set_height_px(self.height_px);
            });

        (self.width_px, self.ascender_px) = self
            .regular
            .iter()
            .chain(self.bold.iter())
            .chain(self.italic.iter())
            .chain(self.bold_italic.iter())
            .chain(self.fallback.iter())
            .map(|font| (font.em_advance_px(), font.ascender_px()))
            .next() /* first is fine */
            .unwrap_or((self.height_px / 2, self.height_px * 4 / 5));

        if self.width_px == 0 {
            self.width_px = 1;
        }

        self.regular
            .iter_mut()
            .chain(self.bold.iter_mut())
            .chain(self.bold_italic.iter_mut())
            .chain(self.italic.iter_mut())
            .chain(self.fallback.iter_mut())
            .for_each(|f| {
                f.set_width_px(self.width_px);
            });

        assert_ne!(self.width_px, 0);
    }

    /// The minimum width (in pixels) across all fonts.
    pub fn width_px(&self) -> u32 {
        self.width_px
    }

    /// The height (in pixels) of all fonts.
    #[inline]
    pub fn height_px(&self) -> u32 {
        self.height_px
    }

    /// Size of a cell with the current font in px.
    pub fn cell_box(&self) -> CellBox {
        CellBox {
            width: self.width_px,
            height: self.height_px,
            ascender: self.ascender_px,
        }
    }

    pub(crate) fn shape_plan_count(&self) -> usize {
        1 + self.bold.len() + self.italic.len() + self.bold_italic.len() + self.regular.len()
    }

    pub(crate) fn get_by_id(&self, id: fontdb::ID) -> &Font {
        self.regular
            .iter()
            .chain(self.bold.iter())
            .chain(self.italic.iter())
            .chain(self.bold_italic.iter())
            .chain(self.fallback.iter())
            .find(|v| v.id() == id)
            .expect("font")
    }

    pub(crate) fn default_font_for_cell(&self) -> fontdb::ID {
        self.select_font(" ", self.regular.iter().chain(self.fallback.iter()))
    }

    pub(crate) fn font_for_cell(&self, cell: &Cell) -> fontdb::ID {
        if cell.modifier.contains(Modifier::BOLD | Modifier::ITALIC) {
            self.select_font(
                cell.symbol(),
                self.bold_italic
                    .iter()
                    .chain(self.italic.iter())
                    .chain(self.bold.iter())
                    .chain(self.regular.iter())
                    .chain(self.fallback.iter()),
            )
        } else if cell.modifier.contains(Modifier::BOLD) {
            self.select_font(
                cell.symbol(),
                self.bold
                    .iter()
                    .chain(self.regular.iter())
                    .chain(self.fallback.iter()),
            )
        } else if cell.modifier.contains(Modifier::ITALIC) {
            self.select_font(
                cell.symbol(),
                self.italic
                    .iter()
                    .chain(self.regular.iter())
                    .chain(self.fallback.iter()),
            )
        } else {
            self.select_font(
                cell.symbol(),
                self.regular.iter().chain(self.fallback.iter()),
            )
        }
    }

    fn select_font<'a>(
        &'a self,
        cluster: &str,
        fonts: impl IntoIterator<Item = &'a Font>,
    ) -> fontdb::ID {
        let mut max = 0;
        let mut font = None;
        let mut last_resort = None;

        for candidate in fonts.into_iter() {
            // try to map the complete cluster to a single font.
            // the first font that can map it completely wins, otherwise
            // the one with the max matched glyphs.
            let (count, last_idx) =
                cluster
                    .chars()
                    .enumerate()
                    .fold((0, 0), |(mut count, _), (idx, ch)| {
                        count += usize::from(candidate.have_glyph(ch));
                        (count, idx)
                    });

            if count > max {
                max = count;
                font = Some(candidate.id());
            }

            if count == last_idx + 1 {
                break;
            }

            last_resort = Some(candidate.id());
        }

        font.unwrap_or_else(|| {
            if let Some(last_resort) = last_resort {
                last_resort
            } else {
                panic!("at least one font must be set.");
            }
        })
    }
}
