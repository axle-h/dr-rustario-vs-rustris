use crate::config::InputConfig;
use crate::controller::{InputEvent, PadButton};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuInputKey {
    Up,
    Down,
    Left,
    Right,
    Start,
    Select,
    Back,
    Quit,
}

pub struct MenuInputContext {
    mapping: HashMap<Keycode, MenuInputKey>,
}

impl MenuInputContext {
    pub fn new(config: InputConfig) -> Self {
        Self {
            mapping: config.menu_map(),
        }
    }

    pub fn parse<I>(&self, sdl_events: I) -> Vec<MenuInputKey>
    where
        I: IntoIterator<Item = InputEvent>,
    {
        let mut result: Vec<MenuInputKey> = vec![];
        for event in sdl_events {
            let maybe_key = match event {
                InputEvent::Sdl(Event::KeyDown {
                    keycode: Some(keycode),
                    ..
                }) => self.mapping.get(&keycode).copied(),
                InputEvent::Sdl(Event::Quit { .. }) => Some(MenuInputKey::Quit),
                InputEvent::Pad {
                    button,
                    pressed: true,
                    ..
                } => Self::map_from_pad(button),
                _ => None,
            };
            if let Some(key) = maybe_key {
                result.push(key)
            }
        }
        result
    }

    /// any player's pad drives the menus: d-pad/stick navigates, A selects, B backs out, start
    /// starts
    fn map_from_pad(button: PadButton) -> Option<MenuInputKey> {
        Some(match button {
            PadButton::DPadUp => MenuInputKey::Up,
            PadButton::DPadDown => MenuInputKey::Down,
            PadButton::DPadLeft => MenuInputKey::Left,
            PadButton::DPadRight => MenuInputKey::Right,
            PadButton::A => MenuInputKey::Select,
            PadButton::Start => MenuInputKey::Start,
            PadButton::B | PadButton::Back => MenuInputKey::Back,
            _ => return None,
        })
    }
}
