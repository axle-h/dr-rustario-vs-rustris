//! Native SDL game controller support. Controllers are opened as they appear and assigned a
//! player slot by order of arrival; their buttons, d-pad and left stick are translated into a
//! fixed, keyboard-independent set of [`PadButton`]s that the menu and game input contexts map
//! onto their own keys. Keyboard events pass through untouched.

use sdl2::controller::{Axis, Button, GameController};
use sdl2::event::Event;
use sdl2::EventPump;
use sdl2::GameControllerSubsystem;

/// Left-stick travel (of ±1) before it registers as a d-pad press.
const STICK_DEADZONE: f32 = 0.5;

/// A button on a pad in SDL's positional naming (A = south, B = east, X = west, Y = north).
#[derive(Hash, Clone, Copy, Debug, PartialEq, Eq)]
pub enum PadButton {
    DPadUp,
    DPadDown,
    DPadLeft,
    DPadRight,
    A,
    B,
    X,
    Y,
    LeftShoulder,
    RightShoulder,
    Start,
    Back,
    Guide,
}

impl PadButton {
    fn from_sdl(button: Button) -> Option<Self> {
        Some(match button {
            Button::DPadUp => PadButton::DPadUp,
            Button::DPadDown => PadButton::DPadDown,
            Button::DPadLeft => PadButton::DPadLeft,
            Button::DPadRight => PadButton::DPadRight,
            Button::A => PadButton::A,
            Button::B => PadButton::B,
            Button::X => PadButton::X,
            Button::Y => PadButton::Y,
            Button::LeftShoulder => PadButton::LeftShoulder,
            Button::RightShoulder => PadButton::RightShoulder,
            Button::Start => PadButton::Start,
            Button::Back => PadButton::Back,
            Button::Guide => PadButton::Guide,
            _ => return None,
        })
    }
}

/// An input event after controller translation.
#[derive(Clone, Debug)]
pub enum InputEvent {
    /// an untouched SDL event (keyboard, quit, window ...)
    Sdl(Event),
    /// a pad button on player `player`'s controller changed state
    Pad {
        player: u32,
        button: PadButton,
        pressed: bool,
    },
}

struct Pad {
    controller: GameController,
    player: u32,
    /// the d-pad direction the left stick is currently emulating on each axis
    stick_x: Option<PadButton>,
    stick_y: Option<PadButton>,
}

pub struct Controllers {
    subsystem: GameControllerSubsystem,
    pads: Vec<Pad>,
    max_players: u32,
}

impl Controllers {
    pub fn new(subsystem: GameControllerSubsystem, max_players: u32) -> Self {
        Self {
            subsystem,
            pads: vec![],
            max_players,
        }
    }

    /// the lowest player slot not held by an open controller
    fn free_slot(&self) -> Option<u32> {
        (0..self.max_players).find(|slot| !self.pads.iter().any(|p| p.player == *slot))
    }

    fn open(&mut self, device_index: u32) {
        if !self.subsystem.is_game_controller(device_index) {
            return;
        }
        let Some(player) = self.free_slot() else {
            return;
        };
        match self.subsystem.open(device_index) {
            Ok(controller) => {
                let id = controller.instance_id();
                if self.pads.iter().any(|p| p.controller.instance_id() == id) {
                    return;
                }
                println!("controller {} -> player {}", controller.name(), player + 1);
                self.pads.push(Pad {
                    controller,
                    player,
                    stick_x: None,
                    stick_y: None,
                })
            }
            Err(e) => println!("could not open controller {}: {}", device_index, e),
        }
    }

    fn close(&mut self, instance_id: u32) {
        self.pads
            .retain(|p| p.controller.instance_id() != instance_id);
    }

    fn pad_mut(&mut self, instance_id: u32) -> Option<&mut Pad> {
        self.pads
            .iter_mut()
            .find(|p| p.controller.instance_id() == instance_id)
    }

    /// Drain the event pump, opening and closing controllers as they come and go, and
    /// translate controller events to pad presses.
    pub fn poll(&mut self, event_pump: &mut EventPump) -> Vec<InputEvent> {
        let mut result = vec![];
        for event in event_pump.poll_iter() {
            match event {
                Event::ControllerDeviceAdded { which, .. } => self.open(which),
                Event::ControllerDeviceRemoved { which, .. } => self.close(which),
                Event::ControllerButtonDown { which, button, .. } => {
                    self.button(&mut result, which, button, true)
                }
                Event::ControllerButtonUp { which, button, .. } => {
                    self.button(&mut result, which, button, false)
                }
                Event::ControllerAxisMotion {
                    which, axis, value, ..
                } => self.axis(&mut result, which, axis, value),
                other => result.push(InputEvent::Sdl(other)),
            }
        }
        result
    }

    fn button(&mut self, out: &mut Vec<InputEvent>, which: u32, button: Button, pressed: bool) {
        let Some(button) = PadButton::from_sdl(button) else {
            return;
        };
        if let Some(pad) = self.pad_mut(which) {
            out.push(InputEvent::Pad {
                player: pad.player,
                button,
                pressed,
            });
        }
    }

    fn axis(&mut self, out: &mut Vec<InputEvent>, which: u32, axis: Axis, value: i16) {
        let (negative, positive) = match axis {
            Axis::LeftX => (PadButton::DPadLeft, PadButton::DPadRight),
            Axis::LeftY => (PadButton::DPadUp, PadButton::DPadDown),
            _ => return,
        };
        let Some(pad) = self.pad_mut(which) else {
            return;
        };
        let travel = value as f32 / i16::MAX as f32;
        let direction = if travel <= -STICK_DEADZONE {
            Some(negative)
        } else if travel >= STICK_DEADZONE {
            Some(positive)
        } else {
            None
        };
        let current = match axis {
            Axis::LeftX => &mut pad.stick_x,
            _ => &mut pad.stick_y,
        };
        if *current == direction {
            return;
        }
        if let Some(released) = current.take() {
            out.push(InputEvent::Pad {
                player: pad.player,
                button: released,
                pressed: false,
            });
        }
        if let Some(pressed) = direction {
            out.push(InputEvent::Pad {
                player: pad.player,
                button: pressed,
                pressed: true,
            });
        }
        *current = direction;
    }
}
