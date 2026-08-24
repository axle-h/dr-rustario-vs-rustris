use crate::high_score::table::{HighScore, HighScoreTable, Ranking};

use crate::font::{FontTexture, FontType};
use crate::high_score::event::HighScoreEntryEvent;
use crate::high_score::NewHighScore;
use sdl2::pixels::Color;
use sdl2::rect::{Point, Rect};
use sdl2::render::{BlendMode, Texture, TextureCreator, WindowCanvas};
use crate::font::Font;
use sdl2::video::WindowContext;
use std::cmp::min;
use sdl2::pixels::PixelFormatEnum::RGBA8888;

const NAME_CHARACTERS: usize = 5;
const CARET_HEIGHT: u32 = 2;
const FONT_COLOR: Color = Color::WHITE;

fn char_carets(font: &Font, text: &str) -> Result<Vec<Rect>, String> {
    let mut char_carets = vec![];
    for (i, char) in text.char_indices() {
        let x = if i == 0 {
            0
        } else {
            let (width, _) = font.size_of(&text[..i]);
            width + 1
        };
        let (width, height) = font.size_of_char(char);
        let rect = Rect::new(x as i32, height as i32 - 2, width - 1, CARET_HEIGHT);
        char_carets.push(rect);
    }
    Ok(char_carets)
}

struct HighScoreTableRow<'a> {
    ordinal: FontTexture<'a>,
    name: FontTexture<'a>,
    score: FontTexture<'a>,
}

impl<'a> HighScoreTableRow<'a> {
    fn new(
        font: &Font,
        texture_creator: &'a TextureCreator<WindowContext>,
        ordinal: &str,
        name: &str,
        score: &str,
    ) -> Result<Self, String> {
        Ok(Self {
            ordinal: FontTexture::from_string(font, texture_creator, ordinal, FONT_COLOR)?,
            name: FontTexture::from_string(font, texture_creator, name, FONT_COLOR)?,
            score: FontTexture::from_string(font, texture_creator, score, FONT_COLOR)?,
        })
    }

    fn height(&self) -> u32 {
        self.ordinal
            .height
            .max(self.name.height)
            .max(self.score.height)
    }
}

struct Entry {
    ordinal: usize,
    ranking: Ranking,
    high_score: NewHighScore,
    name: [char; NAME_CHARACTERS],
    current_char: usize,
    char_carets: Vec<Rect>,
}

impl Entry {
    fn new(
        ordinal: usize,
        ranking: Ranking,
        high_score: NewHighScore,
        font: &Font,
    ) -> Result<Self, String> {
        Ok(Self {
            ordinal,
            ranking,
            high_score,
            name: [' '; NAME_CHARACTERS],
            current_char: 0,
            char_carets: char_carets(font, &" ".repeat(NAME_CHARACTERS))?,
        })
    }

    fn update_carets(&mut self, font: &Font) -> Result<(), String> {
        self.char_carets = char_carets(font, &self.name())?;
        Ok(())
    }

    fn current_caret(&self) -> Rect {
        *self.char_carets.get(self.current_char).unwrap()
    }

    fn title_text(&self) -> String {
        format!(
            "{} Player {}",
            self.ranking.new_entry_title(),
            self.high_score.player + 1
        )
    }

    fn name(&self) -> String {
        self.name.iter().collect::<String>()
    }

    fn to_high_score(&self) -> Option<HighScore> {
        let name = self.name().trim().to_string();
        if name.is_empty() {
            None
        } else {
            Some(HighScore::from_string(name, self.high_score.score))
        }
    }

    fn up(&mut self) -> Option<HighScoreEntryEvent> {
        self.move_char(-1)
    }

    fn down(&mut self) -> Option<HighScoreEntryEvent> {
        self.move_char(1)
    }

    fn left(&mut self) -> Option<HighScoreEntryEvent> {
        if self.current_char > 0 {
            self.current_char -= 1;
            Some(HighScoreEntryEvent::CursorLeft)
        } else {
            None
        }
    }

