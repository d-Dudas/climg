use image::ImageReader;
use image::Luma;
use image::{DynamicImage, ImageBuffer};
use std::env;

fn get_terminal_size() -> std::result::Result<(u16, u16), std::io::Error> {
    use crossterm::terminal::size;
    let (cols, rows) = size()?;
    Ok((cols, rows))
}

fn to_grayscale_luma8(img: DynamicImage) -> ImageBuffer<Luma<u8>, Vec<u8>> {
    img.to_luma8()
}

fn otsu_threshold(img: &ImageBuffer<Luma<u8>, Vec<u8>>) -> u8 {
    let mut hist = [0u32; 256];
    for Luma([v]) in img.pixels() {
        hist[*v as usize] += 1;
    }

    let total: u32 = img.width() * img.height();
    if total == 0 {
        return 128;
    }

    let mut sum_total: f64 = 0.0;
    for (i, &h) in hist.iter().enumerate() {
        sum_total += (i as f64) * (h as f64);
    }

    let mut sum_b: f64 = 0.0;
    let mut w_b: f64 = 0.0;
    let mut w_f: f64;
    let mut max_var: f64 = -1.0;
    let mut threshold: u8 = 0;

    for (t, &h) in hist.iter().enumerate() {
        w_b += h as f64;
        if w_b == 0.0 {
            continue;
        }
        w_f = (total as f64) - w_b;
        if w_f == 0.0 {
            break;
        }
        sum_b += (t as f64) * (h as f64);

        let m_b = sum_b / w_b;
        let m_f = (sum_total - sum_b) / w_f;

        let var_between = w_b * w_f * (m_b - m_f) * (m_b - m_f);
        if var_between > max_var {
            max_var = var_between;
            threshold = t as u8;
        }
    }

    threshold
}

#[inline]
fn bit_if_on(img: &ImageBuffer<Luma<u8>, Vec<u8>>, x: u32, y: u32, t: u8, invert: bool) -> u8 {
    if x >= img.width() || y >= img.height() {
        return 0;
    }
    let v = img.get_pixel(x, y)[0];
    let on = if invert { v < t } else { v >= t };
    if on { 1 } else { 0 }
}

fn fit_image(img: &DynamicImage) -> DynamicImage {
    let image_width = img.width();
    let image_height = img.height();

    let (mut terminal_width, mut terminal_height) = get_terminal_size().unwrap_or((100, 200));
    terminal_height -= 2;
    terminal_height *= 4;
    terminal_width *= 2;
    let mut target_height = terminal_height as u32;
    let mut target_width = terminal_width as u32;
    let mut aspect = image_height as f32 / image_width as f32;

    if aspect < 1.0 {
        target_height = (target_width as f32 * aspect).round() as u32;
        if target_height > terminal_height as u32 {
            aspect = terminal_height as f32 / target_height as f32;
            target_height = (target_height as f32 * aspect).round() as u32;
            target_width = (target_width as f32 * aspect).round() as u32;
        }
    } else if aspect > 1.0 {
        target_width = (target_height as f32 * aspect).round() as u32;
        if target_width > terminal_width as u32 {
            aspect = terminal_width as f32 / target_width as f32;
            target_height = (target_height as f32 * aspect).round() as u32;
            target_width = (target_width as f32 * aspect).round() as u32;
        }
    } else {
        use std::cmp::min;
        target_height = min(target_height, target_width);
        target_width = min(target_height, target_width);
    }

    img.resize(
        target_width,
        target_height,
        image::imageops::FilterType::Lanczos3,
    )
}

fn get_image_matrix(input: String) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut img = ImageReader::open(&input)?.with_guessed_format()?.decode()?;

    img = fit_image(&img);

    let gray = to_grayscale_luma8(img);
    let invert = env::args().nth(2).as_deref() == Some("invert");

    let t = otsu_threshold(&gray);

    let (w, h) = gray.dimensions();
    for y in (0..h).step_by(4) {
        let mut line = String::with_capacity((w as usize / 2) + 8);
        for x in (0..w).step_by(2) {
            let mut bits: u8 = 0;

            bits |= bit_if_on(&gray, x, y, t, invert);
            bits |= bit_if_on(&gray, x, y + 1, t, invert) << 1;
            bits |= bit_if_on(&gray, x, y + 2, t, invert) << 2;
            bits |= bit_if_on(&gray, x + 1, y, t, invert) << 3;
            bits |= bit_if_on(&gray, x + 1, y + 1, t, invert) << 4;
            bits |= bit_if_on(&gray, x + 1, y + 2, t, invert) << 5;
            bits |= bit_if_on(&gray, x, y + 3, t, invert) << 6;
            bits |= bit_if_on(&gray, x + 1, y + 3, t, invert) << 7;

            let ch = char::from_u32(0x2800 + bits as u32).unwrap_or('\u{2800}');
            line.push(ch);
        }
        println!("{line}");
    }

    Ok(())
}

fn main() {
    let mut args = env::args().skip(1);
    let input = args.next().expect("Usage: climg <input-image> [invert]");

    if let Err(e) = get_image_matrix(input) {
        eprintln!("Error processing image: {}", e);
    }
}
