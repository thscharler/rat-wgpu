use crate::backend::builder::{build_img_bindings, build_wgpu_state};
use crate::backend::plan_cache::PlanCache;
use crate::backend::surface::RenderSurface;
use crate::backend::{
    ImageInfo, ImgVertexMember, NULL_CELL, RenderInfo, Rendered, TextBgVertexMember,
    TextVertexMember, TuiSurface, WgpuAtlas, WgpuBase, WgpuImage, WgpuImages, WgpuPipeline,
    WgpuVertices,
};
use crate::colors::{ColorTable, Rgb};
use crate::cursor::{Blinking, CursorStyle};
use crate::font::rasterize::rasterize_glyph;
use crate::font::{Font, Fonts};
use crate::image::ImageHandle;
use crate::image::{ImageCell, ImageFrame};
use crate::postprocessor::{PostProcessor, PostProcessorBuilder};
use crate::text_atlas::Key;
use crate::util::clip_uv;
use crate::{CellBox, Error};
use bitvec::slice::BitSlice;
use ratatui_core::backend::{Backend, ClearType, WindowSize};
use ratatui_core::buffer::Cell;
use ratatui_core::style::Modifier;
use rustybuzz::ttf_parser::GlyphId;
use rustybuzz::{GlyphBuffer, UnicodeBuffer, shape_with_plan};
use std::mem;
use std::num::NonZeroU64;
use std::sync::{Arc, Mutex};
use unicode_bidi::ParagraphBidiInfo;
use unicode_properties::{
    GeneralCategory, GeneralCategoryGroup, UnicodeEmoji, UnicodeGeneralCategory,
};
use unicode_width::UnicodeWidthChar;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    Buffer, BufferUsages, BufferView, CommandEncoderDescriptor, Device, Extent3d, IndexFormat,
    LoadOp, Operations, Origin3d, PollType, Queue, RenderPass, RenderPassColorAttachment,
    RenderPassDescriptor, StoreOp, TextureAspect, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TextureViewDescriptor,
};

/// A ratatui backend leveraging wgpu for rendering.
///
/// Constructed using a [`Builder`](crate::Builder).
///
/// The first lifetime parameter is the lifetime of the data for referenced
/// [`Font`] objects. The second lifetime parameter is the lifetime of the
/// referenced [`Surface`] (typically the lifetime of your window object).
///
/// Limitations:
/// - The cursor is tracked but not rendered.
/// - No builtin accessibilty, although [`WgpuBackend::get_text`] is provided to
///   access the screen's contents.
pub struct WgpuBackend<'f, 's> {
    // active fonts
    pub(super) fonts: Fonts<'f>,

    // ratatui state
    pub(super) tui_surface: TuiSurface,

    // positioned glyphs.
    pub(super) rendered: Vec<Rendered>,

    // temporaries for shaping
    pub(super) tmp_plan_cache: PlanCache,
    pub(super) tmp_rowbuf: String,
    pub(super) tmp_rowbuf_to_cell: Vec<u16>,
    pub(super) tmp_buffer: UnicodeBuffer,

    // wgpu input
    pub(super) wgpu_base: WgpuBase<'s>,
    pub(super) wgpu_vertices: WgpuVertices,
    pub(super) wgpu_atlas: WgpuAtlas,
    pub(super) wgpu_images: WgpuImages,
    pub(super) wgpu_post_process: Box<dyn PostProcessor + 'static>,
    pub(super) wgpu_pipeline: WgpuPipeline,
}

impl<'s> Backend for WgpuBackend<'_, 's> {
    type Error = std::io::Error;

    fn draw<'a, I>(&mut self, mut content: I) -> std::io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        let bounds = self.size()?;
        draw_tui(
            bounds,
            &self.fonts,
            &mut content,
            &mut self.tui_surface,
            &mut self.rendered,
        );
        Ok(())
    }

    fn hide_cursor(&mut self) -> std::io::Result<()> {
        self.tui_surface.cursor_visible = false;
        Ok(())
    }

    fn show_cursor(&mut self) -> std::io::Result<()> {
        self.tui_surface.cursor_visible = true;
        Ok(())
    }

    fn get_cursor_position(&mut self) -> std::io::Result<ratatui_core::layout::Position> {
        Ok(ratatui_core::layout::Position::new(
            self.tui_surface.cursor.0,
            self.tui_surface.cursor.1,
        ))
    }

    fn set_cursor_position<Pos: Into<ratatui_core::layout::Position>>(
        &mut self,
        position: Pos,
    ) -> std::io::Result<()> {
        let bounds = self.size()?;
        let pos = position.into();

        // old cursor
        self.tui_surface
            .dirty_rows
            .set(self.tui_surface.cursor.1 as usize, true);
        self.tui_surface.dirty_cells.set(
            self.tui_surface.cursor.1 as usize * bounds.width as usize
                + self.tui_surface.cursor.0 as usize,
            true,
        );

        self.tui_surface.cursor = (pos.x.min(bounds.width - 1), pos.y.min(bounds.height - 1));
        self.tui_surface
            .dirty_rows
            .set(self.tui_surface.cursor.1 as usize, true);
        self.tui_surface.dirty_cells.set(
            self.tui_surface.cursor.1 as usize * bounds.width as usize
                + self.tui_surface.cursor.0 as usize,
            true,
        );

        Ok(())
    }

    fn clear(&mut self) -> std::io::Result<()> {
        self.tui_surface.cells.clear();
        self.tui_surface.cell_font.clear();
        self.tui_surface.dirty_rows.clear();
        self.rendered.clear();
        self.tui_surface.fast_blinking.clear();
        self.tui_surface.slow_blinking.clear();
        self.tui_surface.cursor = (0, 0);

        Ok(())
    }

    fn clear_region(&mut self, clear_type: ClearType) -> std::io::Result<()> {
        let bounds = self.size()?;
        let line_start = self.tui_surface.cursor.1 as usize * bounds.width as usize;
        let idx = line_start + self.tui_surface.cursor.0 as usize;

        match clear_type {
            ClearType::All => self.clear(),
            ClearType::AfterCursor => {
                self.tui_surface.cells.truncate(idx + 1);
                self.tui_surface.cell_font.truncate(idx + 1);
                self.tui_surface.cell_remap.truncate(idx + 1);
                self.tui_surface
                    .dirty_rows
                    .truncate(self.tui_surface.cursor.1 as usize);
                self.tui_surface.dirty_cells.truncate(idx + 1);
                self.tui_surface.fast_blinking.truncate(idx + 1);
                self.tui_surface.slow_blinking.truncate(idx + 1);
                Ok(())
            }
            ClearType::BeforeCursor => {
                self.tui_surface.cells[..idx].fill(Cell::EMPTY);
                self.tui_surface.cell_font[..idx].fill(0);
                self.tui_surface.cell_remap[..idx].fill(0);
                self.tui_surface.dirty_rows[..=self.tui_surface.cursor.1 as usize].fill(true);
                self.tui_surface.dirty_cells[..idx].fill(true);
                self.tui_surface.fast_blinking[..idx].fill(false);
                self.tui_surface.slow_blinking[..idx].fill(false);
                Ok(())
            }
            ClearType::CurrentLine => {
                self.tui_surface.cells[line_start..line_start + bounds.width as usize]
                    .fill(Cell::EMPTY);
                self.tui_surface.cell_font[line_start..line_start + bounds.width as usize].fill(0);
                self.tui_surface.cell_remap[line_start..line_start + bounds.width as usize].fill(0);
                self.tui_surface
                    .dirty_rows
                    .set(self.tui_surface.cursor.1 as usize, true);
                self.tui_surface.dirty_cells[line_start..line_start + bounds.width as usize]
                    .fill(true);
                self.tui_surface.fast_blinking[line_start..line_start + bounds.width as usize]
                    .fill(false);
                self.tui_surface.slow_blinking[line_start..line_start + bounds.width as usize]
                    .fill(false);
                Ok(())
            }
            ClearType::UntilNewLine => {
                let remain = (bounds.width - self.tui_surface.cursor.0) as usize;
                self.tui_surface.cells[idx..idx + remain].fill(Cell::EMPTY);
                self.tui_surface.cell_font[idx..idx + remain].fill(0);
                self.tui_surface.cell_remap[idx..idx + remain].fill(0);
                self.tui_surface
                    .dirty_rows
                    .set(self.tui_surface.cursor.1 as usize, true);
                self.tui_surface.dirty_cells[idx..idx + remain].fill(true);
                self.tui_surface.fast_blinking[idx..idx + remain].fill(false);
                self.tui_surface.slow_blinking[idx..idx + remain].fill(false);
                Ok(())
            }
        }
    }

    fn size(&self) -> std::io::Result<ratatui_core::layout::Size> {
        let font_box = self.fonts.cell_box();
        let width = self.wgpu_base.surface_config.width;
        let height = self.wgpu_base.surface_config.height;

        Ok(ratatui_core::layout::Size {
            width: (width / font_box.width) as u16,
            height: (height / font_box.height) as u16,
        })
    }

    fn window_size(&mut self) -> std::io::Result<WindowSize> {
        let font_box = self.fonts.cell_box();
        let width = self.wgpu_base.surface_config.width;
        let height = self.wgpu_base.surface_config.height;

        Ok(WindowSize {
            columns_rows: ratatui_core::layout::Size {
                width: (width / font_box.width) as u16,
                height: (height / font_box.height) as u16,
            },
            pixels: ratatui_core::layout::Size {
                width: width as u16,
                height: height as u16,
            },
        })
    }

    fn flush(&mut self) -> std::io::Result<()> {
        let bounds = self.size()?;

        flush_tui(
            bounds,
            &self.fonts,
            &mut self.tui_surface,
            &mut self.rendered,
            &mut self.wgpu_atlas,
            &self.wgpu_base.queue,
            &mut self.tmp_plan_cache,
            &mut self.tmp_rowbuf,
            &mut self.tmp_rowbuf_to_cell,
            &mut self.tmp_buffer,
        );

        append_dirty_rows(
            &mut self.tui_surface,
            self.wgpu_post_process.as_ref(),
            &self.rendered,
            &mut self.wgpu_vertices,
        );

        render(
            self.window_size().expect("window_size"),
            self.fonts.cell_box(),
            self.tui_surface.reset_bg,
            &self.wgpu_base,
            &self.wgpu_images,
            &self.wgpu_pipeline,
            self.wgpu_post_process.as_mut(),
            &self.wgpu_vertices,
        );

        self.wgpu_vertices.clear();
        drop_images(&mut self.tui_surface, &mut self.wgpu_images);

        Ok(())
    }
}

