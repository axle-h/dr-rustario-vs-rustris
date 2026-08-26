use sdl2::pixels::PixelFormatEnum::{self, RGBA8888};

const RGBA32: PixelFormatEnum = PixelFormatEnum::RGBA32;
use sdl2::render::{BlendMode, Texture, TextureCreator};
use sdl2::video::WindowContext;

pub trait TextureQuery {
    fn size(&self) -> (u32, u32);
}

impl TextureQuery for Texture<'_> {
    fn size(&self) -> (u32, u32) {
        let query = self.query();
        (query.width, query.height)
    }
}

/// Decodes an embedded PNG into tightly packed RGBA bytes.
pub fn decode_png(buf: &[u8]) -> Result<image::RgbaImage, String> {
    image::load_from_memory_with_format(buf, image::ImageFormat::Png)
        .map(|img| img.into_rgba8())
        .map_err(|e| e.to_string())
}

pub trait TextureFactory {
    fn create_texture_target_blended(&self, width: u32, height: u32)
        -> Result<Texture<'_>, String>;
    /// Loads a PNG into a static texture with alpha blending enabled.
    fn load_texture_bytes(&self, buf: &[u8]) -> Result<Texture<'_>, String>;
    fn load_texture_bytes_blended(&self, buf: &[u8]) -> Result<Texture<'_>, String>;
}

impl TextureFactory for TextureCreator<WindowContext> {
    fn create_texture_target_blended(
        &self,
        width: u32,
        height: u32,
    ) -> Result<Texture<'_>, String> {
        let mut texture = self
            .create_texture_target(RGBA8888, width, height)
            .map_err(|e| e.to_string())?;
        texture.set_blend_mode(BlendMode::Blend);
        Ok(texture)
    }

    fn load_texture_bytes(&self, buf: &[u8]) -> Result<Texture<'_>, String> {
        let image = decode_png(buf)?;
        let (width, height) = image.dimensions();
        let mut texture = self
            .create_texture_static(RGBA32, width, height)
            .map_err(|e| e.to_string())?;
        texture
            .update(None, image.as_raw(), width as usize * 4)
            .map_err(|e| e.to_string())?;
        texture.set_blend_mode(BlendMode::Blend);
        Ok(texture)
    }

    fn load_texture_bytes_blended(&self, buf: &[u8]) -> Result<Texture<'_>, String> {
        self.load_texture_bytes(buf)
    }
}
