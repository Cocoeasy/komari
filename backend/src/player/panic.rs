use super::{Player, actions::PanicTo, timeout::Timeout};
use crate::{
    bridge::KeyKind,
    ecs::Resources,
    minimap::Minimap,
    player::{
        PlayerEntity, next_action,
        timeout::{Lifecycle, next_timeout_lifecycle},
    },
    transition, transition_from_action, transition_if,
};

const MAX_RETRY: u32 = 3;

/// States of panicking mode.
#[derive(Debug, Clone, Copy)]
enum State {
    /// Cycling through channels.
    ChangingChannel(Timeout, u32),
    /// Going to town.
    GoingToTown(Timeout, u32),
    Completing(Timeout, bool),
}

#[derive(Debug, Clone, Copy)]
pub struct Panicking {
    state: State,
    pub to: PanicTo,
}

impl Panicking {
    pub fn new(to: PanicTo) -> Self {
        Self {
            state: match to {
                PanicTo::Channel => State::ChangingChannel(Timeout::default(), 0),
                PanicTo::Town => State::GoingToTown(Timeout::default(), 0),
            },
            to,
        }
    }
}

/// Updates [`Player::Panicking`] contextual state.
pub fn update_panicking_state(
    resources: &Resources,
    player: &mut PlayerEntity,
    minimap_state: Minimap,
    mut panicking: Panicking,
) {
    match panicking.state {
        State::ChangingChannel(_, _) => update_changing_channel(
            resources,
            &mut panicking,
            minimap_state,
            player.context.config.change_channel_key,
        ),
        State::GoingToTown(_, _) => {
            update_going_to_town(resources, &mut panicking, player.context.config.to_town_key)
        }
        State::Completing(_, _) => update_completing(&mut panicking, minimap_state),
    };

    let player_next_state = if matches!(panicking.state, State::Completing(_, true)) {
        Player::Idle
    } else {
        Player::Panicking(panicking)
    };

    match next_action(&player.context) {
        Some(_) => transition_from_action!(
            player,
            player_next_state,
            matches!(player_next_state, Player::Idle)
        ),
        None => transition_if!(
            player,
            // Allow continuing for town even if the bot has already halted
            player_next_state,
            // Force cancel if it is not initiated from an action for other panic kind
            Player::Idle,
            matches!(panicking.to, PanicTo::Town)
        ),
    }
}

fn update_changing_channel(
    resources: &Resources,
    panicking: &mut Panicking,
    minimap_state: Minimap,
    key: KeyKind,
) {
    const PRESS_RIGHT_AT_AFTER: u32 = 15;
    const PRESS_ENTER_AT_AFTER: u32 = 30;
    const TIMEOUT_AFTER: u32 = 50;

    const TIMEOUT_INITIAL: u32 = 220;
    const PRESS_RIGHT_AT_INITIAL: u32 = 170;
    const PRESS_ENTER_AT_INITIAL: u32 = 200;

    let State::ChangingChannel(timeout, retry_count) = panicking.state else {
        panic!("panicking state is not changing channel")
    };
    let max_timeout = if retry_count == 0 {
        TIMEOUT_INITIAL
    } else {
        TIMEOUT_AFTER
    };
    match next_timeout_lifecycle(timeout, max_timeout) {
        Lifecycle::Started(timeout) => {
            transition!(panicking, State::ChangingChannel(timeout, retry_count), {
                if !resources.detector().detect_change_channel_menu_opened() {
                    resources.input.send_key(key);
                }
            })
        }
        Lifecycle::Ended => {
            transition_if!(
                panicking,
                State::Completing(Timeout::default(), false),
                !matches!(minimap_state, Minimap::Idle(_))
            );
            transition_if!(
                panicking,
                State::ChangingChannel(Timeout::default(), retry_count + 1),
                State::Completing(Timeout::default(), true),
                retry_count < MAX_RETRY
            );
        }
        Lifecycle::Updated(timeout) => {
            transition!(panicking, State::ChangingChannel(timeout, retry_count), {
                let (press_right_at, press_enter_at) = if retry_count == 0 {
                    (PRESS_RIGHT_AT_INITIAL, PRESS_ENTER_AT_INITIAL)
                } else {
                    (PRESS_RIGHT_AT_AFTER, PRESS_ENTER_AT_AFTER)
                };
                match timeout.current {
                    tick if tick == press_right_at => {
                        if resources.detector().detect_change_channel_menu_opened() {
                            resources.input.send_key(KeyKind::Right);
                        }
                    }
                    tick if tick == press_enter_at => {
                        if resources.detector().detect_change_channel_menu_opened() {
                            resources.input.send_key(KeyKind::Enter);
                        }
                    }
                    _ => (),
                }
            })
        }
    }
}

