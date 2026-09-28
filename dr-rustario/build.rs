//! Halves the particle theme's Dr. sheets into `OUT_DIR` for the `portmaster`, `browser` and
//! `android` builds, which `theme/modern/mod.rs` includes from. At full size they cost most of
//! a 1 GiB handheld's memory at startup and exceed a Mali G31's 4096 pixel texture limit.

/// where the sheets are, relative to the crate root
const SHEET_DIR: &str = "src/theme/modern/dr";

/// the sheets to halve; the theme includes them back out of `OUT_DIR` under the same names
const SHEETS: [&str; 4] = ["game-over.png", "idle.png", "throw.png", "victory.png"];

fn main() {
    for sheet in SHEETS {
        println!("cargo:rerun-if-changed={SHEET_DIR}/{sheet}");
    }
    #[cfg(any(feature = "portmaster", feature = "browser", feature = "android"))]
    halved::write_all();
}

#[cfg(any(feature = "portmaster", feature = "browser", feature = "android"))]
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
        // The theme divides a sheet by its declared grid, so an even sheet keeps every frame
        // origin even and a 2x2 average never bleeds between frames; a wider filter would.
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

    /// The alpha-weighted mean of the 2x2 block at `(x, y)`. Transparent pixels still carry the
    /// sheets' green matte colour, so weighting by alpha keeps it out of the Dr.'s edges.
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
            // transparent black compresses better than the matte
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
