use crate::Error;
use crate::backend::backend::WgpuBackend;
use crate::backend::plan_cache::PlanCache;
use crate::backend::surface::RenderSurface;
use crate::backend::{
    ImgPipeline, ImgVertexMember, TextBgVertexMember, TextCacheBgPipeline, TextCacheFgPipeline,
    TextVertexMember, TuiSurface, WgpuAtlas, WgpuBase, WgpuImages, WgpuPipeline, WgpuVertices,
};
use crate::colors::ColorTable;
use crate::cursor::CursorStyle;
use crate::font::{Font, Fonts, fallback_emoji_font, fallback_font, fallback_symbol_font};
use crate::image::ImageBuffer;
use crate::postprocessor::PostProcessorBuilder;
use crate::postprocessor::default::DefaultPostProcessorBuilder;
use crate::text_atlas::Atlas;
use log::{debug, info};
use ratatui_core::style::Color;
use rustybuzz::UnicodeBuffer;
use std::num::NonZeroU64;
use std::sync::{Arc, Mutex};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    AddressMode, Backends, BindGroup, BindGroupDescriptor, BindGroupEntry,
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType, BlendState,
    Buffer, BufferBindingType, BufferDescriptor, BufferUsages, ColorTargetState, ColorWrites,
    Device, Extent3d, FilterMode, FragmentState, Instance, InstanceDescriptor, InstanceFlags,
    Limits, MemoryHints, MipmapFilterMode, MultisampleState, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PresentMode, PrimitiveState, PrimitiveTopology,
    RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
    Surface, SurfaceTarget, TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType,
    TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension, VertexBufferLayout,
    VertexState, VertexStepMode, include_wgsl, vertex_attr_array,
};

const CACHE_WIDTH: u32 = 1800;
const CACHE_HEIGHT: u32 = 1200;

pub struct WgpuBackendBuilder {
    postprocessor: Box<dyn PostProcessorBuilder>,
    fonts: Fonts,
    backends: String,
    instance: Option<Instance>,
    limits: Option<Limits>,
    present_mode: Option<PresentMode>,
    width: u32,
    height: u32,
    colors: ColorTable,
    reset_fg: Color,
    reset_bg: Color,
    fast_blink: u8,
    slow_blink: u8,
    cursor_blink: u8,
    cursor_style: CursorStyle,
    cursor_color: Color,
}

impl Default for WgpuBackendBuilder {
    fn default() -> Self {
        Self {
            postprocessor: Box::new(DefaultPostProcessorBuilder::default()),
            fonts: Default::default(),
            backends: Default::default(),
            instance: Default::default(),
            limits: Default::default(),
            present_mode: Default::default(),
            width: 100,
            height: 100,
            colors: Default::default(),
            reset_fg: Color::White,
            reset_bg: Color::Black,
            fast_blink: 1,
            slow_blink: 5,
            cursor_blink: 5,
            cursor_style: Default::default(),
            cursor_color: Color::Reset,
        }
    }
}

impl WgpuBackendBuilder {
    pub fn new() -> Self {
        Self::default()
    }
}

impl WgpuBackendBuilder {
    /// New builder with some postprocessing after the rendering.
    pub fn with_postprocessor(mut self, builder: Box<dyn PostProcessorBuilder>) -> Self {
        self.postprocessor = builder;
        self
    }

    /// Use one of the given Backends.
    #[must_use]
    pub fn with_backends(mut self, backends: &str) -> Self {
        self.backends = backends.into();
        self
    }

    /// Use the supplied [`wgpu::Instance`] when building the backend.
    #[must_use]
    pub fn with_instance(mut self, instance: Instance) -> Self {
        self.instance = Some(instance);
        self
    }

    /// Use the specified font size in pixels. Defaults to 24px.
    ///
    /// __Panic__
    ///
    /// Size 0 panics.
    ///
    /// __Note__
    ///
    /// If no fallback fonts have been set before, this will
    /// initialize a builtin list of fallback fonts.
    #[must_use]
    pub fn with_font_size_px(mut self, size: u32) -> Self {
        self.fonts.set_height_px(size);
        self
    }