fn update_going_to_town(resources: &Resources, panicking: &mut Panicking, key: KeyKind) {
    let State::GoingToTown(timeout, retry_count) = panicking.state else {
        panic!("panicking state is not going to town")
    };

    match next_timeout_lifecycle(timeout, 90) {
        Lifecycle::Started(timeout) => {
            transition!(panicking, State::GoingToTown(timeout, retry_count), {
                resources.input.send_key(key);
            })
        }

        Lifecycle::Ended => {
            let has_confirm_button = resources.detector().detect_esc_confirm_button().is_ok();
            if has_confirm_button {
                resources.input.send_key(KeyKind::Enter);
            }

            transition_if!(
                panicking,
                State::GoingToTown(Timeout::default(), retry_count + 1),
                State::Completing(Timeout::default(), true),
                !has_confirm_button && retry_count < MAX_RETRY
            );
        }
        Lifecycle::Updated(timeout) => {
            transition!(panicking, State::GoingToTown(timeout, retry_count))
        }
    }
}

fn update_completing(panicking: &mut Panicking, minimap_state: Minimap) {
    let State::Completing(timeout, completed) = panicking.state else {
        panic!("panicking state is not completing")
    };

    transition_if!(
        panicking,
        State::Completing(timeout, true),
        matches!(panicking.to, PanicTo::Town)
    );

    match next_timeout_lifecycle(timeout, 245) {
        Lifecycle::Ended => match minimap_state {
            Minimap::Idle(idle) => transition_if!(
                panicking,
                State::ChangingChannel(Timeout::default(), 0),
                State::Completing(timeout, true),
                idle.has_any_other_player()
            ),
            Minimap::Detecting => {
                transition!(panicking, State::Completing(Timeout::default(), false))
            }
        },
        Lifecycle::Started(timeout) | Lifecycle::Updated(timeout) => {
            transition!(panicking, State::Completing(timeout, completed))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;

    use anyhow::{Ok, anyhow};
    use mockall::predicate::eq;
    use opencv::core::Rect;

    use super::*;
    use crate::{
        bridge::MockInput,
        detect::MockDetector,
        minimap::{Minimap, MinimapIdle},
    };

    #[test]
    fn update_changing_channel_and_send_key_keys() {
        let mut keys = MockInput::default();
        let mut detector = MockDetector::default();
        detector
            .expect_detect_change_channel_menu_opened()
            .return_const(true);
        keys.expect_send_key().times(2);
        let resources = Resources::new(Some(keys), Some(detector));
        let mut panicking = Panicking::new(PanicTo::Channel);
        panicking.state = State::ChangingChannel(
            Timeout {
                current: 169,
                started: true,
                ..Default::default()
            },
            0,
        );

        update_changing_channel(&resources, &mut panicking, Minimap::Detecting, KeyKind::F1);
        assert_matches!(panicking.state, State::ChangingChannel(_, _));

        panicking.state = State::ChangingChannel(
            Timeout {
                current: 199,
                started: true,
                ..Default::default()
            },
            0,
        );
        update_changing_channel(&resources, &mut panicking, Minimap::Detecting, KeyKind::F1);
        assert_matches!(panicking.state, State::ChangingChannel(_, _));
    }

    #[test]
    fn update_changing_channel_and_send_keys_retry() {
        let mut keys = MockInput::default();
        let mut detector = MockDetector::default();
        detector
            .expect_detect_change_channel_menu_opened()
            .return_const(true);
        keys.expect_send_key().times(2);
        let resources = Resources::new(Some(keys), Some(detector));
        let mut panicking = Panicking::new(PanicTo::Channel);
        panicking.state = State::ChangingChannel(
            Timeout {
                current: 14,
                started: true,
                ..Default::default()
            },
            1,
        );

        update_changing_channel(&resources, &mut panicking, Minimap::Detecting, KeyKind::F1);
        assert_matches!(panicking.state, State::ChangingChannel(_, _));

        panicking.state = State::ChangingChannel(
            Timeout {
                current: 29,
                started: true,
                ..Default::default()
            },
            1,
        );
        update_changing_channel(&resources, &mut panicking, Minimap::Detecting, KeyKind::F1);
        assert_matches!(panicking.state, State::ChangingChannel(_, _));
    }

    #[test]
    fn update_changing_channel_complete_if_minimap_not_idle() {
        let resources = Resources::new(None, None);
        let mut panicking = Panicking::new(PanicTo::Channel);
        panicking.state = State::ChangingChannel(
            Timeout {
                current: 220,
                started: true,
                ..Default::default()
            },
            0,
        );

        update_changing_channel(&resources, &mut panicking, Minimap::Detecting, KeyKind::F1);

        assert_matches!(panicking.state, State::Completing(_, false));
    }

    #[test]
    fn update_changing_channel_complete_if_minimap_not_idle_retry() {
        let resources = Resources::new(None, None);
        let mut panicking = Panicking::new(PanicTo::Channel);
        panicking.state = State::ChangingChannel(
            Timeout {
                current: 50,
                started: true,
                ..Default::default()
            },
            1,
        );

        update_changing_channel(&resources, &mut panicking, Minimap::Detecting, KeyKind::F1);

        assert_matches!(panicking.state, State::Completing(_, false));
    }

    #[test]
    fn update_going_to_town_started_send_key() {
        let mut keys = MockInput::default();
        keys.expect_send_key().once().with(eq(KeyKind::F2));
        let resources = Resources::new(Some(keys), None);
        let mut panicking = Panicking::new(PanicTo::Town);
        panicking.state = State::GoingToTown(Timeout::default(), 0);

        update_going_to_town(&resources, &mut panicking, KeyKind::F2);

        assert_matches!(panicking.state, State::GoingToTown(_, _));
    }

    #[test]
    fn update_going_to_town_ended_send_key_and_complete_if_esc_confirm_opened() {
        let mut keys = MockInput::default();
        keys.expect_send_key().once().with(eq(KeyKind::Enter));
        let mut detector = MockDetector::default();
        detector
            .expect_detect_esc_confirm_button()
            .returning(|| Ok(Rect::default()));
        let resources = Resources::new(Some(keys), Some(detector));
        let mut panicking = Panicking::new(PanicTo::Town);
        panicking.state = State::GoingToTown(
            Timeout {
                started: true,
                current: 90,
                ..Default::default()
            },
            0,
        );

        update_going_to_town(&resources, &mut panicking, KeyKind::F2);

        assert_matches!(panicking.state, State::Completing(_, true));
    }

    #[test]
    fn update_going_to_town_ended_retry() {
        let mut detector = MockDetector::default();
        detector
            .expect_detect_esc_confirm_button()
            .returning(|| Err(anyhow!("button not found")));
        let resources = Resources::new(None, Some(detector));
        let mut panicking = Panicking::new(PanicTo::Town);
        panicking.state = State::GoingToTown(
            Timeout {
                started: true,
                current: 90,
                ..Default::default()
            },
            0,
        );

        update_going_to_town(&resources, &mut panicking, KeyKind::F2);

        assert_matches!(
            panicking.state,
            State::GoingToTown(
                Timeout {
                    started: false,
                    current: 0,
                    ..
                },
                1
            )
        );
    }

    #[test]
    fn update_completing_for_town_immediately_complete() {
        let mut panicking = Panicking::new(PanicTo::Town);
        panicking.state = State::Completing(Timeout::default(), false);

        update_completing(&mut panicking, Minimap::Detecting);

        assert_matches!(panicking.state, State::Completing(_, true));
    }

    #[test]
    fn update_completing_for_channel_switch_to_idle_if_no_players() {
        let mut panicking = Panicking::new(PanicTo::Channel);
        panicking.state = State::Completing(
            Timeout {
                current: 245,
                started: true,
                ..Default::default()
            },
            false,
        );

        update_completing(&mut panicking, Minimap::Idle(MinimapIdle::default()));

        assert_matches!(panicking.state, State::Completing(_, true));
    }
}
