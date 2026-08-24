use ab_glyph::{point, Font as _, FontRef, Glyph, GlyphId, PxScale, PxScaleFont, ScaleFont};
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::render::{BlendMode, Texture, TextureCreator};
use sdl2::video::WindowContext;


const FONT_ROBOTO_REGULAR: &[u8] = include_bytes!("Roboto-Regular.ttf");
const FONT_ROBOTO_BOLD: &[u8] = include_bytes!("Roboto-Bold.ttf");
const FONT_ROBOTO_MONO_REGULAR: &[u8] = include_bytes!("RobotoMono-Regular.ttf");

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FontType {
    Normal,
    Bold,
    Mono,
    Retro,
}

impl FontType {
    /// Loads this face at `size` (em height in pixels, same semantics as SDL_ttf's point size).
    pub fn load(&self, size: u32) -> Result<Font, String> {
        Font::from_bytes(self.bytes(), size)
    }

    fn bytes(&self) -> &'static [u8] {
        match self {
            FontType::Normal => FONT_ROBOTO_REGULAR,
            FontType::Bold => FONT_ROBOTO_BOLD,
            FontType::Mono => FONT_ROBOTO_MONO_REGULAR,
            #[cfg(not(feature = "portmaster"))]
            FontType::Retro => include_bytes!("Handjet.ttf"),
            #[cfg(feature = "portmaster")]
            FontType::Retro => FONT_ROBOTO_REGULAR,
        }
    }
}

/// A TrueType face scaled to a fixed pixel size.
pub struct Font {
    scaled: PxScaleFont<FontRef<'static>>,
}

/// Positioned glyphs for a line of text plus the size of the bitmap that fits them.
struct Line {
    glyphs: Vec<Glyph>,
    width: u32,
    height: u32,
}

impl Font {
    pub fn from_bytes(bytes: &'static [u8], size: u32) -> Result<Self, String> {
        let font = FontRef::try_from_slice(bytes).map_err(|e| e.to_string())?;
        // SDL_ttf treats the point size as the em height in pixels (72 dpi); ab_glyph's
        // PxScale is the ascent-to-descent height, so convert via the face's unscaled metrics.
        let units_per_em = font
            .units_per_em()
            .ok_or_else(|| "font has no units per em".to_string())?;
        let scale = PxScale::from(size as f32 * font.height_unscaled() / units_per_em);
        Ok(Self {
            scaled: font.into_scaled(scale),
        })
    }

    /// Line height in pixels (ascent + descent + line gap), like `TTF_FontHeight`.
    pub fn height(&self) -> u32 {
        let s = &self.scaled;
        (s.ascent() - s.descent() + s.line_gap()).ceil() as u32
    }

    /// Pixel size of `text` when rendered: (width, height). Empty text has zero width.
    pub fn size_of(&self, text: &str) -> (u32, u32) {
        let line = self.layout(text);
        (line.width, line.height)
    }

    pub fn size_of_char(&self, ch: char) -> (u32, u32) {
        self.size_of(&ch.to_string())
    }

    fn layout(&self, text: &str) -> Line {
        let s = &self.scaled;
        let height = self.height();
        let baseline = s.ascent().ceil();
        let mut glyphs = Vec::with_capacity(text.len());
        let mut caret = 0.0f32;
        let mut previous: Option<GlyphId> = None;
        for ch in text.chars() {
            let id = s.glyph_id(ch);
            if let Some(prev) = previous {
                caret += s.kern(prev, id);
            }
            glyphs.push(id.with_scale_and_position(s.scale(), point(caret, baseline)));
            caret += s.h_advance(id);
            previous = Some(id);
        }

        // Glyph outlines may overhang the advance box on either side; shift so nothing is clipped.
        let bounds: Vec<_> = glyphs
            .iter()
            .filter_map(|g| s.outline_glyph(g.clone()).map(|o| o.px_bounds()))
            .collect();
        let min_x = bounds.iter().map(|b| b.min.x).fold(0.0f32, f32::min).floor();
        let max_x = bounds.iter().map(|b| b.max.x).fold(caret, f32::max).ceil();
        if min_x < 0.0 {
            for g in &mut glyphs {
                g.position.x -= min_x;
            }
        }

        Line {
            glyphs,
            width: (max_x - min_x).max(0.0) as u32,
            height,
        }
    }