    /// Add one font for rendering.
    /// You can add multiple fonts, especially all the fonts of a font-family.
    pub fn with_font(mut self, font: Font) -> Self {
        self.fonts.add_font(font);
        self
    }

    /// Add all the given fonts for rendering.
    pub fn with_fonts<I: IntoIterator<Item = Font>>(mut self, fonts: I) -> Self {
        for font in fonts {
            self.fonts.add_font(font);
        }
        self
    }

    /// Add a fallback font.
    /// You can add multiple fallbacks.
    pub fn with_fallback_font(mut self, font: Font) -> Self {
        self.fonts.add_fallback(font);
        self
    }

    /// Add all as fallback font.
    /// You can add multiple fallbacks.
    pub fn with_fallback_fonts<I: IntoIterator<Item = Font>>(mut self, fonts: I) -> Self {
        for font in fonts {
            self.fonts.add_fallback(font);
        }
        self
    }

    /// Use the specified [`wgpu::Limits`]. Defaults to
    /// [`wgpu::Adapter::limits`].
    #[must_use]
    pub fn with_limits(mut self, limits: Limits) -> Self {
        self.limits = Some(limits);
        self
    }

    /// Use the specified [`wgpu::PresentMode`].
    #[must_use]
    pub fn with_present_mode(mut self, mode: PresentMode) -> Self {
        self.present_mode = Some(mode);
        self
    }