    fn right(&mut self) -> Option<HighScoreEntryEvent> {
        self.current_char = min(self.current_char + 1, NAME_CHARACTERS);
        if self.current_char == NAME_CHARACTERS {
            Some(HighScoreEntryEvent::Finished)
        } else {
            Some(HighScoreEntryEvent::CursorRight)
        }
    }

    fn move_char(&mut self, value: i32) -> Option<HighScoreEntryEvent> {
        let current = self.name[self.current_char];
        let mut next = if current == ' ' {
            if value > 0 {
                'A'
            } else {
                'Z'
            }
        } else {
            char::from_u32((current as i32 + value) as u32).unwrap()
        };
        if next > 'Z' {
            next = 'A';
        } else if next < 'A' {
            next = 'Z';
        }
        self.name[self.current_char] = next;
        Some(HighScoreEntryEvent::ChangeChar)
    }
}

pub struct HighScoreRender<'a> {
    texture_creator: &'a TextureCreator<WindowContext>,
    rows: Vec<HighScoreTableRow<'a>>,
    texture: Texture<'a>,
    title_texture: Texture<'a>,
    title_rect: Rect,
    row_height: u32,
    ordinal_column_width: u32,
    padding: u32,
    width: u32,
    rect: Rect,
    entry: Option<Entry>,
    font: Font,
    subtitle: Option<(Texture<'a>, Rect)>,
}

/// TODO music
impl<'a> HighScoreRender<'a> {
    pub fn new(
        table: HighScoreTable,
        texture_creator: &'a TextureCreator<WindowContext>,
        (window_width, window_height): (u32, u32),
        new_high_score: Option<NewHighScore>,
        subtitle: Option<String>,
    ) -> Result<Self, String> {
        let font_size = window_width / 32;
        let font_header = FontType::Bold.load(font_size)?;
        let font_body = FontType::Mono.load(font_size)?;
        let font_title = FontType::Retro.load(window_width / 24)?;

        let ranking = table.ranking();
        let (table, entry) = if let Some(new_high_score) = new_high_score {
            let score_index = table
                .try_get_score_index(new_high_score.score)
                .expect("not a high score");
            let mut new_table = table;
            new_table.add_high_score(HighScore::new(
                &" ".repeat(NAME_CHARACTERS),
                new_high_score.score,
            ));
            (
                new_table,
                Some(Entry::new(score_index, ranking, new_high_score, &font_body)?),
            )
        } else {
            (table, None)
        };

        let mut rows = vec![HighScoreTableRow::new(
            &font_header,
            texture_creator,
            "#",
            "Name",
            ranking.label(),
        )?];
        for (i, row) in table.entries().iter().enumerate() {
            rows.push(HighScoreTableRow::new(
                &font_body,
                texture_creator,
                &(i + 1).to_string(),
                &row.name,
                &ranking.format(row.score),
            )?);
        }

        let n_rows = rows.len() as u32;
        if n_rows == 0 {
            return Err("no high scores".to_string());
        }

        let ordinal_column_width = rows.iter().map(|x| x.ordinal.width).max().unwrap();
        let name_column_width = rows.iter().map(|x| x.name.width).max().unwrap();
        let score_column_width = rows.iter().map(|x| x.score.width).max().unwrap();
        let padding = font_size / 2;
        let width =
            ordinal_column_width + padding + name_column_width + padding + score_column_width;
        // all rows will be same height as the tallest row
        let row_height = rows.iter().map(|r| r.height()).max().unwrap();
        let height = n_rows * row_height + (n_rows - 1) * padding;
        let mut texture = texture_creator
            .create_texture_target(RGBA8888, width, height)
            .map_err(|e| e.to_string())?;
        texture.set_blend_mode(BlendMode::Blend);
        let rect = Rect::from_center(
            Point::new(window_width as i32 / 2, window_height as i32 / 2),
            width,
            height,
        );

        let title_text = entry
            .as_ref()
            .map(|e| e.title_text())
            .unwrap_or(ranking.title().to_string());
        let title =
            FontTexture::from_string(&font_title, texture_creator, &title_text, FONT_COLOR)?;
        let title_rect = Rect::new(
            (window_width - title.width) as i32 / 2,
            padding as i32,
            title.width,
            title.height,
        );

        // which game and mode the table belongs to, under the title
        let subtitle = match subtitle {
            Some(text) => {
                let font_subtitle = FontType::Bold.load(window_width / 48)?;
                let rendered =
                    FontTexture::from_string(&font_subtitle, texture_creator, &text, FONT_COLOR)?;
                let rect = Rect::new(
                    (window_width - rendered.width) as i32 / 2,
                    title_rect.bottom() + padding as i32 / 2,
                    rendered.width,
                    rendered.height,
                );
                Some((rendered.texture, rect))
            }
            None => None,
        };

        Ok(Self {
            texture_creator,
            rows,
            texture,
            title_texture: title.texture,
            title_rect,
            row_height,
            width,
            padding,
            ordinal_column_width,
            rect,
            entry,
            font: font_body,
            subtitle,
        })
    }