impl<'f, 's> WgpuBackend<'f, 's> {
    /// Returns the ImageFrame.
    ///
    /// This will be used by the application to queue images for rendering.
    /// Add the image with [add_image] first, and use the ImageBuffer when
    /// rendering the UI.
    ///
    /// __Info__
    ///
    /// You can keep the ImageFrame around. It's internals will update
    /// if cell_box or window-size changes.
    pub fn image_frame(&self) -> ImageFrame {
        self.tui_surface.image_frame.clone()
    }

    /// Background color or Color::Reset.
    ///
    /// This will also fill the unclaimed area at the right/bottom.
    pub fn set_bg_color(&mut self, color: ratatui_core::style::Color) {
        self.tui_surface.reset_bg = self.tui_surface.colors.c2c(color, [0; 3]);
    }

    /// Foreground color for Color::Reset.
    pub fn set_fg_color(&mut self, color: ratatui_core::style::Color) {
        self.tui_surface.reset_fg = self.tui_surface.colors.c2c(color, [255; 3]);
    }

    /// Set the cursor style.
    pub fn set_cursor_style(&mut self, style: CursorStyle) {
        self.tui_surface.cursor_style = style;
    }

    /// Current cursor style.
    pub fn cursor_style(&self) -> CursorStyle {
        self.tui_surface.cursor_style
    }

    /// Set the cursor color.
    pub fn set_cursor_color(&mut self, color: ratatui_core::style::Color) {
        self.tui_surface.cursor_color = color;
    }

    /// Current cursor color.
    pub fn cursor_color(&self) -> ratatui_core::style::Color {
        self.tui_surface.cursor_color
    }

    /// Map a physical cursor position to a col/row position.
    pub fn pos_to_cell(&self, pos: (i32, i32)) -> (u16, u16) {
        let font_box = self.fonts.cell_box();
        if font_box.width == 0 || font_box.height == 0 {
            // might happen during resize or before the first render.
            return (0, 0);
        }

        let (cell_x, cell_y) =
            self.wgpu_post_process
                .map_to_cell(pos.0, pos.1, self.fonts.cell_box());

        let bounds = self.size().unwrap();
        let offset = (cell_y * bounds.width) as usize;
        if self.tui_surface.cell_remap.len() < offset + bounds.width as usize {
            // might happen during resize or before the first render.
            return (cell_x, cell_y);
        }
        for cell in 0..bounds.width {
            if self.tui_surface.cell_remap[offset + cell as usize] == cell_x {
                return (cell, cell_y);
            }
        }

        (cell_x, cell_y)
    }

    /// Get the [`PostProcessor`] associated with this backend.
    pub fn post_processor(&self) -> &dyn PostProcessor {
        self.wgpu_post_process.as_ref()
    }

    /// Get a mutable reference to the [`PostProcessor`] associated with this
    /// backend.
    pub fn post_processor_mut(&mut self) -> &mut dyn PostProcessor {
        self.wgpu_post_process.as_mut()
    }

    /// Changes the post-processor.
    pub fn update_post_processor<P: PostProcessorBuilder>(&mut self, builder: P) {
        let post_process = builder.compile(
            &self.wgpu_base.device,
            &self.wgpu_base.text_dest_view,
            &self.wgpu_base.surface_config,
        );
        self.wgpu_post_process = Box::new(post_process);
    }

    /// Resize the rendering surface.
    ///
    /// This must be called to keep the backend in sync with your window size.
    pub fn resize(&mut self, width: u32, height: u32) {
        let limits = self.wgpu_base.device.limits();
        let width = width.min(limits.max_texture_dimension_2d);
        let height = height.min(limits.max_texture_dimension_2d);

        if width == self.wgpu_base.surface_config.width
            && height == self.wgpu_base.surface_config.height
            || width == 0
            || height == 0
        {
            return;
        }

        self.wgpu_base.surface_config.width = width;
        self.wgpu_base.surface_config.height = height;

        rebuild_surface(
            self.fonts.cell_box(),
            &mut self.tui_surface,
            &mut self.rendered,
            &mut self.wgpu_base,
            &mut self.wgpu_atlas,
            self.wgpu_post_process.as_mut(),
        );
    }

    /// Get the text currently displayed on the screen.
    pub fn get_text(&self) -> String {
        let bounds = self.size().unwrap();
        self.tui_surface.cells.chunks(bounds.width as usize).fold(
            String::with_capacity((bounds.width + 1) as usize * bounds.height as usize),
            |dest, row| {
                let mut dest = row.iter().fold(dest, |mut dest, s| {
                    dest.push_str(s.symbol());
                    dest
                });
                dest.push('\n');
                dest
            },
        )
    }

    /// Update the color-table used for rendering. This will cause a full
    /// repaint of the screen the next time [`WgpuBackend::flush`] is
    /// called.
    pub fn update_color_table(&mut self, new_colors: ColorTable) {
        self.tui_surface.dirty_rows.clear();
        self.tui_surface.dirty_cells.clear();
        self.tui_surface.colors = new_colors;
    }

