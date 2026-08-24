use crate::font::{FontTexture, FontType};
use crate::game::{Game, MetricKind};
use num_format::{Locale, ToFormattedString};
use crate::render::helper::TextureFactory;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{BlendMode, Texture, TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;
use std::collections::HashMap;
use sdl2::pixels::PixelFormatEnum::RGBA8888;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum FontAlign {
    Left { zero_fill: bool },
    Right,
}

impl FontAlign {
    fn is_right(&self) -> bool {
        self == &FontAlign::Right
    }
}

/// Where a HUD number goes and how wide it may get. Values beyond `max_value` are shown in
/// hex if that fits the width, otherwise clamped.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct MetricSnips {
    point: Point,
    max_value: u32,
    max_chars: u32,
    align: FontAlign,
}

fn char_length(value: u32) -> u32 {
    format!("{}", value).len() as u32
}

/// the largest value `max_chars` digits of `base` can show
fn max_value(base: u32, max_chars: u32) -> u32 {
    (base as u64)
        .checked_pow(max_chars)
        .map(|v| (v - 1).min(u32::MAX as u64) as u32)
        .unwrap_or(u32::MAX)
}

impl MetricSnips {
    pub fn new<P: Into<Point>>(align: FontAlign, point: P, max_value: u32) -> Self {
        Self {
            point: point.into(),
            max_value,
            max_chars: char_length(max_value),
            align,
        }
    }

    pub fn zero_fill<P: Into<Point>>(point: P, max_value: u32) -> Self {
        Self::new(FontAlign::Left { zero_fill: true }, point, max_value)
    }

    pub fn left<P: Into<Point>>(point: P, max_value: u32) -> Self {
        Self::new(FontAlign::Left { zero_fill: false }, point, max_value)
    }

    /// right-aligned at `point`
    pub fn right<P: Into<Point>>(point: P, max_value: u32) -> Self {
        Self::new(FontAlign::Right, point, max_value)
    }

    /// sized by digit count rather than value
    pub fn chars<P: Into<Point>>(align: FontAlign, point: P, max_chars: u32) -> Self {
        Self {
            point: point.into(),
            max_value: max_value(10, max_chars),
            max_chars,
            align,
        }
    }

    pub fn offset(&self, x: i32, y: i32) -> Self {
        Self {
            point: self.point.offset(x, y),
            max_value: self.max_value,
            max_chars: self.max_chars,
            align: self.align,
        }
    }

    pub fn point(&self) -> Point {
        self.point
    }

    pub fn max_decimal_value(&self) -> u32 {
        self.max_value
    }

    pub fn max_hex_value(&self) -> u32 {
        max_value(16, self.max_chars)
    }

    fn zero_fill_chars(&self) -> Option<u32> {
        match self.align {
            FontAlign::Left { zero_fill } if zero_fill => Some(self.max_chars),
            _ => None,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct FontSprite {
    value: char,
    snip: Rect,
}

impl From<(char, Rect)> for FontSprite {
    fn from((value, snip): (char, Rect)) -> Self {
        Self { value, snip }
    }
}

impl FontSprite {
    pub fn new(value: char, snip: Rect) -> Self {
        Self { value, snip }
    }
}

pub enum FontRenderOptions {
    Sprites {
        file_bytes: &'static [u8],
        sprites: Vec<FontSprite>,
        spacing: u32,
    },
}

impl FontRenderOptions {
    pub fn build<'a>(
        &self,
        texture_creator: &'a TextureCreator<WindowContext>,
    ) -> Result<FontRender<'a>, String> {
        match self {
            FontRenderOptions::Sprites {
                file_bytes,
                sprites,
                spacing,
            } => FontRender::from_sprites(texture_creator, file_bytes, sprites.clone(), *spacing),
        }
    }

    pub fn numeric_sprites(
        file_bytes: &'static [u8],
        texture_creator: &TextureCreator<WindowContext>,
        spacing: u32,
    ) -> Result<Self, String> {
        let texture = texture_creator.load_texture_bytes(file_bytes)?;
        let query = texture.query();
        let width = query.width / 10;
        Ok(Self::Sprites {
            file_bytes,
            sprites: alpha_sprites(
                (0..10)
                    .map(|i| Point::new((i * width) as i32, 0))
                    .collect::<Vec<Point>>()
                    .try_into()
                    .unwrap(),
                width,
                query.height,
            ),
            spacing,
        })
    }
}

pub fn alpha_sprites(snips: [Point; 10], width: u32, height: u32) -> Vec<FontSprite> {
    snips
        .iter()
        .enumerate()
        .map(|(i, p)| {
            FontSprite::new(
                char::from_u32('0' as u32 + i as u32).unwrap(),
                Rect::new(p.x(), p.y(), width, height),
            )
        })
        .collect()
}

pub struct FontRender<'a> {
    texture: Texture<'a>,
    sprites: HashMap<char, Rect>,
    spacing: u32,
}

