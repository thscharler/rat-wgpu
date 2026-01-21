use crate::backend::text_atlas::{CacheRect, Entry};
use crate::font::outline::{Outline, Painter};
use bitvec::order::Lsb0;
use bitvec::slice::BitSlice;
use raqote::{DrawOptions, DrawTarget, SolidSource, StrokeStyle, Transform};
use rustybuzz::ttf_parser::{
    GlyphId, OutlineBuilder, RasterGlyphImage, RasterImageFormat, RgbaColor,
};
use unicode_properties::GeneralCategory;

pub(super) fn rasterize_glyph(
    cached: Entry,
    metrics: &rustybuzz::Face,
    info: &rustybuzz::GlyphInfo,
    bold: bool,
    italic: bool,
    advance_scale: f32,
    mut ascender: f32,
    emoji: bool,
    block_char: bool,
    category: GeneralCategory,
    is_fallback: bool,
) -> (CacheRect, Vec<u32>) {
    let actual_width = metrics
        .glyph_hor_advance(GlyphId(info.glyph_id as _))
        .unwrap_or_default();
    let actual_width_px = if actual_width == 0 {
        cached.width
    } else {
        (actual_width as f32 * advance_scale) as u32
    };

    let computed_offset_x;
    let computed_offset_y;

    let scale;
    let scale_y;
    if is_fallback && block_char {
        let rect_scale_y = cached.height as f32 / (metrics.height() as f32);

        ascender = (metrics.ascender() as f32) * (rect_scale_y / advance_scale);

        computed_offset_x = 0.0;
        computed_offset_y = 0.0;

        scale = rect_scale_y * 2.0;
        scale_y = rect_scale_y * 2.0;
    } else if is_fallback {
        // glyphs from a fallback font will probably not fit.
        // scale them down either vertically or horizontally, whatever fits.
        // then align them centered.
        // and later render them at the same baseline as the regular font.

        let mut rect_scale_x = cached.width as f32 / (actual_width as f32);
        let rect_scale_y = cached.height as f32 / metrics.height() as f32;

        if rect_scale_x / rect_scale_y > 1.0 {
            rect_scale_x = rect_scale_y;
            computed_offset_x = (cached.width as f32 - actual_width as f32 * rect_scale_y) / 2.0;
        } else {
            computed_offset_x = 0.0;
        }
        computed_offset_y = 0.0;

        scale = rect_scale_x * 2.0;
        scale_y = rect_scale_y * 2.0;
    } else if !metrics.is_monospaced() {
        let mut rect_scale_x = cached.width as f32 / (actual_width as f32);

        if rect_scale_x / advance_scale > 1.0 {
            rect_scale_x = advance_scale;
            computed_offset_x = (cached.width as f32 - actual_width as f32 * advance_scale) / 2.0;
        } else {
            computed_offset_x = 0.0;
        }
        computed_offset_y = 0.0;

        scale = rect_scale_x * 2.0;
        scale_y = advance_scale * 2.0;
    } else {
        // regular fonts will probably be from one font family and therefore have
        // more regular properties.
        let rect_scale = cached.width as f32 / actual_width_px as f32;

        // don't offset. font should fit.
        computed_offset_x = 0.0;
        computed_offset_y = 0.0;

        scale = rect_scale * advance_scale * 2.0;
        scale_y = scale;
    }

    let skew = if !emoji && !metrics.is_italic() && italic {
        Transform::new(
            /* scale x */ 1.0,
            /* skew x */ 0.0,
            /* skew y */ -0.25,
            /* scale y */ 1.0,
            /* translate x */ -0.25 * cached.width as f32,
            /* translate y */ 0.0,
        )
    } else {
        Transform::default()
    };

    if info.glyph_id == 0 {
        // the glyph provided by the font is ugly most of the time.
        let width = cached.width as usize;
        let height = cached.height as usize;

        let mut image = vec![0u32; width * height];

        let mut target = DrawTarget::from_backing(width as i32, height as i32, &mut image[..]);

        let w1 = width as f32 * 0.33;
        let w2 = width as f32 * 0.67;
        let h1 = height as f32 * 0.33;
        let h2 = height as f32 * 0.67;

        let mut render = Outline::default();
        render.move_to(w1, h1);
        render.line_to(w2, h1);
        render.line_to(w2, h2);
        render.line_to(w1, h2);
        render.close();
        let path = render.finish();

        target.stroke(
            &path,
            &raqote::Source::Solid(SolidSource::from_unpremultiplied_argb(255, 255, 255, 255)),
            &StrokeStyle {
                width: 1.5,
                ..Default::default()
            },
            &DrawOptions::new(),
        );

        return (
            CacheRect {
                color: false,
                ..*cached
            },
            image,
        );
    }

    let mut image = vec![0u32; cached.width as usize * 2 * cached.height as usize * 2];
    let mut target = DrawTarget::from_backing(
        cached.width as i32 * 2,
        cached.height as i32 * 2,
        &mut image[..],
    );

    let mut painter = Painter::new(
        metrics,
        &mut target,
        skew,
        scale,
        ascender * advance_scale * 2.0 + computed_offset_y,
        computed_offset_x,
    );
    if metrics
        .paint_color_glyph(
            GlyphId(info.glyph_id as _),
            0,
            RgbaColor::new(255, 255, 255, 255),
            &mut painter,
        )
        .is_some()
    {
        let mut final_image = DrawTarget::new(cached.width as i32, cached.height as i32);
        final_image.draw_image_with_size_at(
            cached.width as f32,
            cached.height as f32,
            0.,
            0.,
            &raqote::Image {
                width: cached.width as i32 * 2,
                height: cached.height as i32 * 2,
                data: &image,
            },
            &DrawOptions {
                blend_mode: raqote::BlendMode::Src,
                antialias: raqote::AntialiasMode::None,
                ..Default::default()
            },
        );

        let mut final_image = final_image.into_vec();
        for argb in final_image.iter_mut() {
            let [a, r, g, b] = argb.to_be_bytes();
            *argb = u32::from_le_bytes([r, g, b, a]);
        }

        return (
            CacheRect {
                color: true,
                ..*cached
            },
            final_image,
        );
    }

    if let Some(raster) = metrics.glyph_raster_image(GlyphId(info.glyph_id as _), u16::MAX) {
        if let Some((cache_rect, image)) =
            extract_color_image(&mut image, raster, cached, advance_scale)
        {
            return (
                CacheRect {
                    color: true,
                    ..cache_rect
                },
                image,
            );
        }
    }

    let mut render = Outline::default();
    if let Some(bounds) = metrics.outline_glyph(GlyphId(info.glyph_id as _), &mut render) {
        let path = render.finish();

        // Some fonts return bounds that are entirely negative. I'm not sure why this
        // is, but it means the glyph won't render at all. We check for this here and
        // offset it if so. This seems to let those fonts render correctly.
        let x_off = if bounds.x_max < 0 {
            -bounds.x_min as f32
        } else {
            0.
        };
        let x_off = x_off * scale + computed_offset_x;
        let y_off = ascender * advance_scale * 2.0 + computed_offset_y;

        let mut target = DrawTarget::from_backing(
            cached.width as i32 * 2,
            cached.height as i32 * 2,
            &mut image[..],
        );
        target.set_transform(
            &Transform::scale(scale, -scale_y)
                .then(&skew)
                .then_translate((x_off, y_off).into()),
        );

        target.fill(
            &path,
            &raqote::Source::Solid(SolidSource::from_unpremultiplied_argb(255, 255, 255, 255)),
            &DrawOptions::default(),
        );

        if !metrics.is_bold() && bold {
            target.stroke(
                &path,
                &raqote::Source::Solid(SolidSource::from_unpremultiplied_argb(255, 255, 255, 255)),
                &StrokeStyle {
                    width: 1.5 / scale,
                    ..Default::default()
                },
                &DrawOptions::new(),
            );
        } else if emoji {
            // noto-emoji and open-moji need this.
            target.stroke(
                &path,
                &raqote::Source::Solid(SolidSource::from_unpremultiplied_argb(255, 255, 255, 255)),
                &StrokeStyle {
                    width: 1.0 / scale,
                    ..Default::default()
                },
                &DrawOptions::new(),
            );
        } else if is_fallback && category == GeneralCategory::OtherSymbol {
            // noto-emoji and open-moji need this.
            target.stroke(
                &path,
                &raqote::Source::Solid(SolidSource::from_unpremultiplied_argb(255, 255, 255, 255)),
                &StrokeStyle {
                    width: 1.0 / scale,
                    ..Default::default()
                },
                &DrawOptions::new(),
            );
        }

        let mut final_image = DrawTarget::new(cached.width as i32, cached.height as i32);
        final_image.draw_image_with_size_at(
            cached.width as f32,
            cached.height as f32,
            0.,
            0.,
            &raqote::Image {
                width: cached.width as i32 * 2,
                height: cached.height as i32 * 2,
                data: &image,
            },
            &DrawOptions {
                blend_mode: raqote::BlendMode::Src,
                antialias: raqote::AntialiasMode::None,
                ..Default::default()
            },
        );

        return (
            CacheRect {
                color: false,
                ..*cached
            },
            final_image.into_vec(),
        );
    }

    if let Some(raster) = metrics.glyph_raster_image(GlyphId(info.glyph_id as _), u16::MAX) {
        if raster.width != 0 && raster.height != 0 {
            if let Some((cached, image)) =
                extract_bw_image(&mut image, raster, cached, advance_scale)
            {
                return (
                    CacheRect {
                        color: false,
                        ..cached
                    },
                    image,
                );
            }
        }
    }

    (
        CacheRect {
            color: false,
            ..*cached
        },
        vec![0u32; cached.width as usize * cached.height as usize],
    )
}

