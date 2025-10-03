use super::{
    PingPong, Player, PlayerAction,
    actions::{update_from_auto_mob_action, update_from_ping_pong_action},
    state::LastMovement,
    timeout::{MovingLifecycle, next_moving_lifecycle_with_axis},
};
use crate::{
    ecs::Resources,
    minimap::Minimap,
    player::{MOVE_TIMEOUT, PlayerEntity, next_action, timeout::ChangeAxis},
    transition, transition_if, transition_to_moving, transition_to_moving_if,
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
pub fn update_grappling_state(
    resources: &Resources,
    player: &mut PlayerEntity,
    minimap_state: Minimap,
) {
    let Player::Grappling(moving) = player.state else {
        panic!("state is not grappling");
    };
    let key = player
        .context
        .config
        .grappling_key
        .expect("cannot transition if not set");
    let prev_pos = moving.pos;

    match next_moving_lifecycle_with_axis(
        moving,
        player.context.last_known_pos.expect("in positional state"),
        TIMEOUT,
        ChangeAxis::Vertical,
    ) {
        MovingLifecycle::Started(moving) => transition!(player, Player::Grappling(moving), {
            player.context.last_movement = Some(LastMovement::Grappling);
            resources.input.send_key(key);
        }),
        MovingLifecycle::Ended(moving) => transition_to_moving!(player, moving),
        MovingLifecycle::Updated(mut moving) => {
            let cur_pos = moving.pos;
            let (y_distance, y_direction) = moving.y_distance_direction_from(true, cur_pos);
            let x_changed = prev_pos.x != cur_pos.x;

            if moving.timeout.current >= MOVE_TIMEOUT && x_changed {
                // During double jump and grappling failed
                moving.timeout.current = TIMEOUT;
                moving.completed = true;
            }
            if !moving.completed {
                if y_direction <= 0 || y_distance <= stopping_threshold(player.context.velocity.1) {
                    resources.input.send_key(key);
                    moving.completed = true;
                }
            } else if moving.timeout.current >= STOPPING_TIMEOUT {
                moving.timeout.current = TIMEOUT;
            }
            // Sets initial next state first
            player.state = Player::Grappling(moving);

            let action = next_action(&player.context);
            match action {
                Some(PlayerAction::AutoMob(_)) => {
                    transition_if!(!moving.completed);
                    transition_to_moving_if!(player, moving, moving.is_destination_intermediate());
                    transition_if!(
                        player.context.config.teleport_key.is_some() && !moving.completed
                    );

                    let (x_distance, _) = moving.x_distance_direction_from(false, cur_pos);
                    let (y_distance, _) = moving.y_distance_direction_from(false, cur_pos);
                    update_from_auto_mob_action(
                        resources,
                        player,
                        minimap_state,
                        action.expect("must be some"),
                        false,
                        cur_pos,
                        x_distance,
                        y_distance,
                    )
                }
                Some(PlayerAction::PingPong(PingPong {
                    bound, direction, ..
                })) => {
                    transition_if!(
                        cur_pos.y < bound.y
                            || !resources.rng.random_perlin_bool(
                                cur_pos.x,
                                cur_pos.y,
                                resources.tick,
                                0.7,
                            )
                    );
                    update_from_ping_pong_action(
                        resources,
                        player,
                        minimap_state,
                        cur_pos,
                        bound,
                        direction,
                    );
                }
                None
                | Some(PlayerAction::Key(_) | PlayerAction::Move(_) | PlayerAction::SolveRune) => {}
                _ => unreachable!(),
            }
        }
    }
}

/// Converts vertical velocity to a stopping threshold.
#[inline]
fn stopping_threshold(velocity: f32) -> i32 {
    (STOPPING_THRESHOLD as f32 + 1.07 * velocity).round() as i32
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;

    use mockall::predicate::eq;
    use opencv::core::Point;

    use super::*;
    use crate::{
        bridge::{KeyKind, MockInput},
        player::{PlayerContext, moving::Moving, timeout::Timeout},
    };

    const POS: Point = Point { x: 100, y: 100 };

    fn mock_player_entity_with_grapple(pos: Point) -> PlayerEntity {
        let mut context = PlayerContext::default();
        context.last_known_pos = Some(pos);
        context.config.grappling_key = Some(KeyKind::F);

        PlayerEntity {
            state: Player::Idle,
            context,
        }
    }

    fn mock_moving(pos: Point) -> Moving {
        Moving::new(pos, pos, false, None)
    }

    #[test]
    fn update_grappling_state_started() {
        let moving = mock_moving(POS);
        let mut player = mock_player_entity_with_grapple(POS);
        player.state = Player::Grappling(moving);

        let mut keys = MockInput::new();
        keys.expect_send_key().once().with(eq(KeyKind::F));
        let resources = Resources::new(Some(keys), None);

        update_grappling_state(&resources, &mut player, Minimap::Detecting);

        assert_matches!(
            player.state,
            Player::Grappling(Moving {
                timeout: Timeout { started: true, .. },
                ..
            })
        );
        assert_eq!(player.context.last_movement, Some(LastMovement::Grappling));
    }

    #[test]
    fn update_grappling_state_updated_timeout_x_changed() {
        let mut moving = mock_moving(Point::new(POS.x + 10, POS.y)); // x changed
        moving.timeout.current = MOVE_TIMEOUT;
        moving.timeout.started = true;
        let mut player = mock_player_entity_with_grapple(POS);
        player.state = Player::Grappling(moving);

        let resources = Resources::new(None, None);

        update_grappling_state(&resources, &mut player, Minimap::Detecting);

        assert_matches!(
            player.state,
            Player::Grappling(Moving {
                completed: true,
                timeout: Timeout {
                    current: TIMEOUT,
                    ..
                },
                ..
            })
        );
    }

    #[test]
    fn update_grappling_state_updated_auto_complete_on_stopping_threshold() {
        let mut moving = mock_moving(Point::new(100, 100));
        moving.timeout.started = true;
        moving.timeout.current = STOPPING_TIMEOUT;
        let mut player = mock_player_entity_with_grapple(moving.pos);
        player.state = Player::Grappling(moving);

        let mut keys = MockInput::new();
        keys.expect_send_key().once().with(eq(KeyKind::F));
        let resources = Resources::new(Some(keys), None);

        update_grappling_state(&resources, &mut player, Minimap::Detecting);
        assert_matches!(
            player.state,
            Player::Grappling(Moving {
                completed: true,
                ..
            })
        );

        update_grappling_state(&resources, &mut player, Minimap::Detecting);
        assert_matches!(
            player.state,
            Player::Grappling(Moving {
                completed: true,
                timeout: Timeout {
                    current: TIMEOUT,
                    ..
                },
                ..
            })
        );
    }

    // TODO: Add tests for next_action
}
