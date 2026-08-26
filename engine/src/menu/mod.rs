pub mod sound;

use crate::draw::CanvasExt;
use crate::font::{FontTexture, FontType};
use crate::menu_input::MenuInputKey;

use crate::app_info;
use crate::font::Font;
use crate::render::helper::TextureFactory;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Texture, TextureCreator, WindowCanvas};
use sdl2::video::WindowContext;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MenuAction {
    Select,
    SelectList { items: Vec<String>, current: usize },
}

impl MenuAction {
    fn is_select(&self) -> bool {
        self == &Self::Select
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MenuItem {
    name: String,
    action: MenuAction,
}

impl MenuItem {
    pub fn select(name: &str) -> Self {
        Self {
            name: name.to_string(),
            action: MenuAction::Select,
        }
    }

    pub fn select_list(name: &str, items: Vec<String>, current: usize) -> Self {
        Self {
            name: name.to_string(),
            action: MenuAction::SelectList { items, current },
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn with_prefix(self, prefix: &str) -> Self {
        Self {
            name: format!("{}{}", prefix, self.name),
            action: self.action,
        }
    }
}

struct MenuRow<'a> {
    item: MenuItem,
    name_texture: Texture<'a>,
    name_height: u32,
    name_width: u32,
    selected_texture: Texture<'a>,
    action_textures: Vec<FontTexture<'a>>,
}

impl<'a> MenuRow<'a> {
    fn new(
        canvas: &mut WindowCanvas,
        texture_creator: &'a TextureCreator<WindowContext>,
        font: &Font,
        item: MenuItem,
    ) -> Result<Self, String> {
        let name_texture = Self::name_texture(
            canvas,
            texture_creator,
            font,
            &item.name,
            Color::WHITE,
            None,
        )?;
        let selected_texture = Self::name_texture(
            canvas,
            texture_creator,
            font,
            &item.name,
            Color::BLACK,
            Some(Color::WHITE),
        )?;

        let mut action_textures = vec![];
        if let MenuAction::SelectList { items, current: _ } = &item.action {
            for text in items {
                action_textures.push(FontTexture::from_string(
                    font,
                    texture_creator,
                    text,
                    Color::WHITE,
                )?);
            }
        }

        let name_query = name_texture.query();
        Ok(Self {
            item,
            name_texture,
            name_width: name_query.width,
            name_height: name_query.height,
            selected_texture,
            action_textures,
        })
    }

    fn max_action_width(&self) -> u32 {
        self.action_textures
            .iter()
            .map(|a| a.width)
            .max()
            .unwrap_or(0)
    }

    fn current_action_id(&self) -> Option<usize> {
        match &self.item.action {
            MenuAction::Select => None,
            MenuAction::SelectList { current, .. } => Some(*current),
        }
    }

    fn name_texture(
        canvas: &mut WindowCanvas,
        texture_creator: &'a TextureCreator<WindowContext>,
        font: &Font,
        name: &str,
        text_color: Color,
        background_color: Option<Color>,
    ) -> Result<Texture<'a>, String> {
        let name_text = FontTexture::from_string(font, texture_creator, name, text_color)?;
        let rect = Rect::new(
            0,
            0,
            name_text.width + font.height() as u32,
            font.height() as u32 + 10,
        );
        let mut texture =
            texture_creator.create_texture_target_blended(rect.width(), rect.height())?;
        canvas
            .with_texture_canvas(&mut texture, |c| {
                c.set_draw_color(Color::RGBA(0, 0, 0, 0));
                c.clear();
                if let Some(background_color) = background_color {
                    c.fill_rounded_rect(rect, rect.height() / 2, background_color)
                        .unwrap();
                }
                c.copy(
                    &name_text.texture,
                    None,
                    Rect::from_center(rect.center(), name_text.width, name_text.height),
                )
                .unwrap();
            })
            .map_err(|e| e.to_string())?;
        Ok(texture)
    }
}

struct SnippedTexture<'a> {
    texture: Texture<'a>,
    snip: Rect,
}

impl<'a> SnippedTexture<'a> {
    pub fn new(texture: Texture<'a>, snip: Rect) -> Self {
        Self { texture, snip }
    }
}

/// The body layout: where the rows sit, the texture they are drawn into, and the pill
/// drawn behind the selected list. Recomputed whenever a row's values change, since the
/// widest value is what the body is sized around.
struct Body<'a> {
    row_rects: Vec<Rect>,
    /// the rows are drawn into this, then it is drawn to the window
    target: SnippedTexture<'a>,
    select_list_background: Texture<'a>,
}