fn extract_color_image(
    image: &mut Vec<u32>,
    raster: RasterGlyphImage,
    cached: Entry,
    scale: f32,
) -> Option<(CacheRect, Vec<u32>)> {
    match raster.format {
        RasterImageFormat::PNG => {
            #[cfg(feature = "png")]
            {
                let decoder = png::Decoder::new(std::io::Cursor::new(raster.data));
                if let Ok(mut info) = decoder.read_info() {
                    image.resize(
                        info.output_buffer_size().unwrap_or_default() / size_of::<u32>(),
                        0,
                    );
                    if info.next_frame(bytemuck::cast_slice_mut(image)).is_err() {
                        return None;
                    }

                    for rgba in image.iter_mut() {
                        let [r, g, b, a] = rgba.to_be_bytes();
                        *rgba = u32::from_be_bytes([a, r, g, b]);
                    }
                } else {
                    return None;
                }
            }
            #[cfg(not(feature = "png"))]
            return None;
        }
        RasterImageFormat::BitmapPremulBgra32 => {
            image.resize(raster.width as usize * raster.height as usize, 0);
            for (y, row) in raster.data.chunks(raster.width as usize * 4).enumerate() {
                for (x, pixel) in row.chunks(4).enumerate() {
                    let pixel: &[u8; 4] = pixel.try_into().expect("Invalid chunk size");
                    let [b, g, r, a] = *pixel;
                    let pixel = u32::from_be_bytes([
                        a,
                        r.saturating_mul(255 / a),
                        g.saturating_mul(255 / a),
                        b.saturating_mul(255 / a),
                    ]);
                    image[y * raster.width as usize + x] = pixel;
                }
            }
        }
        _ => return None,
    }

    let mut final_image = DrawTarget::new(cached.width as i32, cached.height as i32);
    final_image.draw_image_with_size_at(
        cached.width as f32,
        cached.height as f32,
        raster.x as f32 * scale,
        raster.y as f32 * scale,
        &raqote::Image {
            width: raster.width as i32,
            height: raster.height as i32,
            data: &*image,
        },
        &DrawOptions {
            blend_mode: raqote::BlendMode::Src,
            antialias: raqote::AntialiasMode::None,
            ..Default::default()
        },
    );

    let mut final_image = final_image.into_vec();
    for argb in final_image.iter_mut() {
        let [a, r, g, b] = argb.to_be_bytes();
        *argb = u32::from_le_bytes([r, g, b, a]);
    }

    Some((*cached, final_image))
}

