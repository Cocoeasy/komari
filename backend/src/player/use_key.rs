use std::assert_matches::assert_matches;

use super::{
    AutoMob, PingPongDirection, PlayerContext, Timeout,
    actions::{Key, PingPong, PlayerAction, update_from_ping_pong_action},
    double_jump::DoubleJumping,
    timeout::{Lifecycle, next_timeout_lifecycle},
};
use crate::{
    ActionKeyDirection, ActionKeyWith, Class, KeyBinding, LinkKeyBinding, Position,
    bridge::KeyKind,
    ecs::Resources,
    minimap::Minimap,
    player::{LastMovement, MOVE_TIMEOUT, Moving, Player, PlayerEntity, next_action},
    transition, transition_from_action, transition_if,
};

/// The total number of ticks for changing direction before timing out.
const CHANGE_DIRECTION_TIMEOUT: u32 = 3;

/// The tick to which the actual key will be pressed for [`LinkKeyBinding::Along`].
const LINK_ALONG_PRESS_TICK: u32 = 2;

#[derive(Clone, Copy, Debug)]
enum ActionInfo {
    AutoMobbing { should_terminate: bool },
}

/// The different states of using key.
#[derive(Clone, Copy, Debug)]
enum State {
    /// Checks whether [`ActionKeyWith`] and [`ActionKeyDirection`] are satisfied and stalls
    /// for [`UseKey::wait_before_use_ticks`].
    Precondition,
    /// Changes direction to match [`ActionKeyDirection`].
    ///
    /// Returns to [`State::Precondition`] upon timeout.
    ChangingDirection(Timeout),
    /// Ensures player double jumped or is stationary.
    ///
    /// Returns to [`State::Precondition`] if player is stationary or
    /// transfers to [`Player::DoubleJumping`].
    EnsuringUseWith,
    /// Uses the actual key with optional [`LinkKeyBinding`] and stalls
    /// for [`UseKey::wait_after_use_ticks`].
    Using(Timeout, bool),
    /// Ensures all [`UseKey::count`] times executed.
    Postcondition,
}

#[derive(Clone, Copy, Debug)]
enum PendingTransition {
    None,
    WaitBefore,
    WaitAfter,
    DoubleJump,
}

#[derive(Clone, Copy, Debug)]
pub struct UseKey {
    key: KeyBinding,
    link_key: Option<LinkKeyBinding>,
    count: u32,
    current_count: u32,
    direction: ActionKeyDirection,
    with: ActionKeyWith,
    wait_before_use_ticks: u32,
    wait_after_use_ticks: u32,
    pending_transition: PendingTransition,
    action_info: Option<ActionInfo>,
    state: State,
}

impl UseKey {
    pub fn from_key(key: Key) -> Self {
        let Key {
            key,
            link_key,
            count,
            direction,
            with,
            wait_before_use_ticks,
            wait_before_use_ticks_random_range,
            wait_after_use_ticks,
            wait_after_use_ticks_random_range,
            ..
        } = key;
        let wait_before =
            random_wait_ticks(wait_before_use_ticks, wait_before_use_ticks_random_range);
        let wait_after = random_wait_ticks(wait_after_use_ticks, wait_after_use_ticks_random_range);

        Self {
            key,
            link_key,
            count,
            current_count: 0,
            direction,
            with,
            wait_before_use_ticks: wait_before,
            wait_after_use_ticks: wait_after,
            pending_transition: PendingTransition::None,
            action_info: None,
            state: State::Precondition,
        }
    }

    pub fn from_auto_mob(
        mob: AutoMob,
        direction: ActionKeyDirection,
        should_terminate: bool,
    ) -> Self {
        let wait_before =
            random_wait_ticks(mob.wait_before_ticks, mob.wait_before_ticks_random_range);
        let wait_after = random_wait_ticks(mob.wait_after_ticks, mob.wait_after_ticks_random_range);

        Self {
            key: mob.key,
            link_key: mob.link_key,
            count: mob.count,
            current_count: 0,
            direction,
            with: mob.with,
            wait_before_use_ticks: wait_before,
            wait_after_use_ticks: wait_after,
            pending_transition: PendingTransition::None,
            action_info: Some(ActionInfo::AutoMobbing { should_terminate }),
            state: State::Precondition,
        }
    }

