//! Minimal 2D tile rendering: each tile is a colored square keyed by its
//! dominant substance. Polish (atlases, smooth blending, lighting) is out of
//! scope for this pass.

use bevy::prelude::*;

use crate::world_stream::{ClientWorld, PlayerCharacter};

#[derive(Component)]
pub(crate) struct TileSprite;

#[derive(Component)]
pub(crate) struct GameplayEntity;

pub(crate) fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera2d);
}

pub(crate) fn setup_playing(mut commands: Commands) {
    commands.spawn((
        PlayerCharacter,
        GameplayEntity,
        Sprite {
            color: Color::srgb(0.95, 0.85, 0.2),
            custom_size: Some(Vec2::splat(0.8)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, 10.0),
    ));
}

pub(crate) fn cleanup_playing(
    mut commands: Commands,
    entities: Query<Entity, With<GameplayEntity>>,
) {
    for entity in &entities {
        commands.entity(entity).despawn();
    }
}

/// Dominant-substance color used for tile rendering; substances not listed
/// fall back to a visibly "unknown" magenta so gaps are obvious.
fn dominant_color(tile: &eni_domain::Composition) -> Color {
    let Some((id, _)) = tile
        .mass_kg
        .iter()
        .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
    else {
        return Color::NONE;
    };
    match id.0.as_str() {
        "air" => Color::srgba(0.7, 0.85, 1.0, 0.1),
        "water" => Color::srgb(0.16, 0.4, 0.85),
        "nacl" => Color::srgb(0.9, 0.9, 0.85),
        "rock" => Color::srgb(0.4, 0.38, 0.36),
        "dirt" => Color::srgb(0.45, 0.32, 0.2),
        _ => Color::srgb(0.9, 0.1, 0.9),
    }
}

/// Re-render every loaded chunk's tiles whenever `ClientWorld` changes.
/// Crude (despawn-and-respawn) but correct, and cheap enough at chunk scale
/// for this foundation; a real tilemap renderer is future polish.
pub(crate) fn render_tiles(
    mut commands: Commands,
    world: Res<ClientWorld>,
    existing: Query<Entity, With<TileSprite>>,
) {
    if !world.is_changed() {
        return;
    }
    for entity in &existing {
        commands.entity(entity).despawn();
    }
    for (coord, tiles) in &world.chunks {
        let origin_x = coord.x * eni_domain::CHUNK_SIZE;
        let origin_y = coord.y * eni_domain::CHUNK_SIZE;
        for (index, tile) in tiles.iter().enumerate() {
            let local_x = (index % eni_domain::CHUNK_SIZE_U32 as usize) as i32;
            let local_y = (index / eni_domain::CHUNK_SIZE_U32 as usize) as i32;
            let color = dominant_color(tile);
            commands.spawn((
                TileSprite,
                GameplayEntity,
                Sprite {
                    color,
                    custom_size: Some(Vec2::splat(1.0)),
                    ..default()
                },
                Transform::from_xyz(
                    (origin_x + local_x) as f32,
                    (origin_y + local_y) as f32,
                    0.0,
                ),
            ));
        }
    }
}