impl<'a> Body<'a> {
    fn new(
        rows: &[MenuRow<'a>],
        canvas: &mut WindowCanvas,
        texture_creator: &'a TextureCreator<WindowContext>,
        vertical_gutter: u32,
        horizontal_gutter: u32,
    ) -> Result<Self, String> {
        let (window_width, window_height) = canvas.window().size();

        let row_height = rows.iter().map(|r| r.name_height).max().unwrap();
        let body_height =
            row_height * rows.len() as u32 + vertical_gutter * (rows.len() - 1) as u32;

        let name_width = rows.iter().map(|r| r.name_width).max().unwrap();
        let max_action_width = rows.iter().map(|r| r.max_action_width()).max().unwrap();
        // + body height as buffer for select list bg
        let body_width = name_width + horizontal_gutter + max_action_width + row_height / 2;

        let body_rect = Rect::from_center(
            Rect::new(0, 0, window_width, window_height).center(),
            body_width,
            body_height,
        );

        let mut row_rects = vec![];
        let mut y = 0;
        for _ in 0..rows.len() {
            row_rects.push(Rect::new(0, y, body_width, row_height));
            y += row_height as i32 + vertical_gutter as i32;
        }

        let body_texture =
            texture_creator.create_texture_target_blended(body_width, body_height)?;

        let mut select_list_background =
            texture_creator.create_texture_target_blended(body_width, row_height)?;
        canvas
            .with_texture_canvas(&mut select_list_background, |c| {
                c.set_draw_color(Color::RGBA(0, 0, 0, 0));
                c.clear();

                let rect = Rect::new(0, 0, body_width, row_height);
                c.fill_rounded_rect(rect, rect.height() / 2, Color::RGBA(0xff, 0xff, 0xff, 0x80))
                    .unwrap();
            })
            .map_err(|e| e.to_string())?;

        Ok(Self {
            row_rects,
            target: SnippedTexture::new(body_texture, body_rect),
            select_list_background,
        })
    }
}

pub struct Menu<'a> {
    rows: Vec<MenuRow<'a>>,
    current_row_id: usize,
    title: SnippedTexture<'a>,
    subtitle: Option<SnippedTexture<'a>>,
    watermark: SnippedTexture<'a>,
    body: Body<'a>,
    texture_creator: &'a TextureCreator<WindowContext>,
    font: Font,
    vertical_gutter: u32,
    horizontal_gutter: u32,
}

impl<'a> Menu<'a> {
    pub fn new<ST: Into<Option<String>>>(
        menu_items: Vec<MenuItem>,
        canvas: &mut WindowCanvas,
        texture_creator: &'a TextureCreator<WindowContext>,
        title_text: String,
        subtitle_text: ST,
    ) -> Result<Self, String> {
        assert!(!menu_items.is_empty());

        let (window_width, window_height) = canvas.window().size();
        let font_size = window_width / 32;
        let font = FontType::Retro.load(font_size)?;

        let vertical_gutter = font_size / 3;
        let horizontal_gutter = font_size * 2;

        let rows: Vec<MenuRow> = menu_items
            .into_iter()
            .map(|mi| MenuRow::new(canvas, texture_creator, &font, mi).unwrap())
            .collect();

        let body = Body::new(
            &rows,
            canvas,
            texture_creator,
            vertical_gutter,
            horizontal_gutter,
        )?;

        let watermark_font_size = 3 * font_size / 5;
        let watermark_font = FontType::Retro.load(watermark_font_size)?;
        let watermark = format!(
            "{} v{} by {}",
            app_info::get().name,
            app_info::get().version,
            app_info::get().authors
        );
        let watermark_texture =
            FontTexture::from_string(&watermark_font, texture_creator, &watermark, Color::WHITE)?;
        let watermark_rect = Rect::new(
            (window_width - watermark_texture.width - watermark_font_size) as i32,
            (window_height - watermark_texture.height - watermark_font_size) as i32,
            watermark_texture.width,
            watermark_texture.height,
        );

        let title_font_size = window_width / 24;
        let title_font = FontType::Retro.load(title_font_size)?;
        let title_texture =
            FontTexture::from_string(&title_font, texture_creator, &title_text, Color::WHITE)?;
        let title_rect = Rect::from_center(
            Rect::new(
                0,
                vertical_gutter as i32,
                window_width,
                title_texture.height,
            )
            .center(),
            title_texture.width,
            title_texture.height,
        );

        let subtitle = subtitle_text.into().map(|text| {
            let texture =
                FontTexture::from_string(&font, texture_creator, &text, Color::WHITE).unwrap();
            let rect = Rect::from_center(
                Rect::new(
                    0,
                    title_texture.height as i32 + vertical_gutter as i32,
                    window_width,
                    texture.height,
                )
                .center(),
                texture.width,
                texture.height,
            );
            SnippedTexture::new(texture.texture, rect)
        });

        Ok(Self {
            rows,
            current_row_id: 0,
            title: SnippedTexture::new(title_texture.texture, title_rect),
            subtitle,
            watermark: SnippedTexture::new(watermark_texture.texture, watermark_rect),
            body,
            texture_creator,
            font,
            vertical_gutter,
            horizontal_gutter,
        })
    }