fn extract_bw_image(
    image: &mut Vec<u32>,
    raster: RasterGlyphImage,
    cached: Entry,
    scale: f32,
) -> Option<(CacheRect, Vec<u32>)> {
    image.resize(raster.width as usize * raster.height as usize, 0);

    match raster.format {
        RasterImageFormat::BitmapMono => {
            from_gray_unpacked::<1, 2>(image, raster, LUT_1);
        }
        RasterImageFormat::BitmapMonoPacked => {
            from_gray_packed::<1, 2>(image, raster, LUT_1);
        }
        RasterImageFormat::BitmapGray2 => {
            from_gray_unpacked::<2, 4>(image, raster, LUT_2);
        }
        RasterImageFormat::BitmapGray2Packed => {
            from_gray_packed::<2, 4>(image, raster, LUT_2);
        }
        RasterImageFormat::BitmapGray4 => {
            from_gray_unpacked::<4, 16>(image, raster, LUT_4);
        }
        RasterImageFormat::BitmapGray4Packed => {
            from_gray_packed::<4, 16>(image, raster, LUT_4);
        }
        RasterImageFormat::BitmapGray8 => {
            for (byte, dst) in raster.data.iter().zip(image.iter_mut()) {
                *dst = u32::from_be_bytes([*byte, 255, 255, 255]);
            }
        }
        _ => return None,
    }

    let mut final_image = DrawTarget::new(cached.width as i32, cached.height as i32);
    final_image.draw_image_with_size_at(
        cached.width as f32,
        cached.height as f32,
        raster.x as f32 * scale,
        raster.y as f32 * scale,
        &raqote::Image {
            width: raster.width as i32,
            height: raster.height as i32,
            data: &*image,
        },
        &DrawOptions {
            blend_mode: raqote::BlendMode::Src,
            antialias: raqote::AntialiasMode::None,
            ..Default::default()
        },
    );

    let mut final_image = final_image.into_vec();
    for argb in final_image.iter_mut() {
        let [a, r, g, b] = argb.to_be_bytes();
        *argb = u32::from_le_bytes([r, g, b, a]);
    }

    Some((*cached, final_image))
}

