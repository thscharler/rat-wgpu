use std::num::NonZeroU32;

use image::ImageBuffer;
use image::Rgba;
use image::load_from_memory;
use image::{ExtendedColorType, GenericImageView};
use rat_wgpu::Builder;
use rat_wgpu::font::{Font, Fonts};
use rat_wgpu::postprocessor::default::DefaultPostProcessorBuilder;
use ratatui_core::style::Color;
use ratatui_core::style::Stylize;
use ratatui_core::terminal::Terminal;
use ratatui_core::text::Line;
use ratatui_widgets::block::Block;
use ratatui_widgets::paragraph::Paragraph;
use rustybuzz::ttf_parser::RasterGlyphImage;
use rustybuzz::ttf_parser::RasterImageFormat;
use serial_test::serial;
use wgpu::CommandEncoderDescriptor;
use wgpu::Device;
use wgpu::Extent3d;
use wgpu::Queue;
use wgpu::TextureFormat;
use wgpu::wgt::PollType;

#[test]
#[serial]
fn a_z() {
    let mut terminal = Terminal::new(
        futures_lite::future::block_on(
            Builder::<DefaultPostProcessorBuilder>::default()
                .with_fallback_fonts(Fonts::new(
                    Font::new(include_bytes!("fonts/CascadiaMono-Regular.ttf"))
                        .expect("Invalid font file"),
                    22,
                ))
                .with_width_and_height(512, 72)
                .with_bg_color(Color::White)
                .with_fg_color(Color::Black)
                .build_headless(),
        )
        .unwrap(),
    )
    .unwrap();

    terminal
        .draw(|f: &mut ratatui_core::terminal::Frame| {
            let block = Block::bordered();
            let area = block.inner(f.area());
            f.render_widget(block, f.area());
            f.render_widget(Paragraph::new("ABCDEFGHIJKLMNOPQRSTUVWXYZ"), area);
        })
        .unwrap();

    let buffer = terminal
        .backend()
        .map_headless_buffer()
        .expect("headless buffer");

    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(512, 72, &*buffer).unwrap();

    image::save_buffer(
        "az.png",
        image.as_flat_samples().samples,
        512,
        72,
        ExtendedColorType::Rgba8,
    )
    .expect("save_buffer");

    let pixels = image.pixels().copied().collect::<Vec<_>>();
    let golden = load_from_memory(include_bytes!("goldens/a_z.png")).unwrap();
    let golden_pixels = golden.pixels().map(|(_, _, px)| px).collect::<Vec<_>>();

    assert!(
        pixels == golden_pixels,
        "Rendered image differs from golden"
    );

    terminal.backend().unmap_headless_buffer();
}
//
// #[test]
// #[serial]
// fn arabic() {
//     let mut terminal = Terminal::new(
//         futures_lite::future::block_on(
//             Builder::<DefaultPostProcessorBuilder>::from_font(
//                 Font::new(include_bytes!("fonts/CascadiaMono-Regular.ttf"))
//                     .expect("Invalid font file"),
//             )
//             .with_dimensions(Dimensions {
//                 width: NonZeroU32::new(256).unwrap(),
//                 height: NonZeroU32::new(72).unwrap(),
//             })
//             .build_headless(),
//         )
//         .unwrap(),
//     )
//     .unwrap();
//
//     terminal
//         .draw(|f: &mut ratatui_core::terminal::Frame| {
//             let block = Block::bordered();
//             let area = block.inner(f.area());
//             f.render_widget(block, f.area());
//             f.render_widget(Paragraph::new("مرحبا بالعالم"), area);
//         })
//         .unwrap();
//
//     let surface = &terminal.backend().wgpu_base.surface;
//     tex2buffer(
//         &terminal.backend().wgpu_base.device,
//         &terminal.backend().wgpu_base.queue,
//         surface,
//     );
//     let surface = surface.headless().expect("headless");
//     {
//         let buffer = surface.buffer.as_ref().unwrap().slice(..);
//
//         let (send, recv) = oneshot::channel();
//         buffer.map_async(wgpu::MapMode::Read, move |data| {
//             send.send(data).unwrap();
//         });
//         terminal
//             .backend()
//             .wgpu_base
//             .device
//             .poll(PollType::Wait {
//                 submission_index: None,
//                 timeout: None,
//             })
//             .unwrap();
//         recv.recv().unwrap().unwrap();
//
//         let data = buffer.get_mapped_range();
//         let image =
//             ImageBuffer::<Rgba<u8>, _>::from_raw(surface.width, surface.height, data).unwrap();
//
//         let pixels = image.pixels().copied().collect::<Vec<_>>();
//         let golden = load_from_memory(include_bytes!("goldens/arabic.png")).unwrap();
//         let golden_pixels = golden.pixels().map(|(_, _, px)| px).collect::<Vec<_>>();
//
//         assert!(
//             pixels == golden_pixels,
//             "Rendered image differs from golden"
//         );
//     }
//
//     surface.buffer.as_ref().unwrap().unmap();
// }
//
// #[test]
// #[serial]
// fn really_wide() {
//     let mut terminal = Terminal::new(
//         futures_lite::future::block_on(
//             Builder::<DefaultPostProcessorBuilder>::from_font(
//                 Font::new(include_bytes!("fonts/Fairfax.ttf")).expect("Invalid font file"),
//             )
//             .with_dimensions(Dimensions {
//                 width: NonZeroU32::new(512).unwrap(),
//                 height: NonZeroU32::new(72).unwrap(),
//             })
//             .build_headless(),
//         )
//         .unwrap(),
//     )
//     .unwrap();
//
//     terminal
//         .draw(|f: &mut ratatui_core::terminal::Frame| {
//             let block = Block::bordered();
//             let area = block.inner(f.area());
//             f.render_widget(block, f.area());
//             f.render_widget(Paragraph::new("Ｈｅｌｌｏ, ｗｏｒｌｄ!"), area);
//         })
//         .unwrap();
//
//     let surface = &terminal.backend().wgpu_base.surface;
//     tex2buffer(
//         &terminal.backend().wgpu_base.device,
//         &terminal.backend().wgpu_base.queue,
//         surface,
//     );
//     let surface = surface.headless().expect("headless");
//     {
//         let buffer = surface.buffer.as_ref().unwrap().slice(..);
//
//         let (send, recv) = oneshot::channel();
//         buffer.map_async(wgpu::MapMode::Read, move |data| {
//             send.send(data).unwrap();
//         });
//         terminal
//             .backend()
//             .wgpu_base
//             .device
//             .poll(PollType::Wait {
//                 submission_index: None,
//                 timeout: None,
//             })
//             .unwrap();
//         recv.recv().unwrap().unwrap();
//
//         let data = buffer.get_mapped_range();
//         let image =
//             ImageBuffer::<Rgba<u8>, _>::from_raw(surface.width, surface.height, data).unwrap();
//
//         let pixels = image.pixels().copied().collect::<Vec<_>>();
//         let golden = load_from_memory(include_bytes!("goldens/really_wide.png")).unwrap();
//         let golden_pixels = golden.pixels().map(|(_, _, px)| px).collect::<Vec<_>>();
//
//         assert!(
//             pixels == golden_pixels,
//             "Rendered image differs from golden"
//         );
//     }
//
//     surface.buffer.as_ref().unwrap().unmap();
// }
//
// #[test]
// #[serial]
// fn mixed() {
//     let mut terminal = Terminal::new(
//         futures_lite::future::block_on(
//             Builder::<DefaultPostProcessorBuilder>::from_font(
//                 Font::new(include_bytes!("fonts/CascadiaMono-Regular.ttf"))
//                     .expect("Invalid font file"),
//             )
//             .with_dimensions(Dimensions {
//                 width: NonZeroU32::new(512).unwrap(),
//                 height: NonZeroU32::new(72).unwrap(),
//             })
//             .build_headless(),
//         )
//         .unwrap(),
//     )
//     .unwrap();
//
//     terminal
//         .draw(|f: &mut ratatui_core::terminal::Frame| {
//             let block = Block::bordered();
//             let area = block.inner(f.area());
//             f.render_widget(block, f.area());
//             f.render_widget(
//                 Paragraph::new("Hello World! مرحبا بالعالم 0123456789000000000"),
//                 area,
//             );
//         })
//         .unwrap();
//
//     let surface = &terminal.backend().wgpu_base.surface;
//     tex2buffer(
//         &terminal.backend().wgpu_base.device,
//         &terminal.backend().wgpu_base.queue,
//         surface,
//     );
//     let surface = surface.headless().expect("headless");
//     {
//         let buffer = surface.buffer.as_ref().unwrap().slice(..);
//
//         let (send, recv) = oneshot::channel();
//         buffer.map_async(wgpu::MapMode::Read, move |data| {
//             send.send(data).unwrap();
//         });
//         terminal
//             .backend()
//             .wgpu_base
//             .device
//             .poll(PollType::Wait {
//                 submission_index: None,
//                 timeout: None,
//             })
//             .unwrap();
//         recv.recv().unwrap().unwrap();
//
//         let data = buffer.get_mapped_range();
//         let image =
//             ImageBuffer::<Rgba<u8>, _>::from_raw(surface.width, surface.height, data).unwrap();
//
//         let pixels = image.pixels().copied().collect::<Vec<_>>();
//         let golden = load_from_memory(include_bytes!("goldens/mixed.png")).unwrap();
//         let golden_pixels = golden.pixels().map(|(_, _, px)| px).collect::<Vec<_>>();
//
//         assert!(
//             pixels == golden_pixels,
//             "Rendered image differs from golden"
//         );
//     }
//
//     surface.buffer.as_ref().unwrap().unmap();
// }
//
// #[test]
// #[serial]
// fn mixed_colors() {
//     let mut terminal = Terminal::new(
//         futures_lite::future::block_on(
//             Builder::<DefaultPostProcessorBuilder>::from_font(
//                 Font::new(include_bytes!("fonts/CascadiaMono-Regular.ttf"))
//                     .expect("Invalid font file"),
//             )
//             .with_dimensions(Dimensions {
//                 width: NonZeroU32::new(512).unwrap(),
//                 height: NonZeroU32::new(72).unwrap(),
//             })
//             .build_headless(),
//         )
//         .unwrap(),
//     )
//     .unwrap();
//
//     terminal
//         .draw(|f: &mut ratatui_core::terminal::Frame| {
//             let block = Block::bordered();
//             let area = block.inner(f.area());
//             f.render_widget(block, f.area());
//             f.render_widget(
//                 Paragraph::new(Line::from(vec![
//                     "Hello World!".green(),
//                     "مرحبا بالعالم".blue(),
//                     "0123456789".dim(),
//                 ])),
//                 area,
//             );
//         })
//         .unwrap();
//
//     let surface = &terminal.backend().wgpu_base.surface;
//     tex2buffer(
//         &terminal.backend().wgpu_base.device,
//         &terminal.backend().wgpu_base.queue,
//         surface,
//     );
//     let surface = surface.headless().expect("headless");
//     {
//         let buffer = surface.buffer.as_ref().unwrap().slice(..);
//
//         let (send, recv) = oneshot::channel();
//         buffer.map_async(wgpu::MapMode::Read, move |data| {
//             send.send(data).unwrap();
//         });
//         terminal
//             .backend()
//             .wgpu_base
//             .device
//             .poll(PollType::Wait {
//                 submission_index: None,
//                 timeout: None,
//             })
//             .unwrap();
//         recv.recv().unwrap().unwrap();
//
//         let data = buffer.get_mapped_range();
//         let image =
//             ImageBuffer::<Rgba<u8>, _>::from_raw(surface.width, surface.height, data).unwrap();
//
//         let pixels = image.pixels().copied().collect::<Vec<_>>();
//         let golden = load_from_memory(include_bytes!("goldens/mixed_colors.png")).unwrap();
//         let golden_pixels = golden.pixels().map(|(_, _, px)| px).collect::<Vec<_>>();
//
//         assert!(
//             pixels == golden_pixels,
//             "Rendered image differs from golden"
//         );
//     }
//
//     surface.buffer.as_ref().unwrap().unmap();
// }
//
// #[test]
// #[serial]
// fn overlap() {
//     let mut terminal = Terminal::new(
//         futures_lite::future::block_on(
//             Builder::<DefaultPostProcessorBuilder>::from_font(
//                 Font::new(include_bytes!("fonts/Fairfax.ttf")).expect("Invalid font file"),
//             )
//             .with_dimensions(Dimensions {
//                 width: NonZeroU32::new(256).unwrap(),
//                 height: NonZeroU32::new(72).unwrap(),
//             })
//             .build_headless(),
//         )
//         .unwrap(),
//     )
//     .unwrap();
//
//     terminal
//         .draw(|f: &mut ratatui_core::terminal::Frame| {
//             let block = Block::bordered();
//             let area = block.inner(f.area());
//             f.render_widget(block, f.area());
//             f.render_widget(Paragraph::new("H̴̢͕̠͖͇̻͓̙̞͔͕͓̰͋͛͂̃̌͂͆͜͠".underlined()), area);
//         })
//         .unwrap();
//
//     let surface = &terminal.backend().wgpu_base.surface;
//     tex2buffer(
//         &terminal.backend().wgpu_base.device,
//         &terminal.backend().wgpu_base.queue,
//         surface,
//     );
//     let surface = surface.headless().expect("headless");
//     {
//         let buffer = surface.buffer.as_ref().unwrap().slice(..);
//
//         let (send, recv) = oneshot::channel();
//         buffer.map_async(wgpu::MapMode::Read, move |data| {
//             send.send(data).unwrap();
//         });
//         terminal
//             .backend()
//             .wgpu_base
//             .device
//             .poll(PollType::Wait {
//                 submission_index: None,
//                 timeout: None,
//             })
//             .unwrap();
//         recv.recv().unwrap().unwrap();
//
//         let data = buffer.get_mapped_range();
//         let image =
//             ImageBuffer::<Rgba<u8>, _>::from_raw(surface.width, surface.height, data).unwrap();
//
//         let pixels = image.pixels().copied().collect::<Vec<_>>();
//         let golden = load_from_memory(include_bytes!("goldens/overlap_initial.png")).unwrap();
//         let golden_pixels = golden.pixels().map(|(_, _, px)| px).collect::<Vec<_>>();
//
//         assert!(
//             pixels == golden_pixels,
//             "Rendered image differs from golden"
//         );
//     }
//     surface.buffer.as_ref().unwrap().unmap();
//
//     terminal
//         .draw(|f: &mut ratatui_core::terminal::Frame| {
//             let block = Block::bordered();
//             let area = block.inner(f.area());
//             f.render_widget(block, f.area());
//             f.render_widget(Paragraph::new("H".underlined()), area);
//         })
//         .unwrap();
//
//     let surface = &terminal.backend().wgpu_base.surface;
//     tex2buffer(
//         &terminal.backend().wgpu_base.device,
//         &terminal.backend().wgpu_base.queue,
//         surface,
//     );
//     let surface = surface.headless().expect("headless");
//     {
//         let buffer = surface.buffer.as_ref().unwrap().slice(..);
//
//         let (send, recv) = oneshot::channel();
//         buffer.map_async(wgpu::MapMode::Read, move |data| {
//             send.send(data).unwrap();
//         });
//         terminal
//             .backend()
//             .wgpu_base
//             .device
//             .poll(PollType::Wait {
//                 submission_index: None,
//                 timeout: None,
//             })
//             .unwrap();
//         recv.recv().unwrap().unwrap();
//
//         let data = buffer.get_mapped_range();
//         let image =
//             ImageBuffer::<Rgba<u8>, _>::from_raw(surface.width, surface.height, data).unwrap();
//
//         let pixels = image.pixels().copied().collect::<Vec<_>>();
//         let golden = load_from_memory(include_bytes!("goldens/overlap_post.png")).unwrap();
//         let golden_pixels = golden.pixels().map(|(_, _, px)| px).collect::<Vec<_>>();
//
//         assert!(
//             pixels == golden_pixels,
//             "Rendered image differs from golden"
//         );
//     }
//
//     surface.buffer.as_ref().unwrap().unmap();
// }
//
// #[test]
// #[serial]
// fn overlap_colors() {
//     let mut terminal = Terminal::new(
//         futures_lite::future::block_on(
//             Builder::<DefaultPostProcessorBuilder>::from_font(
//                 Font::new(include_bytes!("fonts/Fairfax.ttf")).expect("Invalid font file"),
//             )
//             .with_dimensions(Dimensions {
//                 width: NonZeroU32::new(256).unwrap(),
//                 height: NonZeroU32::new(72).unwrap(),
//             })
//             .build_headless(),
//         )
//         .unwrap(),
//     )
//     .unwrap();
//
//     terminal
//         .draw(|f: &mut ratatui_core::terminal::Frame| {
//             let block = Block::bordered();
//             let area = block.inner(f.area());
//             f.render_widget(block, f.area());
//             f.render_widget(Paragraph::new("H̴̢͕̠͖͇̻͓̙̞͔͕͓̰͋͛͂̃̌͂͆͜͠".blue().on_red().underlined()), area);
//         })
//         .unwrap();
//
//     let surface = &terminal.backend().wgpu_base.surface;
//     tex2buffer(
//         &terminal.backend().wgpu_base.device,
//         &terminal.backend().wgpu_base.queue,
//         surface,
//     );
//     let surface = surface.headless().expect("headless");
//     {
//         let buffer = surface.buffer.as_ref().unwrap().slice(..);
//
//         let (send, recv) = oneshot::channel();
//         buffer.map_async(wgpu::MapMode::Read, move |data| {
//             send.send(data).unwrap();
//         });
//         terminal
//             .backend()
//             .wgpu_base
//             .device
//             .poll(PollType::Wait {
//                 submission_index: None,
//                 timeout: None,
//             })
//             .unwrap();
//         recv.recv().unwrap().unwrap();
//
//         let data = buffer.get_mapped_range();
//         let image =
//             ImageBuffer::<Rgba<u8>, _>::from_raw(surface.width, surface.height, data).unwrap();
//
//         let pixels = image.pixels().copied().collect::<Vec<_>>();
//         let golden = load_from_memory(include_bytes!("goldens/overlap_colors.png")).unwrap();
//         let golden_pixels = golden.pixels().map(|(_, _, px)| px).collect::<Vec<_>>();
//
//         assert!(
//             pixels == golden_pixels,
//             "Rendered image differs from golden"
//         );
//     }
//     surface.buffer.as_ref().unwrap().unmap();
// }
//
// #[test]
// #[serial]
// fn rgb_conversion() {
//     let mut terminal = Terminal::new(
//         futures_lite::future::block_on(
//             Builder::<DefaultPostProcessorBuilder>::from_font(
//                 Font::new(include_bytes!("fonts/Fairfax.ttf")).expect("Invalid font file"),
//             )
//             .with_dimensions(Dimensions {
//                 width: NonZeroU32::new(256).unwrap(),
//                 height: NonZeroU32::new(72).unwrap(),
//             })
//             .with_bg_color(Color::Rgb(0x1E, 0x23, 0x26))
//             .with_fg_color(Color::White)
//             .build_headless_with_format(TextureFormat::Rgba8Unorm),
//         )
//         .unwrap(),
//     )
//     .unwrap();
//
//     terminal
//         .draw(|f: &mut ratatui_core::terminal::Frame| {
//             let block = Block::bordered();
//             let area = block.inner(f.area());
//             f.render_widget(block, f.area());
//             f.render_widget(Paragraph::new("TEST"), area);
//         })
//         .unwrap();
//
//     let surface = &terminal.backend().wgpu_base.surface;
//     tex2buffer(
//         &terminal.backend().wgpu_base.device,
//         &terminal.backend().wgpu_base.queue,
//         surface,
//     );
//     let surface = surface.headless().expect("headless");
//     {
//         let buffer = surface.buffer.as_ref().unwrap().slice(..);
//
//         let (send, recv) = oneshot::channel();
//         buffer.map_async(wgpu::MapMode::Read, move |data| {
//             send.send(data).unwrap();
//         });
//         terminal
//             .backend()
//             .wgpu_base
//             .device
//             .poll(PollType::Wait {
//                 submission_index: None,
//                 timeout: None,
//             })
//             .unwrap();
//         recv.recv().unwrap().unwrap();
//
//         let data = buffer.get_mapped_range();
//         let image =
//             ImageBuffer::<Rgba<u8>, _>::from_raw(surface.width, surface.height, data).unwrap();
//
//         let pixels = image.pixels().copied().collect::<Vec<_>>();
//         let golden = load_from_memory(include_bytes!("goldens/rgb_conversion.png")).unwrap();
//         let golden_pixels = golden.pixels().map(|(_, _, px)| px).collect::<Vec<_>>();
//
//         assert!(
//             pixels == golden_pixels,
//             "Rendered image differs from golden"
//         );
//     }
//     surface.buffer.as_ref().unwrap().unmap();
// }
//
// #[test]
// #[serial]
// fn srgb_conversion() {
//     let mut terminal = Terminal::new(
//         futures_lite::future::block_on(
//             Builder::<DefaultPostProcessorBuilder>::from_font(
//                 Font::new(include_bytes!("fonts/Fairfax.ttf")).expect("Invalid font file"),
//             )
//             .with_dimensions(Dimensions {
//                 width: NonZeroU32::new(256).unwrap(),
//                 height: NonZeroU32::new(72).unwrap(),
//             })
//             .with_bg_color(Color::Rgb(0x1E, 0x23, 0x26))
//             .with_fg_color(Color::White)
//             .build_headless_with_format(TextureFormat::Rgba8UnormSrgb),
//         )
//         .unwrap(),
//     )
//     .unwrap();
//
//     terminal
//         .draw(|f: &mut ratatui_core::terminal::Frame| {
//             let block = Block::bordered();
//             let area = block.inner(f.area());
//             f.render_widget(block, f.area());
//             f.render_widget(Paragraph::new("TEST"), area);
//         })
//         .unwrap();
//
//     let surface = &terminal.backend().wgpu_base.surface;
//     tex2buffer(
//         &terminal.backend().wgpu_base.device,
//         &terminal.backend().wgpu_base.queue,
//         surface,
//     );
//     let surface = surface.headless().expect("headless");
//     {
//         let buffer = surface.buffer.as_ref().unwrap().slice(..);
//
//         let (send, recv) = oneshot::channel();
//         buffer.map_async(wgpu::MapMode::Read, move |data| {
//             send.send(data).unwrap();
//         });
//         terminal
//             .backend()
//             .wgpu_base
//             .device
//             .poll(PollType::Wait {
//                 submission_index: None,
//                 timeout: None,
//             })
//             .unwrap();
//         recv.recv().unwrap().unwrap();
//
//         let data = buffer.get_mapped_range();
//         let image =
//             ImageBuffer::<Rgba<u8>, _>::from_raw(surface.width, surface.height, data).unwrap();
//
//         let pixels = image.pixels().copied().collect::<Vec<_>>();
//         let golden = load_from_memory(include_bytes!("goldens/srgb_conversion.png")).unwrap();
//         let golden_pixels = golden.pixels().map(|(_, _, px)| px).collect::<Vec<_>>();
//
//         assert!(
//             pixels == golden_pixels,
//             "Rendered image differs from golden"
//         );
//     }
//     surface.buffer.as_ref().unwrap().unmap();
// }
//
// #[test]
// #[cfg(feature = "png")]
// fn png() {
//     use crate::backend::wgpu_backend::extract_color_image;
//     let golden = load_from_memory(include_bytes!("goldens/A.png")).unwrap();
//     let raster = RasterGlyphImage {
//         x: 0,
//         y: 0,
//         width: golden.width() as u16,
//         height: golden.height() as u16,
//         pixels_per_em: 0,
//         format: RasterImageFormat::PNG,
//         data: include_bytes!("goldens/A.png"),
//     };
//
//     let mut image = vec![];
//     let extracted = extract_color_image(
//         &mut image,
//         raster,
//         Entry::Cached(CacheRect {
//             color: false,
//             x: 0,
//             y: 0,
//             width: golden.width(),
//             height: golden.height(),
//         }),
//         1.0,
//     )
//     .expect("Didn't extract png")
//     .1;
//
//     for (l, r) in bytemuck::cast_slice::<_, u8>(&extracted)
//         .chunks(4)
//         .zip(golden.pixels())
//     {
//         let [r, g, b, a] = r.2.0;
//         assert_eq!(l, [a, b, g, r]);
//     }
// }
//
// #[test]
// fn bgra() {
//     use crate::backend::wgpu_backend::extract_color_image;
//
//     const BLUE: u8 = 2;
//     const GREEN: u8 = 4;
//     const RED: u8 = 8;
//     const ALPHA: u8 = 127;
//     let data = [BLUE, GREEN, RED, ALPHA];
//     let raster = RasterGlyphImage {
//         x: 0,
//         y: 0,
//         width: 1,
//         height: 1,
//         pixels_per_em: 0,
//         format: RasterImageFormat::BitmapPremulBgra32,
//         data: &data,
//     };
//
//     let mut image = vec![];
//     let extracted = extract_color_image(
//         &mut image,
//         raster,
//         Entry::Cached(CacheRect {
//             color: false,
//             x: 0,
//             y: 0,
//             width: 1,
//             height: 1,
//         }),
//         1.0,
//     )
//     .expect("Didn't extract bgra")
//     .1;
//
//     assert_eq!(
//         bytemuck::bytes_of(&extracted[0]),
//         [RED * 2, GREEN * 2, BLUE * 2, ALPHA]
//     );
// }
//
// #[test]
// fn bmp1() {
//     let data0 = 0b1000_0001;
//     let data1 = 0b0001_1000;
//     let raster = RasterGlyphImage {
//         x: 0,
//         y: 0,
//         width: 4,
//         height: 2,
//         pixels_per_em: 0,
//         format: RasterImageFormat::BitmapMono,
//         data: &[data0, data1],
//     };
//
//     let mut image = vec![];
//     let extracted = extract_bw_image(
//         &mut image,
//         raster,
//         Entry::Cached(CacheRect {
//             color: false,
//             x: 0,
//             y: 0,
//             width: 4,
//             height: 2,
//         }),
//         1.0,
//     )
//     .expect("Didn't extract bmp1")
//     .1;
//
//     assert_eq!(
//         bytemuck::cast_slice::<_, u8>(&extracted),
//         bytemuck::cast_slice(&[
//             [
//                 [255u8, 255, 255, 255,],
//                 [255, 255, 255, 0,],
//                 [255, 255, 255, 0,],
//                 [255, 255, 255, 0,],
//             ],
//             [
//                 [255, 255, 255, 0,],
//                 [255, 255, 255, 0,],
//                 [255, 255, 255, 0,],
//                 [255, 255, 255, 255,],
//             ],
//         ])
//     );
// }
//
// #[test]
// fn bmp1_packed() {
//     let data0 = 0b1000_0001;
//     let data1 = 0b0001_1000;
//     let raster = RasterGlyphImage {
//         x: 0,
//         y: 0,
//         width: 8,
//         height: 2,
//         pixels_per_em: 0,
//         format: RasterImageFormat::BitmapMonoPacked,
//         data: &[data0, data1],
//     };
//
//     let mut image = vec![];
//     let extracted = extract_bw_image(
//         &mut image,
//         raster,
//         Entry::Cached(CacheRect {
//             color: false,
//             x: 0,
//             y: 0,
//             width: 8,
//             height: 2,
//         }),
//         1.0,
//     )
//     .expect("Didn't extract bmp1")
//     .1;
//
//     assert_eq!(
//         bytemuck::cast_slice::<_, u8>(&extracted),
//         bytemuck::cast_slice(&[
//             [
//                 [255u8, 255, 255, 255,],
//                 [255, 255, 255, 0,],
//                 [255, 255, 255, 0,],
//                 [255, 255, 255, 0,],
//                 [255, 255, 255, 0,],
//                 [255, 255, 255, 0,],
//                 [255, 255, 255, 0,],
//                 [255, 255, 255, 255,],
//             ],
//             [
//                 [255, 255, 255, 0,],
//                 [255, 255, 255, 0,],
//                 [255, 255, 255, 0,],
//                 [255, 255, 255, 255,],
//                 [255, 255, 255, 255,],
//                 [255, 255, 255, 0,],
//                 [255, 255, 255, 0,],
//                 [255, 255, 255, 0,],
//             ],
//         ])
//     );
// }
//
// #[test]
// fn bmp2() {
//     let data0 = 0b1010_1101u8.reverse_bits();
//     let data1 = 0b0000_1000u8.reverse_bits();
//     let data2 = 0b0111_1010u8.reverse_bits();
//     let data3 = 0b0000_1000u8.reverse_bits();
//     let raster = RasterGlyphImage {
//         x: 0,
//         y: 0,
//         width: 6,
//         height: 2,
//         pixels_per_em: 0,
//         format: RasterImageFormat::BitmapGray2,
//         data: &[data0, data1, data2, data3],
//     };
//
//     let mut image = vec![];
//     let extracted = extract_bw_image(
//         &mut image,
//         raster,
//         Entry::Cached(CacheRect {
//             color: false,
//             x: 0,
//             y: 0,
//             width: 6,
//             height: 2,
//         }),
//         1.0,
//     )
//     .expect("Didn't extract bmp2")
//     .1;
//
//     assert_eq!(
//         bytemuck::cast_slice::<_, u8>(&extracted),
//         bytemuck::cast_slice(&[
//             [
//                 [255, 255, 255, LUT_2[0b10],],
//                 [255, 255, 255, LUT_2[0b10],],
//                 [255, 255, 255, LUT_2[0b11],],
//                 [255, 255, 255, LUT_2[0b01],],
//                 [255, 255, 255, LUT_2[0b00],],
//                 [255, 255, 255, LUT_2[0b00],],
//             ],
//             [
//                 [255, 255, 255, LUT_2[0b01],],
//                 [255, 255, 255, LUT_2[0b11],],
//                 [255, 255, 255, LUT_2[0b10],],
//                 [255, 255, 255, LUT_2[0b10],],
//                 [255, 255, 255, LUT_2[0b00],],
//                 [255, 255, 255, LUT_2[0b00],],
//             ],
//         ])
//     );
// }
//
// #[test]
// fn bmp2_packed() {
//     let data0 = 0b1010_1101u8.reverse_bits();
//     let data1 = 0b0000_1000u8.reverse_bits();
//     let data2 = 0b0111_1010u8.reverse_bits();
//     let data3 = 0b0000_1000u8.reverse_bits();
//     let data4 = 0b0000_1000u8.reverse_bits();
//     let data5 = 0b0000_1000u8.reverse_bits();
//     let raster = RasterGlyphImage {
//         x: 0,
//         y: 0,
//         width: 6,
//         height: 4,
//         pixels_per_em: 0,
//         format: RasterImageFormat::BitmapGray2Packed,
//         data: &[data0, data1, data2, data3, data4, data5],
//     };
//
//     let mut image = vec![];
//     let extracted = extract_bw_image(
//         &mut image,
//         raster,
//         Entry::Cached(CacheRect {
//             color: false,
//             x: 0,
//             y: 0,
//             width: 6,
//             height: 4,
//         }),
//         1.0,
//     )
//     .expect("Didn't extract bmp2")
//     .1;
//
//     assert_eq!(
//         bytemuck::cast_slice::<_, u8>(&extracted),
//         bytemuck::cast_slice(&[
//             [
//                 [255, 255, 255, LUT_2[0b10],],
//                 [255, 255, 255, LUT_2[0b10],],
//                 [255, 255, 255, LUT_2[0b11],],
//                 [255, 255, 255, LUT_2[0b01],],
//                 [255, 255, 255, LUT_2[0b00],],
//                 [255, 255, 255, LUT_2[0b00],],
//             ],
//             [
//                 [255, 255, 255, LUT_2[0b10],],
//                 [255, 255, 255, LUT_2[0b00],],
//                 [255, 255, 255, LUT_2[0b01],],
//                 [255, 255, 255, LUT_2[0b11],],
//                 [255, 255, 255, LUT_2[0b10],],
//                 [255, 255, 255, LUT_2[0b10],],
//             ],
//             [
//                 [255, 255, 255, LUT_2[0b00],],
//                 [255, 255, 255, LUT_2[0b00],],
//                 [255, 255, 255, LUT_2[0b10],],
//                 [255, 255, 255, LUT_2[0b00],],
//                 [255, 255, 255, LUT_2[0b00],],
//                 [255, 255, 255, LUT_2[0b00],],
//             ],
//             [
//                 [255, 255, 255, LUT_2[0b10],],
//                 [255, 255, 255, LUT_2[0b00],],
//                 [255, 255, 255, LUT_2[0b00],],
//                 [255, 255, 255, LUT_2[0b00],],
//                 [255, 255, 255, LUT_2[0b10],],
//                 [255, 255, 255, LUT_2[0b00],],
//             ]
//         ])
//     );
// }
//
// #[test]
// fn bmp4() {
//     let data0 = 0b1010_1000u8.reverse_bits();
//     let data1 = 0b0000_1000u8.reverse_bits();
//     let raster = RasterGlyphImage {
//         x: 0,
//         y: 0,
//         width: 1,
//         height: 2,
//         pixels_per_em: 0,
//         format: RasterImageFormat::BitmapGray4,
//         data: &[data0, data1],
//     };
//
//     let mut image = vec![];
//     let extracted = extract_bw_image(
//         &mut image,
//         raster,
//         Entry::Cached(CacheRect {
//             color: false,
//             x: 0,
//             y: 0,
//             width: 1,
//             height: 2,
//         }),
//         1.0,
//     )
//     .expect("Didn't extract bmp4")
//     .1;
//
//     assert_eq!(
//         bytemuck::cast_slice::<_, u8>(&extracted),
//         bytemuck::cast_slice(&[
//             [[255, 255, 255, LUT_4[0b1010],],],
//             [[255, 255, 255, LUT_4[0b0000],],],
//         ])
//     );
// }
//
// #[test]
// fn bmp4_packed() {
//     let data0 = 0b1111_0001u8.reverse_bits();
//     let data1 = 0b0011_1100u8.reverse_bits();
//     let raster = RasterGlyphImage {
//         x: 0,
//         y: 0,
//         width: 2,
//         height: 2,
//         pixels_per_em: 0,
//         format: RasterImageFormat::BitmapGray4Packed,
//         data: &[data0, data1],
//     };
//
//     let mut image = vec![];
//     let extracted = extract_bw_image(
//         &mut image,
//         raster,
//         Entry::Cached(CacheRect {
//             color: false,
//             x: 0,
//             y: 0,
//             width: 2,
//             height: 2,
//         }),
//         1.0,
//     )
//     .expect("Didn't extract bmp4")
//     .1;
//
//     assert_eq!(
//         bytemuck::cast_slice::<_, u8>(&extracted),
//         bytemuck::cast_slice(&[
//             [
//                 [255, 255, 255, LUT_4[0b1111],],
//                 [255, 255, 255, LUT_4[0b0001],],
//             ],
//             [
//                 [255, 255, 255, LUT_4[0b0011],],
//                 [255, 255, 255, LUT_4[0b1100],],
//             ],
//         ])
//     );
// }