impl<'a> FontRender<'a> {
    pub fn from_sprites(
        texture_creator: &'a TextureCreator<WindowContext>,
        sprite_file: &'static [u8],
        sprites: Vec<FontSprite>,
        spacing: u32,
    ) -> Result<Self, String> {
        let mut texture = texture_creator.load_texture_bytes(sprite_file)?;
        texture.set_blend_mode(BlendMode::Blend);
        let sprites = sprites.iter().map(|&x| (x.value, x.snip)).collect();
        Ok(Self {
            texture,
            sprites,
            spacing,
        })
    }

    pub fn from_font(
        canvas: &mut WindowCanvas,
        texture_creator: &'a TextureCreator<WindowContext>,
        font_type: FontType,
        size: u32,
        color: Color,
    ) -> Result<Self, String> {
        let font = font_type.load(size)?;

        let chars = ('A'..='Z')
            .chain('a'..='z')
            .chain('0'..='9')
            .chain([' ', ',', '.', ':'])
            .map(|c| {
                (
                    c,
                    FontTexture::from_char(&font, texture_creator, c, color).unwrap(),
                )
            })
            .collect::<Vec<(char, FontTexture)>>();

        let width: u32 = chars.iter().map(|(_, t)| t.width).sum();
        let height = chars.iter().map(|(_, t)| t.height).max().unwrap();

        let mut texture = texture_creator
            .create_texture_target(RGBA8888, width, height)
            .map_err(|e| e.to_string())?;
        texture.set_blend_mode(BlendMode::Blend);

        let mut sprites = HashMap::new();
        let mut x = 0;
        canvas
            .with_texture_canvas(&mut texture, |c| {
                c.set_draw_color(Color::RGBA(0, 0, 0, 0));
                c.clear();
                for (ch, font_texture) in chars {
                    let snip = Rect::new(x, 0, font_texture.width, font_texture.height);
                    x += font_texture.width as i32;
                    c.copy(&font_texture.texture, None, snip).unwrap();
                    sprites.insert(ch, snip);
                }
            })
            .map_err(|e| e.to_string())?;

        Ok(Self {
            texture,
            sprites,
            spacing: 0,
        })
    }

    pub fn render_string(
        &self,
        canvas: &mut WindowCanvas,
        dest: Point,
        value: &str,
    ) -> Result<(), String> {
        let mut dest = dest;
        for ch in value.chars() {
            let snip = self.sprite(ch);
            let rect = Rect::new(dest.x(), dest.y(), snip.width(), snip.height());
            canvas.copy(&self.texture, snip, rect)?;
            dest += Point::new((snip.width() + self.spacing) as i32, 0);
        }
        Ok(())
    }

    pub fn render_string_in_center(
        &self,
        canvas: &mut WindowCanvas,
        dest: Rect,
        value: &str,
    ) -> Result<(), String> {
        let (width, height) = self.string_size(value);
        let rect = Rect::from_center(dest.center(), width, height);
        self.render_string(canvas, rect.top_left(), value)
    }

    pub fn render_number(
        &self,
        canvas: &mut WindowCanvas,
        meta: MetricSnips,
        value: u32,
    ) -> Result<(), String> {
        if meta.align.is_right() {
            self.render_number_right(canvas, value, meta)
        } else {
            self.render_number_left(canvas, value, meta)
        }
    }

    fn render_number_left(
        &self,
        canvas: &mut WindowCanvas,
        value: u32,
        meta: MetricSnips,
    ) -> Result<(), String> {
        let chars = self.format_number(value, meta, meta.zero_fill_chars());
        self.render_string(canvas, meta.point, &chars)
    }

    fn render_number_right(
        &self,
        canvas: &mut WindowCanvas,
        value: u32,
        meta: MetricSnips,
    ) -> Result<(), String> {
        let chars = self.format_number(value, meta, None);
        let mut dest = meta.point;
        for ch in chars.chars().rev() {
            let snip = self.sprite(ch);
            dest -= Point::new(snip.width() as i32, 0);
            let rect = Rect::new(dest.x(), dest.y(), snip.width(), snip.height());
            canvas.copy(&self.texture, snip, rect)?;
            dest -= Point::new(self.spacing as i32, 0);
        }
        Ok(())
    }

