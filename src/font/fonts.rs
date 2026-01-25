use crate::CellBox;
use crate::font::font::Font;
use log::warn;
use ratatui_core::buffer::Cell;
use ratatui_core::style::Modifier;

/// A collection of fonts to use for rendering. Supports font fallback.
///
/// It is recommended, but not required, that all fonts have the same/very
/// similar aspect ratio, or you may get unexpected results during rendering due
/// to fallback.
pub struct Fonts<'a> {
    width_px: u32,
    height_px: u32,
    ascender: u32,

    em_advance: f32,

    fallback: Vec<Font<'a>>,

    regular: Vec<Font<'a>>,
    bold: Vec<Font<'a>>,
    italic: Vec<Font<'a>>,
    bold_italic: Vec<Font<'a>>,

    // give an id in insertion order.
    id_count: u64,
}

impl<'a> Fonts<'a> {
    /// Create a new, empty set of fonts. The provided font will be used as a
    /// last-resort fallback if no other fonts can render a particular
    /// character. Rendering will attempt to fake bold/italic styles using this
    /// font where appropriate.
    ///
    /// The provided size_px will be the rendered height in pixels of all fonts
    /// in this collection.
    pub fn new(mut font: Font<'a>, size_px: u32) -> Self {
        font.set_height_px(size_px);
        font.set_width_px(font.base_width_px());
        font.set_fallback(true);
        font.set_id(0);

        Self {
            width_px: font.base_width_px(),
            height_px: size_px,
            ascender: font.ascender(),
            em_advance: font.em_advance(),
            fallback: vec![font],
            regular: vec![],
            bold: vec![],
            italic: vec![],
            bold_italic: vec![],
            id_count: 1,
        }
    }

    /// Create a new, empty set of fonts. The provided fonts will be used as a
    /// last-resort fallback if no other fonts can render a particular
    /// character. Rendering will attempt to fake bold/italic styles using this
    /// font where appropriate.
    ///
    /// The expectation is that the fallback fonts accommodate for missing symbols
    /// and emojis. Any fonts used for actual text display should use [add_fonts]
    ///
    /// The provided size_px will be the rendered height in pixels of all fonts
    /// in this collection.
    pub fn new_vec(mut fonts: Vec<Font<'a>>, size_px: u32) -> Self {
        fonts.iter_mut().enumerate().for_each(|(n, f)| {
            f.set_height_px(size_px);
            f.set_width_px(f.base_width_px());
            f.set_fallback(true);
            f.set_id(n as u64);
        });
        let id_count = fonts.len() as u64;

        Self {
            width_px: size_px / 2, // rough estimate
            height_px: size_px,
            ascender: size_px * 4 / 5,         // rough estimate
            em_advance: size_px as f32 / 2.0, // rough estimate
            fallback: fonts,
            regular: vec![],
            bold: vec![],
            italic: vec![],
            bold_italic: vec![],
            id_count,
        }
    }

    #[inline]
    pub fn ascender(&self) -> u32 {
        self.ascender
    }

    #[inline]
    pub fn em_advance(&self) -> f32 {
        self.em_advance
    }

    /// The height (in pixels) of all fonts.
    #[inline]
    pub fn height_px(&self) -> u32 {
        self.height_px
    }

