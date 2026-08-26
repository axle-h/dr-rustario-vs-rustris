use sdl2::pixels::Color;
use std::ops::{Add, Mul};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleColor {
    red: f64,
    green: f64,
    blue: f64,
}

impl ParticleColor {
    pub const WHITE: ParticleColor = ParticleColor::rgb(1.0, 1.0, 1.0);
    pub const BLACK: ParticleColor = ParticleColor::rgb(0.0, 0.0, 0.0);
    pub const ZERO: ParticleColor = ParticleColor::rgb(0.0, 0.0, 0.0);

    pub const fn rgb(red: f64, green: f64, blue: f64) -> Self {
        Self { red, green, blue }
    }

    pub fn from_sdl(color: Color) -> Self {
        fn to_ratio(value: u8) -> f64 {
            value as f64 / 255.0
        }
        ParticleColor::rgb(to_ratio(color.r), to_ratio(color.g), to_ratio(color.b))
    }

    pub fn red(self) -> f64 {
        self.red
    }
    pub fn green(self) -> f64 {
        self.green
    }
    pub fn blue(self) -> f64 {
        self.blue
    }

    /// hue in degrees, saturation and value 0-1
    pub fn to_hsv(self) -> (f64, f64, f64) {
        let max = self.red.max(self.green).max(self.blue);
        let min = self.red.min(self.green).min(self.blue);
        let chroma = max - min;
        let hue = if chroma <= f64::EPSILON {
            0.0
        } else if max == self.red {
            60.0 * (((self.green - self.blue) / chroma) % 6.0)
        } else if max == self.green {
            60.0 * ((self.blue - self.red) / chroma + 2.0)
        } else {
            60.0 * ((self.red - self.green) / chroma + 4.0)
        };
        let saturation = if max <= f64::EPSILON { 0.0 } else { chroma / max };
        (wrap_hue(hue), saturation, max)
    }

    pub fn from_hsv(hue: f64, saturation: f64, value: f64) -> Self {
        let hue = wrap_hue(hue);
        let saturation = saturation.clamp(0.0, 1.0);
        let value = value.clamp(0.0, 1.0);
        let chroma = value * saturation;
        let h = hue / 60.0;
        let x = chroma * (1.0 - (h % 2.0 - 1.0).abs());
        let (r, g, b) = match h as u32 {
            0 => (chroma, x, 0.0),
            1 => (x, chroma, 0.0),
            2 => (0.0, chroma, x),
            3 => (0.0, x, chroma),
            4 => (x, 0.0, chroma),
            _ => (chroma, 0.0, x),
        };
        let m = value - chroma;
        Self::rgb(r + m, g + m, b + m)
    }

    /// rotate the hue, keeping saturation and value
    pub fn shift_hue(self, degrees: f64) -> Self {
        let (h, s, v) = self.to_hsv();
        Self::from_hsv(h + degrees, s, v)
    }

    pub fn with_saturation(self, saturation: f64) -> Self {
        let (h, _, v) = self.to_hsv();
        Self::from_hsv(h, saturation, v)
    }

    pub fn with_value(self, value: f64) -> Self {
        let (h, s, _) = self.to_hsv();
        Self::from_hsv(h, s, value)
    }

    /// `t` of the way from self to other, in rgb
    pub fn lerp(self, other: Self, t: f64) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self::rgb(
            self.red + (other.red - self.red) * t,
            self.green + (other.green - self.green) * t,
            self.blue + (other.blue - self.blue) * t,
        )
    }

    /// the shorter way round the hue circle from self to other, so red to magenta does not
    /// take the long trip through green
    pub fn lerp_hue(self, other: Self, t: f64) -> Self {
        let t = t.clamp(0.0, 1.0);
        let (h1, s1, v1) = self.to_hsv();
        let (h2, s2, v2) = other.to_hsv();
        let mut delta = h2 - h1;
        if delta > 180.0 {
            delta -= 360.0;
        } else if delta < -180.0 {
            delta += 360.0;
        }
        Self::from_hsv(h1 + delta * t, s1 + (s2 - s1) * t, v1 + (v2 - v1) * t)
    }

    pub fn to_sdl(self, alpha: f64) -> Color {
        Color::RGBA(
            to_byte(self.red),
            to_byte(self.green),
            to_byte(self.blue),
            to_byte(alpha),
        )
    }
}
fn to_byte(value: f64) -> u8 {
    (255.0 * value.max(0.0).min(1.0)).round() as u8
}

/// hue into 0-360
fn wrap_hue(hue: f64) -> f64 {
    let hue = hue % 360.0;
    if hue < 0.0 {
        hue + 360.0
    } else {
        hue
    }
}

impl From<(f64, f64, f64)> for ParticleColor {
    fn from((r, g, b): (f64, f64, f64)) -> Self {
        ParticleColor::rgb(r, g, b)
    }
}

impl From<ParticleColor> for (f64, f64, f64) {
    fn from(val: ParticleColor) -> Self {
        (val.red, val.green, val.blue)
    }
}

impl From<ParticleColor> for (u8, u8, u8) {
    fn from(val: ParticleColor) -> Self {
        (to_byte(val.red), to_byte(val.green), to_byte(val.blue))
    }
}

impl From<Color> for ParticleColor {
    fn from(value: Color) -> Self {
        ParticleColor::from_sdl(value)
    }
}

impl Add for ParticleColor {
    type Output = ParticleColor;

    fn add(self, rhs: Self) -> Self::Output {
        Self::rgb(
            self.red + rhs.red,
            self.green + rhs.green,
            self.blue + rhs.blue,
        )
    }
}

impl Mul for ParticleColor {
    type Output = ParticleColor;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::rgb(
            self.red * rhs.red,
            self.green * rhs.green,
            self.blue * rhs.blue,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(a: ParticleColor, b: ParticleColor) {
        assert!(
            (a.red - b.red).abs() < 1e-9
                && (a.green - b.green).abs() < 1e-9
                && (a.blue - b.blue).abs() < 1e-9,
            "{a:?} != {b:?}"
        );
    }

    #[test]
    fn hsv_round_trips() {
        for color in [
            ParticleColor::rgb(1.0, 0.0, 0.0),
            ParticleColor::rgb(0.0, 1.0, 0.0),
            ParticleColor::rgb(0.0, 0.0, 1.0),
            ParticleColor::rgb(0.882, 0.745, 0.0), // the modern vitamins' golden yellow
            ParticleColor::rgb(0.2, 0.4, 0.7),
            ParticleColor::WHITE,
            ParticleColor::BLACK,
        ] {
            let (h, s, v) = color.to_hsv();
            assert_close(ParticleColor::from_hsv(h, s, v), color);
        }
    }

    #[test]
    fn a_full_turn_of_hue_is_no_change() {
        let color = ParticleColor::rgb(0.1, 0.6, 0.9);
        assert_close(color.shift_hue(360.0), color);
        assert_close(color.shift_hue(-360.0), color);
    }

    #[test]
    fn hue_lerp_takes_the_short_way_round() {
        let red = ParticleColor::rgb(1.0, 0.0, 0.0);
        let magenta = ParticleColor::rgb(1.0, 0.0, 1.0);
        // 0 -> 300 the short way is backwards through 330, not forwards through green
        let (hue, _, _) = red.lerp_hue(magenta, 0.5).to_hsv();
        assert!((hue - 330.0).abs() < 1e-6, "{hue}");
    }
}
