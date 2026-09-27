use crate::config::InputConfig;
use crate::controller::{InputEvent, PadButton};
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use std::collections::HashMap;
use std::time::Duration;

const AUTO_REPEAT_DELAY: Duration = Duration::from_millis(300);
const AUTO_REPEAT_ITERATION: Duration = Duration::from_millis(25);

#[derive(Hash, Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameInputKey {
    MoveLeft { player: u32 },
    MoveRight { player: u32 },
    SoftDrop { player: u32 },
    HardDrop { player: u32 },
    RotateClockwise { player: u32 },
    RotateAnticlockwise { player: u32 },
    Hold { player: u32 },
    Pause,
    // the app has gone into the background (Android's home button, or another app taking the
    // screen): pause if playing, and unlike `Pause` never unpause
    Suspend,
    ReturnToMenu,
    Quit,
    NextTheme,
}

impl GameInputKey {
    pub fn player(&self) -> Option<u32> {
        match self {
            GameInputKey::MoveLeft { player }
            | GameInputKey::MoveRight { player }
            | GameInputKey::SoftDrop { player }
            | GameInputKey::HardDrop { player }
            | GameInputKey::RotateClockwise { player }
            | GameInputKey::RotateAnticlockwise { player }
            | GameInputKey::Hold { player } => Some(*player),
            _ => None,
        }
    }
}

#[derive(Hash, Clone, Copy, Debug, PartialEq, Eq)]
struct GameInput {
    key: GameInputKey,
    duration: Duration,
    repeating: bool,
}

impl GameInput {
    fn new(key: GameInputKey) -> Self {
        Self {
            key,
            duration: Duration::ZERO,
            repeating: false,
        }
    }
}

type KeyMapping = HashMap<Keycode, GameInputKey>;

enum MaybeKey {
    Down(GameInputKey),
    Up(GameInputKey),
    None,
}

pub struct GameInputContext {
    mapping: KeyMapping,
    current: HashMap<GameInputKey, GameInput>,
}

impl GameInputContext {
    pub fn new(config: InputConfig) -> Self {
        Self {
            mapping: config.game_map(),
            current: HashMap::new(),
        }
    }

    pub fn update<I>(&mut self, delta: Duration, sdl_events: I) -> Vec<GameInputKey>
    where
        I: IntoIterator<Item = InputEvent>,
    {
        let mut result: Vec<GameInputKey> = vec![];

        // update any keys that might still be held with the delta
        for event in self.current.values_mut() {
            event.duration += delta;
        }

        for event in sdl_events {
            let maybe_key = match event {
                InputEvent::Sdl(sdl_event) => self.map_from_sdl_event(sdl_event),
                InputEvent::Pad {
                    player,
                    button,
                    pressed,
                } => match Self::map_from_pad(player, button) {
                    None => MaybeKey::None,
                    Some(key) if pressed => MaybeKey::Down(key),
                    Some(key) => MaybeKey::Up(key),
                },
            };
            match maybe_key {
                MaybeKey::None => {}
                MaybeKey::Down(key) => {
                    let event = GameInput::new(key);
                    self.current.insert(key, event);
                    result.push(key);
                }
                MaybeKey::Up(key) => {
                    self.current.remove(&key);
                }
            };
        }

        // check for any held keys that have triggered a repeat
        for event in self.current.values_mut() {
            match event.key {
                GameInputKey::MoveLeft { .. } | GameInputKey::MoveRight { .. } => {
                    // check auto-repeat
                    if event.repeating {
                        if event.duration >= AUTO_REPEAT_ITERATION {
                            event.duration = Duration::ZERO;
                            result.push(event.key);
                        }
                    } else if event.duration >= AUTO_REPEAT_DELAY {
                        event.duration = Duration::ZERO;
                        event.repeating = true;
                        result.push(event.key);
                    }
                }
                GameInputKey::SoftDrop { player } => {
                    result.push(GameInputKey::SoftDrop { player });
                }
                _ => {}
            }
        }

        result
    }

    /// the fixed pad layout: d-pad/stick moves and drops, A/B rotate, shoulders hold,
    /// start pauses, select/back returns to the menu and Y cycles the theme
    fn map_from_pad(player: u32, button: PadButton) -> Option<GameInputKey> {
        Some(match button {
            PadButton::DPadLeft => GameInputKey::MoveLeft { player },
            PadButton::DPadRight => GameInputKey::MoveRight { player },
            PadButton::DPadDown => GameInputKey::SoftDrop { player },
            PadButton::DPadUp => GameInputKey::HardDrop { player },
            PadButton::A => GameInputKey::RotateClockwise { player },
            PadButton::B => GameInputKey::RotateAnticlockwise { player },
            PadButton::X | PadButton::LeftShoulder | PadButton::RightShoulder => {
                GameInputKey::Hold { player }
            }
            PadButton::Y => GameInputKey::NextTheme,
            PadButton::Start => GameInputKey::Pause,
            PadButton::Back => GameInputKey::ReturnToMenu,
            PadButton::Guide => return None,
        })
    }

    fn map_from_sdl_event(&self, event: Event) -> MaybeKey {
        match event {
            Event::Quit { .. } => MaybeKey::Down(GameInputKey::Quit),
            // only a mobile SDL ever sends this
            Event::AppWillEnterBackground { .. } => MaybeKey::Down(GameInputKey::Suspend),
            Event::KeyDown {
                keycode: Some(keycode),
                repeat: false,
                ..
            } => match self.mapping.get(&keycode) {
                None => MaybeKey::None,
                Some(key) => MaybeKey::Down(*key),
            },
            Event::KeyUp {
                keycode: Some(keycode),
                repeat: false,
                ..
            } => match self.mapping.get(&keycode) {
                None => MaybeKey::None,
                Some(key) => MaybeKey::Up(*key),
            },
            _ => MaybeKey::None,
        }
    }
}
