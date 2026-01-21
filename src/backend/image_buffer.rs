use crate::CellBox;
use crate::backend::ImageHandle;
use euclid::Vector2D;
use raqote::Transform;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Positioning of the image relative to the text in the cells.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum ImageZ {
    BelowText,
    #[default]
    AboveText,
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
    FitVerticalStart,
    /// Fit the image to the area. It will be scaled vertically to
    /// make the image fit. The image will be clipped or the background
    /// will be visible.
    FitVerticalCenter,
    /// Fit the image to the area. It will be scaled vertically to
    /// make the image fit. The image will be clipped or the background
    /// will be visible.
    FitVerticalEnd,
}

///
/// An image-buffer that can be used in parallel to rendering the TUI.
///
#[derive(Debug, Default, Clone)]
pub struct ImageBuffer {
    // cell-size. this is updated whenever the font-size or font is changed.
    pub(super) cell_box: Arc<Mutex<CellBox>>,
    // information for all available images.
    pub(super) image_size: Arc<Mutex<HashMap<usize, (u32, u32)>>>,
    // actual render-queue. this will be read when flush() is called and
    // renders the images.
    // - image-id
    // - target rect (x,y,w,h)
    // - transform to access the image-texture
    pub(super) images: Arc<Mutex<Vec<(usize, (u32, u32, u32, u32), ImageZ, Transform)>>>,
}

impl ImageBuffer {
    /// Get the active FontBox
    pub fn cell_box(&self) -> CellBox {
        *self.cell_box.lock().expect("lock")
    }

    /// Get the image-size in px for an added image.
    pub fn image_size(&self, id: &ImageHandle) -> Option<(u32, u32)> {
        self.image_size.lock().expect("lock").get(&id.id).cloned()
    }

    /// Convert the ratatui Rect to a screen-area.
    ///
    /// This will not check if the area is inside the window bounds.
    pub fn rect_px(&self, area: ratatui_core::layout::Rect) -> (u32, u32, u32, u32) {
        let font_box = self.cell_box();

        (
            area.x as u32 * font_box.width,
            area.y as u32 * font_box.height,
            area.width as u32 * font_box.width,
            area.height as u32 * font_box.height,
        )
    }

    /// Render an image.
    ///
    /// To get an ImageHandle add the image first with [add_image]. Add image
    /// will create the texture for the image.
    pub fn render_image(
        &self,
        id: &ImageHandle,
        area: (u32, u32, u32, u32),
        z: ImageZ,
        fit: ImageFit,
    ) {
        let mut images = self.images.lock().expect("lock");
        let tr = match fit {
            ImageFit::Fill => Transform::default(),
            ImageFit::FitStart => {
                let img = self.image_size(id).expect("img1");
                self.scale_to_fit(img, (area.2, area.3), 0, 0)
            }
            ImageFit::FitCenter => {
                let img = self.image_size(id).expect("img1");
                self.scale_to_fit(img, (area.2, area.3), 0, 1)
            }
            ImageFit::FitEnd => {
                let img = self.image_size(id).expect("img1");
                self.scale_to_fit(img, (area.2, area.3), 0, 2)
            }
            ImageFit::HorizontalStart => {
                let img = self.image_size(id).expect("img1");
                self.scale_to_fit(img, (area.2, area.3), 1, 0)
            }
            ImageFit::HorizontalCenter => {
                let img = self.image_size(id).expect("img1");
                self.scale_to_fit(img, (area.2, area.3), 1, 1)
            }
            ImageFit::HorizontalEnd => {
                let img = self.image_size(id).expect("img1");
                self.scale_to_fit(img, (area.2, area.3), 1, 2)
            }
            ImageFit::FitVerticalStart => {
                let img = self.image_size(id).expect("img1");
                self.scale_to_fit(img, (area.2, area.3), 2, 0)
            }
            ImageFit::FitVerticalCenter => {
                let img = self.image_size(id).expect("img1");
                self.scale_to_fit(img, (area.2, area.3), 2, 1)
            }
            ImageFit::FitVerticalEnd => {
                let img = self.image_size(id).expect("img1");
                self.scale_to_fit(img, (area.2, area.3), 2, 2)
            }
        };
        images.push((id.id, area, z, tr));
    }

    /// Scale the image for the best fit in the given area.
    fn scale_to_fit(
        &self,
        img: (u32, u32),
        view: (u32, u32),
        mut scale: u8,
        align: u8,
    ) -> Transform {
        let (view_width, view_height) = (view.0 as f32, view.1 as f32);
        let (img_width, img_height) = (img.0 as f32, img.1 as f32);

        if scale == 0 {
            if view_width * img_height / view_height > img_width {
                // horizontally
                scale = 1;
            } else {
                // vertically
                scale = 2;
            }
        }

        if scale == 1 {
            let w_scale = (view_width * img_height) / (view_height * img_width);
            let h_scale = 1.0f32;
            if align == 0 {
                Transform::scale(w_scale, h_scale)
            } else if align == 1 {
                Transform::scale(w_scale, h_scale)
                    .then_translate(Vector2D::new((1.0 - w_scale) / 2.0, 0.0))
            } else if align == 2 {
                Transform::scale(w_scale, h_scale).then_translate(Vector2D::new(1.0 - w_scale, 0.0))
            } else {
                unreachable!()
            }
        } else if scale == 2 {
            let w_scale = 1.0f32;
            let h_scale = (view_height * img_width) / (view_width * img_height);
            if align == 0 {
                Transform::scale(w_scale, h_scale)
            } else if align == 1 {
                Transform::scale(w_scale, h_scale)
                    .then_translate(Vector2D::new(0.0, (1.0 - h_scale) / 2.0))
            } else if align == 2 {
                Transform::scale(w_scale, h_scale).then_translate(Vector2D::new(0.0, 1.0 - h_scale))
            } else {
                unreachable!()
            }
        } else {
            unreachable!()
        }
    }

    /// Render an image with a Transform.
    ///
    /// This transform will be applied to the UV vector to access the texture.
    ///
    /// To get an ImageHandle add the image first with [add_image]. Add image
    /// will create the texture for the image.
    pub fn render_image_tr(
        &self,
        id: &ImageHandle,
        area: (u32, u32, u32, u32),
        z: ImageZ,
        uv_transform: Transform,
    ) {
        let mut images = self.images.lock().expect("lock");
        images.push((id.id, area, z, uv_transform));
    }
}
