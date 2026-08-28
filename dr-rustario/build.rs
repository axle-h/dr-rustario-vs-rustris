//! Halves the particle theme's Dr. for the two builds that cannot afford him at full size.
//!
//! The particle Dr. is 591 frames of 478 pixels square spread over four sheets, and the
//! largest of them is 7170x7648 - 209 MiB once it is a texture. Every one is loaded whole and
//! then scaled down to the six and a half blocks he is actually drawn at, so a sheet is held
//! twice over while that happens, and the startup peak that costs is most of the memory a
//! 1 GiB handheld has. Three of the four are also past the 4096 pixels a Mali G31 will
//! allocate in one dimension, so they would not become textures there at all.
//!
//! A desktop is what 4k is drawn from and keeps the art as it was drawn. The `portmaster` and
//! `browser` builds get it halved into `OUT_DIR` instead, which the theme includes from
//! rather than from the source tree - see `theme/modern/mod.rs`. Neither the halving nor the
//! `image` crate it needs is compiled for a build that has not asked for it.

/// where the sheets are, relative to the crate root
const SHEET_DIR: &str = "src/theme/modern/dr";

/// the sheets to halve; the theme includes them back out of `OUT_DIR` under the same names
const SHEETS: [&str; 4] = ["game-over.png", "idle.png", "throw.png", "victory.png"];

fn main() {
    for sheet in SHEETS {
        println!("cargo:rerun-if-changed={SHEET_DIR}/{sheet}");
    }
    #[cfg(any(feature = "portmaster", feature = "browser"))]
    halved::write_all();
}

#[cfg(any(feature = "portmaster", feature = "browser"))]
mod halved {
    use super::{SHEETS, SHEET_DIR};
    use image::{ExtendedColorType, ImageEncoder, Rgba, RgbaImage};
    use std::io::BufWriter;
    use std::path::{Path, PathBuf};
    use std::{env, fs};

    pub fn write_all() {
        let dest_dir =
            PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR is set for a build script"))
                .join("dr");
        fs::create_dir_all(&dest_dir).unwrap_or_else(|e| panic!("{}: {e}", dest_dir.display()));
        for sheet in SHEETS {
            halve(&Path::new(SHEET_DIR).join(sheet), &dest_dir.join(sheet));
        }
    }

    /// Writes `source` to `dest` at half the size in each dimension.
    fn halve(source: &Path, dest: &Path) {
        let sheet = image::open(source)
            .unwrap_or_else(|e| panic!("{}: {e}", source.display()))
            .into_rgba8();
        let (width, height) = sheet.dimensions();
        // A sheet is a grid of frames, and the theme works a frame's size out by dividing the
        // sheet by the rows and columns it declares - so the halved frames have to land
        // exactly where it will look for them, and nothing may bleed out of one frame into
        // the next, which two of the four sheets draw right up against. Averaging each 2x2
        // block reads only within itself, so both hold as long as every frame origin is even,
        // and every frame origin is even as long as the sheet is. Any wider a filter would
        // have to be told the grid, which is declared in the theme and would then be declared
        // twice.
        assert!(
            width % 2 == 0 && height % 2 == 0,
            "{} is {width}x{height}: a sheet has to be even in both dimensions to halve cleanly",
            source.display()
        );
        let mut halved = RgbaImage::new(width / 2, height / 2);
        for (x, y, pixel) in halved.enumerate_pixels_mut() {
            *pixel = average(&sheet, x * 2, y * 2);
        }
        let file = fs::File::create(dest).unwrap_or_else(|e| panic!("{}: {e}", dest.display()));
        image::codecs::png::PngEncoder::new_with_quality(
            BufWriter::new(file),
            image::codecs::png::CompressionType::Best,
            image::codecs::png::FilterType::Adaptive,
        )
        .write_image(
            halved.as_raw(),
            halved.width(),
            halved.height(),
            ExtendedColorType::Rgba8,
        )
        .unwrap_or_else(|e| panic!("{}: {e}", dest.display()));
    }

    /// The mean of the 2x2 block whose top left corner is `(x, y)`, with the colours weighted
    /// by their alpha.
    ///
    /// The sheets are palette pngs standing on a flat green matte and transparent by palette
    /// index rather than by alpha, so a fully transparent pixel still carries (71, 112, 76).
    /// Averaging the colours as they come would wash that green into every edge the Dr. has;
    /// weighting each by how much of it there is leaves a transparent pixel contributing
    /// nothing. The source alpha is only ever 0 or 255 - the art is not antialiased - so the
    /// graded edges the halved sheet comes out with are new, and are the point.
    fn average(sheet: &RgbaImage, x: u32, y: u32) -> Rgba<u8> {
        let (mut color, mut alpha) = ([0u32; 3], 0u32);
        for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
            let Rgba([r, g, b, a]) = *sheet.get_pixel(x + dx, y + dy);
            color[0] += r as u32 * a as u32;
            color[1] += g as u32 * a as u32;
            color[2] += b as u32 * a as u32;
            alpha += a as u32;
        }
        if alpha == 0 {
            // nothing to take a colour from, and a transparent black compresses better than
            // the matte the source leaves lying under its transparency
            return Rgba([0, 0, 0, 0]);
        }
        let mean = |channel: u32| ((channel + alpha / 2) / alpha) as u8;
        Rgba([
            mean(color[0]),
            mean(color[1]),
            mean(color[2]),
            ((alpha + 2) / 4) as u8,
        ])
    }
}
