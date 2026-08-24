//! High score name entry, ticked once per frame; the entered name saves on
//! [`NameEntryExit::Done`] via [`NameEntryScreen::finish`].

use crate::app::App;
use crate::frame_rate::FrameRate;
use crate::high_score::event::HighScoreEntryEvent;
use crate::high_score::render::HighScoreRender;
use crate::high_score::table::HighScoreStore;
use crate::high_score::{HighScoreKey, NewHighScore};
use crate::menu_input::{MenuInputContext, MenuInputKey};
use crate::particles::render::ParticleRender;
use sdl2::pixels::Color;
use sdl2::render::TextureCreator;
use sdl2::video::WindowContext;

pub enum NameEntryExit {
    /// entry finished: save via [`NameEntryScreen::finish`]
    Done,
    /// entry abandoned: nothing to save
    Cancelled,
}

pub struct NameEntryScreen<'a> {
    store: HighScoreStore,
    key: HighScoreKey,
    table: HighScoreRender<'a>,
    inputs: MenuInputContext,
    frame_rate: FrameRate,
}

impl<'a> NameEntryScreen<'a> {
    pub fn new(
        app: &mut App,
        texture_creator: &'a TextureCreator<WindowContext>,
        key: &HighScoreKey,
        new_high_score: NewHighScore,
        particles: &mut ParticleRender,
    ) -> Result<Self, String> {
        let inputs = MenuInputContext::new(app.config.input);
        let store = HighScoreStore::load()?;

        let table = HighScoreRender::new(
            store.table(key),
            texture_creator,
            app.canvas.window().size(),
            Some(new_high_score),
            Some(key.title()),
        )?;

        particles.clear();
        particles.add_source(app.fireworks_particle_source());

        app.menu_sound.play_high_score_music()?;
        Ok(Self {
            store,
            key: key.clone(),
            table,
            inputs,
            frame_rate: FrameRate::new(),
        })
    }

    /// One frame; `Ok(Some(_))` when name entry is over.
    pub fn update(
        &mut self,
        app: &mut App,
        particles: &mut ParticleRender,
    ) -> Result<Option<NameEntryExit>, String> {
        let delta = self.frame_rate.update()?;

        for key in self.inputs.parse(app.controllers.poll(&mut app.event_pump)) {
            let event = match key {
                MenuInputKey::Up => self.table.up(),
                MenuInputKey::Down => self.table.down(),
                MenuInputKey::Left => self.table.left(),
                MenuInputKey::Right => self.table.right(),
                MenuInputKey::Start => return Ok(Some(NameEntryExit::Done)),
                MenuInputKey::Back => return Ok(Some(NameEntryExit::Cancelled)),
                _ => None,
            };
            if event.is_none() {
                continue;
            }
            match event.unwrap() {
                HighScoreEntryEvent::Finished => return Ok(Some(NameEntryExit::Done)),
                _ => app.menu_sound.play_chime()?,
            }
        }

        app.canvas.set_draw_color(Color::BLACK);
        app.canvas.clear();

        // particles
        particles.update(delta);
        particles.draw(&mut app.canvas)?;

        self.table.draw(&mut app.canvas)?;

        app.canvas.present();
        Ok(None)
    }

    /// saves the entered name, if any name was entered
    pub fn finish(mut self) -> Result<(), String> {
        if let Some(new_entry) = self.table.new_entry() {
            let mut high_scores = self.store.table(&self.key);
            high_scores.add_high_score(new_entry);
            self.store.set(&self.key, high_scores);
            self.store.save()
        } else {
            Ok(())
        }
    }
}
