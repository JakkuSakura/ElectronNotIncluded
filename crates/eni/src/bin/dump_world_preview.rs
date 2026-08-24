//! Dump a generated chunk's substance grid to a PNG, colored by dominant substance.

use std::path::PathBuf;

use eni_domain::{ChunkCoord, DataError, GameData, generate_chunk};
use image::{ImageError, Rgb, RgbImage};
use thiserror::Error;

const SCALE: u32 = 16;

#[derive(Debug, Error)]
enum PreviewError {
    #[error("failed to load game data: {0}")]
    Data(#[from] DataError),
    #[error("failed to create preview directory: {0}")]
    Directory(#[from] std::io::Error),
    #[error("failed to save PNG: {0}")]
    Image(#[from] ImageError),
}

fn main() -> Result<(), PreviewError> {
    let mut args = std::env::args().skip(1);
    let mut seed: Option<u32> = None;
    let mut output = PathBuf::from("target/preview/world_preview.png");
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--seed" => {
                if let Some(v) = args.next() {
                    seed = v.parse().ok();
                }
            }
            other => {
                output = PathBuf::from(other);
            }
        }
    }
    let seed = seed.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as u32
    });
    println!("using seed: {seed}");

    let game_data = GameData::load("assets/data")?;
    let chunk = generate_chunk(seed, ChunkCoord { x: 0, y: 0 }, &game_data.substances);

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let size = eni_domain::CHUNK_SIZE_U32;
    let mut image = RgbImage::new(size * SCALE, size * SCALE);
    for (lx, ly, tile) in chunk.tiles.iter() {
        let color = color_for_tile(tile);
        for y in ly * SCALE..(ly + 1) * SCALE {
            for x in lx * SCALE..(lx + 1) * SCALE {
                image.put_pixel(x, y, Rgb(color));
            }
        }
    }
    image.save(&output)?;
    println!(
        "wrote chunk preview: {} ({}x{} px)",
        output.display(),
        size * SCALE,
        size * SCALE
    );
    Ok(())
}

fn color_for_tile(tile: &eni_domain::Composition) -> [u8; 3] {
    let Some((id, _)) = tile
        .mass_kg
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
    else {
        return [10, 10, 15];
    };
    match id.0.as_str() {
        "air" => [200, 220, 240],
        "water" => [40, 100, 210],
        "nacl" => [225, 225, 215],
        "rock" => [100, 95, 90],
        "dirt" => [115, 80, 50],
        _ => [230, 30, 230],
    }
}
