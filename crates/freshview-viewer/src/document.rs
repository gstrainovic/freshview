use std::path::Path;

use anyhow::{Context, Result};
use mupdf::{Colorspace, Matrix};

pub struct ViewerDocument {
    doc: mupdf::Document,
    pages: i32,
}

// mupdf::Document is not Sync, but it is Send. 
// We can use it in a background thread as long as we don't access it from multiple threads simultaneously.
unsafe impl Send for ViewerDocument {}

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

        // Limit zoom to prevent massive memory usage
        let zoom = zoom.clamp(0.1, 4.0);
        let mut scale = zoom * 2.0; 
        
        // Ensure we don't exceed a reasonable maximum resolution (e.g., 4096px in any dimension)
        let page_bounds = page.bounds().context("failed to get page bounds")?;
        let max_dim = page_bounds.width().max(page_bounds.height());
        if max_dim * scale > 4096.0 {
            scale = 4096.0 / max_dim;
        }

        let ctm = Matrix::new_scale(scale, scale);
        
        // Request RGBA directly if possible, or convert efficiently
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
            let mut rgba = vec![255u8; pixel_count * 4];
            
            // Fast copy RGB into RGBA buffer using chunks
            for i in 0..pixel_count {
                let src_idx = i * 3;
                let dst_idx = i * 4;
                rgba[dst_idx] = samples[src_idx];
                rgba[dst_idx + 1] = samples[src_idx + 1];
                rgba[dst_idx + 2] = samples[src_idx + 2];
                // Alpha is already 255 from vec! initialization
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
