use crate::CellBox;
use crate::util::intersect;
use euclid::Vector2D;
use raqote::Transform;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Handle for any added image.
///
/// When the handle is dropped, the backing texture will be dropped after
/// the next flush().
#[derive(Debug, Default, Clone)]
pub struct ImageHandle {
    pub(crate) id: usize,
    pub(crate) dropped: Arc<AtomicBool>,
}

impl ImageHandle {
    pub fn id(&self) -> usize {
        self.id
    }
}

impl Drop for ImageHandle {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::Release)
    }
}

/// Fit the image to the render area.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ImageFit {
    /// Fill the whole area. This will not respect the aspect ratio
    /// of the original image.
    #[default]
    Fill,
    /// Fit the image to the area. It will be scaled either horizontally
    /// or vertically to make the image fit.
    FitStart,
    /// Fit the image to the area. It will be scaled either horizontally
    /// or vertically to make the image fit. Center in the other direction.
    FitCenter,
    /// Fit the image to the area. It will be scaled either horizontally
    /// or vertically to make the image fit. It will be right/bottom aligned.
    FitEnd,
    /// Fit the image to the area. It will be scaled horizontally to
    /// make the image fit. The image will be clipped or the background
    /// will be visible.
    HorizontalStart,
    /// Fit the image to the area. It will be scaled horizontally to
    /// make the image fit. The image will be clipped or the background
    /// will be visible.
    HorizontalCenter,
    /// Fit the image to the area. It will be scaled horizontally to
    /// make the image fit. The image will be clipped or the background
    /// will be visible.
    HorizontalEnd,
    /// Fit the image to the area. It will be scaled vertically to
    /// make the image fit. The image will be clipped or the background
    /// will be visible.
    VerticalStart,
    /// Fit the image to the area. It will be scaled vertically to
    /// make the image fit. The image will be clipped or the background
    /// will be visible.
    VerticalCenter,
    /// Fit the image to the area. It will be scaled vertically to
    /// make the image fit. The image will be clipped or the background
    /// will be visible.
    VerticalEnd,
}

/// The rendered data for one image.
#[derive(Debug, Clone)]
pub struct ImageCell {
    pub image_id: usize,
    pub view_rect: (i32, i32, u32, u32),
    pub view_clip: (i32, i32, u32, u32),
    pub below_text: bool,
    pub tr: Transform,
}

#[derive(Debug, Default, Clone)]
pub struct ImageArg {
    view_clip_area: Option<ratatui_core::layout::Rect>,
    view_clip: Option<(i32, i32, u32, u32)>,
    below_text: bool,
    fit: Option<ImageFit>,
    tr: Option<Transform>,
}

impl ImageArg {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clip_area(mut self, area: ratatui_core::layout::Rect) -> Self {
        self.view_clip_area = Some(area);
        self.view_clip = None;
        self
    }

    pub fn clip(mut self, rect: (i32, i32, u32, u32)) -> Self {
        self.view_clip = Some(rect);
        self.view_clip_area = None;
        self
    }

    pub fn above_text(mut self) -> Self {
        self.below_text = false;
        self
    }

    pub fn below_text(mut self) -> Self {
        self.below_text = true;
        self
    }

    pub fn fit(mut self, fit: ImageFit) -> Self {
        self.fit = Some(fit);
        self.tr = None;
        self
    }

    pub fn transform(mut self, tr: Transform) -> Self {
        self.tr = Some(tr);
        self.fit = None;
        self
    }
}

/// The ImageFrame works analogous to the [ratatui_core::terminal::Frame].
/// You tell it what images should be rendered for one render-pass.
///
/// During flush() it will check the data and render what is necessary.
///
#[derive(Debug, Default, Clone)]
pub struct ImageFrame {
    pub(crate) buffer: Arc<Mutex<ImageBuffer>>,
}

impl ImageFrame {
    pub fn buffer(&self) -> Arc<Mutex<ImageBuffer>> {
        self.buffer.clone()
    }
}

