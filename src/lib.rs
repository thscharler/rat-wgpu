use std::fmt::{Display, Formatter};

mod backend;
pub mod colors;
pub mod cursor;
pub mod font;
pub mod postprocessor;

pub use backend::ImageHandle;
pub use backend::backend::WgpuBackend;
pub use backend::builder::Builder;
pub use backend::image_buffer::{ImageBuffer, ImageFit, ImageZ};

/// The metrics needed for rendering.
#[derive(Debug, Default, Clone, Copy)]
pub struct CellBox {
    /// Width in px.
    pub width: u32,
    /// Height in px.
    pub height: u32,
    /// Baseline for glyphs. Measured from the top of the box.
    pub ascender: f32,
    /// Scaling factor from font-coords to px for the primary font.
    pub scale: f32,
}

impl CellBox {
    /// Pixel to cell-size. Rounded up.
    pub fn cell_size(&self, width: u32, height: u32) -> ratatui_core::layout::Size {
        let w = width / self.width + if width % self.width == 0 { 0 } else { 1 };
        let h = height / self.height + if height % self.height == 0 { 0 } else { 1 };

        ratatui_core::layout::Size::new(w as u16, h as u16)
    }

    /// Cell-size to pixel.
    pub fn px_size(&self, width: u16, height: u16) -> (u32, u32) {
        (width as u32 * self.width, height as u32 * self.height)
    }
}

#[derive(Debug)]
pub enum Error {
    SurfaceCreationFailed(Box<dyn std::error::Error>),
    AdapterRequestFailed(Box<dyn std::error::Error>),
    DeviceRequestFailed(Box<dyn std::error::Error>),
    SurfaceConfigurationRequestFailed,
    PollError(Box<dyn std::error::Error>),
    BufferAsyncError(String)
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}
