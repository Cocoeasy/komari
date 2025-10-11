use std::fmt::Debug;

use log::debug;
#[cfg(test)]
use mockall::{automock, concretize};
use opencv::core::{MatTraitConst, MatTraitConstManual, Rect, Vec4b};
use tokio::{
    spawn,
    sync::broadcast::{self, Receiver, Sender},
};

use crate::{
    BoundQuadrant, Character, DatabaseEvent, GameOperation, GameState, KeyBinding,
    KeyBindingConfiguration, Localization, Minimap, Settings,
    bridge::InputReceiver,
    database_event_receiver,
    ecs::{Resources, World},
    minimap,
    operation::Operation,
    player::Quadrant,
    skill::SkillKind,
};

#[derive(Debug)]
pub enum GameEvent {
    ToggleOperation,
    MinimapUpdated(Option<Minimap>),
    CharacterUpdated(Option<Character>),
    SettingsUpdated(Settings),
    LocalizationUpdated(Localization),
    NavigationPathsUpdated,
}

/// A service to handle state broadcasting and event polling.
#[cfg_attr(test, automock)]
pub trait GameService: Debug {
    fn poll_events(
        &mut self,
        minimap_id: Option<i64>,
        character_id: Option<i64>,
        settings: &Settings,
    ) -> Vec<GameEvent>;

    /// Gets a mutable reference to [`InputReceiver`].
    fn input_receiver_mut(&mut self) -> &mut dyn InputReceiver;

    /// Broadcasts game state to listeners.
    #[cfg_attr(test, concretize)]
    fn broadcast_state(&self, resources: &Resources, world: &World, minimap: Option<&Minimap>);

    /// Subscribes to game state.
    fn subscribe_state(&self) -> Receiver<GameState>;

    /// Subscribes to key event.
    fn subscribe_key(&self) -> Receiver<KeyBinding>;
}

#[derive(Debug)]
pub struct DefaultGameService {
    input_rx: Box<dyn InputReceiver>,
    key_tx: Sender<KeyBinding>,
    database_event_rx: Receiver<DatabaseEvent>,
    game_state_tx: Sender<GameState>,
}

impl DefaultGameService {
    pub fn new(input_rx: impl InputReceiver) -> Self {
        Self {
            input_rx: Box::new(input_rx),
            key_tx: broadcast::channel(1).0,
            database_event_rx: database_event_receiver(),
            game_state_tx: broadcast::channel(1).0,
        }
    }
}

impl GameService for DefaultGameService {
    fn poll_events(
        &mut self,
        minimap_id: Option<i64>,
        character_id: Option<i64>,
        settings: &Settings,
    ) -> Vec<GameEvent> {
        let mut events = Vec::new();

        if let Some(event) = poll_key(self, settings) {
            events.push(event);
        }
        if let Some(event) = poll_database(self, minimap_id, character_id) {
            events.push(event);
        }

        events
    }

    fn input_receiver_mut(&mut self) -> &mut dyn InputReceiver {
        self.input_rx.as_mut()
    }

