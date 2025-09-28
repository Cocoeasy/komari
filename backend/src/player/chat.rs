use crate::{
    array::Array,
    bridge::KeyKind,
    ecs::Resources,
    player::{
        Player, PlayerEntity, next_action,
        timeout::{Lifecycle, Timeout, next_timeout_lifecycle},
    },
    transition, transition_from_action, transition_if, try_some_transition,
};

const MAX_RETRY: u32 = 3;
const MAX_CONTENT_LENGTH: usize = 256;

pub type ChattingContent = Array<char, MAX_CONTENT_LENGTH>;

impl ChattingContent {
    pub const MAX_LENGTH: usize = MAX_CONTENT_LENGTH;

    #[inline]
    pub fn from_string(content: String) -> ChattingContent {
        ChattingContent::from_iter(content.into_chars())
    }
}

#[derive(Debug, Clone, Copy)]
enum State {
    OpeningMenu(Timeout, u32),
    Typing(Timeout, usize),
    Completing(Timeout, bool),
}

#[derive(Debug, Clone, Copy)]
pub struct Chatting {
    state: State,
    content: ChattingContent,
}

impl Chatting {
    pub fn new(content: ChattingContent) -> Self {
        Self {
            state: State::OpeningMenu(Timeout::default(), 0),
            content,
        }
    }
}

pub fn update_chatting_state(
    resources: &Resources,
    player: &mut PlayerEntity,
    mut chatting: Chatting,
) {
    match chatting.state {
        State::OpeningMenu(_, _) => update_opening_menu(resources, &mut chatting),
        State::Typing(_, _) => update_typing(resources, &mut chatting),
        State::Completing(_, _) => update_completing(resources, &mut chatting),
    };

    let player_next_state = if matches!(chatting.state, State::Completing(_, true)) {
        Player::Idle
    } else {
        Player::Chatting(chatting)
    };

    match next_action(&player.context) {
        Some(_) => transition_from_action!(
            player,
            player_next_state,
            matches!(player_next_state, Player::Idle)
        ),
        None => transition!(player, Player::Idle), // Force cancel if not from action
    }
}

fn update_opening_menu(resources: &Resources, chatting: &mut Chatting) {
    let State::OpeningMenu(timeout, retry_count) = chatting.state else {
        panic!("chatting state is not opening menu");
    };

    match next_timeout_lifecycle(timeout, 35) {
        Lifecycle::Started(timeout) => {
            transition!(chatting, State::OpeningMenu(timeout, retry_count), {
                resources.input.send_key(KeyKind::Enter);
            })
        }
        Lifecycle::Ended => {
            transition_if!(
                chatting,
                State::Typing(Timeout::default(), 0),
                resources.detector().detect_chat_menu_opened()
            );
            transition_if!(
                chatting,
                State::OpeningMenu(timeout, retry_count + 1),
                State::Completing(timeout, false),
                retry_count < MAX_RETRY
            );
        }
        Lifecycle::Updated(timeout) => {
            transition!(chatting, State::OpeningMenu(timeout, retry_count))
        }
    }
}

fn update_typing(resources: &Resources, chatting: &mut Chatting) {
    let State::Typing(timeout, index) = chatting.state else {
        panic!("chatting state is not typing");
    };

    match next_timeout_lifecycle(timeout, 3) {
        Lifecycle::Started(timeout) | Lifecycle::Updated(timeout) => {
            transition!(chatting, State::Typing(timeout, index))
        }
        Lifecycle::Ended => {
            let key = try_some_transition!(
                chatting,
                State::Completing(Timeout::default(), false),
                chatting
                    .content
                    .as_slice()
                    .get(index)
                    .copied()
                    .and_then(to_key_kind)
            );
            resources.input.send_key(key);
            transition_if!(
                chatting,
                State::Typing(Timeout::default(), index + 1),
                index + 1 < chatting.content.len()
            );

            transition!(chatting, State::Completing(Timeout::default(), false), {
                resources.input.send_key(KeyKind::Enter);
            });
        }
    }
}

fn update_completing(resources: &Resources, chatting: &mut Chatting) {
    let State::Completing(timeout, _) = chatting.state else {
        panic!("chatting state is not completing");
    };

    match next_timeout_lifecycle(timeout, 35) {
        Lifecycle::Updated(timeout) | Lifecycle::Started(timeout) => {
            transition!(chatting, State::Completing(timeout, false));
        }
        Lifecycle::Ended => transition!(chatting, State::Completing(timeout, true), {
            if resources.detector().detect_chat_menu_opened() {
                resources.input.send_key(KeyKind::Esc);
            }
        }),
    }
}

