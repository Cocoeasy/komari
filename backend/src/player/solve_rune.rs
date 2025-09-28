use super::{
    Player,
    actions::PlayerAction,
    timeout::{Lifecycle, next_timeout_lifecycle},
};
use crate::{
    bridge::KeyKind,
    detect::{ArrowsCalibrating, ArrowsState},
    ecs::Resources,
    player::{PlayerContext, PlayerEntity, next_action, timeout::Timeout},
    transition, transition_from_action, transition_if, try_ok_transition,
};

const MAX_RETRY_COUNT: u32 = 2;

/// Representing the current state of rune solving.
#[derive(Debug, Default, Clone, Copy)]
pub enum State {
    // Ensures stationary and all keys cleared before solving.
    #[default]
    Precondition,
    // Finds the region containing the four arrows.
    FindRegion(ArrowsCalibrating, Timeout, Option<Timeout>, u32),
    // Solves for the rune arrows that possibly include spinning arrows.
    Solving(ArrowsCalibrating, Timeout),
    // Presses the keys.
    PressKeys(Timeout, [KeyKind; 4], usize),
    // Terminal stage.
    Completed,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct SolvingRune {
    state: State,
}

/// Updates the [`Player::SolvingRune`] contextual state.
///
/// Note: This state does not use any [`Task`], so all detections are blocking. But this should be
/// acceptable for this state.
pub fn update_solving_rune_state(resources: &Resources, player: &mut PlayerEntity) {
    let Player::SolvingRune(mut solving_rune) = player.state else {
        panic!("state is not solving rune");
    };

    match solving_rune.state {
        State::Precondition => update_precondition(resources, &player.context, &mut solving_rune),
        State::FindRegion(_, _, _, _) => update_find_region(
            resources,
            &mut solving_rune,
            player.context.config.interact_key,
        ),
        State::Solving(_, _) => update_solving(resources, &mut solving_rune),
        State::PressKeys(_, _, _) => update_press_keys(resources, &mut solving_rune),
        State::Completed => unreachable!(),
    }

    let player_next_state = if matches!(solving_rune.state, State::Completed) {
        Player::Idle
    } else {
        Player::SolvingRune(solving_rune)
    };

    match next_action(&player.context) {
        Some(PlayerAction::SolveRune) => {
            let is_terminal = matches!(player_next_state, Player::Idle);
            if is_terminal {
                player.context.start_validating_rune();
            }
            transition_from_action!(player, player_next_state, is_terminal)
        }
        Some(_) => unreachable!(),
        None => transition!(player, Player::Idle), // Force cancel if not from action
    }
}

fn update_precondition(
    resources: &Resources,
    player_context: &PlayerContext,
    solving_rune: &mut SolvingRune,
) {
    transition_if!(
        solving_rune,
        State::FindRegion(ArrowsCalibrating::default(), Timeout::default(), None, 0),
        State::Precondition,
        player_context.is_stationary && resources.input.all_keys_cleared()
    )
}

fn update_find_region(
    resources: &Resources,
    solving_rune: &mut SolvingRune,
    interact_key: KeyKind,
) {
    const COOLDOWN_AND_SOLVE_TIMEOUT: u32 = 125;
    const SOLVE_INTERVAL: u32 = 15;

    let State::FindRegion(calibrating, timeout, cooldown_timeout, retry_count) = solving_rune.state
    else {
        panic!("solving rune state is not finding region")
    };

    // cooldown_timeout is used to wait for rune cooldown around ~4 secs before hitting interact
    // key again.
    if let Some(cooldown_timeout) = cooldown_timeout {
        match next_timeout_lifecycle(cooldown_timeout, COOLDOWN_AND_SOLVE_TIMEOUT) {
            Lifecycle::Updated(cooldown_timeout) | Lifecycle::Started(cooldown_timeout) => {
                transition!(
                    solving_rune,
                    State::FindRegion(calibrating, timeout, Some(cooldown_timeout), retry_count)
                )
            }
            Lifecycle::Ended => transition!(
                solving_rune,
                State::FindRegion(calibrating, timeout, None, retry_count)
            ),
        };
    }

    debug_assert!(cooldown_timeout.is_none());
    match next_timeout_lifecycle(timeout, COOLDOWN_AND_SOLVE_TIMEOUT) {
        Lifecycle::Started(timeout) => transition!(
            solving_rune,
            State::FindRegion(calibrating, timeout, cooldown_timeout, retry_count),
            {
                resources.input.send_key(interact_key);
            }
        ),
        Lifecycle::Ended => transition_if!(
            solving_rune,
            State::FindRegion(
                ArrowsCalibrating::default(),
                Timeout::default(),
                Some(Timeout::default()),
                retry_count + 1
            ),
            State::Completed,
            retry_count < MAX_RETRY_COUNT
        ),
        Lifecycle::Updated(timeout) => {
            if timeout.current.is_multiple_of(SOLVE_INTERVAL) {
                let arrows_state = try_ok_transition!(
                    solving_rune,
                    State::FindRegion(
                        ArrowsCalibrating::default(),
                        timeout,
                        cooldown_timeout,
                        retry_count
                    ),
                    resources.detector().detect_rune_arrows(calibrating)
                );
                match arrows_state {
                    ArrowsState::Calibrating(calibrating) => transition!(
                        solving_rune,
                        State::Solving(calibrating, Timeout::default())
                    ),
                    ArrowsState::Complete(_) => unreachable!(),
                }
            }

            transition!(
                solving_rune,
                State::FindRegion(calibrating, timeout, cooldown_timeout, retry_count)
            );
        }
    }
}

fn update_solving(resources: &Resources, solving_rune: &mut SolvingRune) {
    let State::Solving(calibrating, timeout) = solving_rune.state else {
        panic!("solving rune state is not solving")
    };

    match next_timeout_lifecycle(timeout, 150) {
        Lifecycle::Started(timeout) => {
            transition!(solving_rune, State::Solving(calibrating, timeout))
        }
        Lifecycle::Ended => transition!(solving_rune, State::Completed),
        Lifecycle::Updated(timeout) => {
            let arrows_state = try_ok_transition!(
                solving_rune,
                State::Completed,
                resources.detector().detect_rune_arrows(calibrating)
            );
            match arrows_state {
                ArrowsState::Calibrating(calibrating) => {
                    transition!(solving_rune, State::Solving(calibrating, timeout))
                }
                ArrowsState::Complete(pairs) => transition!(
                    solving_rune,
                    State::PressKeys(Timeout::default(), pairs.map(|(_, key)| key), 0),
                    {
                        #[cfg(debug_assertions)]
                        resources
                            .debug
                            .set_last_rune_result(resources.detector_cloned(), pairs);
                    }
                ),
            }
        }
    }
}

fn update_press_keys(resources: &Resources, solving_rune: &mut SolvingRune) {
    const PRESS_KEY_INTERVAL: u32 = 8;

    let State::PressKeys(timeout, keys, key_index) = solving_rune.state else {
        panic!("solving rune state is not pressing keys")
    };

    match next_timeout_lifecycle(timeout, PRESS_KEY_INTERVAL) {
        Lifecycle::Started(timeout) => {
            transition!(solving_rune, State::PressKeys(timeout, keys, key_index), {
                resources.input.send_key(keys[key_index]);
            })
        }
        Lifecycle::Ended => transition_if!(
            solving_rune,
            State::PressKeys(Timeout::default(), keys, key_index + 1),
            State::Completed,
            key_index + 1 < keys.len()
        ),
        Lifecycle::Updated(timeout) => {
            transition!(solving_rune, State::PressKeys(timeout, keys, key_index))
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
        bridge::{KeyKind, MockInput},
        detect::{ArrowsCalibrating, ArrowsState, MockDetector},
        ecs::Resources,
        player::{Player, PlayerContext, PlayerEntity},
    };

    #[test]
    fn update_solving_rune_precondition_to_find_region_when_stationary_and_keys_cleared() {
        let mut keys = MockInput::default();
        keys.expect_all_keys_cleared().once().returning(|| true);
        let resources = Resources::new(Some(keys), None);

        let mut player = PlayerEntity {
            state: Player::SolvingRune(SolvingRune::default()),
            context: PlayerContext::default(),
        };
        player.context.priority_action = Some(PlayerAction::SolveRune); // Avoid cancellation
        player.context.is_stationary = true;

        update_solving_rune_state(&resources, &mut player);

        assert_matches!(
            player.state,
            Player::SolvingRune(SolvingRune {
                state: State::FindRegion(_, _, None, 0)
            })
        );
    }

    #[test]
    fn update_find_region_to_solving_on_calibrating() {
        let mut detector = MockDetector::default();
        detector
            .expect_detect_rune_arrows()
            .return_once(|_| Ok(ArrowsState::Calibrating(ArrowsCalibrating::default())));
        let resources = Resources::new(None, Some(detector));
        let mut solving_rune = SolvingRune {
            state: State::FindRegion(
                ArrowsCalibrating::default(),
                Timeout {
                    started: true,
                    current: 89,
                    ..Default::default()
                },
                None,
                0,
            ),
        };

        update_find_region(&resources, &mut solving_rune, KeyKind::A);

        assert_matches!(
            solving_rune.state,
            State::Solving(
                _,
                Timeout {
                    started: false,
                    current: 0,
                    ..
                }
            )
        );
    }

    #[test]
    fn update_find_region_retry_on_timeout() {
        let mut detector = MockDetector::default();
        detector
            .expect_detect_rune_arrows()
            .return_once(move |_| Err(anyhow!("rune region not found")));
        let resources = Resources::new(None, Some(detector));
        let mut solving_rune = SolvingRune {
            state: State::FindRegion(
                ArrowsCalibrating::default(),
                Timeout {
                    started: true,
                    current: 125,
                    ..Default::default()
                },
                None,
                0,
            ),
        };

        update_find_region(&resources, &mut solving_rune, KeyKind::A);

        assert_matches!(
            solving_rune.state,
            State::FindRegion(
                _,
                Timeout { started: false, .. },
                Some(Timeout { started: false, .. }),
                1
            )
        );
    }

    #[test]
    fn update_find_region_retry_cooldown_timeout_to_none() {
        let resources = Resources::new(None, None);
        let mut solving_rune = SolvingRune {
            state: State::FindRegion(
                ArrowsCalibrating::default(),
                Timeout::default(),
                Some(Timeout {
                    started: true,
                    current: 125,
                    ..Default::default()
                }),
                1,
            ),
        };

        update_find_region(&resources, &mut solving_rune, KeyKind::A);

        assert_matches!(solving_rune.state, State::FindRegion(_, _, None, 1));
    }

    #[test]
    fn update_solving_to_completed_on_error() {
        let mut detector = MockDetector::default();
        detector
            .expect_detect_rune_arrows()
            .returning(|_| Err(anyhow!("fail")));
        let resources = Resources::new(None, Some(detector));
        let mut solving_rune = SolvingRune {
            state: State::Solving(
                ArrowsCalibrating::default(),
                Timeout {
                    started: true,
                    ..Default::default()
                },
            ),
        };

        update_solving(&resources, &mut solving_rune);

        assert_matches!(solving_rune.state, State::Completed);
    }

    #[test]
    fn update_solving_to_solving_on_incomplete() {
        let mut detector = MockDetector::default();
        detector
            .expect_detect_rune_arrows()
            .return_once(move |_| Ok(ArrowsState::Calibrating(ArrowsCalibrating::default())));
        let resources = Resources::new(None, Some(detector));
        let mut solving_rune = SolvingRune {
            state: State::Solving(
                ArrowsCalibrating::default(),
                Timeout {
                    started: true,
                    ..Default::default()
                },
            ),
        };

        update_solving(&resources, &mut solving_rune);

        assert_matches!(
            solving_rune.state,
            State::Solving(_, Timeout { started: true, .. })
        );
    }

    #[test]
    fn update_solving_to_press_keys_on_complete() {
        let expected_keys = [
            (Rect::default(), KeyKind::A),
            (Rect::default(), KeyKind::S),
            (Rect::default(), KeyKind::D),
            (Rect::default(), KeyKind::F),
        ];
        let mut detector = MockDetector::default();
        detector.expect_clone();
        detector
            .expect_detect_rune_arrows()
            .return_once(move |_| Ok(ArrowsState::Complete(expected_keys)));
        let resources = Resources::new(None, Some(detector));
        let mut solving_rune = SolvingRune {
            state: State::Solving(
                ArrowsCalibrating::default(),
                Timeout {
                    started: true,
                    ..Default::default()
                },
            ),
        };

        update_solving(&resources, &mut solving_rune);

        assert_matches!(
            solving_rune.state,
            State::PressKeys(
                Timeout {
                    started: false,
                    current: 0,
                    ..
                },
                [KeyKind::A, KeyKind::S, KeyKind::D, KeyKind::F],
                0
            )
        );
    }

    #[test]
    fn update_press_keys_to_completed_after_all_keys_sent() {
        let expected_keys = [KeyKind::A, KeyKind::S, KeyKind::D, KeyKind::F];
        let mut solving_rune = SolvingRune {
            state: State::PressKeys(Timeout::default(), expected_keys, 0),
        };

        for idx in 0..expected_keys.len() {
            let mut keys = MockInput::default();
            keys.expect_send_key().with(eq(expected_keys[idx]));
            let resources = Resources::new(Some(keys), None);

            // Start key press
            update_press_keys(&resources, &mut solving_rune);

            // Simulate timeout end (advance or complete)
            solving_rune.state = State::PressKeys(
                Timeout {
                    started: true,
                    current: 8,
                    ..Default::default()
                },
                expected_keys,
                idx,
            );
            update_press_keys(&resources, &mut solving_rune);
        }

        assert_matches!(solving_rune.state, State::Completed);
    }
}