    pub fn number_size(&self, value: u32) -> (u32, u32) {
        let chars = self.format_number(value, MetricSnips::left((0, 0), u32::MAX), None);
        self.string_size(&chars)
    }

    pub fn string_size(&self, value: &str) -> (u32, u32) {
        if value.is_empty() {
            return (0, 0);
        }
        let sprites = value
            .chars()
            .map(|ch| self.sprite(ch))
            .collect::<Vec<Rect>>();
        let total_spacing = (value.len() - 1) as u32 * self.spacing;
        let width = sprites.iter().map(|s| s.width()).sum::<u32>();
        let height = sprites.iter().map(|s| s.height()).max().unwrap();
        (total_spacing + width, height)
    }

    fn sprite(&self, ch: char) -> Rect {
        *self
            .sprites
            .get(&ch)
            .unwrap_or_else(|| panic!("{} is not supported by this font render", ch))
    }

    fn format_number(&self, value: u32, meta: MetricSnips, zero_fill: Option<u32>) -> String {
        let chars = if value > meta.max_decimal_value() && self.sprites.contains_key(&'A') {
            // render in hex when the value breaches what the width can show in decimal
            format!("{:X}", value.min(meta.max_hex_value()))
        } else {
            let clamped = value.min(meta.max_decimal_value());
            if self.sprites.contains_key(&',') {
                clamped.to_formatted_string(&Locale::en)
            } else {
                format!("{}", clamped)
            }
        };
        if let Some(max_chars) = zero_fill {
            let fill_len = max_chars.saturating_sub(chars.len() as u32);
            let mut result: String = (0..fill_len).map(|_| '0').collect();
            result.push_str(&chars);
            result
        } else {
            chars
        }
    }
}

/// A HUD number: which font draws it and where.
#[derive(Clone, Copy, Debug)]
pub struct ThemedNumeric {
    font_index: usize,
    snips: MetricSnips,
}

impl ThemedNumeric {
    pub fn new(font_index: usize, snips: MetricSnips) -> Self {
        Self { font_index, snips }
    }
}

pub struct FontThemeOptions {
    fonts: Vec<FontRenderOptions>,
    metrics: Vec<(MetricKind, ThemedNumeric)>,
}

impl FontThemeOptions {
    pub fn new(fonts: Vec<FontRenderOptions>, metrics: Vec<(MetricKind, ThemedNumeric)>) -> Self {
        Self { fonts, metrics }
    }

    /// one font for every metric
    pub fn simple(font: FontRenderOptions, metrics: Vec<(MetricKind, MetricSnips)>) -> Self {
        Self::new(
            vec![font],
            metrics
                .into_iter()
                .map(|(kind, snips)| (kind, ThemedNumeric::new(0, snips)))
                .collect(),
        )
    }

    pub fn build<'a>(
        &self,
        texture_creator: &'a TextureCreator<WindowContext>,
    ) -> Result<FontTheme<'a>, String> {
        let mut fonts = vec![];
        for options in self.fonts.iter() {
            fonts.push(options.build(texture_creator)?);
        }
        Ok(FontTheme::new(fonts, self.metrics.clone()))
    }
}

/// Draws a game's HUD numbers.
pub struct FontTheme<'a> {
    fonts: Vec<FontRender<'a>>,
    metrics: Vec<(MetricKind, ThemedNumeric)>,
}

impl<'a> FontTheme<'a> {
    pub fn new(fonts: Vec<FontRender<'a>>, metrics: Vec<(MetricKind, ThemedNumeric)>) -> Self {
        Self { fonts, metrics }
    }

    pub fn render_all<G: Game>(&self, canvas: &mut WindowCanvas, game: &G) -> Result<(), String> {
        for (kind, numeric) in self.metrics.iter() {
            if let Some(value) = game.metric(*kind) {
                self.fonts[numeric.font_index].render_number(canvas, numeric.snips, value)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::max_value;

    #[test]
    fn max_value_for_small_widths() {
        assert_eq!(max_value(10, 3), 999);
        assert_eq!(max_value(16, 2), 0xFF);
    }

    #[test]
    fn max_value_saturates_instead_of_overflowing() {
        assert_eq!(max_value(10, 10), u32::MAX);
        assert_eq!(max_value(16, 8), u32::MAX);
    }
}
