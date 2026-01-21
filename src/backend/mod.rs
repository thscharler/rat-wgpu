use crate::backend::image_buffer::{ImageBuffer, ImageZ};
use crate::backend::surface::RenderSurface;
use crate::text_atlas::{Atlas, CacheRect};
use crate::colors::{ColorTable, Rgb};
use crate::cursor::CursorStyle;
use bitvec::vec::BitVec;
use indexmap::IndexMap;
use raqote::Transform;
use ratatui_core::buffer::Cell;
use ratatui_core::style::Modifier;
use rustybuzz::ttf_parser::GlyphId;
use std::collections::HashMap;
use std::hash::RandomState;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use wgpu::{
    BindGroup, BindGroupLayout, Buffer, Device, Queue, RenderPipeline, Sampler,
    SurfaceConfiguration, Texture, TextureView,
};

pub(super) mod backend;
pub(super) mod builder;
pub(super) mod image_buffer;
mod plan_cache;
mod surface;

/// Handle for any added image.
///
/// When the handle is dropped, the backing texture will be dropped after
/// the next flush().
#[derive(Debug, Default, Clone)]
pub struct ImageHandle {
    id: usize,
    dropped: Arc<AtomicBool>,
}

impl Drop for ImageHandle {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::Release)
    }
}

const NULL_CELL: Cell = {
    let mut c = Cell::new("");
    c.skip = true;
    c
};

const ONE_CELL: Cell = Cell::new(" ");

#[derive(Debug)]
struct RenderInfo {
    cached: CacheRect,
    fg: ratatui_core::style::Color,
    bg: ratatui_core::style::Color,
    modifier: Modifier,
    underline_pos_min: u16,
    underline_pos_max: u16,
    strikeout_pos_min: u16,
    strikeout_pos_max: u16,
    cursor_pos_min: u16,
    cursor_pos_max: u16,
}

#[derive(Debug, Clone, Copy)]
struct ImageInfo {
    id: usize,
    view_rect: (u32, u32, u32, u32),

    img_size: (u32, u32),
    z: ImageZ,
    uv_transform: Transform,
}

/// Map from (x, y, glyph) -> (cell index, cache entry).
/// We use an IndexMap because we want a consistent rendering order for
/// vertices.
// todo: need indexmap?
type Rendered = IndexMap<(i32, i32, GlyphId), RenderInfo, RandomState>;

struct TuiSurface {
    // communication with the application. can run in parallel
    // with ratatui's draw() function.
    image_buffer: ImageBuffer,

    // current images
    images: Vec<ImageInfo>,
    // cell data
    cells: Vec<Cell>,
    // font detection
    cell_font: Vec<u64>,
    // bidi can reorder cells in a row.
    // points to the cell with the actual cell-data.
    cell_remap: Vec<u16>,
    // rows marked as dirty for redraw.
    dirty_rows: BitVec,
    // exact cells marked as dirty. only these will be rendered.
    dirty_cells: BitVec,
    // images prepared to render.
    dirty_img: Vec<ImageInfo>,
    // blink flag for each cell
    fast_blinking: BitVec,
    // blink flag for each cell
    slow_blinking: BitVec,

    // screen cursor
    cursor: (u16, u16),
    cursor_color: ratatui_core::style::Color,
    cursor_style: CursorStyle,
    // cursor status set by the application.
    cursor_visible: bool,
    // every time blink() is called this value is increased by 1.
    // if cursor_blink is divisible by cursor_divisor the actual
    // cursor_showing state is switched.
    //
    // this allows to use a single blink for all blinking effects.
    //
    // the cursor is separate, as it will reset to showing+cursor_blink=0
    // when the cursor position changes.
    cursor_blink: u8,
    cursor_divisor: u8,
    // cursor is showing due to the blink rate. combines with cursor_visible
    // for actual rendering.
    cursor_showing: bool,

    // This is increased every time blink() is called. Fast/Slow blinking
    // use a different divisor of this base rate to switch their
    // showing state.
    blink: u8,
    fast_blink_divisor: u8,
    fast_blink_showing: bool,
    slow_blink_divisor: u8,
    slow_blink_showing: bool,

    // Color map for the base16 colors.
    colors: ColorTable,
    // FG-Color for Color::Reset
    reset_fg: Rgb,
    // BG-Color for Color::Reset
    reset_bg: Rgb,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct TextBgVertexMember {
    vertex: [f32; 2],
    bg_color: u32,
}

// Vertex + UVCoord + Color
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct TextVertexMember {
    vertex: [f32; 2],
    uv: [f32; 2],
    uv_x0: f32,
    fg_color: u32,
    color_glyph: u32,
    underline_pos: u32,
    strikeout_pos: u32,
    cursor_pos: u32,
    cursor_color: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ImgVertexMember {
    vertex: [f32; 2],
    uv: [f32; 2],
}

struct ImgPipeline {
    pipeline: RenderPipeline,
    fs_uniforms: BindGroup,
    fragment_shader_layout: BindGroupLayout,
    image_shader_layout: BindGroupLayout,
}

struct TextCacheBgPipeline {
    pipeline: RenderPipeline,
    fs_uniforms: BindGroup,
}

struct TextCacheFgPipeline {
    pipeline: RenderPipeline,
    fs_uniforms: BindGroup,
    atlas_bindings: BindGroup,
}

struct WgpuBase<'s> {
    surface: RenderSurface<'s>,
    surface_config: SurfaceConfiguration,
    device: Device,
    queue: Queue,
    text_dest_view: TextureView,
}

struct WgpuAtlas {
    cached: Atlas,
    text_cache: Texture,
}

struct WgpuImage {
    texture: TextureView,
    width: u32,
    height: u32,
    dropped: Arc<AtomicBool>,
}

struct WgpuImages {
    img_id: usize,
    img: HashMap<usize, WgpuImage>,
}

struct WgpuVertices {
    text_indices: Vec<[u32; 6]>,
    bg_vertices: Vec<TextBgVertexMember>,
    text_vertices: Vec<TextVertexMember>,

    img_render: Vec<ImageInfo>,
    img_indices: Vec<[u32; 6]>,
    img_vertices: Vec<ImgVertexMember>,
}

impl WgpuVertices {
    fn is_empty(&self) -> bool {
        self.bg_vertices.is_empty() && self.text_vertices.is_empty() && self.img_vertices.is_empty()
    }

    fn clear(&mut self) {
        self.text_indices.clear();
        self.bg_vertices.clear();
        self.text_vertices.clear();
        self.img_vertices.clear();
        self.img_indices.clear();
        self.img_render.clear();
    }
}

struct WgpuPipeline {
    sampler: Sampler,

    text_screen_size_buffer: Buffer,

    text_bg_compositor: TextCacheBgPipeline,
    text_fg_compositor: TextCacheFgPipeline,

    img_compositor: ImgPipeline,
}