    #[cfg_attr(test, concretize)]
    fn broadcast_state(
        &self,
        resources: &Resources,
        world: &World,
        minimap_data: Option<&Minimap>,
    ) {
        if self.game_state_tx.is_empty() {
            let position = world
                .player
                .context
                .last_known_pos
                .map(|pos| (pos.x, pos.y));
            let state = world.player.state.to_string();
            let health = world.player.context.health();
            let normal_action = world.player.context.normal_action_name();
            let priority_action = world.player.context.priority_action_name();
            let erda_shower_state = world.skills[SkillKind::ErdaShower].state.to_string();
            let destinations = world
                .player
                .context
                .last_destinations
                .clone()
                .map(|points| {
                    points
                        .into_iter()
                        .map(|point| (point.x, point.y))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let operation = match resources.operation {
                Operation::HaltUntil { instant, .. } => GameOperation::HaltUntil(instant),
                Operation::TemporaryHalting { resume, .. } => {
                    GameOperation::TemporaryHalting(resume)
                }
                Operation::Halting => GameOperation::Halting,
                Operation::Running => GameOperation::Running,
                Operation::RunUntil { instant, .. } => GameOperation::RunUntil(instant),
            };
            let idle = if let minimap::Minimap::Idle(idle) = world.minimap.state {
                Some(idle)
            } else {
                None
            };
            let platforms_bound = if minimap_data.is_some_and(|data| data.auto_mob_platforms_bound)
                && let Some(idle) = idle
            {
                idle.platforms_bound.map(|bound| bound.into())
            } else {
                None
            };
            let portals = if let Some(idle) = idle {
                idle.portals()
                    .into_iter()
                    .map(|portal| portal.into())
                    .collect::<Vec<_>>()
            } else {
                vec![]
            };
            let auto_mob_quadrant = world
                .player
                .context
                .auto_mob_last_quadrant()
                .map(|quadrant| match quadrant {
                    Quadrant::TopLeft => BoundQuadrant::TopLeft,
                    Quadrant::TopRight => BoundQuadrant::TopRight,
                    Quadrant::BottomRight => BoundQuadrant::BottomRight,
                    Quadrant::BottomLeft => BoundQuadrant::BottomLeft,
                });
            let detector = if resources.detector.is_some() {
                Some(resources.detector_cloned())
            } else {
                None
            };
            let sender = self.game_state_tx.clone();

            spawn(async move {
                let frame = if let Some((detector, idle)) = detector.zip(idle) {
                    Some(minimap_frame_from(idle.bbox, detector.mat()))
                } else {
                    None
                };
                let game_state = GameState {
                    position,
                    health,
                    state,
                    normal_action,
                    priority_action,
                    erda_shower_state,
                    destinations,
                    operation,
                    frame,
                    platforms_bound,
                    portals,
                    auto_mob_quadrant,
                };
                let _ = sender.send(game_state);
            });
        }
    }

    fn subscribe_state(&self) -> Receiver<GameState> {
        self.game_state_tx.subscribe()
    }

    fn subscribe_key(&self) -> Receiver<KeyBinding> {
        self.key_tx.subscribe()
    }
}

#[inline]
fn minimap_frame_from(bbox: Rect, mat: &impl MatTraitConst) -> (Vec<u8>, usize, usize) {
    let minimap = mat
        .roi(bbox)
        .unwrap()
        .iter::<Vec4b>()
        .unwrap()
        .flat_map(|bgra| {
            let bgra = bgra.1;
            [bgra[2], bgra[1], bgra[0], 255]
        })
        .collect::<Vec<u8>>();
    (minimap, bbox.width as usize, bbox.height as usize)
}

// TODO: should only handle a single matched key binding
#[inline]
fn poll_key(service: &mut DefaultGameService, settings: &Settings) -> Option<GameEvent> {
    let received_key = service.input_rx.try_recv().ok()?;
    debug!(target: "event", "received key {received_key:?}");

    if let KeyBindingConfiguration { key, enabled: true } = settings.toggle_actions_key
        && key == received_key.into()
    {
        return Some(GameEvent::ToggleOperation);
    }

    let _ = service.key_tx.send(received_key.into());
    None
}

#[inline]
fn poll_database(
    service: &mut DefaultGameService,
    minimap_id: Option<i64>,
    character_id: Option<i64>,
) -> Option<GameEvent> {
    let event = service.database_event_rx.try_recv().ok()?;
    debug!(target: "handler", "received database event {event:?}");

    match event {
        DatabaseEvent::MinimapUpdated(minimap) => {
            let id = minimap
                .id
                .expect("valid minimap id if updated from database");
            if Some(id) == minimap_id {
                return Some(GameEvent::MinimapUpdated(Some(minimap)));
            }
        }
        DatabaseEvent::MinimapDeleted(deleted_id) => {
            if Some(deleted_id) == minimap_id {
                return Some(GameEvent::MinimapUpdated(None));
            }
        }
        DatabaseEvent::NavigationPathsUpdated | DatabaseEvent::NavigationPathsDeleted => {
            return Some(GameEvent::NavigationPathsUpdated);
        }
        DatabaseEvent::SettingsUpdated(settings) => {
            return Some(GameEvent::SettingsUpdated(settings));
        }
        DatabaseEvent::LocalizationUpdated(localization) => {
            return Some(GameEvent::LocalizationUpdated(localization));
        }
        DatabaseEvent::CharacterUpdated(character) => {
            let updated_id = character
                .id
                .expect("valid character id if updated from database");
            if Some(updated_id) == character_id {
                return Some(GameEvent::CharacterUpdated(Some(character)));
            }
        }
        DatabaseEvent::CharacterDeleted(deleted_id) => {
            if Some(deleted_id) == character_id {
                return Some(GameEvent::CharacterUpdated(None));
            }
        }
    }

    None
}