    /// Update the fonts used for rendering. This will cause a full repaint of
    /// the screen the next time [`WgpuBackend::flush`] is called. A call to
    /// [ratatui_core::terminal::Terminal::draw] will do this.
    ///
    /// This will also change the number of cells if the font has a different
    /// aspect ratio for its glyphs.
    pub fn update_fonts(&mut self, new_fonts: Fonts<'f>) {
        self.fonts = new_fonts;

        rebuild_surface(
            self.fonts.cell_box(),
            &mut self.tui_surface,
            &mut self.rendered,
            &mut self.wgpu_base,
            &mut self.wgpu_atlas,
            self.wgpu_post_process.as_mut(),
        );
    }

    /// Replace the fonts used for rendering. This will keep the fallback fonts.
    /// If you want to replace those too, use [update_fonts].
    ///
    /// This will cause a full repaint of the screen the next
    /// time [`WgpuBackend::flush`] is called.
    /// A call to [ratatui_core::terminal::Terminal::draw] will do this.
    pub fn update_font_vec(&mut self, new_fonts: Vec<Font<'f>>) {
        self.fonts.clear_fonts();
        self.fonts.add_fonts(new_fonts);

        rebuild_surface(
            self.fonts.cell_box(),
            &mut self.tui_surface,
            &mut self.rendered,
            &mut self.wgpu_base,
            &mut self.wgpu_atlas,
            self.wgpu_post_process.as_mut(),
        );
    }

    /// Update the font-size used for rendering.
    ///
    /// This will cause a full repaint of
    /// the screen the next time [`WgpuBackend::flush`] is called.
    /// A call to [ratatui_core::terminal::Terminal::draw] will do this.
    pub fn update_font_size(&mut self, new_font_size: u32) {
        self.fonts.set_height_px(new_font_size);

        rebuild_surface(
            self.fonts.cell_box(),
            &mut self.tui_surface,
            &mut self.rendered,
            &mut self.wgpu_base,
            &mut self.wgpu_atlas,
            self.wgpu_post_process.as_mut(),
        );
    }

    /// Toggle blinking.
    ///
    /// This will increase the internal blink-counter and render all
    /// cells marked as 'blink'. It will also make the cursor blink.
    ///
    /// The timing for calling blink is up to the application.
    /// This will give the base-rate for all blink effects. The
    /// actually rate is determined by the divisor for each effect.
    /// You can set the divisors when creating the backend with the
    /// [Builder](crate::Builder).
    pub fn blink(&mut self, blinking: Blinking) {
        let bounds = self.size().expect("size");

        flush_blink(
            blinking,
            bounds,
            self.fonts.cell_box(),
            &mut self.tui_surface,
            &self.rendered,
            &mut self.wgpu_vertices,
        );

        render(
            self.window_size().expect("window_size"),
            self.fonts.cell_box(),
            self.tui_surface.reset_bg,
            &self.wgpu_base,
            &self.wgpu_images,
            &self.wgpu_pipeline,
            self.wgpu_post_process.as_mut(),
            &self.wgpu_vertices,
        );

        self.wgpu_vertices.clear();
    }

    /// Add an image as raw RGBA data.
    ///
    /// This will return an ImageHandle.
    ///
    /// Freeing the image-texture occurs when you drop the last clone of
    /// the ImageHandle. The texture will be dropped after the next render.
    pub fn add_image(&mut self, image: &[u8], width: u32, height: u32) -> ImageHandle {
        let img = self.wgpu_base.device.create_texture(&TextureDescriptor {
            label: Some("Img"),
            size: Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        self.wgpu_base.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &img,
                mip_level: 0,
                origin: Origin3d { x: 0, y: 0, z: 0 },
                aspect: TextureAspect::All,
            },
            bytemuck::cast_slice(&image),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 4 * size_of::<u8>() as u32),
                rows_per_image: Some(height),
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let img_view = img.create_view(&TextureViewDescriptor::default());

        let id = self.wgpu_images.img_id;
        self.wgpu_images.img_id += 1;
        let handle = ImageHandle::new(id);

        self.wgpu_images.handles.insert(handle.clone());
        self.wgpu_images
            .img
            .insert(id, WgpuImage { texture: img_view });

        let image_buffer = self.tui_surface.image_frame.buffer();
        let mut image_buffer = image_buffer.lock().expect("lock");
        image_buffer.image_size.insert(id, (width, height));

        handle
    }

    /// Returns a BufferView for the current rendered result.
    ///
    /// __Info__
    ///
    /// You need to call [unmap_headless_buffer] to release the mapping.
    pub fn map_headless_buffer(&self) -> Result<BufferView, Error> {
        let RenderSurface::Headless(surface) = &self.wgpu_base.surface else {
            panic!("can only be called when initialized as headless.");
        };

        let mut encoder = self
            .wgpu_base
            .device
            .create_command_encoder(&CommandEncoderDescriptor::default());
        encoder.copy_texture_to_buffer(
            surface
                .texture
                .as_ref()
                .expect("headless texture")
                .as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: surface.buffer.as_ref().expect("headless buffer"),
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(surface.buffer_width),
                    rows_per_image: Some(surface.height),
                },
            },
            Extent3d {
                width: surface.width,
                height: surface.height,
                depth_or_array_layers: 1,
            },
        );
        self.wgpu_base.queue.submit(Some(encoder.finish()));

        let buffer = surface.buffer.as_ref().expect("headless buffer").slice(..);
        let data = Arc::new(Mutex::new(None));
        let data_copy = data.clone();
        buffer.map_async(wgpu::MapMode::Read, move |data| {
            let mut guard = data_copy.lock().expect("lock");
            *guard = Some(data);
        });
        self.wgpu_base
            .device
            .poll(PollType::Wait {
                submission_index: None,
                timeout: None,
            })
            .map_err(|e| Error::PollError(Box::new(e)))?;
        let guard = data.lock().expect("lock");
        match guard.as_ref().expect("data") {
            Ok(_) => {}
            Err(e) => return Err(Error::BufferAsyncError(e.to_string())),
        };

        Ok(buffer.get_mapped_range())
    }

    /// Releases the mapping of the headless buffer.
    pub fn unmap_headless_buffer(&self) {
        let RenderSurface::Headless(surface) = &self.wgpu_base.surface else {
            panic!("can only be called when initialized as headless.");
        };

        surface.buffer.as_ref().expect("headless buffer").unmap();
    }
}

// Resize the rendering surface. This should be called e.g. to keep the
// backend in sync with your window size.
fn rebuild_surface(
    cell_box: CellBox,
    tui_surface: &mut TuiSurface,
    rendered: &mut Vec<Rendered>,
    wgpu_base: &mut WgpuBase,
    wgpu_atlas: &mut WgpuAtlas,
    wgpu_post_process: &mut dyn PostProcessor,
) {
    let width = wgpu_base.surface_config.width;
    let height = wgpu_base.surface_config.height;
    wgpu_base
        .surface
        .configure(&wgpu_base.device, &wgpu_base.surface_config);

    let chars_wide = width / cell_box.width;
    let chars_high = height / cell_box.height;

    wgpu_atlas.cached.update_font_box(cell_box);

    tui_surface.images.clear();
    tui_surface.cells.clear();
    tui_surface.cell_font.clear();
    tui_surface.cell_remap.clear();
    tui_surface.fast_blinking.clear();
    tui_surface.slow_blinking.clear();
    // This always needs to be cleared because the surface is cleared when it is
    // resized. If we don't re-render the rows, we end up with a blank surface when
    // the resize is less than a character dimension.
    tui_surface.dirty_rows.clear();
    tui_surface.dirty_cells.clear();
    tui_surface.dirty_img.clear();

    let image_buffer = tui_surface.image_frame.buffer();
    let mut image_buffer = image_buffer.lock().expect("lock");
    image_buffer.cell_box = cell_box;
    image_buffer.area = ratatui_core::layout::Rect::new(0, 0, chars_wide as u16, chars_high as u16);

    rendered.clear();

    wgpu_base.text_dest_view = build_wgpu_state(
        &wgpu_base.device,
        chars_wide * cell_box.width,
        chars_high * cell_box.height,
    );

    wgpu_post_process.resize(
        &wgpu_base.device,
        &wgpu_base.text_dest_view,
        &wgpu_base.surface_config,
    );
}