// TODO: Support non-ASCII characters and ASCII capital characters
#[inline]
fn to_key_kind(character: char) -> Option<KeyKind> {
    match character {
        'A' | 'a' => Some(KeyKind::A),
        'B' | 'b' => Some(KeyKind::B),
        'C' | 'c' => Some(KeyKind::C),
        'D' | 'd' => Some(KeyKind::D),
        'E' | 'e' => Some(KeyKind::E),
        'F' | 'f' => Some(KeyKind::F),
        'G' | 'g' => Some(KeyKind::G),
        'H' | 'h' => Some(KeyKind::H),
        'I' | 'i' => Some(KeyKind::I),
        'J' | 'j' => Some(KeyKind::J),
        'K' | 'k' => Some(KeyKind::K),
        'L' | 'l' => Some(KeyKind::L),
        'M' | 'm' => Some(KeyKind::M),
        'N' | 'n' => Some(KeyKind::N),
        'O' | 'o' => Some(KeyKind::O),
        'P' | 'p' => Some(KeyKind::P),
        'Q' | 'q' => Some(KeyKind::Q),
        'R' | 'r' => Some(KeyKind::R),
        'S' | 's' => Some(KeyKind::S),
        'T' | 't' => Some(KeyKind::T),
        'U' | 'u' => Some(KeyKind::U),
        'V' | 'v' => Some(KeyKind::V),
        'W' | 'w' => Some(KeyKind::W),
        'X' | 'x' => Some(KeyKind::X),
        'Y' | 'y' => Some(KeyKind::Y),
        'Z' | 'z' => Some(KeyKind::Z),

        '0' => Some(KeyKind::Zero),
        '1' => Some(KeyKind::One),
        '2' => Some(KeyKind::Two),
        '3' => Some(KeyKind::Three),
        '4' => Some(KeyKind::Four),
        '5' => Some(KeyKind::Five),
        '6' => Some(KeyKind::Six),
        '7' => Some(KeyKind::Seven),
        '8' => Some(KeyKind::Eight),
        '9' => Some(KeyKind::Nine),

        ' ' => Some(KeyKind::Space),
        '`' | '~' => Some(KeyKind::Tilde),
        '\'' | '"' => Some(KeyKind::Quote),
        ';' => Some(KeyKind::Semicolon),
        ',' => Some(KeyKind::Comma),
        '.' => Some(KeyKind::Period),
        '/' => Some(KeyKind::Slash),

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;

    use mockall::predicate::eq;

    use super::*;
    use crate::{bridge::MockInput, detect::MockDetector};

    #[test]
    fn update_opening_menu_detects_chat_menu_and_transitions_to_typing() {
        let mut detector = MockDetector::default();
        detector.expect_detect_chat_menu_opened().returning(|| true);
        let resources = Resources::new(None, Some(detector));
        let mut chatting = Chatting::new(Array::new());
        chatting.state = State::OpeningMenu(
            Timeout {
                current: 35,
                started: true,
                ..Default::default()
            },
            0,
        );

        update_opening_menu(&resources, &mut chatting);

        assert_matches!(chatting.state, State::Typing(_, 0));
    }

    #[test]
    fn update_opening_menu_retries_when_chat_menu_not_opened() {
        let mut detector = MockDetector::default();
        detector
            .expect_detect_chat_menu_opened()
            .returning(|| false);
        let resources = Resources::new(None, Some(detector));
        let mut chatting = Chatting::new(Array::new());
        chatting.state = State::OpeningMenu(
            Timeout {
                current: 35,
                started: true,
                ..Default::default()
            },
            0,
        );

        update_opening_menu(&resources, &mut chatting);

        assert_matches!(chatting.state, State::OpeningMenu(_, 1));
    }

    #[test]
    fn update_opening_menu_fails_after_max_retries() {
        let mut detector = MockDetector::default();
        detector
            .expect_detect_chat_menu_opened()
            .returning(|| false);
        let resources = Resources::new(None, Some(detector));
        let mut chatting = Chatting::new(Array::new());
        chatting.state = State::OpeningMenu(
            Timeout {
                current: 35,
                started: true,
                ..Default::default()
            },
            MAX_RETRY,
        );

        update_opening_menu(&resources, &mut chatting);

        assert_matches!(chatting.state, State::Completing(_, false));
    }

    #[test]
    fn update_typing_sends_character_key_and_progresses() {
        let mut keys = MockInput::default();
        keys.expect_send_key().once().with(eq(KeyKind::A));
        keys.expect_send_key().once().with(eq(KeyKind::B));
        keys.expect_send_key().once().with(eq(KeyKind::C));
        let resources = Resources::new(Some(keys), None);
        let mut chatting = Chatting::new(Array::from_iter(['a', 'b', 'c', 'd']));

        for i in 0..3 {
            chatting.state = State::Typing(
                Timeout {
                    current: 3,
                    started: true,
                    ..Default::default()
                },
                i,
            );

            update_typing(&resources, &mut chatting);

            assert_matches!(chatting.state, State::Typing(_, index) if index == i + 1);
        }
    }

    #[test]
    fn update_typing_finishes_after_last_character() {
        let mut keys = MockInput::default();
        keys.expect_send_key().once().with(eq(KeyKind::A));
        keys.expect_send_key().once().with(eq(KeyKind::Enter));
        let resources = Resources::new(Some(keys), None);
        let mut chatting = Chatting::new(Array::from_iter(['a']));
        chatting.state = State::Typing(
            Timeout {
                current: 3,
                started: true,
                ..Default::default()
            },
            0,
        );

        update_typing(&resources, &mut chatting);

        assert_matches!(chatting.state, State::Completing(_, false));
    }

    #[test]
    fn update_typing_completes_if_char_not_found() {
        let resources = Resources::new(None, None);
        let mut chatting = Chatting::new(Array::new());
        chatting.state = State::Typing(
            Timeout {
                current: 3,
                started: true,
                ..Default::default()
            },
            0,
        );

        update_typing(&resources, &mut chatting);

        assert_matches!(chatting.state, State::Completing(_, false));
    }

    #[test]
    fn update_completing_sends_esc_if_menu_open() {
        let mut detector = MockDetector::default();
        detector.expect_detect_chat_menu_opened().returning(|| true);
        let mut keys = MockInput::default();
        keys.expect_send_key().once().with(eq(KeyKind::Esc));
        let resources = Resources::new(Some(keys), Some(detector));
        let mut chatting = Chatting::new(Array::new());
        chatting.state = State::Completing(
            Timeout {
                current: 35,
                started: true,
                ..Default::default()
            },
            false,
        );

        update_completing(&resources, &mut chatting);

        assert_matches!(chatting.state, State::Completing(_, true));
    }
}
