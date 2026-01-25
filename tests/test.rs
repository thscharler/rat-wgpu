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
use serial_test::serial;
use std::fs::create_dir_all;
use wgpu::TextureFormat;

#[test]
#[serial]
fn a_z() {
    let mut terminal = Terminal::new(
        futures_lite::future::block_on(
            Builder::<DefaultPostProcessorBuilder>::default()
                .with_fallback_fonts(Fonts::new(
                    Font::new(include_bytes!("fonts/CascadiaMono-Regular.ttf"))
                        .expect("Invalid font file"),
                    24,
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

    _ = create_dir_all("target/tmp");
    image::save_buffer(
        "target/tmp/a_z.png",
        image.as_flat_samples().samples,
        512,
        72,
        ExtendedColorType::Rgba8,
    )
    .expect("save_buffer");

    let pixels = image.pixels().copied().collect::<Vec<_>>();
    let golden = load_from_memory(include_bytes!("goldens/a_z.png")).unwrap();
    let golden_pixels = golden.pixels().map(|(_, _, px)| px).collect::<Vec<_>>();

    assert_eq!(pixels, golden_pixels, "Rendered image differs from golden");

    drop(buffer);
    terminal.backend().unmap_headless_buffer();
}

#[test]
#[serial]
fn arabic() {
    let mut terminal = Terminal::new(
        futures_lite::future::block_on(
            Builder::<DefaultPostProcessorBuilder>::default()
                .with_fallback_fonts(Fonts::new(
                    Font::new(include_bytes!("fonts/CascadiaMono-Regular.ttf"))
                        .expect("Invalid font file"),
                    24,
                ))
                .with_width_and_height(256, 72)
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
            f.render_widget(Paragraph::new("مرحبا بالعالم"), area);
        })
        .unwrap();

    let buffer = terminal
        .backend()
        .map_headless_buffer()
        .expect("headless buffer");

    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(256, 72, &*buffer).unwrap();

    _ = create_dir_all("target/tmp");
    image::save_buffer(
        "target/tmp/arabic.png",
        image.as_flat_samples().samples,
        256,
        72,
        ExtendedColorType::Rgba8,
    )
    .expect("save_buffer");
    let pixels = image.pixels().copied().collect::<Vec<_>>();
    let golden = load_from_memory(include_bytes!("goldens/arabic.png")).unwrap();
    let golden_pixels = golden.pixels().map(|(_, _, px)| px).collect::<Vec<_>>();

    assert_eq!(pixels, golden_pixels, "Rendered image differs from golden");

    drop(buffer);
    terminal.backend().unmap_headless_buffer();
}

#[test]
#[serial]
fn really_wide() {
    let mut terminal = Terminal::new(
        futures_lite::future::block_on(
            Builder::<DefaultPostProcessorBuilder>::default()
                .with_fallback_fonts(Fonts::new(
                    Font::new(include_bytes!("fonts/Fairfax.ttf")).expect("Invalid font file"),
                    24,
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
            f.render_widget(Paragraph::new("Ｈｅｌｌｏ, ｗｏｒｌｄ!"), area);
        })
        .unwrap();

    let buffer = terminal
        .backend()
        .map_headless_buffer()
        .expect("headless buffer");

    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(512, 72, &*buffer).unwrap();

    _ = create_dir_all("target/tmp");
    image::save_buffer(
        "target/tmp/really_wide.png",
        image.as_flat_samples().samples,
        512,
        72,
        ExtendedColorType::Rgba8,
    )
    .expect("save_buffer");
    let pixels = image.pixels().copied().collect::<Vec<_>>();
    let golden = load_from_memory(include_bytes!("goldens/really_wide.png")).unwrap();
    let golden_pixels = golden.pixels().map(|(_, _, px)| px).collect::<Vec<_>>();

    assert_eq!(pixels, golden_pixels, "Rendered image differs from golden");

    drop(buffer);
    terminal.backend().unmap_headless_buffer();
}

#[test]
#[serial]
fn mixed() {
    let mut terminal = Terminal::new(
        futures_lite::future::block_on(
            Builder::<DefaultPostProcessorBuilder>::default()
                .with_fallback_fonts(Fonts::new(
                    Font::new(include_bytes!("fonts/CascadiaMono-Regular.ttf"))
                        .expect("Invalid font file"),
                    24,
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
            f.render_widget(
                Paragraph::new("Hello World! مرحبا بالعالم 0123456789000000000"),
                area,
            );
        })
        .unwrap();

    let buffer = terminal
        .backend()
        .map_headless_buffer()
        .expect("headless buffer");

    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(512, 72, &*buffer).unwrap();

    _ = create_dir_all("target/tmp");
    image::save_buffer(
        "target/tmp/mixed.png",
        image.as_flat_samples().samples,
        512,
        72,
        ExtendedColorType::Rgba8,
    )
    .expect("save_buffer");
    let pixels = image.pixels().copied().collect::<Vec<_>>();
    let golden = load_from_memory(include_bytes!("goldens/mixed.png")).unwrap();
    let golden_pixels = golden.pixels().map(|(_, _, px)| px).collect::<Vec<_>>();

    assert_eq!(pixels, golden_pixels, "Rendered image differs from golden");

    drop(buffer);
    terminal.backend().unmap_headless_buffer();
}

#[test]
#[serial]
fn mixed_colors() {
    let mut terminal = Terminal::new(
        futures_lite::future::block_on(
            Builder::<DefaultPostProcessorBuilder>::default()
                .with_fallback_fonts(Fonts::new(
                    Font::new(include_bytes!("fonts/CascadiaMono-Regular.ttf"))
                        .expect("Invalid font file"),
                    24,
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
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    "Hello World!".green(),
                    "مرحبا بالعالم".blue(),
                    "0123456789".dim(),
                ])),
                area,
            );
        })
        .unwrap();

    let buffer = terminal
        .backend()
        .map_headless_buffer()
        .expect("headless buffer");

    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(512, 72, &*buffer).unwrap();

    _ = create_dir_all("target/tmp");
    image::save_buffer(
        "target/tmp/mixed_colors.png",
        image.as_flat_samples().samples,
        512,
        72,
        ExtendedColorType::Rgba8,
    )
    .expect("save_buffer");
    let pixels = image.pixels().copied().collect::<Vec<_>>();
    let golden = load_from_memory(include_bytes!("goldens/mixed_colors.png")).unwrap();
    let golden_pixels = golden.pixels().map(|(_, _, px)| px).collect::<Vec<_>>();

    assert_eq!(pixels, golden_pixels, "Rendered image differs from golden");

    drop(buffer);
    terminal.backend().unmap_headless_buffer();
}

#[test]
#[serial]
fn overlap() {
    let mut terminal = Terminal::new(
        futures_lite::future::block_on(
            Builder::<DefaultPostProcessorBuilder>::default()
                .with_fallback_fonts(Fonts::new(
                    Font::new(include_bytes!("fonts/Fairfax.ttf")).expect("Invalid font file"),
                    24,
                ))
                .with_width_and_height(256, 72)
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
            f.render_widget(Paragraph::new("H̴̢͕̠͖͇̻͓̙̞͔͕͓̰͋͛͂̃̌͂͆͜͠".underlined()), area);
        })
        .unwrap();

    let buffer = terminal
        .backend()
        .map_headless_buffer()
        .expect("headless buffer");

    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(256, 72, &*buffer).unwrap();

    _ = create_dir_all("target/tmp");
    image::save_buffer(
        "target/tmp/overlap_initial.png",
        image.as_flat_samples().samples,
        256,
        72,
        ExtendedColorType::Rgba8,
    )
    .expect("save_buffer");
    let pixels = image.pixels().copied().collect::<Vec<_>>();
    let golden = load_from_memory(include_bytes!("goldens/overlap_initial.png")).unwrap();
    let golden_pixels = golden.pixels().map(|(_, _, px)| px).collect::<Vec<_>>();

    assert_eq!(pixels, golden_pixels, "Rendered image differs from golden");

    drop(buffer);
    terminal.backend().unmap_headless_buffer();

    terminal
        .draw(|f: &mut ratatui_core::terminal::Frame| {
            let block = Block::bordered();
            let area = block.inner(f.area());
            f.render_widget(block, f.area());
            f.render_widget(Paragraph::new("H".underlined()), area);
        })
        .unwrap();

    let buffer = terminal
        .backend()
        .map_headless_buffer()
        .expect("headless buffer");

    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(256, 72, &*buffer).unwrap();

    _ = create_dir_all("target/tmp");
    image::save_buffer(
        "target/tmp/overlap_post.png",
        image.as_flat_samples().samples,
        256,
        72,
        ExtendedColorType::Rgba8,
    )
    .expect("save_buffer");
    let pixels = image.pixels().copied().collect::<Vec<_>>();
    let golden = load_from_memory(include_bytes!("goldens/overlap_post.png")).unwrap();
    let golden_pixels = golden.pixels().map(|(_, _, px)| px).collect::<Vec<_>>();

    assert_eq!(pixels, golden_pixels, "Rendered image differs from golden");

    drop(buffer);
    terminal.backend().unmap_headless_buffer();
}

#[test]
#[serial]
fn overlap_colors() {
    let mut terminal = Terminal::new(
        futures_lite::future::block_on(
            Builder::<DefaultPostProcessorBuilder>::default()
                .with_fallback_fonts(Fonts::new(
                    Font::new(include_bytes!("fonts/Fairfax.ttf")).expect("Invalid font file"),
                    24,
                ))
                .with_width_and_height(256, 72)
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
            f.render_widget(Paragraph::new("H̴̢͕̠͖͇̻͓̙̞͔͕͓̰͋͛͂̃̌͂͆͜͠".blue().on_red().underlined()), area);
        })
        .unwrap();

    let buffer = terminal
        .backend()
        .map_headless_buffer()
        .expect("headless buffer");

    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(256, 72, &*buffer).unwrap();

    _ = create_dir_all("target/tmp");
    image::save_buffer(
        "target/tmp/overlap_colors.png",
        image.as_flat_samples().samples,
        256,
        72,
        ExtendedColorType::Rgba8,
    )
    .expect("save_buffer");
    let pixels = image.pixels().copied().collect::<Vec<_>>();
    let golden = load_from_memory(include_bytes!("goldens/overlap_colors.png")).unwrap();
    let golden_pixels = golden.pixels().map(|(_, _, px)| px).collect::<Vec<_>>();

    assert_eq!(pixels, golden_pixels, "Rendered image differs from golden");

    drop(buffer);
    terminal.backend().unmap_headless_buffer();
}

#[test]
#[serial]
fn rgb_conversion() {
    let mut terminal = Terminal::new(
        futures_lite::future::block_on(
            Builder::<DefaultPostProcessorBuilder>::default()
                .with_fallback_fonts(Fonts::new(
                    Font::new(include_bytes!("fonts/Fairfax.ttf")).expect("Invalid font file"),
                    24,
                ))
                .with_width_and_height(256, 72)
                .with_bg_color(Color::Rgb(0x1E, 0x23, 0x26))
                .with_fg_color(Color::White)
                .build_headless_with_format(TextureFormat::Rgba8Unorm),
        )
        .unwrap(),
    )
    .unwrap();

    terminal
        .draw(|f: &mut ratatui_core::terminal::Frame| {
            let block = Block::bordered();
            let area = block.inner(f.area());
            f.render_widget(block, f.area());
            f.render_widget(Paragraph::new("TEST"), area);
        })
        .unwrap();

    let buffer = terminal
        .backend()
        .map_headless_buffer()
        .expect("headless buffer");

    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(256, 72, &*buffer).unwrap();

    _ = create_dir_all("target/tmp");
    image::save_buffer(
        "target/tmp/rgb_conversion.png",
        image.as_flat_samples().samples,
        256,
        72,
        ExtendedColorType::Rgba8,
    )
    .expect("save_buffer");
    let pixels = image.pixels().copied().collect::<Vec<_>>();
    let golden = load_from_memory(include_bytes!("goldens/rgb_conversion.png")).unwrap();
    let golden_pixels = golden.pixels().map(|(_, _, px)| px).collect::<Vec<_>>();

    assert_eq!(pixels, golden_pixels, "Rendered image differs from golden");

    drop(buffer);
    terminal.backend().unmap_headless_buffer();
}

#[test]
#[serial]
fn srgb_conversion() {
    let mut terminal = Terminal::new(
        futures_lite::future::block_on(
            Builder::<DefaultPostProcessorBuilder>::default()
                .with_fallback_fonts(Fonts::new(
                    Font::new(include_bytes!("fonts/Fairfax.ttf")).expect("Invalid font file"),
                    24,
                ))
                .with_width_and_height(256, 72)
                .with_bg_color(Color::Rgb(0x1E, 0x23, 0x26))
                .with_fg_color(Color::White)
                .build_headless_with_format(TextureFormat::Rgba8UnormSrgb),
        )
        .unwrap(),
    )
    .unwrap();

    terminal
        .draw(|f: &mut ratatui_core::terminal::Frame| {
            let block = Block::bordered();
            let area = block.inner(f.area());
            f.render_widget(block, f.area());
            f.render_widget(Paragraph::new("TEST"), area);
        })
        .unwrap();

    let buffer = terminal
        .backend()
        .map_headless_buffer()
        .expect("headless buffer");

    let image = ImageBuffer::<Rgba<u8>, _>::from_raw(256, 72, &*buffer).unwrap();

    _ = create_dir_all("target/tmp");
    image::save_buffer(
        "target/tmp/srgb_conversion.png",
        image.as_flat_samples().samples,
        256,
        72,
        ExtendedColorType::Rgba8,
    )
    .expect("save_buffer");

    let pixels = image.pixels().copied().collect::<Vec<_>>();
    let golden = load_from_memory(include_bytes!("goldens/srgb_conversion.png")).unwrap();
    let golden_pixels = golden.pixels().map(|(_, _, px)| px).collect::<Vec<_>>();

    assert_eq!(pixels, golden_pixels, "Rendered image differs from golden");

    drop(buffer);
    terminal.backend().unmap_headless_buffer();
}
