//! Client-side mirror of streamed chunk data, and player input/position sync.

use std::collections::HashMap;

use bevy::prelude::*;
use eni_domain::{
    ChunkCoord, ChunkData, Composition, PlayerMoveIntent, PlayerPosition, UnloadChunk,
};

#[derive(Component)]
pub struct PlayerCharacter;

/// The client's local copy of loaded chunk tile data, kept in sync with the
/// authoritative server via `ChunkData`/`UnloadChunk` messages.
#[derive(Resource, Default)]
pub struct ClientWorld {
    pub chunks: HashMap<ChunkCoord, Vec<Composition>>,
}

pub fn receive_chunks(mut world: ResMut<ClientWorld>, mut reader: MessageReader<ChunkData>) {
    for chunk in reader.read() {
        world.chunks.insert(chunk.chunk_coord, chunk.tiles.clone());
    }
}

pub fn receive_unloads(mut world: ResMut<ClientWorld>, mut reader: MessageReader<UnloadChunk>) {
    for unload in reader.read() {
        world.chunks.remove(&unload.chunk_coord);
    }
}

pub fn apply_player_position(
    mut reader: MessageReader<PlayerPosition>,
    mut query: Query<&mut Transform, With<PlayerCharacter>>,
) {
    for pos in reader.read() {
        for mut transform in &mut query {
            transform.translation.x = pos.x;
            transform.translation.y = pos.y;
        }
    }
}

pub fn read_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut writer: MessageWriter<PlayerMoveIntent>,
) {
    let mut direction = Vec2::ZERO;
    if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
        direction.y += 1.0;
    }
    if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
        direction.y -= 1.0;
    }
    if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
        direction.x += 1.0;
    }
    if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
        direction.x -= 1.0;
    }
    if direction != Vec2::ZERO {
        writer.write(PlayerMoveIntent {
            direction: direction.normalize(),
            delta_seconds: time.delta_secs(),
        });
    }
}