// Remove unreferenced images.
fn drop_images(tui_surface: &mut TuiSurface, wgpu_images: &mut WgpuImages) {
    let mut dropped = Vec::new();
    for img_id in &wgpu_images.handles {
        if img_id.is_last() {
            dropped.push(img_id.clone());
        }
    }
    let image_buffer = tui_surface.image_frame.buffer();
    let mut image_buffer = image_buffer.lock().expect("lock");
    for img_id in dropped {
        wgpu_images.handles.remove(&img_id);
        wgpu_images.img.remove(&img_id.id());
        image_buffer.image_size.remove(&img_id.id());
    }
}

// run the render pipelines.
fn render(
    bounds: WindowSize,
    cell_box: CellBox,
    reset_bg: Rgb,
    base: &WgpuBase,
    images: &WgpuImages,
    pipeline: &WgpuPipeline,
    post_process: &mut dyn PostProcessor,
    vertices: &WgpuVertices,
) {
    if vertices.is_empty() && !post_process.needs_update() {
        return;
    }

    let mut encoder = base
        .device
        .create_command_encoder(&CommandEncoderDescriptor {
            label: Some("Draw Encoder"),
        });

    {
        let mut uniforms = base
            .queue
            .write_buffer_with(
                &pipeline.text_screen_size_buffer,
                0,
                NonZeroU64::new(size_of::<[f32; 4]>() as u64).unwrap(),
            )
            .unwrap();
        uniforms.copy_from_slice(bytemuck::cast_slice(&[
            bounds.columns_rows.width as f32 * cell_box.width as f32,
            bounds.columns_rows.height as f32 * cell_box.height as f32,
            0.0,
            0.0,
        ]));
    }

    let bg_vertices = base.device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Text Bg Vertices"),
        contents: bytemuck::cast_slice(&vertices.bg_vertices),
        usage: BufferUsages::VERTEX,
    });

    let fg_vertices = base.device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Text Vertices"),
        contents: bytemuck::cast_slice(&vertices.text_vertices),
        usage: BufferUsages::VERTEX,
    });

    let txt_indices = base.device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Text Indices"),
        contents: bytemuck::cast_slice(&vertices.text_indices),
        usage: BufferUsages::INDEX,
    });

    let img_vertices = base.device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Image Vertices"),
        contents: bytemuck::cast_slice(&vertices.img_vertices),
        usage: BufferUsages::VERTEX,
    });

    let img_indices = base.device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Image Indices"),
        contents: bytemuck::cast_slice(&vertices.img_indices),
        usage: BufferUsages::INDEX,
    });

    {
        let mut text_render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Text Render Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: &base.text_dest_view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Load,
                    store: StoreOp::Store,
                },
                depth_slice: None,
            })],
            ..Default::default()
        });

        if !vertices.text_indices.is_empty() {
            text_render_pass.set_index_buffer(txt_indices.slice(..), IndexFormat::Uint32);
            text_render_pass.set_pipeline(&pipeline.text_bg_compositor.pipeline);
            text_render_pass.set_bind_group(0, &pipeline.text_bg_compositor.fs_uniforms, &[]);
            text_render_pass.set_vertex_buffer(0, bg_vertices.slice(..));
            text_render_pass.draw_indexed(0..(vertices.bg_vertices.len() as u32 / 4) * 6, 0, 0..1);
        }

        if !vertices.img_vertices.is_empty() {
            render_img(
                &base.device,
                &mut text_render_pass,
                pipeline,
                true,
                images,
                &img_indices,
                &img_vertices,
                &vertices.img_render,
            );
        }

        if !vertices.text_indices.is_empty() {
            text_render_pass.set_index_buffer(txt_indices.slice(..), IndexFormat::Uint32);
            text_render_pass.set_pipeline(&pipeline.text_fg_compositor.pipeline);
            text_render_pass.set_bind_group(0, &pipeline.text_fg_compositor.fs_uniforms, &[]);
            text_render_pass.set_bind_group(1, &pipeline.text_fg_compositor.atlas_bindings, &[]);
            text_render_pass.set_vertex_buffer(0, fg_vertices.slice(..));
            text_render_pass.draw_indexed(
                0..(vertices.text_vertices.len() as u32 / 4) * 6,
                0,
                0..1,
            );
        }

        if !vertices.img_vertices.is_empty() {
            render_img(
                &base.device,
                &mut text_render_pass,
                pipeline,
                false,
                images,
                &img_indices,
                &img_vertices,
                &vertices.img_render,
            );
        }
    }

    let Some(texture) = base.surface.get_current_texture() else {
        return;
    };

    let bg_color_u32 = u32::from_le_bytes([reset_bg[0], reset_bg[1], reset_bg[2], 255]);

    post_process.process(
        bg_color_u32,
        &mut encoder,
        &base.queue,
        &base.text_dest_view,
        &base.surface_config,
        texture.get_view(),
    );

    base.queue.submit(Some(encoder.finish()));

    texture.present();
}

fn render_img(
    device: &Device,
    text_render_pass: &mut RenderPass,
    pipeline: &WgpuPipeline,
    below_text: bool,
    images: &WgpuImages,
    img_indices: &Buffer,
    img_vertices: &Buffer,
    img_render: &[ImageInfo],
) {
    text_render_pass.set_index_buffer(img_indices.slice(..), IndexFormat::Uint32);

    text_render_pass.set_pipeline(&pipeline.img_compositor.pipeline);
    text_render_pass.set_bind_group(0, &pipeline.img_compositor.fs_uniforms, &[]);
    text_render_pass.set_vertex_buffer(0, img_vertices.slice(..));
    for (n, img_info) in img_render.iter().enumerate() {
        let n = n as u32;

        if img_info.below_text != below_text {
            continue;
        }

        let uv_transform_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Image UV-Transform Uniforms Buffer"),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&[[
                img_info.uv_transform.m11,
                img_info.uv_transform.m21,
                img_info.uv_transform.m31,
                0.0f32, // padding
                img_info.uv_transform.m12,
                img_info.uv_transform.m22,
                img_info.uv_transform.m32,
                0.0f32, // padding
            ]]),
        });

        let uv = clip_uv(img_info.view_rect, img_info.view_clip).unwrap_or_default();
        let uv_clip_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Image Clip Uniforms Buffer"),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&uv),
        });

        let img_texture = images.img.get(&img_info.image_id).expect("image");
        let img_bindings = build_img_bindings(
            &pipeline.img_compositor,
            &device,
            &pipeline.sampler,
            &img_texture.texture,
            &uv_transform_buffer,
            &uv_clip_buffer,
        );

        text_render_pass.set_bind_group(1, &img_bindings, &[]);

        text_render_pass.draw_indexed(n * 6..(n + 1) * 6, 0, 0..1);
    }
}

