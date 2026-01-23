use crate::CellBox;
use crate::postprocessor::{PostProcessor, PostProcessorBuilder};
use std::num::NonZeroU64;
use wgpu::{
    AddressMode, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingResource, BindingType, Buffer, BufferBindingType,
    BufferDescriptor, BufferUsages, Color, ColorTargetState, ColorWrites, CommandEncoder, Device,
    FilterMode, FragmentState, LoadOp, MipmapFilterMode, MultisampleState, Operations,
    PipelineCompilationOptions, PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, Queue,
    RenderBundle, RenderBundleDescriptor, RenderBundleEncoderDescriptor, RenderPassColorAttachment,
    RenderPassDescriptor, RenderPipeline, RenderPipelineDescriptor, Sampler, SamplerBindingType,
    SamplerDescriptor, ShaderStages, StoreOp, SurfaceConfiguration, TextureSampleType, TextureView,
    TextureViewDimension, VertexState, include_wgsl,
};

#[derive(Default)]
pub struct DefaultPostProcessorBuilder;

/// The default post-processor. Used when you don't want to perform any custom
/// shading on the output. This just blits the composited text to the surface.
/// This will stretch characters if the render area size falls between multiples
/// of the character size. Use `AspectPreservingDefaultPostProcessor` if you
/// don't want this behavior.
pub struct DefaultPostProcessor {
    size: (u32, u32),
    uniforms: Buffer,
    bindings: BindGroupLayout,
    sampler: Sampler,
    pipeline: RenderPipeline,

    blitter: RenderBundle,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    screen_size: [f32; 2],
    margin_color: u32,
    preserve_aspect: u32,
    use_srgb: u32,
    _fill: u32,
}

impl PostProcessorBuilder for DefaultPostProcessorBuilder {
    type PostProcessor<'a> = DefaultPostProcessor;

    fn compile(
        self,
        device: &Device,
        text_view: &TextureView,
        surface_config: &SurfaceConfiguration,
    ) -> DefaultPostProcessor {
        let uniforms = device.create_buffer(&BufferDescriptor {
            label: Some("Text Blit Uniforms"),
            size: size_of::<Uniforms>() as u64,
            usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });

        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            mipmap_filter: MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Text Blit Bindings Layout"),
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
                        min_binding_size: NonZeroU64::new(size_of::<Uniforms>() as u64),
                    },
                    count: None,
                },
            ],
        });

        let shader = device.create_shader_module(include_wgsl!("blit.wgsl"));

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Text Blit Layout"),
            bind_group_layouts: &[&layout],
            immediate_size: 0,
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Text Blitter Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            fragment: Some(FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: PipelineCompilationOptions::default(),
                targets: &[Some(ColorTargetState {
                    format: surface_config.format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let size = (surface_config.width, surface_config.height);

        let blitter = build_blitter(
            device,
            &layout,
            text_view,
            &sampler,
            &uniforms,
            surface_config,
            &pipeline,
        );

        DefaultPostProcessor {
            size,
            uniforms,
            bindings: layout,
            sampler,
            pipeline,
            blitter,
        }
    }
}

fn build_blitter(
    device: &Device,
    layout: &BindGroupLayout,
    text_view: &TextureView,
    sampler: &Sampler,
    uniforms: &Buffer,
    surface_config: &SurfaceConfiguration,
    pipeline: &RenderPipeline,
) -> RenderBundle {
    let bindings = device.create_bind_group(&BindGroupDescriptor {
        label: Some("Text Blit Bindings"),
        layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::TextureView(text_view),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::Sampler(sampler),
            },
            BindGroupEntry {
                binding: 2,
                resource: uniforms.as_entire_binding(),
            },
        ],
    });

    let mut encoder = device.create_render_bundle_encoder(&RenderBundleEncoderDescriptor {
        label: Some("Text Blit Pass Encoder"),
        color_formats: &[Some(surface_config.format)],
        depth_stencil: None,
        sample_count: 1,
        multiview: None,
    });

    encoder.set_pipeline(pipeline);

    encoder.set_bind_group(0, &bindings, &[]);
    encoder.draw(0..3, 0..1);

    encoder.finish(&RenderBundleDescriptor {
        label: Some("Text Blit Pass Bundle"),
    })
}

impl PostProcessor for DefaultPostProcessor {
    fn map_to_cell(&self, scr_x: i32, scr_y: i32, font_box: CellBox) -> (u16, u16) {
        if scr_x < 0 || scr_y < 0 {
            (0, 0)
        } else {
            (
                (scr_x as u32 / font_box.width) as u16,
                (scr_y as u32 / font_box.height) as u16,
            )
        }
    }

    fn resize(
        &mut self,
        device: &Device,
        text_view: &TextureView,
        surface_config: &SurfaceConfiguration,
    ) {
        self.size = (surface_config.width, surface_config.height);
        self.blitter = build_blitter(
            device,
            &self.bindings,
            text_view,
            &self.sampler,
            &self.uniforms,
            surface_config,
            &self.pipeline,
        );
    }

    fn process(
        &mut self,
        margin_color: u32,
        encoder: &mut CommandEncoder,
        queue: &Queue,
        _text_view: &TextureView,
        surface_config: &SurfaceConfiguration,
        surface_view: &TextureView,
    ) {
        {
            #[cfg(feature = "scale_to_window")]
            let preserve_aspect = false;
            #[cfg(not(feature = "scale_to_window"))]
            let preserve_aspect = true;

            let mut uniforms = queue
                .write_buffer_with(
                    &self.uniforms,
                    0,
                    NonZeroU64::new(size_of::<Uniforms>() as u64).unwrap(),
                )
                .unwrap();
            uniforms.copy_from_slice(bytemuck::bytes_of(&Uniforms {
                screen_size: [surface_config.width as f32, surface_config.height as f32],
                margin_color,
                preserve_aspect: u32::from(preserve_aspect),
                use_srgb: u32::from(surface_config.format.is_srgb()),
                _fill: 0,
            }));
        }

        let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("Text Blit Pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: surface_view,
                resolve_target: None,
                ops: Operations {
                    load: LoadOp::Clear(Color::TRANSPARENT),
                    store: StoreOp::Store,
                },
                depth_slice: None,
            })],
            ..Default::default()
        });

        pass.execute_bundles(Some(&self.blitter));
    }
}