    /// Use the specified height and width when creating the surface.
    ///
    /// Defaults to 100x100.
    /// Minimum size depends on the font. The window is at least 1x1 cells sized.
    #[must_use]
    pub fn with_width_and_height(mut self, width: u32, height: u32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    /// Use the specified [`ColorTable`] for the base-16 colors.
    /// There is a default value for this.
    pub fn with_color_table(mut self, colors: ColorTable) -> Self {
        self.colors = colors;
        self
    }

    /// Use the specified [Color](ratatui_core::style::Color) for
    /// the default foreground color. Defaults to Black.
    #[must_use]
    pub fn with_fg_color(mut self, fg: Color) -> Self {
        self.reset_fg = fg;
        self
    }

    /// Use the specified [Color](ratatui_core::style::Color) for
    /// the default background color. Defaults to White.
    #[must_use]
    pub fn with_bg_color(mut self, bg: Color) -> Self {
        self.reset_bg = bg;
        self
    }

    /// Initial cursor-color.
    #[must_use]
    pub fn with_cursor_color(mut self, color: Color) -> Self {
        self.cursor_color = color;
        self
    }

    /// Initial cursor-style.
    #[must_use]
    pub fn with_cursor_style(mut self, style: CursorStyle) -> Self {
        self.cursor_style = style;
        self
    }

    /// WgpuBackend doesn't have a builtin timer for blinking text and cursor.
    /// Instead, it relies on the application to call [WgpuBackend::blink]
    /// regularly (every 200ms is fine). Every call to blink increases
    /// an internal counter.
    ///
    /// The cursor blink state is switched whenever `internal % counter` is 0.
    ///
    /// Default is 5.
    #[must_use]
    pub fn with_cursor_blink(mut self, counter: u8) -> Self {
        self.cursor_blink = counter;
        self
    }

    /// WgpuBackend doesn't have a builtin timer for blinking text and cursor.
    /// Instead, it relies on the application to call [WgpuBackend::blink]
    /// regularly (every 200ms is fine). Every call to blink increases
    /// an internal counter.
    ///
    /// The blink state is switched whenever `internal % counter` is 0.
    ///
    /// Default is 1.
    #[must_use]
    pub fn with_rapid_blink(mut self, counter: u8) -> Self {
        self.fast_blink = counter;
        self
    }

    /// WgpuBackend doesn't have a builtin timer for blinking text and cursor.
    /// Instead, it relies on the application to call [WgpuBackend::blink]
    /// regularly (every 200ms is fine). Every call to blink increases an
    /// internal counter.
    ///
    /// The blink state is switched whenever `internal % counter` is 0.
    ///
    /// Default is 5.
    #[must_use]
    pub fn with_slow_blink(mut self, counter: u8) -> Self {
        self.slow_blink = counter;
        self
    }
}

impl WgpuBackendBuilder {
    /// Build a new backend with the provided surface target - e.g. a winit
    /// `Window`.
    pub fn build_with_target<'s>(
        self,
        target: impl Into<SurfaceTarget<'s>>,
    ) -> Result<WgpuBackend<'s>, Error> {
        futures_lite::future::block_on(self.build_with_target_async(target))
    }

    /// Build a new backend from this builder with the supplied surface. You
    /// almost certainly want to call this with the instance you used to create
    /// the provided surface - see [`WgpuBackendBuilder::with_instance`]. If one is not
    /// provided, a default instance will be created.
    pub fn build_with_surface<'s>(self, surface: Surface<'s>) -> Result<WgpuBackend<'s>, Error> {
        futures_lite::future::block_on(self.build_with_surface_async(surface))
    }

    /// Build a backend for headless operation.
    pub fn build_headless(self) -> Result<WgpuBackend<'static>, Error> {
        futures_lite::future::block_on(self.build_headless_async())
    }

    /// Build a backend for headless operation.
    pub async fn build_headless_with_format(
        self,
        format: TextureFormat,
    ) -> Result<WgpuBackend<'static>, Error> {
        futures_lite::future::block_on(self.build_headless_with_format_async(format))
    }

    /// Build a new backend with the provided surface target - e.g. a winit
    /// `Window`.
    pub async fn build_with_target_async<'s>(
        mut self,
        target: impl Into<SurfaceTarget<'s>>,
    ) -> Result<WgpuBackend<'s>, Error> {
        let backends = if self.backends == "*" || self.backends == "" {
            Backends::all()
        } else {
            Backends::from_comma_list(&self.backends)
        };

        let instance = self.instance.get_or_insert_with(|| {
            Instance::new(InstanceDescriptor {
                backends,
                flags: InstanceFlags::default(),
                memory_budget_thresholds: Default::default(),
                backend_options: Default::default(),
                display: None,
            })
        });

        let surface = instance
            .create_surface(target)
            .map_err(|e| Error::SurfaceCreationFailed(Box::new(e)))?;

        self.build_with_render_surface(RenderSurface::new_surface(surface))
            .await
    }

    /// Build a new backend from this builder with the supplied surface. You
    /// almost certainly want to call this with the instance you used to create
    /// the provided surface - see [`WgpuBackendBuilder::with_instance`]. If one is not
    /// provided, a default instance will be created.
    pub async fn build_with_surface_async<'s>(
        self,
        surface: Surface<'s>,
    ) -> Result<WgpuBackend<'s>, Error> {
        self.build_with_render_surface(RenderSurface::new_surface(surface))
            .await
    }

    /// Build a backend for headless operation.
    pub async fn build_headless_async(self) -> Result<WgpuBackend<'static>, Error> {
        self.build_with_render_surface(RenderSurface::new_headless())
            .await
    }

    /// Build a backend for headless operation.
    pub async fn build_headless_with_format_async(
        self,
        format: TextureFormat,
    ) -> Result<WgpuBackend<'static>, Error> {
        self.build_with_render_surface(RenderSurface::new_headless_with_format(format))
            .await
    }

    async fn build_with_render_surface<'s>(
        mut self,
        mut surface: RenderSurface<'s>,
    ) -> Result<WgpuBackend<'s>, Error> {
        let backends = if self.backends == "*" || self.backends == "" {
            Backends::all()
        } else {
            Backends::from_comma_list(&self.backends)
        };

        let instance = self.instance.get_or_insert_with(|| {
            Instance::new(InstanceDescriptor {
                backends,
                flags: InstanceFlags::default(),
                memory_budget_thresholds: Default::default(),
                backend_options: Default::default(),
                display: None,
            })
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                compatible_surface: surface.wgpu_surface(),
                ..Default::default()
            })
            .await
            .map_err(|e| Error::AdapterRequestFailed(Box::new(e)))?;

        let limits = if let Some(limits) = self.limits {
            limits
        } else {
            Limits::downlevel_defaults()
        };

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("ratatui-wgpu Device"),
                required_features: Default::default(),
                required_limits: limits,
                experimental_features: Default::default(),
                memory_hints: MemoryHints::MemoryUsage,
                trace: Default::default(),
            })
            .await
            .map_err(|e| Error::DeviceRequestFailed(Box::new(e)))?;

        let mut fonts = self.fonts;
        debug!("fonts from init {:#?}", fonts);

        if !fonts.have_fallback() {
            if let Some(f) = fallback_font() {
                fonts.add_fallback(f);
            }
            if let Some(f) = fallback_symbol_font() {
                fonts.add_fallback(f);
            }
            if let Some(f) = fallback_emoji_font() {
                fonts.add_fallback(f);
            }
        }

        // this may create a surface that is bigger than the window.
        let width = self.width.max(fonts.width_px());
        let height = self.height.max(fonts.height_px());

        let mut surface_config = surface
            .get_default_config(&adapter, width, height)
            .ok_or(Error::SurfaceConfigurationRequestFailed)?;

        if let Some(mode) = self.present_mode {
            surface_config.present_mode = mode;
        }

        surface.configure(&device, &surface_config);

        let drawable_width = surface_config.width;
        let drawable_height = surface_config.height;

        info!(
            "char width x height: {}x{}",
            fonts.width_px(),
            fonts.height_px()
        );

        let text_cache = device.create_texture(&TextureDescriptor {
            label: Some("Text Atlas"),
            size: Extent3d {
                width: CACHE_WIDTH,
                height: CACHE_HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let text_cache_view = text_cache.create_view(&TextureViewDescriptor::default());

        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let text_screen_size_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Text Uniforms Buffer"),
            size: size_of::<[f32; 4]>() as u64,
            mapped_at_creation: false,
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let atlas_size_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Atlas Size buffer"),
            contents: bytemuck::cast_slice(&[CACHE_WIDTH as f32, CACHE_HEIGHT as f32, 0.0, 0.0]),
            usage: BufferUsages::UNIFORM,
        });

        let text_bg_compositor = build_text_bg_compositor(
            &device, //
            &text_screen_size_buffer,
        );

        let text_fg_compositor = build_text_fg_compositor(
            &device,
            &text_screen_size_buffer,
            &atlas_size_buffer,
            &text_cache_view,
            &sampler,
        );

        let img_compositor = build_img_compositor(&device, &text_screen_size_buffer);

        let wgpu_view = build_wgpu_state(
            &device,
            (drawable_width / fonts.width_px()) * fonts.width_px(),
            (drawable_height / fonts.height_px()) * fonts.height_px(),
        );

        let reset_fg = self.colors.c2c(self.reset_fg, [255; 3]);
        let reset_bg = self.colors.c2c(self.reset_bg, [0; 3]);

        let cell_box = fonts.cell_box();
        let shape_plan_count = fonts.shape_plan_count();

        let chars_wide = width / cell_box.width;
        let chars_high = height / cell_box.height;

        let post_process = self
            .postprocessor
            .compile(&device, &wgpu_view, &surface_config);

        Ok(WgpuBackend {
            fonts,
            tui_surface: TuiSurface {
                image_buffer: Arc::new(Mutex::new(ImageBuffer {
                    area: ratatui_core::layout::Rect::new(
                        0,
                        0,
                        chars_wide as u16,
                        chars_high as u16,
                    ),
                    cell_box,
                    image_size: Default::default(),
                    images: Default::default(),
                })),
                images: Default::default(),
                cells: Default::default(),
                cell_font: Default::default(),
                cell_remap: Default::default(),
                dirty_rows: Default::default(),
                dirty_cells: Default::default(),
                dirty_img: Default::default(),
                fast_blinking: Default::default(),
                slow_blinking: Default::default(),
                cursor: (0, 0),
                colors: self.colors,
                reset_fg,
                reset_bg,
                cursor_color: self.cursor_color,
                cursor_style: self.cursor_style,
                cursor_visible: true,
                cursor_blink: 0,
                cursor_divisor: self.cursor_blink,
                cursor_showing: true,
                blink: 0,
                fast_blink_divisor: self.fast_blink,
                fast_blink_showing: true,
                slow_blink_divisor: self.slow_blink,
                slow_blink_showing: true,
            },
            cell_glyphs: Default::default(),

            tmp_plan_cache: PlanCache::new(shape_plan_count.max(2)),
            tmp_buffer: UnicodeBuffer::new(),
            tmp_rowbuf: String::new(),
            tmp_rowbuf_to_cell: Default::default(),

            wgpu_base: WgpuBase {
                surface,
                surface_config,
                device,
                queue,
                text_dest_view: wgpu_view,
            },
            wgpu_vertices: WgpuVertices {
                bg_vertices: Default::default(),
                text_indices: Default::default(),
                text_vertices: Default::default(),
                img_render: Default::default(),
                img_indices: Default::default(),
                img_vertices: Default::default(),
            },
            wgpu_atlas: WgpuAtlas {
                cached: Atlas::new(cell_box, CACHE_WIDTH, CACHE_HEIGHT),
                text_cache,
            },
            wgpu_images: WgpuImages {
                img_id: 1,
                handles: Default::default(),
                img: Default::default(),
            },
            wgpu_post_process_changed: false,
            wgpu_post_process: post_process,
            wgpu_pipeline: WgpuPipeline {
                sampler,
                text_screen_size_buffer,
                text_bg_compositor,
                text_fg_compositor,
                img_compositor,
            },
            #[cfg(feature = "timers")]
            draw_time: Default::default(),
            #[cfg(feature = "timers")]
            flush_time: Default::default(),
        })
    }
}