fn from_gray_unpacked<const BITS: usize, const ENTRIES: usize>(
    image: &mut [u32],
    raster: RasterGlyphImage,
    steps: [u8; ENTRIES],
) {
    for (bits, dst) in raster
        .data
        .chunks((raster.width as usize / (8 / BITS)) + 1)
        .zip(image.chunks_mut(raster.width as usize))
    {
        let bits = BitSlice::<_, Lsb0>::from_slice(bits);
        for (bits, dst) in bits.chunks(BITS).zip(dst.iter_mut()) {
            let mut index = 0;
            for idx in bits.iter_ones() {
                index |= 1 << (BITS - idx - 1);
            }
            let value = steps[index as usize];
            *dst = u32::from_be_bytes([value, 255, 255, 255]);
        }
    }
}

fn from_gray_packed<const BITS: usize, const ENTRIES: usize>(
    image: &mut [u32],
    raster: RasterGlyphImage,
    steps: [u8; ENTRIES],
) {
    let bits = BitSlice::<_, Lsb0>::from_slice(raster.data);
    for (bits, dst) in bits.chunks(BITS).zip(image.iter_mut()) {
        let mut index = 0;
        for idx in bits.iter_ones() {
            index |= 1 << (BITS - idx - 1);
        }
        let value = steps[index as usize];
        *dst = u32::from_be_bytes([value, 255, 255, 255]);
    }
}

const LUT_1: [u8; 2] = [0, 255];
const LUT_2: [u8; 4] = [0, 255 / 3, 2 * (255 / 3), 255];
const LUT_4: [u8; 16] = [
    0,
    (255 / 15),
    2 * (255 / 15),
    3 * (255 / 15),
    4 * (255 / 15),
    5 * (255 / 15),
    6 * (255 / 15),
    7 * (255 / 15),
    8 * (255 / 15),
    9 * (255 / 15),
    10 * (255 / 15),
    11 * (255 / 15),
    12 * (255 / 15),
    13 * (255 / 15),
    14 * (255 / 15),
    255,
];
