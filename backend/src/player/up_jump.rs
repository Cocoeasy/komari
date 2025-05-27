use log::debug;
use platforms::windows::KeyKind;

use super::{
    Player, PlayerActionKey, PlayerActionPingPong, PlayerState,
    actions::on_ping_pong_double_jump_action, moving::Moving, use_key::UseKey,
};
use crate::{
    ActionKeyWith,
    context::Context,
    minimap::Minimap,
    player::{
        MOVE_TIMEOUT, PlayerAction,
        actions::{on_action, on_auto_mob_use_key_action},
        state::LastMovement,
        timeout::{ChangeAxis, update_moving_axis_context},
    },
};

const SPAM_DELAY: u32 = 7;
const STOP_UP_KEY_TICK: u32 = 3;
const TIMEOUT: u32 = MOVE_TIMEOUT + 3;
const UP_JUMPED_Y_VELOCITY_THRESHOLD: f32 = 1.3;
const X_NEAR_STATIONARY_THRESHOLD: f32 = 0.28;
const TELEPORT_UP_JUMP_THRESHOLD: i32 = 14;

/// Updates the [`Player::UpJumping`] contextual state
///
/// This state can only be transitioned via [`Player::Moving`] when the
/// player has reached the destination x-wise. Before performing an up jump, it will check for
/// stationary state and whether the player is currently near a portal. If the player is near
/// a portal, this action is aborted. The up jump action is made to be adapted for various classes
/// that has different up jump key combination.
pub fn update_up_jumping_context(
    context: &Context,
    state: &mut PlayerState,
    moving: Moving,
) -> Player {
    let cur_pos = state.last_known_pos.unwrap();
    let (y_distance, y_direction) = moving.y_distance_direction_from(true, cur_pos);
    let up_jump_key = state.config.upjump_key;
    let has_teleport_key = state.config.teleport_key.is_some();

    if !moving.timeout.started {
        if state.velocity.0 > X_NEAR_STATIONARY_THRESHOLD {
            return Player::UpJumping(moving);
        }
        if let Minimap::Idle(idle) = context.minimap {
            for portal in idle.portals {
                if portal.x <= cur_pos.x
                    && cur_pos.x < portal.x + portal.width
                    && portal.y >= cur_pos.y
                    && portal.y - portal.height < cur_pos.y
                {
                    debug!(target: "player", "abort action due to potential map moving");
                    state.clear_action_completed();
                    return Player::Idle;
                }
            }
        }
        state.last_movement = Some(LastMovement::UpJumping);
    }

    let jump_key = state.config.jump_key;
    update_moving_axis_context(
        moving,
        cur_pos,
        TIMEOUT,
        |moving| {
            // Only send Up key when the key is not of a Demon Slayer
            if !matches!(up_jump_key, Some(KeyKind::Up)) {
                let _ = context.keys.send_down(KeyKind::Up);
            }
            match (up_jump_key, has_teleport_key) {
                // This is a generic class, a mage or a Demon Slayer
                (None, _) | (Some(_), true) | (Some(KeyKind::Up), false) => {
                    // This if is for mage. It means if the player is a mage and the y distance
                    // is less than `TELEPORT_UP_JUMP_THRESHOLD`, do not send jump key.
                    if !can_mage_skip_jump_key(up_jump_key, has_teleport_key, y_distance) {
                        let _ = context.keys.send(jump_key);
                    }
                }
                _ => (),
            }
            Player::UpJumping(moving)
        },
        Some(|| {
            let _ = context.keys.send_up(KeyKind::Up);
        }),
        |mut moving| {
            match (moving.completed, up_jump_key, has_teleport_key) {
                (false, None, true) | (false, Some(KeyKind::Up), false) | (false, None, false) => {
                    if state.velocity.1 <= UP_JUMPED_Y_VELOCITY_THRESHOLD {
                        // Spam jump key until the player y changes
                        // above a threshold as sending jump key twice
                        // doesn't work
                        if moving.timeout.total >= SPAM_DELAY {
                            // This up jump key is Up for Demon Slayer
                            if let Some(key) = up_jump_key {
                                let _ = context.keys.send(key);
                            } else {
                                let _ = context.keys.send(jump_key);
                            }
                        }
                    } else {
                        moving = moving.completed(true);
                    }
                }
                (false, Some(key), _) => {
                    // If the player is a mage and y distance is less
                    // than `TELEPORT_UP_JUMP_THRESHOLD`, send the teleport key immediately.
                    if !has_teleport_key
                        || (y_distance <= TELEPORT_UP_JUMP_THRESHOLD
                            || moving.timeout.total >= SPAM_DELAY)
                    {
                        let _ = context.keys.send(key);
                        moving = moving.completed(true);
                    }
                }
                (true, _, _) => {
                    // This is when up jump like Blaster or mage still requires up key
                    // cancel early to avoid stucking to a rope
                    if up_jump_key.is_some() && moving.timeout.total == STOP_UP_KEY_TICK {
                        let _ = context.keys.send_up(KeyKind::Up);
                    }
                }
            }

            on_action(
                state,
                |action| match action {
                    PlayerAction::AutoMob(_) => {
                        if moving.completed
                            && moving.is_destination_intermediate()
                            && y_direction <= 0
                        {
                            let _ = context.keys.send_up(KeyKind::Up);
                            return Some((
                                Player::Moving(moving.dest, moving.exact, moving.intermediates),
                                false,
                            ));
                        }
                        let (x_distance, _) = moving.x_distance_direction_from(false, cur_pos);
                        let (y_distance, _) = moving.y_distance_direction_from(false, cur_pos);
                        on_auto_mob_use_key_action(context, action, cur_pos, x_distance, y_distance)
                    }
                    PlayerAction::Key(PlayerActionKey {
                        with: ActionKeyWith::Any,
                        ..
                    }) => {
                        if !moving.completed || y_direction > 0 {
                            return None;
                        }
                        Some((Player::UseKey(UseKey::from_action(action)), false))
                    }
                    PlayerAction::PingPong(PlayerActionPingPong {
                        bound, direction, ..
                    }) => {
                        if moving.completed
                            && moving.timeout.total % MOVE_TIMEOUT == 0 // Interval roll dice
                            && rand::random_bool(0.7)
                        {
                            Some(on_ping_pong_double_jump_action(
                                context, cur_pos, bound, direction,
                            ))
                        } else {
                            None
                        }
                    }
                    PlayerAction::Key(PlayerActionKey {
                        with: ActionKeyWith::Stationary | ActionKeyWith::DoubleJump,
                        ..
                    })
                    | PlayerAction::Move(_)
                    | PlayerAction::SolveRune => None,
                },
                || Player::UpJumping(moving),
            )
        },
        ChangeAxis::Vertical,
    )
}