pub(super) fn build_wgpu_state(
    device: &Device,
    drawable_width: u32,
    drawable_height: u32,
) -> TextureView {
    let text_dest = device.create_texture(&TextureDescriptor {
        label: Some("Text Compositor Out"),
        size: Extent3d {
            width: drawable_width.max(1),
            height: drawable_height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8Unorm,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });

    let text_dest_view = text_dest.create_view(&TextureViewDescriptor::default());

    text_dest_view
}

pub(super) fn build_img_bindings(
    img_pipeline: &ImgPipeline,
    device: &Device,
    sampler: &Sampler,
    img_texture: &TextureView,
    uv_transform: &Buffer,
    uv_clip: &Buffer,
) -> BindGroup {
    device.create_bind_group(&BindGroupDescriptor {
        label: Some("Img Compositor Fragment Binding"),
        layout: &img_pipeline.fragment_shader_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::Sampler(sampler),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::TextureView(img_texture),
            },
            BindGroupEntry {
                binding: 2,
                resource: uv_transform.as_entire_binding(),
            },
            BindGroupEntry {
                binding: 3,
                resource: uv_clip.as_entire_binding(),
            },
        ],
    })
}

fn build_img_compositor(device: &Device, screen_size: &Buffer) -> ImgPipeline {
    let shader = device.create_shader_module(include_wgsl!("img.wgsl"));

    let vertex_shader_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("Image Compositor Uniforms Binding Layout"),
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: Some(NonZeroU64::new(size_of::<[f32; 4]>() as u64).unwrap()),
            },
            count: None,
        }],
    });

    let fs_uniforms = device.create_bind_group(&BindGroupDescriptor {
        label: Some("Image Compositor Uniforms Binding"),
        layout: &vertex_shader_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: screen_size.as_entire_binding(),
        }],
    });

    let fragment_shader_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("Img Compositor Fragment Binding Layout"),
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(
                        NonZeroU64::new(size_of::<[[f32; 4]; 2]>() as u64).unwrap(),
                    ),
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 3,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(NonZeroU64::new(size_of::<[f32; 4]>() as u64).unwrap()),
                },
                count: None,
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Img Compositor Layout"),
        bind_group_layouts: &[Some(&vertex_shader_layout), Some(&fragment_shader_layout)],
        immediate_size: 0,
    });

    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Img Compositor Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: PipelineCompilationOptions::default(),
            buffers: &[Some(VertexBufferLayout {
                array_stride: size_of::<ImgVertexMember>() as u64,
                step_mode: VertexStepMode::Vertex,
                attributes: &vertex_attr_array![0 => Float32x2, 1 => Float32x2],
            })],
        },
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: MultisampleState::default(),
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: PipelineCompilationOptions::default(),
            targets: &[Some(ColorTargetState {
                format: TextureFormat::Rgba8Unorm,
                blend: Some(BlendState::ALPHA_BLENDING),
                write_mask: ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });

    ImgPipeline {
        fragment_shader_layout,
        pipeline,
        fs_uniforms,
    }
}

