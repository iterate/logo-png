use std::error::Error;
use std::mem;

use lazy_static::lazy_static;
use parking_lot::RwLock;
use serde::Deserialize;

use crate::{db, live};

lazy_static! {
    // Last logo fetched from the api
    static ref LOGO_CACHE: RwLock<LogoResponse> = RwLock::new(LogoResponse { logo: vec![] });
}

#[derive(Debug, Deserialize, Copy, Clone, Default)]
pub struct LogoOptions {
    size: Option<u32>,
    character: Option<usize>,
    #[serde(default)]
    crop: bool,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct LogoResponse {
    logo: Vec<Vec<Vec<String>>>,
}

pub struct Logo {
    width: usize,
    height: usize,
    data: Vec<u8>,
}

pub fn update_logo() -> Result<(), Box<dyn Error>> {
    let live_logo: LogoResponse = reqwest::get("https://logo-api.g2.iterate.no/logo")?.json()?;
    let old_logo = LOGO_CACHE.read();

    if live_logo != *old_logo {
        // Avoid deadlock
        drop(old_logo);

        let mut logo_cache = LOGO_CACHE.write();
        drop(mem::replace(&mut *logo_cache, live_logo));

        // Avoid deadlock
        drop(logo_cache);

        let logo_png = get_logo_png(LogoOptions::default())?;

        live::send_update(&logo_png);
        if let Err(err) = db::save_logo(&logo_png) {
            eprintln!("Error saving logo to db: {}", err);
        }
    }

    Ok(())
}
pub fn get_logo_png(options: LogoOptions) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut result = Vec::new();
    let logo = get_logo_data(options)?; // An array containing a RGBA sequence. First pixel is red and second pixel is black.

    {
        let mut encoder = png::Encoder::new(&mut result, logo.width as u32, logo.height as u32); // Width is 2 pixels and height is 1.
        encoder.set_color(png::ColorType::RGBA);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();

        writer.write_image_data(&logo.data).unwrap(); // Save
    }

    Ok(result)
}

fn get_logo_data(options: LogoOptions) -> Result<Logo, Box<dyn Error>> {
    let pixel_size = options.size.unwrap_or(1) as usize;
    let live_logo = LOGO_CACHE.read();

    match options.character {
        None => {
            let height = 32 * pixel_size as usize;
            let width = 152 * pixel_size as usize;

            let mut image = vec![0; width * height * 4];

            for (char_index, chr) in live_logo.logo.iter().enumerate() {
                let x = if char_index == 0 {
                    0
                } else {
                    (char_index * 3 - 2) * 8
                };
                write_character(&chr, char_index, pixel_size, width, &mut image, x as i32, 0)?;
            }
            Ok(Logo {
                width,
                height,
                data: image,
            })
        }
        Some(character) => {
            let y: i32 = if options.crop {
                if character == 0 || character == 1 || character == 5 {
                    0
                } else {
                    -8
                }
            } else {
                0
            };

            let height = (y + 32) as usize * pixel_size;
            let width = if character == 0 {
                8 * pixel_size as usize
            } else {
                8 * 3 * pixel_size as usize
            };

            let mut image = vec![0; width * height * 4];

            let chr = live_logo
                .logo
                .get(character)
                .ok_or_else(|| format!("{} is not a valid character", character))?;

            write_character(&chr, character, pixel_size, width, &mut image, 0, y)?;

            Ok(Logo {
                width,
                height,
                data: image,
            })
        }
    }
}

fn write_character(
    chr: &Vec<Vec<String>>,
    char_index: usize,
    pixel_size: usize,
    width: usize,
    image: &mut Vec<u8>,
    letter_x: i32,
    letter_y: i32,
) -> Result<(), Box<dyn Error>> {
    let coords = vec![
        vec![[0, 0], [0, 16], [0, 24], [0, 32]],
        vec![[0, 0], [0, 8], [8, 8], [0, 16], [0, 24], [8, 24], [16, 24]],
        vec![
            [0, 8],
            [8, 8],
            [16, 8],
            [0, 16],
            [16, 16],
            [0, 24],
            [8, 24],
            [16, 24],
        ],
        vec![[0, 8], [8, 8], [16, 8], [0, 16], [0, 24]],
        vec![
            [8, 8],
            [16, 8],
            [0, 16],
            [16, 16],
            [0, 24],
            [8, 24],
            [16, 24],
        ],
        vec![[0, 0], [0, 8], [8, 8], [0, 16], [0, 24], [8, 24], [16, 24]],
        vec![[0, 8], [8, 8], [16, 8], [0, 16], [16, 16], [0, 24], [8, 24]],
    ];

    for (panel_index, panel) in chr.iter().enumerate() {
        for (pixel_index, pixel) in panel.iter().enumerate() {
            let panel_x = pixel_index % 8;
            let panel_y = pixel_index / 8;
            let x = ((coords[char_index][panel_index][0] + panel_x) as i32 + letter_x) as usize;
            let y = ((coords[char_index][panel_index][1] + panel_y) as i32 + letter_y) as usize;

            for extra_x in 0..pixel_size {
                for extra_y in 0..pixel_size {
                    let x = extra_x + (x * pixel_size);
                    let y = extra_y + (y * pixel_size);

                    let (r, g, b) = if pixel.len() == 7 {
                        (
                            u8::from_str_radix(&pixel[1..3], 16)?,
                            u8::from_str_radix(&pixel[3..5], 16)?,
                            u8::from_str_radix(&pixel[5..7], 16)?,
                        )
                    } else if pixel.len() == 6 {
                        (
                            u8::from_str_radix(&pixel[0..2], 16)?,
                            u8::from_str_radix(&pixel[2..4], 16)?,
                            u8::from_str_radix(&pixel[4..6], 16)?,
                        )
                    } else {
                        (155, 155, 155)
                    };

                    let image_idx = (x + y * width) * 4;

                    image[image_idx] = r;
                    image[image_idx + 1] = g;
                    image[image_idx + 2] = b;
                    image[image_idx + 3] = 255;
                }
            }
        }
    }

    Ok(())
}
