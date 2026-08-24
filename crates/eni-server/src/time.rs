use bevy::prelude::*;
use eni_domain::{GameClock, GamePaused};

pub(crate) fn advance_real_time(
    time: Res<Time<Real>>,
    paused: Res<GamePaused>,
    mut game_clock: ResMut<GameClock>,
) {
    if paused.0 {
        return;
    }
    game_clock.advance_real_seconds(time.delta_secs_f64());
}
