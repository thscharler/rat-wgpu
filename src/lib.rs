use std::fmt::{Display, Formatter};

mod backend;
pub mod colors;
pub mod cursor;
pub mod font;
pub mod image;
pub mod postprocessor;
mod text_atlas;
pub(crate) mod util;
#[cfg(feature = "winit-event")]
pub mod events;

pub use backend::backend::WgpuBackend;
pub use backend::builder::Builder;

pub mod wgpu {
    pub use wgpu::Backends;
}

/// The metrics needed for rendering.
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

    /// Cell-size to pixel.
    pub(crate) fn px_size(&self, width: u16, height: u16) -> (u32, u32) {
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
    BufferAsyncError(String),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}