#[inline]
fn can_mage_skip_jump_key(
    up_jump_key: Option<KeyKind>,
    has_teleport_key: bool,
    y_distance: i32,
) -> bool {
    // It means if the player is a mage and the y distance
    // is less than `TELEPORT_UP_JUMP_THRESHOLD`, do not send jump key or wait for stationary.
    up_jump_key.is_some() && has_teleport_key && y_distance <= TELEPORT_UP_JUMP_THRESHOLD
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;

    use opencv::core::Point;
    use platforms::windows::KeyKind;

    use super::{Moving, PlayerState, update_up_jumping_context};
    use crate::{
        bridge::MockKeySender,
        context::Context,
        player::{Player, Timeout},
    };

    #[test]
    fn up_jumping_start() {
        let pos = Point::new(5, 5);
        let moving = Moving {
            pos,
            dest: Point::new(5, 20),
            ..Default::default()
        };
        let mut state = PlayerState::default();
        let mut context = Context::new(None, None);
        state.config.jump_key = KeyKind::Space;
        state.last_known_pos = Some(pos);
        state.is_stationary = true;

        let mut keys = MockKeySender::new();
        keys.expect_send_down()
            .withf(|key| matches!(key, KeyKind::Up))
            .returning(|_| Ok(()))
            .once();
        keys.expect_send()
            .withf(|key| matches!(key, KeyKind::Space))
            .returning(|_| Ok(()))
            .once();
        context.keys = Box::new(keys);
        // Space + Up only
        update_up_jumping_context(&context, &mut state, moving);
        let _ = context.keys; // drop mock for validation

        state.config.upjump_key = Some(KeyKind::C);
        let mut keys = MockKeySender::new();
        keys.expect_send_down()
            .withf(|key| matches!(key, KeyKind::Up))
            .once()
            .returning(|_| Ok(()));
        keys.expect_send()
            .withf(|key| matches!(key, KeyKind::Space))
            .never()
            .returning(|_| Ok(()));
        context.keys = Box::new(keys);
        // Up only
        update_up_jumping_context(&context, &mut state, moving);
        let _ = context.keys; // drop mock for validation

        state.config.teleport_key = Some(KeyKind::Shift);
        let mut keys = MockKeySender::new();
        keys.expect_send_down()
            .withf(|key| matches!(key, KeyKind::Up))
            .once()
            .returning(|_| Ok(()));
        keys.expect_send()
            .withf(|key| matches!(key, KeyKind::Space))
            .once()
            .returning(|_| Ok(()));
        context.keys = Box::new(keys);
        // Space + Up
        update_up_jumping_context(&context, &mut state, moving);
        let _ = context.keys; // drop mock for validation
    }

    #[test]
    fn up_jumping_update() {
        let moving_pos = Point::new(7, 1);
        let moving = Moving {
            pos: moving_pos,
            timeout: Timeout {
                started: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut state = PlayerState::default();
        state.last_known_pos = Some(Point::new(7, 7));
        state.velocity = (0.0, 1.36);
        let context = Context::new(None, None);

        // up jumped because y velocity > 1.35
        assert_matches!(
            update_up_jumping_context(&context, &mut state, moving),
            Player::UpJumping(Moving {
                timeout: Timeout {
                    current: 1,
                    total: 1,
                    ..
                },
                completed: true,
                ..
            })
        );
    }

    #[test]
    fn up_jump_demon_slayer() {
        let pos = Point::new(10, 10);
        let dest = Point::new(10, 20);
        let mut moving = Moving {
            pos,
            dest,
            ..Default::default()
        };
        let mut state = PlayerState::default();
        state.config.upjump_key = Some(KeyKind::Up); // Demon Slayer uses Up
        state.config.jump_key = KeyKind::Space;
        state.last_known_pos = Some(pos);
        state.is_stationary = true;

        let mut keys = MockKeySender::new();
        keys.expect_send_down()
            .withf(|key| *key == KeyKind::Up)
            .never();
        keys.expect_send()
            .withf(|key| *key == KeyKind::Space)
            .once()
            .returning(|_| Ok(()));
        let mut context = Context::new(None, None);
        context.keys = Box::new(keys);

        // Start by sending Space only
        update_up_jumping_context(&context, &mut state, moving);
        let _ = context.keys;

        // Update by sending Up
        let mut keys = MockKeySender::new();
        moving.timeout.total = 7; // SPAM_DELAY
        moving.timeout.started = true;
        keys.expect_send()
            .withf(|key| *key == KeyKind::Up)
            .times(2)
            .returning(|_| Ok(()));
        keys.expect_send()
            .withf(|key| *key == KeyKind::Space)
            .never();
        context.keys = Box::new(keys);
        update_up_jumping_context(&context, &mut state, moving);
        update_up_jumping_context(&context, &mut state, moving);
        let _ = context.keys;
    }

    #[test]
    fn up_jump_mage() {
        let pos = Point::new(10, 10);
        let dest = Point::new(10, 30);
        let mut moving = Moving {
            pos,
            dest,
            ..Default::default()
        };
        let mut state = PlayerState::default();
        // Setting up jump key the same as teleport key
        // means that the mage doesn't have a dedicated up jump like up arrow + space
        state.config.upjump_key = Some(KeyKind::Shift);
        state.config.teleport_key = Some(KeyKind::Shift);
        state.config.jump_key = KeyKind::Space;
        state.last_known_pos = Some(pos);
        state.is_stationary = true;

        let mut keys = MockKeySender::new();
        keys.expect_send_down()
            .withf(|key| *key == KeyKind::Up)
            .once()
            .returning(|_| Ok(()));
        keys.expect_send()
            .withf(|key| *key == KeyKind::Space)
            .once()
            .returning(|_| Ok(()));
        let mut context = Context::new(None, None);
        context.keys = Box::new(keys);

        // Start by sending Up and Space
        update_up_jumping_context(&context, &mut state, moving);
        let _ = context.keys;

        // Change to started
        moving.timeout.started = true;

        // Not sending any key before delay
        let mut keys = MockKeySender::new();
        moving.timeout.total = 4; // Before SPAM_DELAY
        keys.expect_send().never();
        context.keys = Box::new(keys);
        assert_matches!(
            update_up_jumping_context(&context, &mut state, moving),
            Player::UpJumping(Moving {
                completed: false,
                ..
            })
        );
        let _ = context.keys;

        // Send key after delay
        let mut keys = MockKeySender::new();
        moving.timeout.total = 7; // At SPAM_DELAY
        keys.expect_send()
            .withf(|key| *key == KeyKind::Shift)
            .once()
            .returning(|_| Ok(()));
        context.keys = Box::new(keys);
        assert_matches!(
            update_up_jumping_context(&context, &mut state, moving),
            Player::UpJumping(Moving {
                completed: true,
                ..
            })
        );
        let _ = context.keys;
    }
}