fn build_text_bg_compositor(device: &Device, screen_size: &Buffer) -> TextCacheBgPipeline {
    let shader = device.create_shader_module(include_wgsl!("composite_bg.wgsl"));

    let vertex_shader_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("Text Bg Compositor Uniforms Binding Layout"),
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: Some(NonZeroU64::new(size_of::<[f32; 4]>() as u64).unwrap()),
            },
            count: None,
        }],
    });

    let fs_uniforms = device.create_bind_group(&BindGroupDescriptor {
        label: Some("Text Bg Compositor Uniforms Binding"),
        layout: &vertex_shader_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: screen_size.as_entire_binding(),
        }],
    });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Text Bg Compositor Layout"),
        bind_group_layouts: &[Some(&vertex_shader_layout)],
        immediate_size: 0,
    });

    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Text Bg Compositor Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: PipelineCompilationOptions::default(),
            buffers: &[Some(VertexBufferLayout {
                array_stride: size_of::<TextBgVertexMember>() as u64,
                step_mode: VertexStepMode::Vertex,
                attributes: &vertex_attr_array![0 => Float32x2, 1 => Uint32],
            })],
        },
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: MultisampleState::default(),
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: PipelineCompilationOptions::default(),
            targets: &[Some(ColorTargetState {
                format: TextureFormat::Rgba8Unorm,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });

    TextCacheBgPipeline {
        pipeline,
        fs_uniforms,
    }
}

