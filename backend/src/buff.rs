use std::{
    mem,
    ops::{Index, IndexMut},
};

use anyhow::Result;
use strum::EnumIter;

use crate::{
    Character, Settings,
    context::{Context, Contextual, ControlFlow},
    player::Player,
    task::{Task, Update, update_detection_task},
};

const COMMON_FAIL_COUNT: u32 = 5;
const FAMILIAR_FAIL_COUNT: u32 = 2;
const RUNE_FAIL_COUNT: u32 = 1;

/// Stores persistent state of a buff.
#[derive(Debug)]
pub struct BuffState {
    /// The kind of buff.
    kind: BuffKind,
    /// Task for detecting if the coresponding buff exists.
    task: Option<Task<Result<bool>>>,
    /// The number of time [`Buff::Volatile`] has failed to detect if the buff exists.
    fail_count: u32,
    /// The maximum number of time [`Buff::Volatile`] can fail before transitioning
    /// to [`Buff:No`].
    max_fail_count: u32,
    /// Whether a buff is enabled.
    enabled: bool,
}

impl BuffState {
    pub fn new(kind: BuffKind) -> Self {
        Self {
            kind,
            task: None,
            fail_count: 0,
            max_fail_count: match kind {
                BuffKind::Rune => RUNE_FAIL_COUNT,
                BuffKind::Familiar => FAMILIAR_FAIL_COUNT,
                BuffKind::LegionWealth
                | BuffKind::LegionLuck
                | BuffKind::WealthAcquisitionPotion
                | BuffKind::ExpAccumulationPotion
                | BuffKind::SayramElixir
                | BuffKind::AureliaElixir
                | BuffKind::ExpCouponX3
                | BuffKind::BonusExpCoupon
                | BuffKind::ExtremeRedPotion
                | BuffKind::ExtremeBluePotion
                | BuffKind::ExtremeGreenPotion
                | BuffKind::ExtremeGoldPotion => COMMON_FAIL_COUNT,
            },
            enabled: true,
        }
    }

    /// Updates the enabled states of each buff to only detect if enabled.
    pub fn update_enabled_state(&mut self, character: &Character, settings: &Settings) {
        self.enabled = match self.kind {
            BuffKind::Rune => settings.enable_rune_solving,
            BuffKind::Familiar => character.familiar_buff_key.enabled,
            BuffKind::SayramElixir => character.sayram_elixir_key.enabled,
            BuffKind::AureliaElixir => character.aurelia_elixir_key.enabled,
            BuffKind::ExpCouponX3 => character.exp_x3_key.enabled,
            BuffKind::BonusExpCoupon => character.bonus_exp_key.enabled,
            BuffKind::LegionWealth => character.legion_wealth_key.enabled,
            BuffKind::LegionLuck => character.legion_luck_key.enabled,
            BuffKind::WealthAcquisitionPotion => character.wealth_acquisition_potion_key.enabled,
            BuffKind::ExpAccumulationPotion => character.exp_accumulation_potion_key.enabled,
            BuffKind::ExtremeRedPotion => character.extreme_red_potion_key.enabled,
            BuffKind::ExtremeBluePotion => character.extreme_blue_potion_key.enabled,
            BuffKind::ExtremeGreenPotion => character.extreme_green_potion_key.enabled,
            BuffKind::ExtremeGoldPotion => character.extreme_gold_potion_key.enabled,
        };
        if !self.enabled {
            self.fail_count = 0;
            self.task = None;
        }
    }
}

/// Buff contextual state.
#[derive(Clone, Copy, Debug)]
pub enum Buff {
    /// Player does not have this [`BuffKind`].
    No,
    /// Player has this [`BuffKind`].
    Yes,
    /// Player did have this [`BuffKind`] but currently unsure.
    Volatile,
}

#[derive(Clone, Copy, Debug, EnumIter)]
#[cfg_attr(test, derive(PartialEq))]
#[repr(usize)]
pub enum BuffKind {
    // NOTE: Upon failing to solving rune, there is a cooldown
    // that looks exactly like the normal rune buff.
    Rune,
    Familiar,
    SayramElixir,
    AureliaElixir,
    ExpCouponX3,
    BonusExpCoupon,
    LegionWealth,
    LegionLuck,
    WealthAcquisitionPotion,
    ExpAccumulationPotion,
    ExtremeRedPotion,
    ExtremeBluePotion,
    ExtremeGreenPotion,
    ExtremeGoldPotion,
}

impl BuffKind {
    pub const COUNT: usize = mem::variant_count::<BuffKind>();
}

impl Index<BuffKind> for [Buff; BuffKind::COUNT] {
    type Output = Buff;

    fn index(&self, index: BuffKind) -> &Self::Output {
        self.get(index as usize).unwrap()
    }
}

impl IndexMut<BuffKind> for [Buff; BuffKind::COUNT] {
    fn index_mut(&mut self, index: BuffKind) -> &mut Self::Output {
        self.get_mut(index as usize).unwrap()
    }
}

impl Contextual for Buff {
    type Persistent = BuffState;

