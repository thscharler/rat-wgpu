use log::error;
use wgpu::{Adapter, Device, Extent3d, Surface, SurfaceConfiguration, SurfaceTexture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor};

pub(crate) enum RenderTarget {
    Surface {
        texture: SurfaceTexture,
        view: TextureView,
    },
    Headless {
        view: TextureView,
    },
}

pub(crate) enum RenderSurface<'s> {
    Surface(Surface<'s>),
    Headless(Headless),
}

pub(crate) struct Headless {
    pub(crate) texture: Option<wgpu::Texture>,
    pub(crate) buffer: Option<wgpu::Buffer>,
    pub(crate) buffer_width: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) format: TextureFormat,
}

impl RenderTarget {
    pub(crate) fn get_view(&self) -> &TextureView {
        match self {
            RenderTarget::Surface { view, .. } => view,
            RenderTarget::Headless { view } => view,
        }
    }

    pub(crate) fn present(self) {
        match self {
            RenderTarget::Surface { texture, .. } => texture.present(),
            RenderTarget::Headless { .. } => {
                // noop
            }
        }
    }
}

impl<'s> RenderSurface<'s> {
    pub(crate) fn new_surface(surface: Surface<'s>) -> Self {
        Self::Surface(surface)
    }

    pub(crate) fn new_headless() -> Self {
        Self::Headless(Headless {
            texture: Default::default(),
            buffer: Default::default(),
            buffer_width: Default::default(),
            width: Default::default(),
            height: Default::default(),
            format: TextureFormat::Rgba8Unorm,
        })
    }

    pub(crate) fn new_headless_with_format(format: TextureFormat) -> Self {
        Self::Headless(Headless {
            texture: Default::default(),
            buffer: Default::default(),
            buffer_width: Default::default(),
            width: Default::default(),
            height: Default::default(),
            format,
        })
    }

    pub(crate) fn wgpu_surface(&self) -> Option<&Surface<'s>> {
        match self {
            RenderSurface::Surface(surface) => Some(surface),
            RenderSurface::Headless(_) => None,
        }
    } 

    pub(crate) fn get_default_config(
        &self,
        adapter: &Adapter,
        width: u32,
        height: u32,
    ) -> Option<SurfaceConfiguration> {
        match self {
            RenderSurface::Surface(surface) => surface.get_default_config(adapter, width, height),
            RenderSurface::Headless(Headless { format, .. }) => Some(SurfaceConfiguration {
                usage: TextureUsages::RENDER_ATTACHMENT,
                format: *format,
                width,
                height,
                present_mode: wgpu::PresentMode::Immediate,
                desired_maximum_frame_latency: 2,
                alpha_mode: wgpu::CompositeAlphaMode::Auto,
                view_formats: vec![],
            }),
        }
    }

    pub(crate) fn configure(&mut self, device: &Device, config: &SurfaceConfiguration) {
        match self {
            RenderSurface::Surface(surface) => {
                Surface::configure(surface, device, config);
            }
            RenderSurface::Headless(Headless {
                texture,
                buffer,
                buffer_width,
                width,
                height,
                format,
            }) => {
                *texture = Some(device.create_texture(&TextureDescriptor {
                    label: None,
                    size: Extent3d {
                        width: config.width,
                        height: config.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: *format,
                    usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
                    view_formats: &[],
                }));

                *buffer_width = config.width * 4;
                *buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                    label: None,
                    size: (*buffer_width * config.height) as u64,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                }));
                *width = config.width;
                *height = config.height;
            }
        }
    }

    pub(crate) fn get_current_texture(&self) -> Option<RenderTarget> {
        match self {
            RenderSurface::Surface(surface) => {
                let output = match surface.get_current_texture() {
                    Ok(output) => output,
                    Err(err) => {
                        error!("{err}");
                        return None;
                    }
                };

                let view = output
                    .texture
                    .create_view(&TextureViewDescriptor::default());

                Some(RenderTarget::Surface {
                    texture: output,
                    view,
                })
            }
            RenderSurface::Headless(Headless { texture, .. }) => {
                texture.as_ref().map(|t| RenderTarget::Headless {
                    view: t.create_view(&TextureViewDescriptor::default()),
                })
            }
        }
    }
}