    pub fn from_ping_pong(ping_pong: PingPong) -> Self {
        let wait_before = random_wait_ticks(
            ping_pong.wait_before_ticks,
            ping_pong.wait_before_ticks_random_range,
        );
        let wait_after = random_wait_ticks(
            ping_pong.wait_after_ticks,
            ping_pong.wait_after_ticks_random_range,
        );
        let direction = if matches!(ping_pong.direction, PingPongDirection::Left) {
            ActionKeyDirection::Left
        } else {
            ActionKeyDirection::Right
        };

        Self {
            key: ping_pong.key,
            link_key: ping_pong.link_key,
            count: ping_pong.count,
            current_count: 0,
            direction,
            with: ping_pong.with,
            wait_before_use_ticks: wait_before,
            wait_after_use_ticks: wait_after,
            pending_transition: PendingTransition::None,
            action_info: None,
            state: State::Precondition,
        }
    }
}

/// Updates the [`Player::UseKey`] contextual state.
///
/// Like [`Player::SolvingRune`], this state can only be transitioned via a [`PlayerAction`]. It
/// can be transitioned during any of the movement state. Or if there is no position, it will
/// be transitioned to immediately by [`Player::Idle`].
///
/// There are multiple stages to using a key as described by [`UseKeyStage`].
pub fn update_use_key_state(
    resources: &Resources,
    player: &mut PlayerEntity,
    minimap_state: Minimap,
) {
    let Player::UseKey(mut use_key) = player.state else {
        panic!("state is not using key")
    };

    match use_key.state {
        State::Precondition => {
            update_precondition(&player.context, &mut use_key);
            transition_if!(
                player,
                Player::Stalling(Timeout::default(), use_key.wait_before_use_ticks),
                matches!(use_key.pending_transition, PendingTransition::WaitBefore),
                {
                    use_key.pending_transition = PendingTransition::None;
                    use_key.state = State::Using(Timeout::default(), false);
                    player.context.stalling_timeout_state = Some(Player::UseKey(use_key));
                }
            );
        }
        State::ChangingDirection(_) => {
            update_changing_direction(resources, &mut player.context, &mut use_key);
        }
        State::EnsuringUseWith => {
            update_ensuring_use_with(&player.context, &mut use_key);
            transition_if!(
                player,
                Player::DoubleJumping(DoubleJumping::new(
                    Moving::new(
                        player.context.last_known_pos.expect("in positional state"),
                        player.context.last_known_pos.expect("in positional state"),
                        false,
                        None,
                    ),
                    true,
                    true,
                )),
                matches!(use_key.pending_transition, PendingTransition::DoubleJump),
                {
                    use_key.pending_transition = PendingTransition::None;
                }
            );
        }
        State::Using(timeout, completed) => {
            update_using(resources, &player.context, &mut use_key, timeout, completed);
            transition_if!(
                player,
                Player::Stalling(Timeout::default(), use_key.wait_after_use_ticks),
                matches!(use_key.pending_transition, PendingTransition::WaitAfter),
                {
                    use_key.pending_transition = PendingTransition::None;
                    use_key.state = State::Postcondition;
                    player.context.stalling_timeout_state = Some(Player::UseKey(use_key));
                }
            );
        }
        State::Postcondition => {
            use_key.current_count += 1;
            if use_key.current_count < use_key.count {
                use_key.state = State::Precondition;
            }
        }
    };

    let player_next_state = if use_key.current_count >= use_key.count {
        Player::Idle
    } else {
        Player::UseKey(use_key)
    };
    let is_terminal = matches!(player_next_state, Player::Idle);

    match next_action(&player.context) {
        Some(PlayerAction::AutoMob(AutoMob {
            position: Position { y, .. },
            ..
        })) => {
            let should_terminate = matches!(
                use_key.action_info,
                Some(ActionInfo::AutoMobbing {
                    should_terminate: true
                })
            );
            transition_if!(player, player_next_state, !is_terminal || !should_terminate);

            player
                .context
                .auto_mob_track_ignore_xs(minimap_state, false);
            transition_if!(
                player,
                Player::Stalling(Timeout::default(), MOVE_TIMEOUT),
                player.context.auto_mob_reachable_y_require_update(y)
            );

            assert_matches!(player_next_state, Player::Idle);
            transition_from_action!(player, player_next_state);
        }

        Some(PlayerAction::PingPong(ping_pong)) => {
            transition_if!(player, player_next_state, !is_terminal);

            player.context.clear_unstucking(true);
            update_from_ping_pong_action(
                resources,
                player,
                minimap_state,
                ping_pong,
                player.context.last_known_pos.expect("in positional state"),
            )
        }

        Some(PlayerAction::Move(_) | PlayerAction::Key(_)) => {
            transition_from_action!(player, player_next_state, is_terminal)
        }

        None => transition!(player, player_next_state),

        _ => unreachable!(),
    }
}