    fn update(self, context: &Context, state: &mut BuffState) -> ControlFlow<Self> {
        if !state.enabled {
            return ControlFlow::Next(Buff::No);
        }
        let next = if matches!(context.player, Player::CashShopThenExit(_, _)) {
            self
        } else {
            update_context(self, context, state)
        };
        ControlFlow::Next(next)
    }
}

#[inline]
fn update_context(contextual: Buff, context: &Context, state: &mut BuffState) -> Buff {
    let kind = state.kind;
    let Update::Ok(has_buff) =
        update_detection_task(context, 5000, &mut state.task, move |detector| {
            Ok(detector.detect_player_buff(kind))
        })
    else {
        return contextual;
    };
    state.fail_count = if matches!(contextual, Buff::Volatile) && !has_buff {
        state.fail_count + 1
    } else {
        0
    };
    match (has_buff, contextual) {
        (true, Buff::Volatile) | (true, Buff::Yes) | (true, Buff::No) => Buff::Yes,
        (false, Buff::No) => Buff::No,
        (false, Buff::Yes) => {
            if state.max_fail_count > 1 {
                Buff::Volatile
            } else {
                Buff::No
            }
        }
        (false, Buff::Volatile) => {
            if state.fail_count >= state.max_fail_count {
                Buff::No
            } else {
                Buff::Volatile
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{assert_matches::assert_matches, time::Duration};

    use mockall::predicate::eq;
    use strum::IntoEnumIterator;
    use tokio::time::advance;

    use super::*;
    use crate::detect::MockDetector;

    fn detector_with_kind(kind: BuffKind, result: bool) -> MockDetector {
        let mut detector = MockDetector::new();
        detector
            .expect_detect_player_buff()
            .with(eq(kind))
            .return_const(result);
        detector
            .expect_clone()
            .returning(move || detector_with_kind(kind, result));
        detector
    }

    async fn advance_task(contextual: Buff, context: &Context, state: &mut BuffState) -> Buff {
        let mut buff = update_context(contextual, context, state);
        while !state.task.as_ref().unwrap().completed() {
            buff = update_context(buff, context, state);
            advance(Duration::from_millis(1000)).await;
        }
        buff
    }

    #[tokio::test(start_paused = true)]
    async fn buff_no_to_yes() {
        for kind in BuffKind::iter() {
            let detector = detector_with_kind(kind, true);
            let context = Context::new(None, Some(detector));
            let mut state = BuffState::new(kind);

            let buff = advance_task(Buff::No, &context, &mut state).await;
            let buff = update_context(buff, &context, &mut state);
            assert_eq!(state.fail_count, 0);
            assert_matches!(buff, Buff::Yes);
        }
    }

    #[tokio::test(start_paused = true)]
    async fn buff_yes_to_no() {
        for kind in BuffKind::iter() {
            let detector = detector_with_kind(kind, false);
            let context = Context::new(None, Some(detector));
            let mut state = BuffState::new(kind);
            state.max_fail_count = 2;
            state.fail_count = 0;

            let mut buff = Buff::Yes;

            // First failure: Yes -> Volatile
            buff = advance_task(buff, &context, &mut state).await;
            assert_matches!(buff, Buff::Volatile);
            assert_eq!(state.fail_count, 0);

            // Second failure: Volatile -> still Volatile
            buff = advance_task(buff, &context, &mut state).await;
            assert_matches!(buff, Buff::Volatile);
            assert_eq!(state.fail_count, 1);

            // Third failure: Volatile -> No (fail_count reached max)
            buff = advance_task(buff, &context, &mut state).await;
            assert_matches!(buff, Buff::No);
            assert_eq!(state.fail_count, 2); // Still 2, as No resets it on next tick
        }
    }

    #[tokio::test(start_paused = true)]
    async fn buff_volatile_to_yes() {
        for kind in BuffKind::iter() {
            let detector = detector_with_kind(kind, true);
            let context = Context::new(None, Some(detector));
            let mut state = BuffState::new(kind);
            state.max_fail_count = 3;
            state.fail_count = 2;

            let buff = advance_task(Buff::Volatile, &context, &mut state).await;
            assert_matches!(buff, Buff::Yes);
            assert_eq!(state.fail_count, 0);
        }
    }

    #[test]
    fn buff_disabled_reset() {
        let kind = BuffKind::Rune;
        let mut state = BuffState::new(kind);
        state.enabled = true;
        state.fail_count = 5;

        let mut settings = Settings::default();
        let config = Character::default();
        settings.enable_rune_solving = false;

        state.update_enabled_state(&config, &settings);
        assert!(!state.enabled);
        assert_eq!(state.fail_count, 0);
        assert!(state.task.is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn buff_volatile_stay_before_threshold() {
        for kind in BuffKind::iter() {
            let detector = detector_with_kind(kind, false);
            let context = Context::new(None, Some(detector));
            let mut state = BuffState::new(kind);
            state.max_fail_count = 3;
            state.fail_count = 1;

            let buff = advance_task(Buff::Volatile, &context, &mut state).await;
            assert_matches!(buff, Buff::Volatile);
            assert_eq!(state.fail_count, 2);
        }
    }
}
