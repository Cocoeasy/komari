use super::{
    AutoMob, Player, PlayerAction,
    actions::next_action,
    timeout::{Lifecycle, Timeout, next_timeout_lifecycle},
};
use crate::{Position, player::PlayerEntity, transition, transition_from_action, transition_if};

/// Updates the [`Player::Stalling`] contextual state.
///
/// This state stalls for the specified number of `max_timeout`. Upon timing out,
/// it will return to [`PlayerState::stalling_timeout_state`] if [`Some`] or
/// [`Player::Idle`] if [`None`]. And [`Player::Idle`] is considered the terminal state if
/// there is an action. [`PlayerState::stalling_timeout_state`] is currently only [`Some`] when
/// it is transitioned via [`Player::UseKey`].
///
/// If this state timeout in auto mob with terminal state, it will perform
/// auto mob reachable `y` solidifying if needed.
pub fn update_stalling_state(player: &mut PlayerEntity, timeout: Timeout, max_timeout: u32) {
    let next_state = match next_timeout_lifecycle(timeout, max_timeout) {
        Lifecycle::Started(timeout) => Player::Stalling(timeout, max_timeout),
        Lifecycle::Ended => player
            .context
            .stalling_timeout_state
            .take()
            .unwrap_or(Player::Idle),
        Lifecycle::Updated(timeout) => Player::Stalling(timeout, max_timeout),
    };
    let is_terminal = matches!(next_state, Player::Idle);

    match next_action(&player.context) {
        Some(PlayerAction::AutoMob(AutoMob {
            position: Position { y, .. },
            ..
        })) => {
            if is_terminal && player.context.auto_mob_reachable_y_require_update(y) {
                transition_if!(
                    player,
                    Player::Stalling(Timeout::default(), max_timeout),
                    !player.context.is_stationary
                );

                player.context.auto_mob_track_reachable_y(y);
            }

            transition_from_action!(player, next_state, is_terminal);
        }
        Some(PlayerAction::PingPong(_) | PlayerAction::Key(_) | PlayerAction::Move(_)) => {
            transition_from_action!(player, next_state, is_terminal);
        }
        Some(PlayerAction::SolveRune) | None => transition!(player, next_state),
        Some(_) => unreachable!(),
    }
}
