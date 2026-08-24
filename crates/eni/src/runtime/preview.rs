//! Render a `WorldChunk` as a PNG, colored by each tile's dominant substance.

use std::{fs, io::Cursor, path::Path};

use eni_domain::{CHUNK_SIZE_U32, Composition, WorldChunk};
use image::{ImageFormat, Rgb, RgbImage};

use super::RuntimeError;

const SCALE: u32 = 8;

pub(crate) fn render_chunk_png(chunk: &WorldChunk) -> Result<Vec<u8>, RuntimeError> {
    let image = render_chunk_image(chunk);
    let mut bytes = Cursor::new(Vec::new());
    image.write_to(&mut bytes, ImageFormat::Png)?;
    Ok(bytes.into_inner())
}

pub(crate) fn save_world_png(chunk: &WorldChunk, output: &Path) -> Result<(), RuntimeError> {
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    render_chunk_image(chunk).save(output)?;
    Ok(())
}

/// Fallback color when a tile's dominant substance is not one of the known
/// starter substances (e.g. custom data files); visibly "unknown" magenta.
const UNKNOWN_COLOR: [u8; 3] = [230, 30, 230];

pub(crate) fn color_for_tile(tile: &Composition) -> [u8; 3] {
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
        _ => UNKNOWN_COLOR,
    }
}

fn render_chunk_image(chunk: &WorldChunk) -> RgbImage {
    let size = CHUNK_SIZE_U32;
    let mut image = RgbImage::new(size * SCALE, size * SCALE);
    for (lx, ly, tile) in chunk.tiles.iter() {
        let color = color_for_tile(tile);
        for y in ly * SCALE..(ly + 1) * SCALE {
            for x in lx * SCALE..(lx + 1) * SCALE {
                image.put_pixel(x, y, Rgb(color));
            }
        }
    }
    image
}
