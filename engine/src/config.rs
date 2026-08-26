use crate::audio::MAX_VOLUME;
use crate::game_input::GameInputKey;
use crate::menu_input::MenuInputKey;
use confy::ConfyError;
use sdl2::keyboard::Keycode;
use sdl2::sys;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use strum::IntoEnumIterator;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoMode {
    Window { width: u32, height: u32 },
    FullScreen { width: u32, height: u32 },
    FullScreenDesktop,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Config {
    pub video: VideoConfig,
    pub audio: AudioConfig,
    pub input: InputConfig,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct MenuInputConfig {
    pub up: GameKey,
    pub down: GameKey,
    pub left: GameKey,
    pub right: GameKey,
    pub select: GameKey,
    pub start: GameKey,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct GameInputConfig {
    pub move_left: GameKey,
    pub move_right: GameKey,
    pub soft_drop: GameKey,
    pub hard_drop: GameKey,
    pub rotate_clockwise: GameKey,
    pub rotate_anticlockwise: GameKey,
    pub hold: GameKey,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct InputConfig {
    pub menu: MenuInputConfig,
    pub player1: GameInputConfig,
    pub player2: Option<GameInputConfig>,
    pub pause: GameKey,
    pub quit: GameKey,
    pub next_theme: GameKey,
}

impl InputConfig {
    pub fn menu_map(&self) -> HashMap<Keycode, MenuInputKey> {
        HashMap::from([
            (self.menu.up.into(), MenuInputKey::Up),
            (self.menu.down.into(), MenuInputKey::Down),
            (self.menu.left.into(), MenuInputKey::Left),
            (self.menu.right.into(), MenuInputKey::Right),
            (self.menu.start.into(), MenuInputKey::Start),
            (self.menu.select.into(), MenuInputKey::Select),
            (self.quit.into(), MenuInputKey::Back),
        ])
    }

    pub fn game_map(&self) -> HashMap<Keycode, GameInputKey> {
        let mut result = HashMap::from([
            (self.quit.into(), GameInputKey::ReturnToMenu),
            (self.pause.into(), GameInputKey::Pause),
            (self.next_theme.into(), GameInputKey::NextTheme),
            (
                self.player1.move_left.into(),
                GameInputKey::MoveLeft { player: 0 },
            ),
            (
                self.player1.move_right.into(),
                GameInputKey::MoveRight { player: 0 },
            ),
            (
                self.player1.soft_drop.into(),
                GameInputKey::SoftDrop { player: 0 },
            ),
            (
                self.player1.hard_drop.into(),
                GameInputKey::HardDrop { player: 0 },
            ),
            (
                self.player1.rotate_anticlockwise.into(),
                GameInputKey::RotateAnticlockwise { player: 0 },
            ),
            (
                self.player1.rotate_clockwise.into(),
                GameInputKey::RotateClockwise { player: 0 },
            ),
            (self.player1.hold.into(), GameInputKey::Hold { player: 0 }),
        ]);

        match self.player2 {
            None => {}
            Some(p2) => {
                result.insert(p2.move_left.into(), GameInputKey::MoveLeft { player: 1 });
                result.insert(p2.move_right.into(), GameInputKey::MoveRight { player: 1 });
                result.insert(p2.soft_drop.into(), GameInputKey::SoftDrop { player: 1 });
                result.insert(p2.hard_drop.into(), GameInputKey::HardDrop { player: 1 });
                result.insert(
                    p2.rotate_anticlockwise.into(),
                    GameInputKey::RotateAnticlockwise { player: 1 },
                );
                result.insert(
                    p2.rotate_clockwise.into(),
                    GameInputKey::RotateClockwise { player: 1 },
                );
                result.insert(p2.hold.into(), GameInputKey::Hold { player: 1 });
            }
        }

        result
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct AudioConfig {
    pub music_volume: f64,
    pub effects_volume: f64,
}

impl AudioConfig {
    pub fn music_volume(&self) -> i32 {
        (self.music_volume * MAX_VOLUME as f64).round() as i32
    }

    pub fn effects_volume(&self) -> i32 {
        (self.effects_volume * MAX_VOLUME as f64).round() as i32
    }
}

/// How much the modern themes' background particle field draws. Draw calls, not particle
/// count, are what costs: every particle is its own coloured copy, and a handheld linking the
/// firmware's SDL has far less headroom than a desktop.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ParticleDensity {
    /// `High` normally, `Low` on a handheld
    #[default]
    Auto,
    Low,
    Medium,
    High,
    Ultra,
}

impl ParticleDensity {
    #[cfg(feature = "portmaster")]
    const AUTO: ParticleDensity = ParticleDensity::Low;
    #[cfg(not(feature = "portmaster"))]
    const AUTO: ParticleDensity = ParticleDensity::High;

    pub fn resolve(self) -> Self {
        match self {
            ParticleDensity::Auto => Self::AUTO,
            other => other,
        }
    }

    /// how many particles the field keeps alive
    pub fn pool_size(self) -> usize {
        match self.resolve() {
            ParticleDensity::Low => 160,
            ParticleDensity::Medium => 320,
            ParticleDensity::High => 560,
            _ => 900,
        }
    }

    /// the most constellation links drawn in one frame
    pub fn link_budget(self) -> usize {
        match self.resolve() {
            ParticleDensity::Low => 40,
            ParticleDensity::Medium => 120,
            ParticleDensity::High => 240,
            _ => 400,
        }
    }

    /// links are soft textured quads, which take a colour mod and read well at 4k. At the
    /// lowest density they are `SDL_RenderDrawLine` hairlines instead: one call, one physical
    /// pixel, nearly invisible on a big screen but very cheap on a handheld.
    pub fn links_are_quads(self) -> bool {
        !matches!(self.resolve(), ParticleDensity::Low)
    }

    /// roughly how many lattice steps across a silhouette's longest side
    pub fn edge_steps(self) -> u32 {
        match self.resolve() {
            ParticleDensity::Low => 18,
            ParticleDensity::Medium => 26,
            ParticleDensity::High => 40,
            _ => 52,
        }
    }

    /// concurrent shockwaves, comets and the like
    pub fn max_effects(self) -> usize {
        match self.resolve() {
            ParticleDensity::Low => 2,
            ParticleDensity::Medium => 4,
            ParticleDensity::High => 8,
            _ => 12,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct VideoConfig {
    pub mode: VideoMode,
    pub vsync: bool,
    pub disable_screensaver: bool,
    /// scale the themes by whole pixels only. Crisper, but the board steps down to the next
    /// whole multiple of its art, which can waste a lot of the window.
    pub integer_scale: bool,
    /// how much the modern themes' particle background draws. Config file only: there is no
    /// video options menu to hang it on
    #[serde(default)]
    pub particle_density: ParticleDensity,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            video: VideoConfig {
                #[cfg(not(feature = "portmaster"))]
                mode: VideoMode::Window {
                    width: 1280,
                    height: 720,
                },
                // handhelds: fill whatever the panel is (PortMaster runs ports under sway or
                // KMSDRM, where a mode switch is the fragile option) and let the scaler fit it
                #[cfg(feature = "portmaster")]
                mode: VideoMode::FullScreenDesktop,
                vsync: true,
                disable_screensaver: true,

                // smooth scaling fills the window and keeps every theme's board the same
                // size; whole-pixel scaling looks crisper but drops a step to do it
                integer_scale: false,

                particle_density: ParticleDensity::Auto,
            },
            audio: AudioConfig {
                music_volume: 0.5,
                effects_volume: 1.0,
            },
            // keyboard only: game controllers use the fixed layout in `crate::controller`
            input: InputConfig {
                menu: MenuInputConfig {
                    up: GameKey::Up,
                    down: GameKey::Down,
                    left: GameKey::Left,
                    right: GameKey::Right,
                    select: GameKey::X,
                    start: GameKey::Return,
                },
                player1: GameInputConfig {
                    move_left: GameKey::Left,
                    move_right: GameKey::Right,
                    soft_drop: GameKey::Down,
                    hard_drop: GameKey::Up,
                    rotate_clockwise: GameKey::X,
                    rotate_anticlockwise: GameKey::Z,
                    hold: GameKey::LShift,
                },
                player2: None,
                #[cfg(feature = "portmaster")]
                pause: GameKey::Return,
                #[cfg(not(feature = "portmaster"))]
                pause: GameKey::F1,
                #[cfg(feature = "portmaster")]
                next_theme: GameKey::RShift,
                #[cfg(not(feature = "portmaster"))]
                next_theme: GameKey::F2,
                quit: GameKey::Escape,
            },
        }
    }
}

#[cfg(feature = "portmaster")]
pub fn config_path(name: &str) -> Result<PathBuf, String> {
    let mut absolute = std::env::current_dir().map_err(|e| e.to_string())?;
    absolute.push(format!("{}.yml", name));
    Ok(absolute)
}

#[cfg(feature = "browser")]
pub fn config_path(name: &str) -> Result<PathBuf, String> {
    // /data is the IDBFS (IndexedDB) mount set up by the page, see web/index.html
    Ok(PathBuf::from(format!("/data/{}.yml", name)))
}

#[cfg(not(any(feature = "portmaster", feature = "browser")))]
pub fn config_path(name: &str) -> Result<PathBuf, String> {
    confy::get_configuration_file_path(crate::app_info::get().name, name).map_err(|e| e.to_string())
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let config_path = config_path("config")?;

        #[cfg(debug_assertions)]
        println!("loading config: {}", config_path.to_str().unwrap());

        match confy::load_path(&config_path) {
            Ok(config) => Ok(config),
            Err(ConfyError::BadYamlData(error)) => {
                println!(
                    "Bad config file at {}, {}, loading defaults",
                    config_path.to_str().unwrap(),
                    error
                );
                Ok(Self::default())
            }
            Err(error) => Err(format!("{}", error)),
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug, Serialize, Deserialize, strum::EnumIter)]
#[repr(i32)]
pub enum GameKey {
    Backspace = sys::SDL_KeyCode::SDLK_BACKSPACE as i32,
    Tab = sys::SDL_KeyCode::SDLK_TAB as i32,
    Return = sys::SDL_KeyCode::SDLK_RETURN as i32,
    Escape = sys::SDL_KeyCode::SDLK_ESCAPE as i32,
    Space = sys::SDL_KeyCode::SDLK_SPACE as i32,
    Exclaim = sys::SDL_KeyCode::SDLK_EXCLAIM as i32,
    Quotedbl = sys::SDL_KeyCode::SDLK_QUOTEDBL as i32,
    Hash = sys::SDL_KeyCode::SDLK_HASH as i32,
    Dollar = sys::SDL_KeyCode::SDLK_DOLLAR as i32,
    Percent = sys::SDL_KeyCode::SDLK_PERCENT as i32,
    Ampersand = sys::SDL_KeyCode::SDLK_AMPERSAND as i32,
    Quote = sys::SDL_KeyCode::SDLK_QUOTE as i32,
    LeftParen = sys::SDL_KeyCode::SDLK_LEFTPAREN as i32,
    RightParen = sys::SDL_KeyCode::SDLK_RIGHTPAREN as i32,
    Asterisk = sys::SDL_KeyCode::SDLK_ASTERISK as i32,
    Plus = sys::SDL_KeyCode::SDLK_PLUS as i32,
    Comma = sys::SDL_KeyCode::SDLK_COMMA as i32,
    Minus = sys::SDL_KeyCode::SDLK_MINUS as i32,
    Period = sys::SDL_KeyCode::SDLK_PERIOD as i32,
    Slash = sys::SDL_KeyCode::SDLK_SLASH as i32,
    Num0 = sys::SDL_KeyCode::SDLK_0 as i32,
    Num1 = sys::SDL_KeyCode::SDLK_1 as i32,
    Num2 = sys::SDL_KeyCode::SDLK_2 as i32,
    Num3 = sys::SDL_KeyCode::SDLK_3 as i32,
    Num4 = sys::SDL_KeyCode::SDLK_4 as i32,
    Num5 = sys::SDL_KeyCode::SDLK_5 as i32,
    Num6 = sys::SDL_KeyCode::SDLK_6 as i32,
    Num7 = sys::SDL_KeyCode::SDLK_7 as i32,
    Num8 = sys::SDL_KeyCode::SDLK_8 as i32,
    Num9 = sys::SDL_KeyCode::SDLK_9 as i32,
    Colon = sys::SDL_KeyCode::SDLK_COLON as i32,
    Semicolon = sys::SDL_KeyCode::SDLK_SEMICOLON as i32,
    Less = sys::SDL_KeyCode::SDLK_LESS as i32,
    Equals = sys::SDL_KeyCode::SDLK_EQUALS as i32,
    Greater = sys::SDL_KeyCode::SDLK_GREATER as i32,
    Question = sys::SDL_KeyCode::SDLK_QUESTION as i32,
    At = sys::SDL_KeyCode::SDLK_AT as i32,
    LeftBracket = sys::SDL_KeyCode::SDLK_LEFTBRACKET as i32,
    Backslash = sys::SDL_KeyCode::SDLK_BACKSLASH as i32,
    RightBracket = sys::SDL_KeyCode::SDLK_RIGHTBRACKET as i32,
    Caret = sys::SDL_KeyCode::SDLK_CARET as i32,
    Underscore = sys::SDL_KeyCode::SDLK_UNDERSCORE as i32,
    Backquote = sys::SDL_KeyCode::SDLK_BACKQUOTE as i32,
    A = sys::SDL_KeyCode::SDLK_a as i32,
    B = sys::SDL_KeyCode::SDLK_b as i32,
    C = sys::SDL_KeyCode::SDLK_c as i32,
    D = sys::SDL_KeyCode::SDLK_d as i32,
    E = sys::SDL_KeyCode::SDLK_e as i32,
    F = sys::SDL_KeyCode::SDLK_f as i32,
    G = sys::SDL_KeyCode::SDLK_g as i32,
    H = sys::SDL_KeyCode::SDLK_h as i32,
    I = sys::SDL_KeyCode::SDLK_i as i32,
    J = sys::SDL_KeyCode::SDLK_j as i32,
    K = sys::SDL_KeyCode::SDLK_k as i32,
    L = sys::SDL_KeyCode::SDLK_l as i32,
    M = sys::SDL_KeyCode::SDLK_m as i32,
    N = sys::SDL_KeyCode::SDLK_n as i32,
    O = sys::SDL_KeyCode::SDLK_o as i32,
    P = sys::SDL_KeyCode::SDLK_p as i32,
    Q = sys::SDL_KeyCode::SDLK_q as i32,
    R = sys::SDL_KeyCode::SDLK_r as i32,
    S = sys::SDL_KeyCode::SDLK_s as i32,
    T = sys::SDL_KeyCode::SDLK_t as i32,
    U = sys::SDL_KeyCode::SDLK_u as i32,
    V = sys::SDL_KeyCode::SDLK_v as i32,
    W = sys::SDL_KeyCode::SDLK_w as i32,
    X = sys::SDL_KeyCode::SDLK_x as i32,
    Y = sys::SDL_KeyCode::SDLK_y as i32,
    Z = sys::SDL_KeyCode::SDLK_z as i32,
    Delete = sys::SDL_KeyCode::SDLK_DELETE as i32,
    CapsLock = sys::SDL_KeyCode::SDLK_CAPSLOCK as i32,
    F1 = sys::SDL_KeyCode::SDLK_F1 as i32,
    F2 = sys::SDL_KeyCode::SDLK_F2 as i32,
    F3 = sys::SDL_KeyCode::SDLK_F3 as i32,
    F4 = sys::SDL_KeyCode::SDLK_F4 as i32,
    F5 = sys::SDL_KeyCode::SDLK_F5 as i32,
    F6 = sys::SDL_KeyCode::SDLK_F6 as i32,
    F7 = sys::SDL_KeyCode::SDLK_F7 as i32,
    F8 = sys::SDL_KeyCode::SDLK_F8 as i32,
    F9 = sys::SDL_KeyCode::SDLK_F9 as i32,
    F10 = sys::SDL_KeyCode::SDLK_F10 as i32,
    F11 = sys::SDL_KeyCode::SDLK_F11 as i32,
    F12 = sys::SDL_KeyCode::SDLK_F12 as i32,
    PrintScreen = sys::SDL_KeyCode::SDLK_PRINTSCREEN as i32,
    ScrollLock = sys::SDL_KeyCode::SDLK_SCROLLLOCK as i32,
    Pause = sys::SDL_KeyCode::SDLK_PAUSE as i32,
    Insert = sys::SDL_KeyCode::SDLK_INSERT as i32,
    Home = sys::SDL_KeyCode::SDLK_HOME as i32,
    PageUp = sys::SDL_KeyCode::SDLK_PAGEUP as i32,
    End = sys::SDL_KeyCode::SDLK_END as i32,
    PageDown = sys::SDL_KeyCode::SDLK_PAGEDOWN as i32,
    Right = sys::SDL_KeyCode::SDLK_RIGHT as i32,
    Left = sys::SDL_KeyCode::SDLK_LEFT as i32,
    Down = sys::SDL_KeyCode::SDLK_DOWN as i32,
    Up = sys::SDL_KeyCode::SDLK_UP as i32,
    NumLockClear = sys::SDL_KeyCode::SDLK_NUMLOCKCLEAR as i32,
    KpDivide = sys::SDL_KeyCode::SDLK_KP_DIVIDE as i32,
    KpMultiply = sys::SDL_KeyCode::SDLK_KP_MULTIPLY as i32,
    KpMinus = sys::SDL_KeyCode::SDLK_KP_MINUS as i32,
    KpPlus = sys::SDL_KeyCode::SDLK_KP_PLUS as i32,
    KpEnter = sys::SDL_KeyCode::SDLK_KP_ENTER as i32,
    Kp1 = sys::SDL_KeyCode::SDLK_KP_1 as i32,
    Kp2 = sys::SDL_KeyCode::SDLK_KP_2 as i32,
    Kp3 = sys::SDL_KeyCode::SDLK_KP_3 as i32,
    Kp4 = sys::SDL_KeyCode::SDLK_KP_4 as i32,
    Kp5 = sys::SDL_KeyCode::SDLK_KP_5 as i32,
    Kp6 = sys::SDL_KeyCode::SDLK_KP_6 as i32,
    Kp7 = sys::SDL_KeyCode::SDLK_KP_7 as i32,
    Kp8 = sys::SDL_KeyCode::SDLK_KP_8 as i32,
    Kp9 = sys::SDL_KeyCode::SDLK_KP_9 as i32,
    Kp0 = sys::SDL_KeyCode::SDLK_KP_0 as i32,
    KpPeriod = sys::SDL_KeyCode::SDLK_KP_PERIOD as i32,
    Application = sys::SDL_KeyCode::SDLK_APPLICATION as i32,
    Power = sys::SDL_KeyCode::SDLK_POWER as i32,
    KpEquals = sys::SDL_KeyCode::SDLK_KP_EQUALS as i32,
    F13 = sys::SDL_KeyCode::SDLK_F13 as i32,
    F14 = sys::SDL_KeyCode::SDLK_F14 as i32,
    F15 = sys::SDL_KeyCode::SDLK_F15 as i32,
    F16 = sys::SDL_KeyCode::SDLK_F16 as i32,
    F17 = sys::SDL_KeyCode::SDLK_F17 as i32,
    F18 = sys::SDL_KeyCode::SDLK_F18 as i32,
    F19 = sys::SDL_KeyCode::SDLK_F19 as i32,
    F20 = sys::SDL_KeyCode::SDLK_F20 as i32,
    F21 = sys::SDL_KeyCode::SDLK_F21 as i32,
    F22 = sys::SDL_KeyCode::SDLK_F22 as i32,
    F23 = sys::SDL_KeyCode::SDLK_F23 as i32,
    F24 = sys::SDL_KeyCode::SDLK_F24 as i32,
    Execute = sys::SDL_KeyCode::SDLK_EXECUTE as i32,
    Help = sys::SDL_KeyCode::SDLK_HELP as i32,
    Menu = sys::SDL_KeyCode::SDLK_MENU as i32,
    Select = sys::SDL_KeyCode::SDLK_SELECT as i32,
    Stop = sys::SDL_KeyCode::SDLK_STOP as i32,
    Again = sys::SDL_KeyCode::SDLK_AGAIN as i32,
    Undo = sys::SDL_KeyCode::SDLK_UNDO as i32,
    Cut = sys::SDL_KeyCode::SDLK_CUT as i32,
    Copy = sys::SDL_KeyCode::SDLK_COPY as i32,
    Paste = sys::SDL_KeyCode::SDLK_PASTE as i32,
    Find = sys::SDL_KeyCode::SDLK_FIND as i32,
    Mute = sys::SDL_KeyCode::SDLK_MUTE as i32,
    VolumeUp = sys::SDL_KeyCode::SDLK_VOLUMEUP as i32,
    VolumeDown = sys::SDL_KeyCode::SDLK_VOLUMEDOWN as i32,
    KpComma = sys::SDL_KeyCode::SDLK_KP_COMMA as i32,
    KpEqualsAS400 = sys::SDL_KeyCode::SDLK_KP_EQUALSAS400 as i32,
    AltErase = sys::SDL_KeyCode::SDLK_ALTERASE as i32,
    Sysreq = sys::SDL_KeyCode::SDLK_SYSREQ as i32,
    Cancel = sys::SDL_KeyCode::SDLK_CANCEL as i32,
    Clear = sys::SDL_KeyCode::SDLK_CLEAR as i32,
    Prior = sys::SDL_KeyCode::SDLK_PRIOR as i32,
    Return2 = sys::SDL_KeyCode::SDLK_RETURN2 as i32,
    Separator = sys::SDL_KeyCode::SDLK_SEPARATOR as i32,
    Out = sys::SDL_KeyCode::SDLK_OUT as i32,
    Oper = sys::SDL_KeyCode::SDLK_OPER as i32,
    ClearAgain = sys::SDL_KeyCode::SDLK_CLEARAGAIN as i32,
    CrSel = sys::SDL_KeyCode::SDLK_CRSEL as i32,
    ExSel = sys::SDL_KeyCode::SDLK_EXSEL as i32,
    Kp00 = sys::SDL_KeyCode::SDLK_KP_00 as i32,
    Kp000 = sys::SDL_KeyCode::SDLK_KP_000 as i32,
    ThousandsSeparator = sys::SDL_KeyCode::SDLK_THOUSANDSSEPARATOR as i32,
    DecimalSeparator = sys::SDL_KeyCode::SDLK_DECIMALSEPARATOR as i32,
    CurrencyUnit = sys::SDL_KeyCode::SDLK_CURRENCYUNIT as i32,
    CurrencySubUnit = sys::SDL_KeyCode::SDLK_CURRENCYSUBUNIT as i32,
    KpLeftParen = sys::SDL_KeyCode::SDLK_KP_LEFTPAREN as i32,
    KpRightParen = sys::SDL_KeyCode::SDLK_KP_RIGHTPAREN as i32,
    KpLeftBrace = sys::SDL_KeyCode::SDLK_KP_LEFTBRACE as i32,
    KpRightBrace = sys::SDL_KeyCode::SDLK_KP_RIGHTBRACE as i32,
    KpTab = sys::SDL_KeyCode::SDLK_KP_TAB as i32,
    KpBackspace = sys::SDL_KeyCode::SDLK_KP_BACKSPACE as i32,
    KpA = sys::SDL_KeyCode::SDLK_KP_A as i32,
    KpB = sys::SDL_KeyCode::SDLK_KP_B as i32,
    KpC = sys::SDL_KeyCode::SDLK_KP_C as i32,
    KpD = sys::SDL_KeyCode::SDLK_KP_D as i32,
    KpE = sys::SDL_KeyCode::SDLK_KP_E as i32,
    KpF = sys::SDL_KeyCode::SDLK_KP_F as i32,
    KpXor = sys::SDL_KeyCode::SDLK_KP_XOR as i32,
    KpPower = sys::SDL_KeyCode::SDLK_KP_POWER as i32,
    KpPercent = sys::SDL_KeyCode::SDLK_KP_PERCENT as i32,
    KpLess = sys::SDL_KeyCode::SDLK_KP_LESS as i32,
    KpGreater = sys::SDL_KeyCode::SDLK_KP_GREATER as i32,
    KpAmpersand = sys::SDL_KeyCode::SDLK_KP_AMPERSAND as i32,
    KpDblAmpersand = sys::SDL_KeyCode::SDLK_KP_DBLAMPERSAND as i32,
    KpVerticalBar = sys::SDL_KeyCode::SDLK_KP_VERTICALBAR as i32,
    KpDblVerticalBar = sys::SDL_KeyCode::SDLK_KP_DBLVERTICALBAR as i32,
    KpColon = sys::SDL_KeyCode::SDLK_KP_COLON as i32,
    KpHash = sys::SDL_KeyCode::SDLK_KP_HASH as i32,
    KpSpace = sys::SDL_KeyCode::SDLK_KP_SPACE as i32,
    KpAt = sys::SDL_KeyCode::SDLK_KP_AT as i32,
    KpExclam = sys::SDL_KeyCode::SDLK_KP_EXCLAM as i32,
    KpMemStore = sys::SDL_KeyCode::SDLK_KP_MEMSTORE as i32,
    KpMemRecall = sys::SDL_KeyCode::SDLK_KP_MEMRECALL as i32,
    KpMemClear = sys::SDL_KeyCode::SDLK_KP_MEMCLEAR as i32,
    KpMemAdd = sys::SDL_KeyCode::SDLK_KP_MEMADD as i32,
    KpMemSubtract = sys::SDL_KeyCode::SDLK_KP_MEMSUBTRACT as i32,
    KpMemMultiply = sys::SDL_KeyCode::SDLK_KP_MEMMULTIPLY as i32,
    KpMemDivide = sys::SDL_KeyCode::SDLK_KP_MEMDIVIDE as i32,
    KpPlusMinus = sys::SDL_KeyCode::SDLK_KP_PLUSMINUS as i32,
    KpClear = sys::SDL_KeyCode::SDLK_KP_CLEAR as i32,
    KpClearEntry = sys::SDL_KeyCode::SDLK_KP_CLEARENTRY as i32,
    KpBinary = sys::SDL_KeyCode::SDLK_KP_BINARY as i32,
    KpOctal = sys::SDL_KeyCode::SDLK_KP_OCTAL as i32,
    KpDecimal = sys::SDL_KeyCode::SDLK_KP_DECIMAL as i32,
    KpHexadecimal = sys::SDL_KeyCode::SDLK_KP_HEXADECIMAL as i32,
    LCtrl = sys::SDL_KeyCode::SDLK_LCTRL as i32,
    LShift = sys::SDL_KeyCode::SDLK_LSHIFT as i32,
    LAlt = sys::SDL_KeyCode::SDLK_LALT as i32,
    LGui = sys::SDL_KeyCode::SDLK_LGUI as i32,
    RCtrl = sys::SDL_KeyCode::SDLK_RCTRL as i32,
    RShift = sys::SDL_KeyCode::SDLK_RSHIFT as i32,
    RAlt = sys::SDL_KeyCode::SDLK_RALT as i32,
    RGui = sys::SDL_KeyCode::SDLK_RGUI as i32,
    Mode = sys::SDL_KeyCode::SDLK_MODE as i32,
    AudioNext = sys::SDL_KeyCode::SDLK_AUDIONEXT as i32,
    AudioPrev = sys::SDL_KeyCode::SDLK_AUDIOPREV as i32,
    AudioStop = sys::SDL_KeyCode::SDLK_AUDIOSTOP as i32,
    AudioPlay = sys::SDL_KeyCode::SDLK_AUDIOPLAY as i32,
    AudioMute = sys::SDL_KeyCode::SDLK_AUDIOMUTE as i32,
    MediaSelect = sys::SDL_KeyCode::SDLK_MEDIASELECT as i32,
    Www = sys::SDL_KeyCode::SDLK_WWW as i32,
    Mail = sys::SDL_KeyCode::SDLK_MAIL as i32,
    Calculator = sys::SDL_KeyCode::SDLK_CALCULATOR as i32,
    Computer = sys::SDL_KeyCode::SDLK_COMPUTER as i32,
    AcSearch = sys::SDL_KeyCode::SDLK_AC_SEARCH as i32,
    AcHome = sys::SDL_KeyCode::SDLK_AC_HOME as i32,
    AcBack = sys::SDL_KeyCode::SDLK_AC_BACK as i32,
    AcForward = sys::SDL_KeyCode::SDLK_AC_FORWARD as i32,
    AcStop = sys::SDL_KeyCode::SDLK_AC_STOP as i32,
    AcRefresh = sys::SDL_KeyCode::SDLK_AC_REFRESH as i32,
    AcBookmarks = sys::SDL_KeyCode::SDLK_AC_BOOKMARKS as i32,
    BrightnessDown = sys::SDL_KeyCode::SDLK_BRIGHTNESSDOWN as i32,
    BrightnessUp = sys::SDL_KeyCode::SDLK_BRIGHTNESSUP as i32,
    DisplaySwitch = sys::SDL_KeyCode::SDLK_DISPLAYSWITCH as i32,
    KbdIllumToggle = sys::SDL_KeyCode::SDLK_KBDILLUMTOGGLE as i32,
    KbdIllumDown = sys::SDL_KeyCode::SDLK_KBDILLUMDOWN as i32,
    KbdIllumUp = sys::SDL_KeyCode::SDLK_KBDILLUMUP as i32,
    Eject = sys::SDL_KeyCode::SDLK_EJECT as i32,
    Sleep = sys::SDL_KeyCode::SDLK_SLEEP as i32,
}

impl From<Keycode> for GameKey {
    fn from(value: Keycode) -> Self {
        let code = value.into_i32();
        Self::iter()
            .find(|&e| code == e as i32)
            .expect("Invalid keycode")
    }
}

impl Into<Keycode> for GameKey {
    fn into(self) -> Keycode {
        Keycode::from_i32(self as i32).expect("Invalid keycode")
    }
}