fn update_precondition(context: &PlayerContext, use_key: &mut UseKey) {
    transition_if!(
        use_key,
        State::ChangingDirection(Timeout::default()),
        !ensure_direction(context, use_key.direction)
    );

    transition_if!(
        use_key,
        State::EnsuringUseWith,
        !ensure_use_with(context, use_key.with)
    );

    transition_if!(
        use_key,
        State::Using(Timeout::default(), false),
        use_key.wait_before_use_ticks == 0
    );

    use_key.pending_transition = PendingTransition::WaitBefore;
}

#[inline]
fn ensure_direction(context: &PlayerContext, direction: ActionKeyDirection) -> bool {
    match direction {
        ActionKeyDirection::Any => true,
        ActionKeyDirection::Left | ActionKeyDirection::Right => {
            direction == context.last_known_direction
        }
    }
}

#[inline]
fn ensure_use_with(context: &PlayerContext, with: ActionKeyWith) -> bool {
    match with {
        ActionKeyWith::Any => true,
        ActionKeyWith::Stationary => context.is_stationary,
        ActionKeyWith::DoubleJump => {
            matches!(context.last_movement, Some(LastMovement::DoubleJumping))
        }
    }
}

fn update_using(
    resources: &Resources,
    context: &PlayerContext,
    use_key: &mut UseKey,
    timeout: Timeout,
    completed: bool,
) {
    match use_key.link_key {
        Some(LinkKeyBinding::After(_)) => {
            if !timeout.started {
                resources.input.send_key(use_key.key.into());
            }
            if !completed {
                return update_link_key(
                    resources,
                    use_key,
                    context.config.class,
                    context.config.jump_key,
                    timeout,
                    completed,
                );
            }
        }
        Some(LinkKeyBinding::AtTheSame(key)) => {
            resources.input.send_key(key.into());
            resources.input.send_key(use_key.key.into());
        }
        Some(LinkKeyBinding::Along(_)) => {
            if !completed {
                return update_link_key(
                    resources,
                    use_key,
                    context.config.class,
                    context.config.jump_key,
                    timeout,
                    completed,
                );
            }
        }
        Some(LinkKeyBinding::Before(_)) | None => {
            if use_key.link_key.is_some() && !completed {
                return update_link_key(
                    resources,
                    use_key,
                    context.config.class,
                    context.config.jump_key,
                    timeout,
                    completed,
                );
            }
            resources.input.send_key(use_key.key.into());
        }
    }

    transition_if!(
        use_key,
        State::Postcondition,
        use_key.wait_after_use_ticks == 0
    );

    use_key.pending_transition = PendingTransition::WaitAfter;
}

fn update_ensuring_use_with(context: &PlayerContext, use_key: &mut UseKey) {
    match use_key.with {
        ActionKeyWith::Any => unreachable!(),
        ActionKeyWith::Stationary => transition_if!(
            use_key,
            State::Precondition,
            State::EnsuringUseWith,
            context.is_stationary
        ),
        ActionKeyWith::DoubleJump => {
            use_key.pending_transition = PendingTransition::DoubleJump;
        }
    }
}

fn update_changing_direction(
    resources: &Resources,
    context: &mut PlayerContext,
    use_key: &mut UseKey,
) {
    let State::ChangingDirection(timeout) = use_key.state else {
        panic!("using key state is not changing direction");
    };
    let key = match use_key.direction {
        ActionKeyDirection::Left => KeyKind::Left,
        ActionKeyDirection::Right => KeyKind::Right,
        ActionKeyDirection::Any => unreachable!(),
    };

    match next_timeout_lifecycle(timeout, CHANGE_DIRECTION_TIMEOUT) {
        Lifecycle::Started(timeout) => transition!(use_key, State::ChangingDirection(timeout), {
            resources.input.send_key_down(key);
        }),
        Lifecycle::Ended => transition!(use_key, State::Precondition, {
            resources.input.send_key_up(key);
            context.last_known_direction = use_key.direction;
        }),
        Lifecycle::Updated(timeout) => transition!(use_key, State::ChangingDirection(timeout)),
    }
}

