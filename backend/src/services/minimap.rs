use std::fmt::Debug;

#[cfg(test)]
use mockall::automock;

use crate::{
    database::Minimap as MinimapData,
    minimap::{Minimap, MinimapContext, MinimapEntity},
    pathing::Platform,
    player::PlayerContext,
};

/// A service to handle minimap-related incoming requests.
#[cfg_attr(test, automock)]
pub trait MinimapService: Debug {
    /// Creates a new [`MinimapData`] from currently detected minimap with `name`.
    fn create(&self, minimap_state: Minimap, name: String) -> Option<MinimapData>;

    /// Gets the currently in use [`MinimapData`].
    #[allow(clippy::needless_lifetimes)]
    fn minimap<'a>(&'a self) -> Option<&'a MinimapData>;

    /// Gets the currently in use preset.
    fn preset(&self) -> Option<String>;

    /// Sets new `minimap` and `preset` to be used.
    fn update_minimap_preset(&mut self, minimap: Option<MinimapData>, preset: Option<String>);

    /// Updates `minimap_context` and `player_context` with information from the currently in use
    /// [`MinimapData`] and preset.
    fn apply(&self, minimap_context: &mut MinimapContext, player_context: &mut PlayerContext);

    /// Re-detects current minimap.
    fn redetect(&self, minimap: &mut MinimapEntity);
}

#[derive(Debug, Default)]
pub struct DefaultMinimapService {
    minimap: Option<MinimapData>,
    preset: Option<String>,
}

impl MinimapService for DefaultMinimapService {
    fn create(&self, minimap_state: Minimap, name: String) -> Option<MinimapData> {
        if let Minimap::Idle(idle) = minimap_state {
            Some(MinimapData {
                name,
                width: idle.bbox.width,
                height: idle.bbox.height,
                ..MinimapData::default()
            })
        } else {
            None
        }
    }

    fn minimap(&self) -> Option<&MinimapData> {
        self.minimap.as_ref()
    }

    fn preset(&self) -> Option<String> {
        self.preset.clone()
    }

    fn update_minimap_preset(&mut self, minimap: Option<MinimapData>, preset: Option<String>) {
        self.minimap = minimap;
        self.preset = preset;
    }

    fn apply(&self, minimap_context: &mut MinimapContext, player_context: &mut PlayerContext) {
        let platforms = self
            .minimap()
            .map(|data| {
                data.platforms
                    .iter()
                    .copied()
                    .map(Platform::from)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        minimap_context.set_platforms(platforms);

        player_context.reset();
        if let Some(minimap) = self.minimap() {
            player_context.config.rune_platforms_pathing = minimap.rune_platforms_pathing;
            player_context.config.rune_platforms_pathing_up_jump_only =
                minimap.rune_platforms_pathing_up_jump_only;
            player_context.config.auto_mob_platforms_pathing = minimap.auto_mob_platforms_pathing;
            player_context
                .config
                .auto_mob_platforms_pathing_up_jump_only =
                minimap.auto_mob_platforms_pathing_up_jump_only;
            player_context.config.auto_mob_platforms_bound = minimap.auto_mob_platforms_bound;
            player_context.config.auto_mob_use_key_when_pathing =
                minimap.auto_mob_use_key_when_pathing;
            player_context
                .config
                .auto_mob_use_key_when_pathing_update_millis =
                minimap.auto_mob_use_key_when_pathing_update_millis;
        }
    }

    fn redetect(&self, minimap: &mut MinimapEntity) {
        minimap.state = Minimap::Detecting;
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;

    use opencv::core::Rect;

    use super::*;
    use crate::{
        Platform as DatabasePlatform,
        minimap::{Minimap, MinimapIdle},
        pathing::Platform,
    };

    fn mock_idle_minimap() -> Minimap {
        let mut idle = MinimapIdle::default();
        idle.bbox = Rect::new(0, 0, 100, 100);
        Minimap::Idle(idle)
    }

    fn mock_minimap_data() -> MinimapData {
        MinimapData {
            name: "MapData".to_string(),
            width: 100,
            height: 100,
            rune_platforms_pathing: true,
            rune_platforms_pathing_up_jump_only: true,
            auto_mob_platforms_pathing: true,
            auto_mob_platforms_bound: true,
            ..Default::default()
        }
    }

    #[test]
    fn create_returns_some_when_idle_minimap() {
        let service = DefaultMinimapService::default();

        let result = service.create(mock_idle_minimap(), "MapData".to_string());

        assert!(result.is_some());
        assert_eq!(
            result.unwrap(),
            MinimapData {
                name: "MapData".to_string(),
                width: 100,
                height: 100,
                ..Default::default()
            }
        );
    }

    #[test]
    fn create_returns_none_when_not_idle_minimap() {
        let service = DefaultMinimapService::default();

        let result = service.create(Minimap::Detecting, "ShouldNotExist".to_string());

        assert!(result.is_none());
    }

    #[test]
    fn set_minimap_and_preset() {
        let mut service = DefaultMinimapService::default();
        let minimap = mock_minimap_data();
        let preset = Some("custom".to_string());

        service.update_minimap_preset(Some(minimap.clone()), preset.clone());

        assert_eq!(service.minimap, Some(minimap));
        assert_eq!(service.preset, preset);
    }

    #[test]
    fn redetect_sets_minimap_to_detecting() {
        let service = DefaultMinimapService::default();
        let mut minimap = MinimapEntity {
            state: mock_idle_minimap(),
            context: MinimapContext::default(),
        };

        service.redetect(&mut minimap);

        assert_matches!(minimap.state, Minimap::Detecting);
    }

    #[test]
    fn update_reset_minimap_state_platforms() {
        let service = DefaultMinimapService::default();
        let mut player_context = PlayerContext::default();
        let mut minimap = MinimapEntity {
            state: mock_idle_minimap(),
            context: MinimapContext::default(),
        };
        minimap
            .context
            .set_platforms(vec![Platform::from(DatabasePlatform {
                x_start: 3,
                x_end: 3,
                y: 10,
            })]);

        service.apply(&mut minimap.context, &mut player_context);

        assert!(service.minimap.is_none());
        assert!(service.preset.is_none());
        assert!(minimap.context.platforms().is_empty());
    }

    #[test]
    fn update_keep_player_config() {
        let service = DefaultMinimapService::default();
        let mut minimap_context = MinimapContext::default();
        let mut player_context = PlayerContext::default();
        player_context.config.rune_platforms_pathing = true;
        player_context.config.rune_platforms_pathing_up_jump_only = true;

        service.apply(&mut minimap_context, &mut player_context);
        assert!(player_context.config.rune_platforms_pathing); // Doesn't change
        assert!(player_context.config.rune_platforms_pathing_up_jump_only); // Doesn't change
    }

    #[test]
    fn update_change_player_config() {
        let service = DefaultMinimapService {
            minimap: Some(mock_minimap_data()),
            preset: Some("preset".to_string()),
        };
        let mut minimap_context = MinimapContext::default();
        let mut player_state = PlayerContext::default();

        service.apply(&mut minimap_context, &mut player_state);

        assert!(player_state.config.rune_platforms_pathing);
        assert!(player_state.config.rune_platforms_pathing_up_jump_only);
        assert!(player_state.config.auto_mob_platforms_pathing);
        assert!(player_state.config.auto_mob_platforms_bound);
    }
}
