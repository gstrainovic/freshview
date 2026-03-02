use std::path::Path;

use anyhow::{Context, Result};
use mupdf::{Colorspace, Matrix};

pub struct ViewerDocument {
    doc: mupdf::Document,
    pages: i32,
}

impl ViewerDocument {
    /// Open a PDF or image file
    pub fn open(path: &Path) -> Result<Self> {
        let doc = mupdf::Document::open(path.to_str().context("invalid path encoding")?)
            .context("failed to open document")?;
        let pages = doc.page_count().context("failed to get page count")?;
        Ok(Self { doc, pages })
    }

    /// Number of pages (1 for images)
    pub fn page_count(&self) -> i32 {
        self.pages
    }

    /// Render a page to RGBA bytes. Returns (rgba_bytes, width, height)
    pub fn render_page(&self, page_idx: i32, zoom: f32) -> Result<(Vec<u8>, u32, u32)> {
        let page = self
            .doc
            .load_page(page_idx)
            .context("failed to load page")?;

        let scale = zoom * 2.0;
        let ctm = Matrix::new_scale(scale, scale);
        let cs = Colorspace::device_rgb();

        let pixmap = page
            .to_pixmap(&ctm, &cs, false, true)
            .context("failed to render page to pixmap")?;

        let width = pixmap.width();
        let height = pixmap.height();
        let n = pixmap.n();
        let samples = pixmap.samples();

        let rgba = if n == 4 {
            // Already RGBA
            samples.to_vec()
        } else if n == 3 {
            // RGB -> convert to RGBA by appending alpha=255
            let pixel_count = (width * height) as usize;
            let mut rgba = Vec::with_capacity(pixel_count * 4);
            for pixel in samples.chunks_exact(3) {
                rgba.extend_from_slice(pixel);
                rgba.push(255);
            }
            rgba
        } else {
            anyhow::bail!(
                "unexpected pixel format: {} components per pixel",
                n
            );
        };

        Ok((rgba, width, height))
    }
}