#[inline]
fn update_link_key(
    resources: &Resources,
    use_key: &mut UseKey,
    class: Class,
    jump_key: KeyKind,
    timeout: Timeout,
    completed: bool,
) {
    let link_key = use_key.link_key.unwrap();
    let link_key_timeout = if matches!(link_key, LinkKeyBinding::Along(_)) {
        4
    } else {
        match class {
            Class::Cadena => 4,
            Class::Blaster => 8,
            Class::Ark => 10,
            Class::Generic => 5,
        }
    };

    match next_timeout_lifecycle(timeout, link_key_timeout) {
        Lifecycle::Started(timeout) => transition!(use_key, State::Using(timeout, completed), {
            match link_key {
                LinkKeyBinding::Before(key) => {
                    resources.input.send_key(key.into());
                }
                LinkKeyBinding::Along(key) => {
                    resources.input.send_key_down(key.into());
                }
                LinkKeyBinding::AtTheSame(_) | LinkKeyBinding::After(_) => (),
            }
        }),
        Lifecycle::Ended => transition!(use_key, State::Using(timeout, true), {
            match link_key {
                LinkKeyBinding::After(key) => {
                    resources.input.send_key(key.into());
                    if matches!(class, Class::Blaster) && KeyKind::from(key) != jump_key {
                        resources.input.send_key(jump_key);
                    }
                }
                LinkKeyBinding::Along(key) => {
                    resources.input.send_key_up(key.into());
                }
                LinkKeyBinding::AtTheSame(_) | LinkKeyBinding::Before(_) => (),
            }
        }),
        Lifecycle::Updated(timeout) => {
            transition!(use_key, State::Using(timeout, completed), {
                if matches!(link_key, LinkKeyBinding::Along(_))
                    && timeout.total == LINK_ALONG_PRESS_TICK
                {
                    resources.input.send_key(use_key.key.into());
                }
            })
        }
    }
}