    /// Replace the values a select list offers, keeping the selection the caller asks for.
    /// The menu is otherwise fixed: this is for a list whose options depend on another
    /// item, such as the modes on offer once a single theme is picked.
    pub fn set_items(
        &mut self,
        canvas: &mut WindowCanvas,
        items: &[MenuItem],
    ) -> Result<(), String> {
        let mut changed = false;
        for item in items {
            let Some(row) = self.rows.iter_mut().find(|r| r.item.name == item.name) else {
                continue;
            };
            if row.item.action == item.action {
                continue;
            }
            *row = MenuRow::new(canvas, self.texture_creator, &self.font, item.clone())?;
            changed = true;
        }
        if changed {
            // the widest value on offer sizes the body, so the whole layout moves with it
            self.body = Body::new(
                &self.rows,
                canvas,
                self.texture_creator,
                self.vertical_gutter,
                self.horizontal_gutter,
            )?;
        }
        Ok(())
    }

    pub fn up(&mut self) {
        self.current_row_id = match self.current_row_id {
            0 => self.rows.len() - 1,
            _ => self.current_row_id - 1,
        };
    }

    pub fn down(&mut self) {
        self.current_row_id += 1;
        self.current_row_id %= self.rows.len();
    }

    pub fn left(&mut self) -> Option<(&str, &str)> {
        self.direction(-1)
    }

    pub fn right(&mut self) -> Option<(&str, &str)> {
        self.direction(1)
    }

    pub fn read_key(&mut self, key: MenuInputKey) -> Option<(&str, &str)> {
        match key {
            MenuInputKey::Up => {
                self.up();
                None
            }
            MenuInputKey::Down => {
                self.down();
                None
            }
            MenuInputKey::Left => self.left(),
            MenuInputKey::Right => self.right(),
            MenuInputKey::Select => self.select(),
            // special case for pressing "start" on an action e.g. "quit" I would expect it to quit
            MenuInputKey::Start if self.rows[self.current_row_id].item.action.is_select() => {
                self.select()
            }
            _ => None,
        }
    }

    fn direction(&mut self, direction: i32) -> Option<(&str, &str)> {
        let row = self.rows.get_mut(self.current_row_id).unwrap();
        let result = match &mut row.item.action {
            MenuAction::SelectList { items, current } => {
                let current_plus = *current as i32 + direction;
                *current = if current_plus < 0 {
                    items.len() - 1
                } else {
                    (current_plus as usize) % items.len()
                };
                Some(&items[*current])
            }
            _ => None,
        };

        result.map(|r| (&row.item.name as &str, r as &str))
    }

    pub fn select(&mut self) -> Option<(&str, &str)> {
        let row = self.rows.get_mut(self.current_row_id).unwrap();
        let result = match &mut row.item.action {
            MenuAction::Select => "",
            MenuAction::SelectList { items, current } => {
                *current = (*current + 1) % items.len();
                &items[*current]
            }
        };
        Some((&row.item.name, result))
    }

    pub fn draw(&mut self, canvas: &mut WindowCanvas) -> Result<(), String> {
        canvas.copy(&self.title.texture, None, self.title.snip)?;

        if let Some(subtitle) = self.subtitle.as_ref() {
            canvas.copy(&subtitle.texture, None, subtitle.snip)?;
        }

        canvas
            .with_texture_canvas(&mut self.body.target.texture, |tc| {
                tc.set_draw_color(Color::RGBA(0, 0, 0, 0));
                tc.clear();

                for (row_id, (row, row_rect)) in
                    self.rows.iter().zip(self.body.row_rects.iter()).enumerate()
                {
                    let is_selected = row_id == self.current_row_id;

                    // draw select list background
                    if is_selected && !row.item.action.is_select() {
                        tc.copy(&self.body.select_list_background, None, *row_rect)
                            .unwrap();
                    }

                    // draw name
                    let name_rect =
                        Rect::new(row_rect.x, row_rect.y, row.name_width, row.name_height);
                    let name_texture = if is_selected {
                        &row.selected_texture
                    } else {
                        &row.name_texture
                    };
                    tc.copy(name_texture, None, name_rect).unwrap();

                    // draw value
                    if let Some(current_action) = row.current_action_id() {
                        let texture = &row.action_textures[current_action];
                        // right-aligned at its natural size (stretching to the row distorts
                        // the text), inset half a row height for the rounded bg
                        let rect = Rect::new(
                            row_rect.right() - texture.width as i32 - row_rect.height() as i32 / 2,
                            row_rect.y() + (row_rect.height() as i32 - texture.height as i32) / 2,
                            texture.width,
                            texture.height,
                        );
                        tc.copy(&texture.texture, None, rect).unwrap();
                    }
                }
            })
            .map_err(|e| e.to_string())?;

        canvas.copy(&self.body.target.texture, None, self.body.target.snip)?;
        canvas.copy(&self.watermark.texture, None, self.watermark.snip)
    }
}
