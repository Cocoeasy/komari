use super::{
    Player, PlayerAction, PlayerActionPingPong, PlayerState,
    actions::{on_action, on_auto_mob_use_key_action, on_ping_pong_double_jump_action},
    moving::Moving,
    state::LastMovement,
};
use crate::{
    context::Context,
    player::{
        MOVE_TIMEOUT,
        timeout::{ChangeAxis, update_moving_axis_context},
    },
};

/// Minimum y distance from the destination required to perform a grappling hook.
pub const GRAPPLING_THRESHOLD: i32 = 24;

/// Maximum y distance from the destination allowed to perform a grappling hook.
pub const GRAPPLING_MAX_THRESHOLD: i32 = 41;

/// Timeout for grappling.
const TIMEOUT: u32 = MOVE_TIMEOUT * 8;

/// Timeout after stopping grappling.
const STOPPING_TIMEOUT: u32 = MOVE_TIMEOUT + 3;

/// Maximum y distance allowed to stop grappling.
const STOPPING_THRESHOLD: i32 = 3;

/// Updates the [`Player::Grappling`] contextual state.
///
/// This state can only be transitioned via [`Player::Moving`] or [`Player::DoubleJumping`]
/// when the player has reached or close to the destination x-wise.
///
/// This state will use the Rope Lift skill.
pub fn update_grappling_context(
    context: &Context,
    state: &mut PlayerState,
    moving: Moving,
) -> Player {
    update_moving_axis_context(
        moving,
        state.last_known_pos.expect("in positional context"),
        TIMEOUT,
        move |moving| {
            let key = state
                .config
                .grappling_key
                .expect("cannot transition if not set");
            let _ = context.keys.send(key);
            state.last_movement = Some(LastMovement::Grappling);

            Player::Grappling(moving)
        },
        None::<fn()>,
        move |mut moving| {
            let key = state
                .config
                .grappling_key
                .expect("cannot transition if not set");
            let cur_pos = moving.pos;
            let (y_distance, y_direction) = moving.y_distance_direction_from(true, cur_pos);
            let x_changed = cur_pos.x != moving.pos.x;

            if moving.timeout.current >= MOVE_TIMEOUT && x_changed {
                // during double jump and grappling failed
                moving = moving.timeout_current(TIMEOUT).completed(true);
            }
            if !moving.completed {
                if y_direction <= 0 || y_distance <= stopping_threshold(state.velocity.1) {
                    let _ = context.keys.send(key);
                    moving = moving.completed(true);
                }
            } else if moving.timeout.current >= STOPPING_TIMEOUT {
                moving = moving.timeout_current(TIMEOUT);
            }

            on_action(
                state,
                |action| match action {
                    PlayerAction::AutoMob(_) => {
                        if moving.completed && moving.is_destination_intermediate() {
                            return Some((
                                Player::Moving(moving.dest, moving.exact, moving.intermediates),
                                false,
                            ));
                        }
                        let (x_distance, _) = moving.x_distance_direction_from(false, cur_pos);
                        let (y_distance, _) = moving.y_distance_direction_from(false, cur_pos);
                        on_auto_mob_use_key_action(context, action, cur_pos, x_distance, y_distance)
                    }
                    PlayerAction::PingPong(PlayerActionPingPong {
                        bound, direction, ..
                    }) => {
                        if cur_pos.y >= bound.y
                            && context.rng.random_perlin_bool(
                                cur_pos.x,
                                cur_pos.y,
                                context.tick,
                                0.7,
                            )
                        {
                            Some(on_ping_pong_double_jump_action(
                                context, cur_pos, bound, direction,
                            ))
                        } else {
                            None
                        }
                    }
                    PlayerAction::Key(_) | PlayerAction::Move(_) | PlayerAction::SolveRune => None,
                    PlayerAction::Panic(_) | PlayerAction::FamiliarsSwapping(_) => unreachable!(),
                },
                || Player::Grappling(moving),
            )
        },
        ChangeAxis::Vertical,
    )
}

/// Converts vertical velocity to a stopping threshold.
#[inline]
fn stopping_threshold(velocity: f32) -> i32 {
    (STOPPING_THRESHOLD as f32 + 1.1 * velocity).ceil() as i32
}