// called by draw()
fn draw_tui(
    bounds: ratatui_core::layout::Size,
    fonts: &Fonts,
    content: &mut dyn Iterator<Item = (u16, u16, &'_ Cell)>,
    tui_surface: &mut TuiSurface,
    rendered: &mut Vec<Rendered>,
) {
    tui_surface
        .cells
        .resize(bounds.height as usize * bounds.width as usize, Cell::EMPTY);
    tui_surface
        .cell_font
        .resize(bounds.height as usize * bounds.width as usize, 0);
    tui_surface
        .cell_remap
        .resize(bounds.height as usize * bounds.width as usize, 0);
    tui_surface
        .fast_blinking
        .resize(bounds.height as usize * bounds.width as usize, false);
    tui_surface
        .slow_blinking
        .resize(bounds.height as usize * bounds.width as usize, false);
    tui_surface.dirty_rows.resize(bounds.height as usize, true);
    tui_surface
        .dirty_cells
        .resize(bounds.height as usize * bounds.width as usize, true);

    let cell_box = fonts.cell_box();

    rendered.resize_with(
        bounds.height as usize * bounds.width as usize,
        Rendered::default,
    );

    for (x, y, cell) in content {
        let offset = y as usize * bounds.width as usize;
        let index = offset + x as usize;

        tui_surface
            .fast_blinking
            .set(index, cell.modifier.contains(Modifier::RAPID_BLINK));
        tui_surface
            .slow_blinking
            .set(index, cell.modifier.contains(Modifier::SLOW_BLINK));

        // every other cell any of the glyphs has touched is dirty now.
        for (rx, ry, _glyph_id, render_info) in &rendered[index] {
            let glyph_pos = cell_box.cell_pos(*rx, *ry, bounds);
            let glyph_pos2 = cell_box.cell_pos(
                *rx + render_info.cached.width as i32,
                *ry + render_info.cached.height as i32,
                bounds,
            );

            for y in glyph_pos.y..=glyph_pos2.y {
                for x in glyph_pos.x..=glyph_pos2.x {
                    tui_surface
                        .dirty_cells
                        .set((y * bounds.width + x) as usize, true);
                }
                tui_surface.dirty_rows.set(y as usize, true);
            }
        }

        tui_surface.cells[index] = cell.clone();
        tui_surface.cell_font[index] = fonts.font_for_cell(cell);
        tui_surface.dirty_cells.set(index, true);

        let new_symbol_width = tui_surface.cells[index]
            .symbol()
            .chars()
            .filter(|c| c.general_category() != GeneralCategory::Format)
            .next()
            .unwrap_or(' ')
            .width()
            .unwrap_or(1);
        if index + 1 < index + new_symbol_width {
            tui_surface.cells[index + 1..index + new_symbol_width].fill(NULL_CELL);
            tui_surface.dirty_cells[index + 1..index + new_symbol_width].fill(true);
        }

        tui_surface.dirty_rows.set(y as usize, true);
    }

    {
        let mut images = Vec::new();
        let image_buffer = tui_surface.image_frame.buffer();
        let mut image_buffer = image_buffer.lock().expect("lock");
        for ImageCell {
            image_id,
            view_rect,
            view_clip,
            below_text,
            tr,
        } in image_buffer.images.iter()
        {
            let img_info = ImageInfo {
                image_id: *image_id,
                view_rect: *view_rect,
                view_clip: *view_clip,
                below_text: *below_text,
                uv_transform: *tr,
            };

            images.push(img_info);

            // find dirty images
            if let Some(pos) = tui_surface
                .images
                .iter()
                .position(|test| test.image_id == *image_id && test.view_rect == *view_rect)
            {
                let test = tui_surface.images[pos];

                if test.below_text != img_info.below_text
                    || test.uv_transform != img_info.uv_transform
                {
                    // existing image differs in render parameters.
                    tui_surface.dirty_img.push(img_info);
                } else {
                    // any row the image covers is marked as dirty.
                    let img_pos =
                        cell_box.cell_pos(img_info.view_rect.0, img_info.view_rect.1, bounds);
                    let img_pos2 = cell_box.cell_pos(
                        img_info.view_rect.0 + img_info.view_rect.2 as i32,
                        img_info.view_rect.1 + img_info.view_rect.3 as i32,
                        bounds,
                    );
                    for y in img_pos.y..=img_pos2.y {
                        if tui_surface.dirty_rows[y as usize] {
                            tui_surface.dirty_img.push(img_info);
                        }
                    }
                }
                tui_surface.images.remove(pos);
            } else {
                // new image
                tui_surface.dirty_img.push(img_info);
            }
        }

        // clear communication buffer
        image_buffer.images.clear();

        // overlapping cells of removed or dirty images must be marked as dirty.
        for img_info in tui_surface
            .images
            .iter()
            .chain(tui_surface.dirty_img.iter())
        {
            // any row the image covers is marked as dirty.
            let img_pos = cell_box.cell_pos(img_info.view_rect.0, img_info.view_rect.1, bounds);
            let img_pos2 = cell_box.cell_pos(
                img_info.view_rect.0 + img_info.view_rect.2 as i32,
                img_info.view_rect.1 + img_info.view_rect.3 as i32,
                bounds,
            );

            for y in img_pos.y..=img_pos2.y {
                for x in img_pos.x..=img_pos2.x {
                    tui_surface
                        .dirty_cells
                        .set((y * bounds.width + x) as usize, true);
                }
                tui_surface.dirty_rows.set(y as usize, true);
            }
        }
        tui_surface.images = images;
    }
}

fn flush_tui(
    bounds: ratatui_core::layout::Size,
    fonts: &Fonts<'_>,
    tui_surface: &mut TuiSurface,
    rendered: &mut Vec<Rendered>,
    wgpu_atlas: &mut WgpuAtlas,
    queue: &Queue,
    //
    tmp_plan_cache: &mut PlanCache,
    tmp_rowbuf: &mut String,
    tmp_rowbuf_to_cell: &mut Vec<u16>,
    tmp_buffer: &mut UnicodeBuffer,
) {
    // always show cursor on flush.
    tui_surface.cursor_showing = true;
    // reset blink, removes flickering.
    tui_surface.cursor_blink = 0;

    if bounds.width == 0 || bounds.height == 0 {
        return;
    }

    for (row_idx, row_cells) in tui_surface.cells.chunks(bounds.width as usize).enumerate() {
        if !tui_surface.dirty_rows[row_idx] {
            continue;
        }

        let row_offset = row_idx * bounds.width as usize;

        // This block concatenates the strings for the row into one string for bidi
        // resolution, then maps bytes for the string to their associated cell index. It
        // also maps the row's cell index to the font that can source all glyphs for
        // that cell.
        tmp_rowbuf.clear();
        tmp_rowbuf_to_cell.clear();

        for (cell_idx, cell) in row_cells.iter().enumerate() {
            if !cell.skip {
                tmp_rowbuf.push_str(cell.symbol());
                tmp_rowbuf_to_cell.resize(
                    tmp_rowbuf_to_cell.len() + cell.symbol().len(),
                    cell_idx as u16,
                );
            }

            tui_surface.cell_remap[row_offset + cell_idx] = cell_idx as u16;
        }

        // run text shaping
        let bidi = ParagraphBidiInfo::new(&tmp_rowbuf, None);
        let (levels, runs) = bidi.visual_runs(0..bidi.levels.len());

        // when bidi kicks in dirty_cell ceases to work...
        if runs.len() > 1 {
            for cell_idx in 0..bounds.width as usize {
                tui_surface.dirty_cells.set(row_offset + cell_idx, true);
            }
        }

        // rebuild rendered glyphs from scratch
        for cell_idx in 0..bounds.width as usize {
            if tui_surface.dirty_cells[row_offset + cell_idx] {
                rendered[row_offset + cell_idx].clear();
            }
        }

        let mut current_font_id = None;
        let mut current_level = None;
        let mut current_cell_idx = -1;
        for (level, range) in runs.into_iter().map(|run| (levels[run.start], run)) {
            let bidi_run_chars = &tmp_rowbuf[range.clone()];
            let bidi_run_cells = &tmp_rowbuf_to_cell[range.clone()];
            let min_cell_idx = *bidi_run_cells.first().expect("first") as usize;
            let max_cell_idx = *bidi_run_cells.last().expect("last") as usize;
            let mut start_cell_idx = None;

            for (ch_idx, ch) in bidi_run_chars.char_indices() {
                if ch.general_category() == GeneralCategory::Format {
                    // skip Format, no longer needed after bidi.
                    continue;
                }

                let cell_idx = bidi_run_cells[ch_idx] as usize;

                let font_id = tui_surface.cell_font[row_offset + cell_idx];
                if let (Some(current_font_id), Some(current_level)) =
                    (current_font_id, current_level)
                    && (font_id != current_font_id || level != current_level)
                {
                    let mut buffer = mem::take(tmp_buffer);
                    let current_font = fonts.get_by_id(current_font_id);

                    *tmp_buffer = shape(
                        row_idx,
                        row_cells,
                        &tui_surface.dirty_cells[row_offset..row_offset + bounds.width as usize],
                        &tui_surface.cell_remap[row_offset..row_offset + bounds.width as usize],
                        &tmp_rowbuf,
                        &tmp_rowbuf_to_cell,
                        shape_with_plan(
                            current_font.face(),
                            tmp_plan_cache.get(current_font_id, current_font, &mut buffer),
                            buffer,
                        ),
                        current_font_id,
                        fonts.cell_box(),
                        current_font,
                        tui_surface.cursor_visible,
                        tui_surface.cursor,
                        &mut rendered[row_offset..row_offset + bounds.width as usize],
                        wgpu_atlas,
                        queue,
                    );
                }

                if current_cell_idx == -1 {
                    current_cell_idx += 1;
                } else if current_cell_idx != cell_idx as i32 {
                    let symbol_width = row_cells[current_cell_idx as usize]
                        .symbol()
                        .chars()
                        .filter(|c| c.general_category() != GeneralCategory::Format)
                        .next()
                        .unwrap_or(' ')
                        .width()
                        .unwrap_or(1);
                    current_cell_idx = current_cell_idx + symbol_width as i32;
                }

                if start_cell_idx.is_none() {
                    start_cell_idx = Some(current_cell_idx);
                }

                if level.is_rtl() {
                    // rtl flip visible cell index for this run.
                    let start_cell_idx = start_cell_idx.expect("start_cell_idx");
                    let len_rtl = (max_cell_idx - min_cell_idx) as i32;
                    let in_rtl = current_cell_idx - start_cell_idx;
                    let view_idx = start_cell_idx + len_rtl - in_rtl;

                    if (cell_idx as u16, row_idx as u16) == tui_surface.cursor {
                        tui_surface.cursor_style = tui_surface.cursor_style.to_rtl();
                    }

                    tui_surface.cell_remap[row_offset + cell_idx] = view_idx as u16;
                } else {
                    if (cell_idx as u16, row_idx as u16) == tui_surface.cursor {
                        tui_surface.cursor_style = tui_surface.cursor_style.to_ltr();
                    }
                    tui_surface.cell_remap[row_offset + cell_idx] = current_cell_idx as u16;
                }

                tmp_buffer.add(ch, (range.start + ch_idx) as u32);

                current_font_id = Some(font_id);
                current_level = Some(level);
            }
        }

        if let Some(current_font_id) = current_font_id {
            let mut buffer = mem::take(tmp_buffer);
            let current_font = fonts.get_by_id(current_font_id);
            *tmp_buffer = shape(
                row_idx,
                row_cells,
                &tui_surface.dirty_cells[row_offset..row_offset + bounds.width as usize],
                &tui_surface.cell_remap[row_offset..row_offset + bounds.width as usize],
                &tmp_rowbuf,
                tmp_rowbuf_to_cell,
                shape_with_plan(
                    current_font.face(),
                    tmp_plan_cache.get(current_font_id, current_font, &mut buffer),
                    buffer,
                ),
                current_font_id,
                fonts.cell_box(),
                current_font,
                tui_surface.cursor_visible,
                tui_surface.cursor,
                &mut rendered[row_offset..row_offset + bounds.width as usize],
                wgpu_atlas,
                queue,
            );
        }
    }
}

// shape a part of one row.
//
// the glyphs come as a GlyphBuffer provided by the bidi algorithm.
// each glyph is mapped to a cell, which in turn might be mapped to a
// visible cell if there is any reordering during bidi.
//
// then the glyph is positioned and rendered if it is not already in the
// glyph-cache.
//
// Positioning of glyphs always restarts with each new cell.
// This ensures that the output is mostly cell-aligned and makes
// the final result more predictable.
fn shape(
    row_idx: usize,
    row: &[Cell],
    dirty_cells: &BitSlice,
    cell_remap: &[u16],
    buf_str: &str,
    buf_to_cell: &[u16],
    buffer: GlyphBuffer,
    font_id: u64,
    cell_box: CellBox,
    font: &Font<'_>,
    cursor_visible: bool,
    cursor: (u16, u16),
    rendered: &mut [Rendered],
    wgpu_atlas: &mut WgpuAtlas,
    queue: &Queue,
) -> UnicodeBuffer {
    let metrics = font.face();

    let mut x = 0;
    let mut default_chars_wide = 1;
    #[allow(unused_assignments)]
    let mut chars_wide = 1;
    let mut last_cell_idx: Option<usize> = None;
    let mut last_advance = 0;
    for (info, position) in buffer
        .glyph_infos()
        .iter()
        .zip(buffer.glyph_positions().iter())
    {
        let cell_idx = buf_to_cell[info.cluster as usize] as usize;

        if !dirty_cells[cell_idx] {
            continue;
        }

        let cell = &row[cell_idx];
        let ch = buf_str[info.cluster as usize..]
            .chars()
            .next()
            .unwrap_or_default();

        // Every cell has it's defined position on the grid.
        // This position is used as a starting point from which
        // every glyph in the cell is positioned.
        let mut first_glyph = false;
        if last_cell_idx != Some(cell_idx) {
            x = cell_remap[cell_idx] as i32 * cell_box.width as i32;
            // zero width are still 1 cell wide.
            // there is KHMER SIGN BEYYAL with width 3.
            // we ignore that one completely.
            default_chars_wide = ch.width().unwrap_or(1).max(1).min(2);
            chars_wide = default_chars_wide;
            assert_ne!(chars_wide, 0);
            last_advance = 0;
            first_glyph = true;
        } else {
            // zero width are still 1 cell wide.
            // there is KHMER SIGN BEYYAL with width 3.
            // we ignore that one completely.
            chars_wide = ch.width().unwrap_or(default_chars_wide).max(1).min(2);
            assert_ne!(chars_wide, 0);
        }

        // if we have a combining '.undef'. skip it completely.
        if last_cell_idx == Some(cell_idx) {
            if ch.general_category_group() == GeneralCategoryGroup::Mark && info.glyph_id == 0 {
                continue;
            }
        }

        let block_char = (ch as u32) >= 0x2500 && (ch as u32) <= 0x259F;
        let advance_scale = font.scale_x(info.glyph_id as u16, block_char, chars_wide as u32);
        let advance_scale_y = font.scale_y(info.glyph_id as u16, block_char);

        let basey = row_idx as i32 * cell_box.height as i32
            + (position.y_offset as f32 * advance_scale_y) as i32;

        let glyph_advance = (position.x_advance as f32 * advance_scale) as i32;
        let glyph_offset = (position.x_offset as f32 * advance_scale) as i32;

        // combining glyph
        let basex;
        if last_cell_idx == Some(cell_idx) {
            if glyph_offset < 0 {
                basex = x + glyph_offset;
                last_advance += glyph_advance;
                x += glyph_advance;
            } else {
                basex = x + glyph_offset - last_advance;
                last_advance += glyph_advance;
                x += glyph_advance;
            }
        } else {
            basex = x + glyph_offset;
            last_advance = glyph_advance;
            x += glyph_advance;
        }

        last_cell_idx = Some(cell_idx);

        let key = Key {
            style: cell
                .modifier
                .intersection(Modifier::BOLD | Modifier::ITALIC),
            glyph: info.glyph_id,
            width: chars_wide as u8,
            font: font_id,
        };

        let cached =
            wgpu_atlas
                .cached
                .get(&key, chars_wide as u32 * cell_box.width, cell_box.height);

        let mut view_modifier = cell.modifier;
        if !first_glyph {
            view_modifier.set(Modifier::UNDERLINED, false);
            view_modifier.set(Modifier::CROSSED_OUT, false);
        }

        let cursor_pos =
            if first_glyph && cursor_visible && (cell_idx as u16, row_idx as u16) == cursor {
                font.underline_metrics(cell_box.ascender, cached.height)
            } else {
                (0, 0)
            };

        let underline_pos = if view_modifier.contains(Modifier::UNDERLINED) {
            font.underline_metrics(cell_box.ascender, cached.height)
        } else {
            (0, 0)
        };
        let strikeout_pos = if view_modifier.contains(Modifier::CROSSED_OUT) {
            font.strikeout_metrics(cell_box.ascender)
        } else {
            (0, 0)
        };

        if cached.cached() {
            rendered[cell_idx].push((
                basex,
                basey,
                GlyphId(info.glyph_id as _),
                RenderInfo {
                    cached: *cached,
                    fg: cell.fg,
                    bg: cell.bg,
                    modifier: view_modifier,
                    underline_pos_min: underline_pos.0 as u16,
                    underline_pos_max: underline_pos.1 as u16,
                    strikeout_pos_min: strikeout_pos.0 as u16,
                    strikeout_pos_max: strikeout_pos.1 as u16,
                    cursor_pos_min: cursor_pos.0 as u16,
                    cursor_pos_max: cursor_pos.1 as u16,
                },
            ));

            continue;
        }

        let is_emoji =
            ch.is_emoji_char() && ch.general_category_group() != GeneralCategoryGroup::Number;

        let (cached, image) = rasterize_glyph(
            cached,
            metrics,
            info,
            view_modifier.contains(Modifier::BOLD),
            view_modifier.contains(Modifier::ITALIC),
            advance_scale,
            advance_scale_y,
            cell_box.ascender,
            is_emoji,
            block_char,
            ch.general_category(),
            font.is_fallback(),
        );

        // remember colored flag for the glyph.
        wgpu_atlas.cached.update_colored(&key, cached.color);

        rendered[cell_idx].push((
            basex,
            basey,
            GlyphId(info.glyph_id as _),
            RenderInfo {
                cached,
                fg: cell.fg,
                bg: cell.bg,
                modifier: view_modifier,
                underline_pos_min: underline_pos.0 as u16,
                underline_pos_max: underline_pos.1 as u16,
                strikeout_pos_min: strikeout_pos.0 as u16,
                strikeout_pos_max: strikeout_pos.1 as u16,
                cursor_pos_min: cursor_pos.0 as u16,
                cursor_pos_max: cursor_pos.1 as u16,
            },
        ));

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &wgpu_atlas.text_cache,
                mip_level: 0,
                origin: Origin3d {
                    x: cached.x,
                    y: cached.y,
                    z: 0,
                },
                aspect: TextureAspect::All,
            },
            bytemuck::cast_slice(&image),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(cached.width * size_of::<u32>() as u32),
                rows_per_image: Some(cached.height),
            },
            Extent3d {
                width: cached.width,
                height: cached.height,
                depth_or_array_layers: 1,
            },
        );
    }

    buffer.clear()
}

