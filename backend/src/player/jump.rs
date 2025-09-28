use super::{
    Player,
    moving::{MOVE_TIMEOUT, Moving},
    state::LastMovement,
    timeout::{ChangeAxis, MovingLifecycle, next_moving_lifecycle_with_axis},
};
use crate::{ecs::Resources, player::PlayerEntity, transition, transition_to_moving};

const TIMEOUT: u32 = MOVE_TIMEOUT + 3;

pub fn update_jumping_state(resources: &Resources, player: &mut PlayerEntity, moving: Moving) {
    match next_moving_lifecycle_with_axis(
        moving,
        player.context.last_known_pos.expect("in positional state"),
        TIMEOUT,
        ChangeAxis::Vertical,
    ) {
        MovingLifecycle::Started(moving) => transition!(player, Player::Jumping(moving), {
            resources.input.send_key(player.context.config.jump_key);
            player.context.last_movement = Some(LastMovement::Jumping);
        }),
        MovingLifecycle::Ended(moving) => transition_to_moving!(player, moving),
        MovingLifecycle::Updated(moving) => transition!(player, Player::Jumping(moving)),
    }
}