fn build_text_fg_compositor(
    device: &Device,
    screen_size: &Buffer,
    atlas_size: &Buffer,
    cache_view: &TextureView,
    sampler: &Sampler,
) -> TextCacheFgPipeline {
    let shader = device.create_shader_module(include_wgsl!("composite_fg.wgsl"));

    let vertex_shader_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("Text Compositor Uniforms Binding Layout"),
        entries: &[BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: Some(NonZeroU64::new(size_of::<[f32; 4]>() as u64).unwrap()),
            },
            count: None,
        }],
    });

    let fragment_shader_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
        label: Some("Text Compositor Fragment Binding Layout"),
        entries: &[
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture {
                    sample_type: TextureSampleType::Float { filterable: true },
                    view_dimension: TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 1,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Sampler(SamplerBindingType::Filtering),
                count: None,
            },
            BindGroupLayoutEntry {
                binding: 2,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(NonZeroU64::new(size_of::<[f32; 4]>() as u64).unwrap()),
                },
                count: None,
            },
        ],
    });

    let fs_uniforms = device.create_bind_group(&BindGroupDescriptor {
        label: Some("Text Compositor Uniforms Binding"),
        layout: &vertex_shader_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: screen_size.as_entire_binding(),
        }],
    });

    let atlas_bindings = device.create_bind_group(&BindGroupDescriptor {
        label: Some("Text Compositor Fragment Binding"),
        layout: &fragment_shader_layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(cache_view),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Sampler(sampler),
            },
            BindGroupEntry {
                binding: 2,
                resource: atlas_size.as_entire_binding(),
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
        label: Some("Text Compositor Layout"),
        bind_group_layouts: &[Some(&vertex_shader_layout), Some(&fragment_shader_layout)],
        immediate_size: 0,
    });

    let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
        label: Some("Text Compositor Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: PipelineCompilationOptions::default(),
            buffers: &[Some(VertexBufferLayout {
                array_stride: size_of::<TextVertexMember>() as u64,
                step_mode: VertexStepMode::Vertex,
                attributes: &vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32, 3 => Uint32, 4 => Uint32, 5 => Uint32, 6 => Uint32, 7 => Uint32, 8 => Uint32 ],
            })],
        },
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: MultisampleState::default(),
        fragment: Some(FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: PipelineCompilationOptions::default(),
            targets: &[Some(ColorTargetState {
                format: TextureFormat::Rgba8Unorm,
                blend: Some(BlendState::ALPHA_BLENDING),
                write_mask: ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });

    TextCacheFgPipeline {
        pipeline,
        fs_uniforms,
        atlas_bindings,
    }
}