fn flush_blink(
    blinking: Blinking,
    bounds: ratatui_core::layout::Size,
    cell_box: CellBox,
    tui_surface: &mut TuiSurface,
    rendered: &Vec<Rendered>,
    wgpu_vertices: &mut WgpuVertices,
) {
    wgpu_vertices.clear();

    tui_surface.blink = tui_surface.blink.wrapping_add(1);
    if tui_surface.fast_blink_divisor != 0
        && tui_surface.blink % tui_surface.fast_blink_divisor == 0
    {
        tui_surface.fast_blink_showing = !tui_surface.fast_blink_showing;
    }
    if tui_surface.slow_blink_divisor != 0
        && tui_surface.blink % tui_surface.slow_blink_divisor == 0
    {
        tui_surface.slow_blink_showing = !tui_surface.slow_blink_showing;
    }

    tui_surface.cursor_blink = tui_surface.cursor_blink.wrapping_add(1);
    if tui_surface.cursor_divisor != 0 && tui_surface.cursor_blink % tui_surface.cursor_divisor == 0
    {
        tui_surface.cursor_showing = !tui_surface.cursor_showing;
    }

    let mut cell_indexes = if blinking & Blinking::TEXT {
        tui_surface
            .fast_blinking
            .iter_ones()
            .chain(tui_surface.slow_blinking.iter_ones())
            .collect()
    } else {
        Vec::new()
    };
    if tui_surface.cursor_visible && blinking & Blinking::CURSOR {
        cell_indexes.push(
            tui_surface.cursor.1 as usize * bounds.width as usize + tui_surface.cursor.0 as usize,
        )
    };

    let mut index_offset = 0;
    for index in cell_indexes.iter() {
        if let Some(to_render) = rendered.get(*index) {
            append_rendered(&tui_surface, to_render, &mut index_offset, wgpu_vertices);
        }
    }

    // overlapping cells of removed or dirty images must be marked as dirty.
    let mut index_offset = 0;
    for img_info in tui_surface
        .images
        .iter()
        .chain(tui_surface.dirty_img.iter())
    {
        // any row the image covers is marked as dirty.
        let img_pos = cell_box.cell_pos(img_info.view_rect.0, img_info.view_rect.1, bounds);
        let img_pos2 = cell_box.cell_pos(
            img_info.view_rect.0 + img_info.view_rect.2 as i32,
            img_info.view_rect.1 + img_info.view_rect.3 as i32,
            bounds,
        );

        for y in img_pos.y..=img_pos2.y {
            for x in img_pos.x..=img_pos2.x {
                let index = (y * bounds.width + x) as usize;

                if cell_indexes.contains(&index) {
                    append_rendered_image(
                        &ImageInfo {
                            view_clip: (
                                x as i32 * cell_box.width as i32,
                                y as i32 * cell_box.height as i32,
                                cell_box.width,
                                cell_box.height,
                            ),
                            ..*img_info
                        },
                        &mut index_offset,
                        wgpu_vertices,
                    );
                }
            }
            tui_surface.dirty_rows.set(y as usize, true);
        }
    }
}