    pub fn up(&mut self) -> Option<HighScoreEntryEvent> {
        self.update_entry_texture(|e| e.up())
    }

    pub fn down(&mut self) -> Option<HighScoreEntryEvent> {
        self.update_entry_texture(|e| e.down())
    }

    pub fn left(&mut self) -> Option<HighScoreEntryEvent> {
        self.update_entry_texture(|e| e.left())
    }

    pub fn right(&mut self) -> Option<HighScoreEntryEvent> {
        self.update_entry_texture(|e| e.right())
    }

    pub fn new_entry(&self) -> Option<HighScore> {
        self.entry
            .as_ref()
            .expect("no new high score")
            .to_high_score()
    }

    fn update_entry_texture<F: FnMut(&mut Entry) -> Option<HighScoreEntryEvent>>(
        &mut self,
        mut f: F,
    ) -> Option<HighScoreEntryEvent> {
        let entry = self.entry.as_mut().expect("no new high score");
        let result = f(entry);
        if result.is_some() {
            let row = self.rows.get_mut(entry.ordinal + 1).unwrap();
            row.name = FontTexture::from_string(
                &self.font,
                self.texture_creator,
                &entry.name(),
                FONT_COLOR,
            )
            .unwrap();
            entry.update_carets(&self.font).unwrap();
        }
        result
    }

    pub fn draw(&mut self, canvas: &mut WindowCanvas) -> Result<(), String> {
        canvas
            .with_texture_canvas(&mut self.texture, |c| {
                c.set_draw_color(Color::RGBA(0, 0, 0, 0));
                c.clear();

                let mut y = 0;
                for (i, row) in self.rows.iter().enumerate() {
                    c.copy(
                        &row.ordinal.texture,
                        None,
                        Rect::new(0, y, row.ordinal.width, row.ordinal.height),
                    )
                    .unwrap();

                    let name_rect = Rect::new(
                        (self.ordinal_column_width + self.padding) as i32,
                        y,
                        row.name.width,
                        row.name.height,
                    );
                    c.copy(&row.name.texture, None, name_rect).unwrap();

                    if let Some(entry) = self.entry.as_ref() {
                        if entry.ordinal + 1 == i {
                            c.set_draw_color(Color::RED);
                            let caret = entry.current_caret();
                            c.fill_rect(Rect::new(
                                name_rect.x() + caret.x(),
                                name_rect.y() + caret.y(),
                                caret.width(),
                                caret.height(),
                            ))
                            .unwrap();
                        }
                    }

                    c.copy(
                        &row.score.texture,
                        None,
                        Rect::new(
                            self.width as i32 - row.score.width as i32,
                            y,
                            row.score.width,
                            row.score.height,
                        ),
                    )
                    .unwrap();
                    y += self.row_height as i32;
                }
            })
            .map_err(|e| e.to_string())?;
        canvas.copy(&self.texture, None, self.rect)?;
        canvas.copy(&self.title_texture, None, self.title_rect)?;
        if let Some((texture, rect)) = &self.subtitle {
            canvas.copy(texture, None, *rect)?;
        }
        Ok(())
    }
}
