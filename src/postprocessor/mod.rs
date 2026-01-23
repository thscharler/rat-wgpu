use crate::CellBox;
use std::any::Any;
use wgpu::{CommandEncoder, Device, Queue, SurfaceConfiguration, TextureView};

pub mod default;
pub mod crt;

pub trait PostProcessorBuilder {
    /// Resulting postprocessor.
    type PostProcessor<'a>: PostProcessor + 'a;

    /// Called during initialization of the backend. This should fully
    /// initialize the post processor for rendering. Note that you are expected
    /// to render to the final surface during [`PostProcessor::process`].
    fn compile(
        self,
        device: &Device,
        text_view: &TextureView,
        surface_config: &SurfaceConfiguration,
    ) -> Self::PostProcessor<'static>;
}

/// A pipeline for post-processing rendered text.
pub trait PostProcessor: Any {
    /// Map the screen-coordinates to cell-coordinates.
    fn map_to_cell(&self, scr_x: i32, scr_y: i32, font_box: CellBox) -> (u16, u16);

    /// Called after the drawing dimensions have changed (e.g. the surface was
    /// resized).
    fn resize(
        &mut self,
        device: &Device,
        text_view: &TextureView,
        surface_config: &SurfaceConfiguration,
    );

    /// Called after text has finished compositing. The provided `text_view` is
    /// the composited text. The final output of your implementation should
    /// render to the provided `surface_view`.
    ///
    /// <div class="warning">
    ///
    /// Retaining a reference to the provided surface view will cause a panic if
    /// the swapchain is recreated.
    ///
    /// </div>
    fn process(
        &mut self,
        margin_color: u32,
        encoder: &mut CommandEncoder,
        queue: &Queue,
        text_view: &TextureView,
        surface_config: &SurfaceConfiguration,
        surface_view: &TextureView,
    );

    /// Called to see if this post processor wants to update the screen. By
    /// default, the backend only runs the compositor and post processor when
    /// the text changes. Returning true from this will override that behavior
    /// and cause the processor to be invoked after a call to flush, even if no
    /// text changes occurred.
    fn needs_update(&self) -> bool {
        false
    }
}