fn append_dirty_rows(
    tui_surface: &mut TuiSurface,
    wgpu_post_process: &dyn PostProcessor,
    rendered: &Vec<Rendered>,
    wgpu_vertices: &mut WgpuVertices,
) {
    if wgpu_post_process.needs_update()
        || tui_surface.dirty_rows.any()
        || !tui_surface.dirty_img.is_empty()
    {
        wgpu_vertices.clear();

        let mut index_offset = 0;
        for cell_idx in tui_surface.dirty_cells.iter_ones() {
            let to_render = &rendered[cell_idx];
            append_rendered(tui_surface, to_render, &mut index_offset, wgpu_vertices);
        }

        let mut index_offset = 0;
        for img_info in tui_surface.dirty_img.iter() {
            append_rendered_image(img_info, &mut index_offset, wgpu_vertices);
        }

        tui_surface
            .dirty_rows
            .iter_mut()
            .for_each(|mut v| *v = false);
        tui_surface
            .dirty_cells
            .iter_mut()
            .for_each(|mut v| *v = false);
        tui_surface.dirty_img.clear();
    }
}

fn append_rendered_image(
    to_render: &ImageInfo,
    index_offset: &mut u32,
    vertices: &mut WgpuVertices,
) {
    let x = to_render.view_rect.0 as f32;
    let y = to_render.view_rect.1 as f32;
    let width = to_render.view_rect.2 as f32;
    let height = to_render.view_rect.3 as f32;
    let uvx = 0.0f32;
    let uvy = 0.0f32;

    vertices.img_render.push(*to_render);

    vertices.img_indices.push([
        *index_offset,     // x, y
        *index_offset + 1, // x + w, y
        *index_offset + 2, // x, y + h
        *index_offset + 2, // x, y + h
        *index_offset + 3, // x + w, y + h
        *index_offset + 1, // x + w, y
    ]);
    *index_offset += 4;

    vertices.img_vertices.push(ImgVertexMember {
        vertex: [x, y],
        uv: [uvx, uvy],
    });
    vertices.img_vertices.push(ImgVertexMember {
        vertex: [x + width, y],
        uv: [uvx + 1.0, uvy],
    });
    vertices.img_vertices.push(ImgVertexMember {
        vertex: [x, y + height],
        uv: [uvx, uvy + 1.0],
    });
    vertices.img_vertices.push(ImgVertexMember {
        vertex: [x + width, y + height],
        uv: [uvx + 1.0, uvy + 1.0],
    });
}

