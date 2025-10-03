use opencv::core::Point;

use super::timeout::{Lifecycle, Timeout, next_timeout_lifecycle};
use crate::{
    bridge::KeyKind,
    ecs::Resources,
    minimap::Minimap,
    player::{MOVE_TIMEOUT, Player, PlayerEntity},
    transition,
};

/// A threshold to consider spamming falling action
///
/// This is when the player is inside the top edge of minimap. At least for higher level maps, this
/// seems rare but one possible map is The Forest Of Earth in Arcana.
const Y_IGNORE_THRESHOLD: i32 = 18;

/// Updates the [`Player::Unstucking`] contextual state
///
/// This state can only be transitioned to when [`PlayerState::unstuck_counter`] reached the fixed
/// threshold or when the player moved into the edges of the minimap.
/// If [`PlayerState::unstuck_consecutive_counter`] has not reached the threshold and the player
/// moved into the left/right/top edges of the minimap, it will try to move
/// out as appropriate. It will also try to press ESC key to exit any dialog.
///
/// Each initial transition to [`Player::Unstucking`] increases
/// the [`PlayerState::unstuck_consecutive_counter`] by one. If the threshold is reached, this
/// state will enter GAMBA mode. And by definition, it means `random bullsh*t go`.
pub fn update_unstucking_state(
    resources: &Resources,
    player: &mut PlayerEntity,
    minimap_state: Minimap,
    timeout: Timeout,
    gamba_mode: bool,
) {
    let Minimap::Idle(idle) = minimap_state else {
        transition!(player, Player::Detecting);
    };
    let context = &mut player.context;
    let pos = context
        .last_known_pos
        .map(|pos| Point::new(pos.x, idle.bbox.height - pos.y));
    let gamba_mode = gamba_mode || pos.is_none();

    match next_timeout_lifecycle(timeout, MOVE_TIMEOUT) {
        Lifecycle::Started(timeout) => {
            if (!gamba_mode && resources.detector().detect_esc_settings())
                || (gamba_mode && resources.rng.random_bool(0.5))
            {
                resources.input.send_key(KeyKind::Esc);
            }

            let to_right = match (gamba_mode, pos) {
                (true, _) => resources.rng.random_bool(0.5),
                (_, Some(Point { y, .. })) if y <= Y_IGNORE_THRESHOLD => {
                    transition!(player, Player::Unstucking(timeout, gamba_mode))
                }
                (_, Some(Point { x, .. })) => x <= idle.bbox.width / 2,
                (_, None) => unreachable!(),
            };
            if to_right {
                resources.input.send_key_down(KeyKind::Right);
            } else {
                resources.input.send_key_up(KeyKind::Left);
            }

            transition!(player, Player::Unstucking(timeout, gamba_mode));
        }
        Lifecycle::Ended => transition!(player, Player::Detecting, {
            resources.input.send_key_up(KeyKind::Right);
            resources.input.send_key_up(KeyKind::Left);
        }),
        Lifecycle::Updated(timeout) => {
            transition!(player, Player::Unstucking(timeout, gamba_mode), {
                let send_space = match (gamba_mode, pos) {
                    (true, _) => true,
                    (_, Some(pos)) if pos.y > Y_IGNORE_THRESHOLD => true,
                    _ => false,
                };
                if send_space {
                    resources.input.send_key(context.config.jump_key);
                }
            })
        }
    }
}