    /// Anti-aliased render of `text` into straight-alpha RGBA bytes, like `TTF_RenderUTF8_Blended`.
    pub(crate) fn render_rgba(&self, text: &str, color: Color) -> (u32, u32, Vec<u8>) {
        let line = self.layout(text);
        let (width, height) = (line.width.max(1), line.height.max(1));
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        let (r, g, b, a) = color.rgba();
        for glyph in line.glyphs {
            let Some(outline) = self.scaled.outline_glyph(glyph) else {
                continue;
            };
            let bounds = outline.px_bounds();
            outline.draw(|x, y, coverage| {
                let px = bounds.min.x as i32 + x as i32;
                let py = bounds.min.y as i32 + y as i32;
                if px < 0 || py < 0 || px >= width as i32 || py >= height as i32 {
                    return;
                }
                let i = ((py as u32 * width + px as u32) * 4) as usize;
                let alpha = (coverage.clamp(0.0, 1.0) * a as f32).round() as u8;
                if alpha > pixels[i + 3] {
                    pixels[i] = r;
                    pixels[i + 1] = g;
                    pixels[i + 2] = b;
                    pixels[i + 3] = alpha;
                }
            });
        }
        (width, height, pixels)
    }
}

pub struct FontTexture<'a> {
    pub texture: Texture<'a>,
    pub width: u32,
    pub height: u32,
}

impl<'a> FontTexture<'a> {
    pub fn from_string(
        font: &Font,
        texture_creator: &'a TextureCreator<WindowContext>,
        text: &str,
        color: Color,
    ) -> Result<Self, String> {
        let (width, height, pixels) = font.render_rgba(text, color);
        let mut texture = texture_creator
            .create_texture_static(PixelFormatEnum::RGBA32, width, height)
            .map_err(|e| e.to_string())?;
        texture
            .update(None, &pixels, width as usize * 4)
            .map_err(|e| e.to_string())?;
        texture.set_blend_mode(BlendMode::Blend);

        Ok(Self {
            texture,
            width,
            height,
        })
    }

    pub fn from_char(
        font: &Font,
        texture_creator: &'a TextureCreator<WindowContext>,
        ch: char,
        color: Color,
    ) -> Result<Self, String> {
        Self::from_string(font, texture_creator, &ch.to_string(), color)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn height_matches_sdl_ttf_line_height() {
        // Values observed from TTF_FontHeight for the same faces and sizes.
        assert_eq!(FontType::Normal.load(40).unwrap().height(), 47);
        assert_eq!(FontType::Normal.load(80).unwrap().height(), 94);
    }

    #[test]
    fn empty_text_has_zero_width_and_font_height() {
        let font = FontType::Mono.load(32).unwrap();
        assert_eq!(font.size_of(""), (0, font.height()));
    }

    #[test]
    fn mono_font_widths_are_additive() {
        let font = FontType::Mono.load(32).unwrap();
        let (one, _) = font.size_of_char('A');
        let (three, h) = font.size_of("AAA");
        assert_eq!(h, font.height());
        assert!(one > 0);
        assert!((three as i64 - 3 * one as i64).abs() <= 2, "{three} vs 3*{one}");
    }

    #[test]
    fn render_has_opaque_pixels_only_inside_the_bitmap() {
        let font = FontType::Bold.load(24).unwrap();
        let (w, h, px) = font.render_rgba("Hi", Color::RGB(10, 20, 30));
        assert_eq!(px.len(), (w * h * 4) as usize);
        let opaque = px.chunks(4).filter(|p| p[3] == 255).count();
        assert!(opaque > 0);
        assert!(px.chunks(4).filter(|p| p[3] > 0).all(|p| p[..3] == [10, 20, 30]));
    }
}