fn append_rendered(
    tui_surface: &TuiSurface,
    to_render: &Rendered,
    index_offset: &mut u32,
    vertices: &mut WgpuVertices,
) {
    for (
        x,
        y,
        _,
        RenderInfo {
            cached,
            fg,
            bg,
            modifier,
            underline_pos_min,
            underline_pos_max,
            strikeout_pos_min,
            strikeout_pos_max,
            cursor_pos_min,
            cursor_pos_max,
        },
    ) in to_render.iter()
    {
        let alpha = if modifier.contains(Modifier::HIDDEN)
            | (modifier.contains(Modifier::RAPID_BLINK) && !tui_surface.fast_blink_showing)
            | (modifier.contains(Modifier::SLOW_BLINK) && !tui_surface.slow_blink_showing)
        {
            0
        } else if modifier.contains(Modifier::DIM) {
            127
        } else {
            255
        };

        let reverse = modifier.contains(Modifier::REVERSED);
        let fg_color = if reverse {
            tui_surface.colors.c2c(*bg, tui_surface.reset_bg)
        } else {
            tui_surface.colors.c2c(*fg, tui_surface.reset_fg)
        };
        let fg_color_u32: u32 = u32::from_le_bytes([fg_color[0], fg_color[1], fg_color[2], alpha]);

        let cursor_color_u32 = if tui_surface.cursor_color != ratatui_core::style::Color::Reset {
            let cur_color = tui_surface
                .colors
                .c2c(tui_surface.cursor_color, tui_surface.reset_fg);
            u32::from_le_bytes([cur_color[0], cur_color[1], cur_color[2], 99])
        } else {
            u32::from_le_bytes([fg_color[0], fg_color[1], fg_color[2], 99])
        };

        let bg_color = if reverse {
            tui_surface.colors.c2c(*fg, tui_surface.reset_fg)
        } else {
            tui_surface.colors.c2c(*bg, tui_surface.reset_bg)
        };
        let bg_color_u32 = u32::from_le_bytes([bg_color[0], bg_color[1], bg_color[2], 255]);

        let underline_pos =
            ((*underline_pos_min as u32 + cached.y) << 16) | (*underline_pos_max as u32 + cached.y);
        let strikeout_pos =
            ((*strikeout_pos_min as u32 + cached.y) << 16) | (*strikeout_pos_max as u32 + cached.y);

        let mut cursor_pos = 0x0000_0000;
        if tui_surface.cursor_visible
            && tui_surface.cursor_showing
            && cursor_pos_min != cursor_pos_max
        {
            match tui_surface.cursor_style {
                CursorStyle::Block => {
                    cursor_pos = 0x0002_0000 | cached.width << 8 | 0x0000_0000;
                    // horizontal
                }
                CursorStyle::Underscore => {
                    cursor_pos = 0x0003_0000
                        | (*cursor_pos_max as u32 + cached.y + 1) << 8
                        | (*cursor_pos_min as u32 + cached.y);
                }
                CursorStyle::BoldUnderscore => {
                    cursor_pos = 0x0003_0000
                        | (*cursor_pos_max as u32 + cached.y + 3) << 8
                        | (*cursor_pos_min as u32 + cached.y);
                }
                CursorStyle::Bar => {
                    let cursor_width = (*cursor_pos_max).abs_diff(*cursor_pos_min) as u32;
                    cursor_pos = 0x0002_0000 | (cursor_width + 1) << 8 | 0x0000_0000;
                }
                CursorStyle::BoldBar => {
                    let cursor_width = (*cursor_pos_max).abs_diff(*cursor_pos_min) as u32;
                    cursor_pos = 0x0002_0000 | (cursor_width + 3) << 8 | 0x0000_0000;
                }
                CursorStyle::RtlBar => {
                    let cursor_width = (*cursor_pos_max).abs_diff(*cursor_pos_min) as u32;
                    cursor_pos = 0x0002_0000
                        | cached.width << 8
                        | (cached.width.saturating_sub(cursor_width + 1));
                }
                CursorStyle::RtlBoldBar => {
                    let cursor_width = (*cursor_pos_max).abs_diff(*cursor_pos_min) as u32;
                    cursor_pos = 0x0002_0000
                        | cached.width << 8
                        | (cached.width.saturating_sub(cursor_width + 3))
                }
            }
        }

        vertices.text_indices.push([
            *index_offset,     // x, y
            *index_offset + 1, // x + w, y
            *index_offset + 2, // x, y + h
            *index_offset + 2, // x, y + h
            *index_offset + 3, // x + w, y + h
            *index_offset + 1, // x + w, y
        ]);
        *index_offset += 4;

        let x = *x as f32;
        let y = *y as f32;
        let width = cached.width as f32;
        let height = cached.height as f32;
        let uvx = cached.x as f32;
        let uvy = cached.y as f32;

        vertices.bg_vertices.push(TextBgVertexMember {
            vertex: [x, y],
            bg_color: bg_color_u32,
        });
        vertices.bg_vertices.push(TextBgVertexMember {
            vertex: [x + width, y],
            bg_color: bg_color_u32,
        });
        vertices.bg_vertices.push(TextBgVertexMember {
            vertex: [x, y + height],
            bg_color: bg_color_u32,
        });
        vertices.bg_vertices.push(TextBgVertexMember {
            vertex: [x + width, y + height],
            bg_color: bg_color_u32,
        });

        vertices.text_vertices.push(TextVertexMember {
            vertex: [x, y],
            uv: [uvx, uvy],
            uv_x0: uvx,
            fg_color: fg_color_u32,
            color_glyph: cached.color as u32,
            underline_pos,
            strikeout_pos,
            cursor_pos,
            cursor_color: cursor_color_u32,
        });
        vertices.text_vertices.push(TextVertexMember {
            vertex: [x + width, y],
            uv: [uvx + width, uvy],
            uv_x0: uvx,
            fg_color: fg_color_u32,
            color_glyph: cached.color as u32,
            underline_pos,
            strikeout_pos,
            cursor_pos,
            cursor_color: cursor_color_u32,
        });
        vertices.text_vertices.push(TextVertexMember {
            vertex: [x, y + height],
            uv: [uvx, uvy + height],
            uv_x0: uvx,
            fg_color: fg_color_u32,
            color_glyph: cached.color as u32,
            underline_pos,
            strikeout_pos,
            cursor_pos,
            cursor_color: cursor_color_u32,
        });
        vertices.text_vertices.push(TextVertexMember {
            vertex: [x + width, y + height],
            uv: [uvx + width, uvy + height],
            uv_x0: uvx,
            fg_color: fg_color_u32,
            color_glyph: cached.color as u32,
            underline_pos,
            strikeout_pos,
            cursor_pos,
            cursor_color: cursor_color_u32,
        });
    }
}
