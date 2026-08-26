//! A menu, ticked once per frame until the player starts, backs out or quits.

use crate::app::{App, MenuExit, MenuMusic};
use crate::frame_rate::FrameRate;
use crate::menu::{Menu, MenuItem};
use crate::menu_input::{MenuInputContext, MenuInputKey};
use crate::particles::prescribed::RaceTheme;
use crate::particles::render::ParticleRender;
use sdl2::pixels::Color;
use sdl2::render::TextureCreator;
use sdl2::video::WindowContext;

pub struct MenuScreen<'a> {
    menu: Menu<'a>,
    inputs: MenuInputContext,
    frame_rate: FrameRate,
}

impl<'a> MenuScreen<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        app: &mut App,
        texture_creator: &'a TextureCreator<WindowContext>,
        items: Vec<MenuItem>,
        title: String,
        subtitle: Option<String>,
        music: MenuMusic,
        particles: &mut ParticleRender,
        race: &[RaceTheme],
    ) -> Result<Self, String> {
        let inputs = MenuInputContext::new(app.config.input);
        let menu = Menu::new(items, &mut app.canvas, texture_creator, title, subtitle)?;

        particles.clear();
        particles.add_source(app.race_particle_source(race));

        match music {
            MenuMusic::Title => app.menu_sound.play_title_music()?,
            MenuMusic::Menu => app.menu_sound.play_menu_music()?,
        }
        Ok(Self {
            menu,
            inputs,
            frame_rate: FrameRate::new(),
        })
    }

    /// Refresh the values a select list offers, for a list whose options depend on
    /// another item. Call it after [`Self::update`] rather than from its callback: the
    /// menu is borrowed for the frame.
    pub fn set_items(&mut self, app: &mut App, items: &[MenuItem]) -> Result<(), String> {
        self.menu.set_items(&mut app.canvas, items)
    }

    /// One frame: input, update, render, present. `on_select` sees every change of a list
    /// item and every selected item by name and may end the menu with a value.
    pub fn update<R>(
        &mut self,
        app: &mut App,
        particles: &mut ParticleRender,
        mut on_select: impl FnMut(&str, &str) -> Option<MenuExit<R>>,
    ) -> Result<Option<MenuExit<R>>, String> {
        let delta = self.frame_rate.update()?;
        for key in self
            .inputs
            .parse(app.controllers.poll(&mut app.event_pump))
            .into_iter()
        {
            if key == MenuInputKey::Quit {
                return Ok(Some(MenuExit::Quit));
            }

            match self.menu.read_key(key) {
                None => match key {
                    MenuInputKey::Start => {
                        app.menu_sound.play_select()?;
                        return Ok(Some(MenuExit::Start));
                    }
                    MenuInputKey::Back => return Ok(Some(MenuExit::Back)),
                    _ => {}
                },
                Some((name, action)) => {
                    if let Some(exit) = on_select(name, action) {
                        if !matches!(exit, MenuExit::Back) {
                            app.menu_sound.play_select()?;
                        }
                        return Ok(Some(exit));
                    }
                }
            }
            app.menu_sound.play_chime()?;
        }

        app.canvas.set_draw_color(Color::BLACK);
        app.canvas.clear();

        // particles
        particles.update(delta);
        particles.draw(&mut app.canvas)?;

        // menu
        self.menu.draw(&mut app.canvas)?;

        app.canvas.present();
        Ok(None)
    }
}
