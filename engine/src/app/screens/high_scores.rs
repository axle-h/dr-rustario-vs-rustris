//! Browses the high score tables, one page per table; ticked once per frame.
//! Left/right (or up/down) pages between tables and any other key leaves.

use crate::app::App;
use crate::frame_rate::FrameRate;
use crate::high_score::render::HighScoreRender;
use crate::high_score::table::{HighScoreStore, HighScoreTable};
use crate::high_score::HighScoreKey;
use crate::menu_input::{MenuInputContext, MenuInputKey};
use crate::particles::render::ParticleRender;
use sdl2::pixels::Color;
use sdl2::render::TextureCreator;
use sdl2::video::WindowContext;

pub struct HighScoreViewScreen<'a> {
    texture_creator: &'a TextureCreator<WindowContext>,
    tables: Vec<(String, String, HighScoreTable)>,
    index: usize,
    window_size: (u32, u32),
    view: HighScoreRender<'a>,
    inputs: MenuInputContext,
    frame_rate: FrameRate,
}

impl<'a> HighScoreViewScreen<'a> {
    /// The tables of `keys`, in that order, each at its defaults until a score is saved;
    /// any saved tables under other keys (e.g. from older versions) follow.
    /// `None` when there is nothing to show.
    pub fn new(
        app: &mut App,
        texture_creator: &'a TextureCreator<WindowContext>,
        keys: &[HighScoreKey],
        particles: &mut ParticleRender,
    ) -> Result<Option<Self>, String> {
        let inputs = MenuInputContext::new(app.config.input);
        let store = HighScoreStore::load()?;
        let mut tables: Vec<(String, String, HighScoreTable)> = keys
            .iter()
            .map(|key| (key.game.clone(), key.mode.clone(), store.table(key)))
            .collect();
        for (game, mode, table) in store.all() {
            if !tables.iter().any(|(g, m, _)| g == game && m == mode) {
                tables.push((game.to_string(), mode.to_string(), table.clone()));
            }
        }
        if tables.is_empty() {
            return Ok(None);
        }

        let window_size = app.canvas.window().size();
        let view = Self::build(texture_creator, &tables, window_size, 0)?;

        particles.clear();
        particles.add_source(app.fireworks_particle_source());

        app.menu_sound.play_high_score_music()?;
        Ok(Some(Self {
            texture_creator,
            tables,
            index: 0,
            window_size,
            view,
            inputs,
            frame_rate: FrameRate::new(),
        }))
    }

    fn build(
        texture_creator: &'a TextureCreator<WindowContext>,
        tables: &[(String, String, HighScoreTable)],
        window_size: (u32, u32),
        index: usize,
    ) -> Result<HighScoreRender<'a>, String> {
        let (game, mode, table) = &tables[index];
        let mut subtitle = format!("{}, {}", game, mode);
        if tables.len() > 1 {
            subtitle = format!("{} ({}/{})", subtitle, index + 1, tables.len());
        }
        HighScoreRender::new(
            table.clone(),
            texture_creator,
            window_size,
            None,
            Some(subtitle),
        )
    }

    /// One frame; `Ok(Some(()))` when the player leaves.
    pub fn update(
        &mut self,
        app: &mut App,
        particles: &mut ParticleRender,
    ) -> Result<Option<()>, String> {
        let delta = self.frame_rate.update()?;
        for key in self.inputs.parse(app.controllers.poll(&mut app.event_pump)) {
            match key {
                MenuInputKey::Left | MenuInputKey::Up if self.tables.len() > 1 => {
                    self.index = (self.index + self.tables.len() - 1) % self.tables.len();
                    self.view = Self::build(
                        self.texture_creator,
                        &self.tables,
                        self.window_size,
                        self.index,
                    )?;
                    app.menu_sound.play_chime()?;
                }
                MenuInputKey::Right | MenuInputKey::Down if self.tables.len() > 1 => {
                    self.index = (self.index + 1) % self.tables.len();
                    self.view = Self::build(
                        self.texture_creator,
                        &self.tables,
                        self.window_size,
                        self.index,
                    )?;
                    app.menu_sound.play_chime()?;
                }
                _ => return Ok(Some(())),
            }
        }
        app.canvas.set_draw_color(Color::BLACK);
        app.canvas.clear();

        // particles
        particles.update(delta);
        particles.draw(&mut app.canvas)?;

        self.view.draw(&mut app.canvas)?;

        app.canvas.present();
        Ok(None)
    }
}