#[derive(Debug, Default, Clone)]
pub struct ImageBuffer {
    // The buffer area.
    pub(crate) area: ratatui_core::layout::Rect,
    // cell-size. this is updated whenever the font-size or font is changed.
    pub(crate) cell_box: CellBox,
    // information for all available images.
    pub(crate) image_size: HashMap<usize, (u32, u32)>,
    // actual render-queue. this will be read when flush() is called and
    // renders the images.
    // - image-id
    // - target rect (x,y,w,h)
    // - transform to access the image-texture
    pub(crate) images: Vec<ImageCell>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImageScale {
    XY,
    X,
    Y,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImageAlign {
    Start,
    Center,
    End,
}

impl ImageBuffer {
    pub fn new(
        area: ratatui_core::layout::Rect,
        cell_box: CellBox,
        image_size: HashMap<usize, (u32, u32)>,
    ) -> Self {
        Self {
            area,
            cell_box,
            image_size,
            images: Default::default(),
        }
    }

    /// Create a new ImageBuffer with the same cell_box and image-sizes,
    /// but a new area and an empty image list.
    pub fn derive(&self, area: ratatui_core::layout::Rect) -> Self {
        Self {
            area,
            cell_box: self.cell_box,
            image_size: self.image_size.clone(),
            images: Default::default(),
        }
    }

    /// Add all the images from the given buffer.
    ///
    /// Shift each image by shift cells and clip each image everything.
    pub fn append(
        &mut self,
        buf: ImageBuffer,
        shift: (i16, i16),
        clip: ratatui_core::layout::Rect,
    ) {
        let shift = (
            shift.0 as i32 * self.cell_box.width as i32,
            shift.1 as i32 * self.cell_box.height as i32,
        );
        let clip = self.rect_px(clip);

        for mut img in buf.images {
            img.view_rect.0 += shift.0;
            img.view_rect.1 += shift.1;
            img.view_clip.0 += shift.0;
            img.view_clip.1 += shift.1;
            img.view_clip = intersect(img.view_clip, clip).unwrap_or_default();

            self.images.push(img);
        }
    }

    /// Get the area of the buffer in cells.
    pub fn area(&self) -> ratatui_core::layout::Rect {
        self.area
    }

    /// Get the area of the buffer in pixel.
    pub fn area_px(&self) -> (i32, i32, u32, u32) {
        self.rect_px(self.area)
    }

    /// Convert the ratatui Rect to a screen-area.
    ///
    /// This will not check if the area is inside the window bounds.
    pub fn rect_px(&self, area: ratatui_core::layout::Rect) -> (i32, i32, u32, u32) {
        let font_box = self.cell_box();
        (
            area.x as i32 * font_box.width as i32,
            area.y as i32 * font_box.height as i32,
            area.width as u32 * font_box.width,
            area.height as u32 * font_box.height,
        )
    }

    /// Get the active FontBox
    pub fn cell_box(&self) -> CellBox {
        self.cell_box
    }

    /// Get the image-size in px for an added image.
    pub fn image_size(&self, id: &ImageHandle) -> Option<(u32, u32)> {
        self.image_size.get(&id.id).cloned()
    }

    /// Get the rendered images.
    pub fn images(&self) -> &[ImageCell] {
        &self.images
    }

    /// Render an image.
    pub fn render(&mut self, id: &ImageHandle, area: ratatui_core::layout::Rect, arg: ImageArg) {
        self.render_px(id, self.rect_px(area), arg)
    }

    /// Render an image.
    ///
    /// To get an ImageHandle add the image first with [add_image]. Add image
    /// will create the texture for the image.
    pub fn render_px(&mut self, id: &ImageHandle, rect: (i32, i32, u32, u32), arg: ImageArg) {
        let tr = if let Some(fit) = arg.fit {
            use ImageAlign::*;
            use ImageScale::*;

            match fit {
                ImageFit::Fill => Transform::default(),
                ImageFit::FitStart => {
                    let img = self.image_size(id).expect("img1");
                    self.scale_to_fit(img, (rect.2, rect.3), XY, Start)
                }
                ImageFit::FitCenter => {
                    let img = self.image_size(id).expect("img1");
                    self.scale_to_fit(img, (rect.2, rect.3), XY, Center)
                }
                ImageFit::FitEnd => {
                    let img = self.image_size(id).expect("img1");
                    self.scale_to_fit(img, (rect.2, rect.3), XY, End)
                }
                ImageFit::HorizontalStart => {
                    let img = self.image_size(id).expect("img1");
                    self.scale_to_fit(img, (rect.2, rect.3), X, Start)
                }
                ImageFit::HorizontalCenter => {
                    let img = self.image_size(id).expect("img1");
                    self.scale_to_fit(img, (rect.2, rect.3), X, Center)
                }
                ImageFit::HorizontalEnd => {
                    let img = self.image_size(id).expect("img1");
                    self.scale_to_fit(img, (rect.2, rect.3), X, End)
                }
                ImageFit::VerticalStart => {
                    let img = self.image_size(id).expect("img1");
                    self.scale_to_fit(img, (rect.2, rect.3), Y, Start)
                }
                ImageFit::VerticalCenter => {
                    let img = self.image_size(id).expect("img1");
                    self.scale_to_fit(img, (rect.2, rect.3), Y, Center)
                }
                ImageFit::VerticalEnd => {
                    let img = self.image_size(id).expect("img1");
                    self.scale_to_fit(img, (rect.2, rect.3), Y, End)
                }
            }
        } else if let Some(tr) = arg.tr {
            tr
        } else {
            Transform::default()
        };

        let clip = if let Some(clip) = arg.view_clip {
            clip
        } else if let Some(area) = arg.view_clip_area {
            self.rect_px(area)
        } else {
            rect
        };

        self.images.push(ImageCell {
            image_id: id.id(),
            view_rect: rect,
            view_clip: clip,
            below_text: arg.below_text,
            tr,
        });
    }

    /// Scale the image for the best fit in the given area.
    fn scale_to_fit(
        &self,
        img: (u32, u32),
        view: (u32, u32),
        mut scale: ImageScale,
        align: ImageAlign,
    ) -> Transform {
        use ImageAlign::*;
        use ImageScale::*;

        let (view_width, view_height) = (view.0 as f32, view.1 as f32);
        let (img_width, img_height) = (img.0 as f32, img.1 as f32);

        if scale == XY {
            if view_width * img_height / view_height > img_width {
                // horizontally
                scale = Y;
            } else {
                // vertically
                scale = X;
            }
        }

        if scale == Y {
            let w_scale = (view_width * img_height) / (view_height * img_width);
            let h_scale = 1.0f32;
            if align == Start {
                Transform::scale(w_scale, h_scale)
            } else if align == Center {
                Transform::scale(w_scale, h_scale)
                    .then_translate(Vector2D::new((1.0 - w_scale) / 2.0, 0.0))
            } else if align == End {
                Transform::scale(w_scale, h_scale).then_translate(Vector2D::new(1.0 - w_scale, 0.0))
            } else {
                unreachable!()
            }
        } else if scale == X {
            let w_scale = 1.0f32;
            let h_scale = (view_height * img_width) / (view_width * img_height);
            if align == Start {
                Transform::scale(w_scale, h_scale)
            } else if align == Center {
                Transform::scale(w_scale, h_scale)
                    .then_translate(Vector2D::new(0.0, (1.0 - h_scale) / 2.0))
            } else if align == End {
                Transform::scale(w_scale, h_scale).then_translate(Vector2D::new(0.0, 1.0 - h_scale))
            } else {
                unreachable!()
            }
        } else {
            unreachable!()
        }
    }
}
