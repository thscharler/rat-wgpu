#![doc = include_str!("../readme.md")]

use std::fmt::{Display, Formatter};

mod backend;
pub mod colors;
pub mod cursor;
#[cfg(feature = "winit-event")]
pub mod event;
pub mod font;
pub mod image;
pub mod postprocessor;
mod text_atlas;
pub(crate) mod util;

pub use backend::WgpuBackend;
pub use backend::WgpuBackendBuilder;

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