#[inline]
fn random_wait_ticks(wait_base_ticks: u32, wait_random_range: u32) -> u32 {
    // TODO: Replace rand with Rng
    let wait_min = wait_base_ticks.saturating_sub(wait_random_range);
    let wait_max = wait_base_ticks.saturating_add(wait_random_range + 1);
    rand::random_range(wait_min..wait_max)
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;

    use mockall::Sequence;
    use opencv::core::Point;

    use crate::{
        ActionKeyDirection, ActionKeyWith, KeyBinding, LinkKeyBinding,
        bridge::{KeyKind, MockInput},
        ecs::Resources,
        minimap::Minimap,
        player::{
            Player, PlayerContext, PlayerEntity, Timeout,
            double_jump::DoubleJumping,
            use_key::{PendingTransition, State, UseKey, update_use_key_state},
        },
    };

    fn make_player(use_key: UseKey) -> PlayerEntity {
        PlayerEntity {
            state: Player::UseKey(use_key),
            context: PlayerContext::default(),
        }
    }

    #[test]
    fn update_use_key_state_ensuring_use_with_stationary() {
        let resources = Resources::new(None, None);
        let mut player = make_player(UseKey {
            key: KeyBinding::A,
            link_key: None,
            count: 1,
            current_count: 0,
            direction: ActionKeyDirection::Any,
            with: ActionKeyWith::Stationary,
            wait_before_use_ticks: 0,
            wait_after_use_ticks: 0,
            action_info: None,
            state: State::Precondition,
            pending_transition: PendingTransition::None,
        });

        // Start EnsuringUseWith
        update_use_key_state(&resources, &mut player, Minimap::Detecting);
        assert_matches!(
            player.state,
            Player::UseKey(UseKey {
                state: State::EnsuringUseWith,
                ..
            })
        );

        // Complete EnsuringUseWith when stationary
        player.context.is_stationary = true;
        update_use_key_state(&resources, &mut player, Minimap::Detecting);
        assert_matches!(
            player.state,
            Player::UseKey(UseKey {
                state: State::Precondition,
                ..
            })
        );
    }

    #[test]
    fn update_use_key_state_ensuring_use_with_double_jump() {
        let resources = Resources::new(None, None);
        let mut player = make_player(UseKey {
            key: KeyBinding::A,
            link_key: None,
            count: 1,
            current_count: 0,
            direction: ActionKeyDirection::Any,
            with: ActionKeyWith::DoubleJump,
            wait_before_use_ticks: 0,
            wait_after_use_ticks: 0,
            action_info: None,
            state: State::Precondition,
            pending_transition: PendingTransition::None,
        });
        player.context.last_known_pos = Some(Point::default());

        // Start EnsuringUseWith
        update_use_key_state(&resources, &mut player, Minimap::Detecting);
        assert_matches!(
            player.state,
            Player::UseKey(UseKey {
                state: State::EnsuringUseWith,
                ..
            })
        );

        // Transitions to double jump
        update_use_key_state(&resources, &mut player, Minimap::Detecting);
        assert_matches!(
            player.state,
            Player::DoubleJumping(DoubleJumping { forced: true, .. })
        );
    }

    #[test]
    fn update_use_key_state_changing_direction() {
        let mut keys = MockInput::new();
        keys.expect_send_key_down()
            .withf(|k| matches!(k, KeyKind::Left));
        keys.expect_send_key_up()
            .withf(|k| matches!(k, KeyKind::Left));
        let resources = Resources::new(Some(keys), None);
        let mut use_key = UseKey {
            key: KeyBinding::A,
            link_key: None,
            count: 1,
            current_count: 0,
            direction: ActionKeyDirection::Left,
            with: ActionKeyWith::Any,
            wait_before_use_ticks: 0,
            wait_after_use_ticks: 0,
            action_info: None,
            state: State::Precondition,
            pending_transition: PendingTransition::None,
        };
        let mut player = make_player(use_key);

        // Transition into ChangingDirection
        update_use_key_state(&resources, &mut player, Minimap::Detecting);
        assert_matches!(
            player.state,
            Player::UseKey(UseKey {
                state: State::ChangingDirection(Timeout { started: false, .. }),
                ..
            })
        );

        // Sends down on started
        update_use_key_state(&resources, &mut player, Minimap::Detecting);
        assert_matches!(
            player.state,
            Player::UseKey(UseKey {
                state: State::ChangingDirection(Timeout { started: true, .. }),
                ..
            })
        );

        // Simulate completion of ChangingDirection
        use_key.state = State::ChangingDirection(Timeout {
            started: true,
            total: 3,
            current: 3,
        });
        player = make_player(use_key);
        update_use_key_state(&resources, &mut player, Minimap::Detecting);
        assert_matches!(
            player.context.last_known_direction,
            ActionKeyDirection::Left
        );
        assert_matches!(
            player.state,
            Player::UseKey(UseKey {
                state: State::Precondition,
                ..
            })
        );
    }

    #[test]
    fn update_use_key_state_repeats_until_count_reached() {
        let mut keys = MockInput::new();
        keys.expect_send_key()
            .times(3)
            .withf(|k| matches!(k, KeyKind::A));
        let resources = Resources::new(Some(keys), None);
        let use_key = UseKey {
            key: KeyBinding::A,
            link_key: None,
            count: 3,
            current_count: 0,
            direction: ActionKeyDirection::Any,
            with: ActionKeyWith::Any,
            wait_before_use_ticks: 0,
            wait_after_use_ticks: 0,
            action_info: None,
            state: State::Precondition,
            pending_transition: PendingTransition::None,
        };
        let mut player = make_player(use_key);

        for i in 0..3 {
            update_use_key_state(&resources, &mut player, Minimap::Detecting);
            assert_matches!(
                player.state,
                Player::UseKey(UseKey {
                    state: State::Using(_, _),
                    ..
                })
            );

            update_use_key_state(&resources, &mut player, Minimap::Detecting);
            assert_matches!(
                player.state,
                Player::UseKey(UseKey {
                    state: State::Postcondition,
                    ..
                })
            );

            update_use_key_state(&resources, &mut player, Minimap::Detecting);
            if i == 2 {
                assert_matches!(player.state, Player::Idle);
            } else {
                assert_matches!(
                    player.state,
                    Player::UseKey(UseKey {
                        state: State::Precondition,
                        ..
                    })
                );
            }
        }
    }

    #[test]
    fn update_use_key_state_waits_before() {
        let resources = Resources::new(None, None);
        let use_key = UseKey {
            key: KeyBinding::A,
            link_key: None,
            count: 1,
            current_count: 0,
            direction: ActionKeyDirection::Any,
            with: ActionKeyWith::Any,
            wait_before_use_ticks: 5,
            wait_after_use_ticks: 0,
            action_info: None,
            state: State::Precondition,
            pending_transition: PendingTransition::None,
        };
        let mut player = make_player(use_key);

        update_use_key_state(&resources, &mut player, Minimap::Detecting);

        assert_matches!(player.state, Player::Stalling(_, 5));
        assert_matches!(
            player.context.stalling_timeout_state,
            Some(Player::UseKey(UseKey {
                state: State::Using(_, _),
                pending_transition: PendingTransition::None,
                ..
            }))
        );
    }

    #[test]
    fn update_use_key_state_waits_after() {
        let mut keys = MockInput::new();
        keys.expect_send_key()
            .withf(|k| matches!(k, KeyKind::A))
            .once();
        let resources = Resources::new(Some(keys), None);
        let use_key = UseKey {
            key: KeyBinding::A,
            link_key: None,
            count: 1,
            current_count: 0,
            direction: ActionKeyDirection::Any,
            with: ActionKeyWith::Any,
            wait_before_use_ticks: 0,
            wait_after_use_ticks: 7,
            action_info: None,
            state: State::Using(Timeout::default(), false),
            pending_transition: PendingTransition::None,
        };
        let mut player = make_player(use_key);

        update_use_key_state(&resources, &mut player, Minimap::Detecting);

        assert_matches!(player.state, Player::Stalling(_, 7));
        assert_matches!(
            player.context.stalling_timeout_state,
            Some(Player::UseKey(UseKey {
                state: State::Postcondition,
                pending_transition: PendingTransition::None,
                ..
            }))
        );
    }

    #[test]
    fn update_use_key_state_link_key_along() {
        let mut sequence = Sequence::new();
        let mut keys = MockInput::new();
        keys.expect_send_key_down()
            .withf(|k| matches!(k, KeyKind::Alt))
            .once()
            .in_sequence(&mut sequence);
        keys.expect_send_key()
            .withf(|k| matches!(k, KeyKind::A))
            .once()
            .in_sequence(&mut sequence);
        keys.expect_send_key_up()
            .withf(|k| matches!(k, KeyKind::Alt))
            .once()
            .in_sequence(&mut sequence);
        let resources = Resources::new(Some(keys), None);
        let mut use_key = UseKey {
            key: KeyBinding::A,
            link_key: Some(LinkKeyBinding::Along(KeyBinding::Alt)),
            count: 1,
            current_count: 0,
            direction: ActionKeyDirection::Any,
            with: ActionKeyWith::Any,
            wait_before_use_ticks: 0,
            wait_after_use_ticks: 0,
            action_info: None,
            state: State::Using(Timeout::default(), false),
            pending_transition: PendingTransition::None,
        };
        let mut player = make_player(use_key);

        // Hold Alt
        update_use_key_state(&resources, &mut player, Minimap::Detecting);
        assert_matches!(
            player.state,
            Player::UseKey(UseKey {
                state: State::Using(_, false),
                ..
            })
        );

        // Press A
        use_key.state = State::Using(
            Timeout {
                current: 1,
                total: 1,
                started: true,
            },
            false,
        );
        player = make_player(use_key);
        update_use_key_state(&resources, &mut player, Minimap::Detecting);
        assert_matches!(
            player.state,
            Player::UseKey(UseKey {
                state: State::Using(_, false),
                ..
            })
        );

        // Release Alt
        use_key.state = State::Using(
            Timeout {
                current: 4,
                total: 4,
                started: true,
            },
            false,
        );
        player = make_player(use_key);
        update_use_key_state(&resources, &mut player, Minimap::Detecting);
        assert_matches!(
            player.state,
            Player::UseKey(UseKey {
                state: State::Using(_, true),
                ..
            })
        );
    }

    #[test]
    fn update_use_key_state_link_key_before() {
        let mut sequence = Sequence::new();
        let mut keys = MockInput::new();
        keys.expect_send_key()
            .withf(|k| matches!(k, KeyKind::Alt))
            .once()
            .in_sequence(&mut sequence);
        keys.expect_send_key()
            .withf(|k| matches!(k, KeyKind::A))
            .once()
            .in_sequence(&mut sequence);
        let resources = Resources::new(Some(keys), None);

        let mut use_key = UseKey {
            key: KeyBinding::A,
            link_key: Some(LinkKeyBinding::Before(KeyBinding::Alt)),
            count: 1,
            current_count: 0,
            direction: ActionKeyDirection::Any,
            with: ActionKeyWith::Any,
            wait_before_use_ticks: 0,
            wait_after_use_ticks: 0,
            action_info: None,
            state: State::Using(Timeout::default(), false),
            pending_transition: PendingTransition::None,
        };
        let mut player = make_player(use_key);

        // Press Alt
        update_use_key_state(&resources, &mut player, Minimap::Detecting);
        assert_matches!(
            player.state,
            Player::UseKey(UseKey {
                state: State::Using(_, false),
                ..
            })
        );

        // Press A
        use_key.state = State::Using(Timeout::default(), true);
        player = make_player(use_key);
        update_use_key_state(&resources, &mut player, Minimap::Detecting);
    }

    #[test]
    fn update_use_key_state_link_key_after() {
        let mut sequence = Sequence::new();
        let mut keys = MockInput::new();
        keys.expect_send_key()
            .withf(|k| matches!(k, KeyKind::A))
            .once()
            .in_sequence(&mut sequence);
        keys.expect_send_key()
            .withf(|k| matches!(k, KeyKind::Alt))
            .once()
            .in_sequence(&mut sequence);
        let resources = Resources::new(Some(keys), None);

        let mut use_key = UseKey {
            key: KeyBinding::A,
            link_key: Some(LinkKeyBinding::After(KeyBinding::Alt)),
            count: 1,
            current_count: 0,
            direction: ActionKeyDirection::Any,
            with: ActionKeyWith::Any,
            wait_before_use_ticks: 0,
            wait_after_use_ticks: 0,
            action_info: None,
            state: State::Using(Timeout::default(), false),
            pending_transition: PendingTransition::None,
        };
        let mut player = make_player(use_key);

        // Press A
        update_use_key_state(&resources, &mut player, Minimap::Detecting);
        assert_matches!(
            player.state,
            Player::UseKey(UseKey {
                state: State::Using(_, false),
                ..
            })
        );

        // Press Alt
        use_key.state = State::Using(
            Timeout {
                current: 5, // Generic class
                started: true,
                ..Default::default()
            },
            false,
        );
        player = make_player(use_key);
        update_use_key_state(&resources, &mut player, Minimap::Detecting);
    }

    #[test]
    fn update_use_key_state_link_key_at_the_same() {
        let mut sequence = Sequence::new();
        let mut keys = MockInput::new();
        keys.expect_send_key()
            .withf(|k| matches!(k, KeyKind::Alt))
            .once()
            .in_sequence(&mut sequence);
        keys.expect_send_key()
            .withf(|k| matches!(k, KeyKind::A))
            .once()
            .in_sequence(&mut sequence);
        let resources = Resources::new(Some(keys), None);

        let use_key = UseKey {
            key: KeyBinding::A,
            link_key: Some(LinkKeyBinding::AtTheSame(KeyBinding::Alt)),
            count: 1,
            current_count: 0,
            direction: ActionKeyDirection::Any,
            with: ActionKeyWith::Any,
            wait_before_use_ticks: 0,
            wait_after_use_ticks: 0,
            action_info: None,
            state: State::Using(Timeout::default(), false),
            pending_transition: PendingTransition::None,
        };
        let mut player = make_player(use_key);

        // Press Alt then A
        update_use_key_state(&resources, &mut player, Minimap::Detecting);
        assert_matches!(
            player.state,
            Player::UseKey(UseKey {
                state: State::Postcondition,
                ..
            })
        );
    }
}