    /// Change the height of all fonts in this collection to the specified
    /// height in pixels.
    pub fn set_height_px(&mut self, height_px: u32) {
        self.height_px = height_px;

        self.regular
            .iter_mut()
            .chain(self.bold.iter_mut())
            .chain(self.bold_italic.iter_mut())
            .chain(self.italic.iter_mut())
            .chain(self.fallback.iter_mut())
            .for_each(|f| {
                f.set_height_px(height_px);
            });

        if !self.regular.is_empty()
            || !self.bold.is_empty()
            || !self.italic.is_empty()
            || !self.bold_italic.is_empty()
        {
            (self.width_px, self.ascender, self.em_advance) = self
                .regular
                .iter()
                .chain(self.bold.iter())
                .chain(self.italic.iter())
                .chain(self.bold_italic.iter())
                .map(|font| (font.base_width_px(), font.ascender(), font.em_advance()))
                .next() /* first is fine */
                .expect("font");
        } else {
            self.width_px = self.height_px / 2;
            self.ascender = self.height_px * 4 / 5;
            self.em_advance = self.height_px as f32 / 2.0;
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

        assert_ne!(self.height_px, 0);
        assert_ne!(self.width_px, 0);
    }

    /// The minimum width (in pixels) across all fonts.
    pub fn width_px(&self) -> u32 {
        self.width_px
    }

    /// Remove the non-fallback fonts.
    pub fn clear_fonts(&mut self) {
        self.bold_italic.clear();
        self.italic.clear();
        self.bold.clear();
        self.regular.clear();
        self.set_height_px(self.height_px);
    }

    /// Add a collection of fonts for various styles. They will automatically be
    /// added to the appropriate fallback font list based on the font's
    /// bold/italic properties. Note that this will automatically organize fonts
    /// by relative width in order to optimize fallback rendering quality. The
    /// ordering of already provided fonts will remain unchanged.
    ///
    /// Adding more fonts will not have any effect, if the text can be rendered
    /// with a prior font.
    pub fn add_fonts(&mut self, fonts: impl IntoIterator<Item = Font<'a>>) {
        for mut font in fonts {
            font.set_id(self.id_count);
            self.id_count += 1;

            if !font.face().is_monospaced() {
                warn!("Non monospace font used in add_fonts, this may cause unexpected rendering.");
            }
            if font.face().is_italic() && font.face().is_bold() {
                self.bold_italic.push(font);
            } else if font.face().is_italic() {
                self.italic.push(font);
            } else if font.face().is_bold() {
                self.bold.push(font);
            } else {
                self.regular.push(font);
            }
        }
        self.set_height_px(self.height_px);
    }

    /// Add a new collection of fonts for regular styled text. These fonts will
    /// come _after_ previously provided fonts in the fallback order.
    pub fn add_regular_fonts(&mut self, fonts: impl IntoIterator<Item = Font<'a>>) {
        for mut font in fonts {
            font.set_id(self.id_count);
            self.id_count += 1;
            self.regular.push(font);
        }
        self.set_height_px(self.height_px);
    }

    /// Add a new collection of fonts for bold styled text. These fonts will
    /// come _after_ previously provided fonts in the fallback order.
    ///
    /// You do not have to provide these for bold text to be supported. If no
    /// bold fonts are supplied, rendering will fallback to the regular fonts
    /// with fake bolding.
    pub fn add_bold_fonts(&mut self, fonts: impl IntoIterator<Item = Font<'a>>) {
        for mut font in fonts {
            font.set_id(self.id_count);
            self.id_count += 1;
            self.bold.push(font);
        }
        self.set_height_px(self.height_px);
    }

    /// Add a new collection of fonts for italic styled text. These fonts will
    /// come _after_ previously provided fonts in the fallback order.
    ///
    /// It is recommended, but not required, that you provide italic fonts if
    /// your application intends to make use of italics. If no italic fonts
    /// are supplied, rendering will fallback to the regular fonts with fake
    /// italics.
    pub fn add_italic_fonts(&mut self, fonts: impl IntoIterator<Item = Font<'a>>) {
        for mut font in fonts {
            font.set_id(self.id_count);
            self.id_count += 1;
            self.italic.push(font);
        }
        self.set_height_px(self.height_px);
    }

    /// Add a new collection of fonts for bold italic styled text. These fonts
    /// will come _after_ previously provided fonts in the fallback order.
    ///
    /// You do not have to provide these for bold text to be supported. If no
    /// bold fonts are supplied, rendering will fallback to the italic fonts
    /// with fake bolding.
    pub fn add_bold_italic_fonts(&mut self, fonts: impl IntoIterator<Item = Font<'a>>) {
        for mut font in fonts {
            font.set_id(self.id_count);
            self.id_count += 1;
            self.bold_italic.push(font);
        }
        self.set_height_px(self.height_px);
    }

    /// Size of a cell with the current font in px.
    pub fn cell_box(&self) -> CellBox {
        CellBox {
            width: self.width_px(),
            height: self.height_px(),
            ascender: self.ascender(),
        }
    }


    pub(crate) fn count(&self) -> usize {
        1 + self.bold.len() + self.italic.len() + self.bold_italic.len() + self.regular.len()
    }

    pub(crate) fn get_by_id(&'a self, id: u64) -> &'a Font<'a> {
        self.regular
            .iter()
            .chain(self.bold.iter())
            .chain(self.italic.iter())
            .chain(self.bold_italic.iter())
            .chain(self.fallback.iter())
            .find(|v| v.id() == id)
            .expect("font")
    }

    pub(crate) fn font_for_cell(&'_ self, cell: &Cell) -> u64 {
        if cell.modifier.contains(Modifier::BOLD | Modifier::ITALIC) {
            self.select_font(
                cell.symbol(),
                self.bold_italic
                    .iter()
                    .map(|f| f)
                    .chain(self.italic.iter().map(|f| f))
                    .chain(self.bold.iter().map(|f| f))
                    .chain(self.regular.iter().map(|f| f))
                    .chain(self.fallback.iter().map(|f| f)),
            )
        } else if cell.modifier.contains(Modifier::BOLD) {
            self.select_font(
                cell.symbol(),
                self.bold
                    .iter()
                    .map(|f| f)
                    .chain(self.regular.iter().map(|f| f))
                    .chain(self.fallback.iter().map(|f| f)),
            )
        } else if cell.modifier.contains(Modifier::ITALIC) {
            self.select_font(
                cell.symbol(),
                self.italic
                    .iter()
                    .map(|f| f)
                    .chain(self.regular.iter().map(|f| f))
                    .chain(self.fallback.iter().map(|f| f)),
            )
        } else {
            self.select_font(
                cell.symbol(),
                self.regular
                    .iter()
                    .map(|f| f)
                    .chain(self.fallback.iter().map(|f| f)),
            )
        }
    }

    fn select_font<'fonts>(
        &'fonts self,
        cluster: &str,
        fonts: impl IntoIterator<Item = &'fonts Font<'a>>,
    ) -> u64 {
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
                        count += usize::from(candidate.face().glyph_index(ch).is_some());
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
