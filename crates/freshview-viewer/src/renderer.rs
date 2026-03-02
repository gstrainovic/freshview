use egui::{ColorImage, TextureHandle, TextureOptions};

/// Convert raw RGBA pixel data into an egui texture handle.
pub fn rgba_to_texture(
    ctx: &egui::Context,
    name: &str,
    rgba: &[u8],
    width: u32,
    height: u32,
) -> TextureHandle {
    let image = ColorImage::from_rgba_unmultiplied([width as usize, height as usize], rgba);
    ctx.load_texture(name, image, TextureOptions::LINEAR)
}
