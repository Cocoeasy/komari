#[cfg(test)]
use std::rc::Rc;
#[cfg(debug_assertions)]
use std::time::Instant;
use std::{cell::RefCell, time::Duration};

use dyn_clone::clone_box;
#[cfg(debug_assertions)]
use opencv::core::Rect;

use crate::{
    CycleRunStopMode, bridge::Input, buff::BuffEntities, detect::Detector, minimap::MinimapEntity,
    notification::DiscordNotification, player::PlayerEntity, rng::Rng, skill::SkillEntities,
};
#[cfg(test)]
use crate::{Settings, bridge::MockInput, detect::MockDetector};
#[cfg(debug_assertions)]
use crate::{bridge::KeyKind, debug::save_rune_for_training};

#[macro_export]
macro_rules! transition {
    ($entity:expr, $state:expr) => {{
        $entity.state = $state;
        return;
    }};
    ($entity:expr, $state:expr, $block:block) => {{
        $block
        $entity.state = $state;
        return;
    }};
}

#[macro_export]
macro_rules! transition_if {
    ($cond:expr) => {{
        if $cond {
            return;
        }
    }};
    ($entity:expr, $state:expr, $cond:expr) => {{
        if $cond {
            $entity.state = $state;
            return;
        }
    }};
    ($entity:expr, $state:expr, $cond:expr, $block:block) => {{
        if $cond {
            $block
            $entity.state = $state;
            return;
        }
    }};
    ($entity:expr, $true_state:expr, $false_state:expr, $cond:expr) => {{
        $entity.state = if $cond { $true_state } else { $false_state };
        return;
    }};
}

#[macro_export]
macro_rules! try_some_transition {
    ($entity:expr, $state:expr, $expr:expr) => {
        match $expr {
            Some(val) => val,
            None => {
                $entity.state = $state;
                return;
            }
        }
    };
}

#[macro_export]
macro_rules! try_ok_transition {
    ($entity:expr, $state:expr, $expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(_) => {
                $entity.state = $state;
                return;
            }
        }
    };
}

/// Current operating state of the bot.
#[derive(Debug, Clone, Copy)]
pub enum Operation {
    HaltUntil {
        instant: Instant,
        run_duration_millis: u64,
        stop_duration_millis: u64,
    },
    TemporaryHalting {
        resume: Duration,
        run_duration_millis: u64,
        stop_duration_millis: u64,
        once: bool,
    },
    Halting,
    Running,
    RunUntil {
        instant: Instant,
        run_duration_millis: u64,
        stop_duration_millis: u64,
        once: bool,
    },
}

impl Operation {
    #[inline]
    pub fn halting(&self) -> bool {
        matches!(
            self,
            Operation::Halting | Operation::HaltUntil { .. } | Operation::TemporaryHalting { .. }
        )
    }

    pub fn update_current(
        self,
        cycle_run_stop: CycleRunStopMode,
        run_duration_millis: u64,
        stop_duration_millis: u64,
    ) -> Operation {
        match self {
            Operation::HaltUntil {
                stop_duration_millis: current_stop_duration_millis,
                ..
            } => match cycle_run_stop {
                CycleRunStopMode::None | CycleRunStopMode::Once => Operation::Halting,
                CycleRunStopMode::Repeat => {
                    if current_stop_duration_millis == stop_duration_millis {
                        self
                    } else {
                        Operation::halt_until(run_duration_millis, stop_duration_millis)
                    }
                }
            },
            Operation::TemporaryHalting {
                run_duration_millis: current_run_duration_millis,
                ..
            } => {
                if current_run_duration_millis != run_duration_millis
                    || matches!(cycle_run_stop, CycleRunStopMode::None)
                {
                    Operation::Halting
                } else {
                    self
                }
            }
            Operation::Halting => Operation::Halting,
            Operation::Running | Operation::RunUntil { .. } => match cycle_run_stop {
                CycleRunStopMode::None => Operation::Running,
                CycleRunStopMode::Once | CycleRunStopMode::Repeat => Operation::run_until(
                    run_duration_millis,
                    stop_duration_millis,
                    matches!(cycle_run_stop, CycleRunStopMode::Once),
                ),
            },
        }
    }

    pub fn update(self) -> Operation {
        let now = Instant::now();
        match self {
            // Imply run/stop cycle enabled
            Operation::HaltUntil {
                instant,
                run_duration_millis,
                stop_duration_millis,
            } => {
                if now < instant {
                    self
                } else {
                    Operation::run_until(run_duration_millis, stop_duration_millis, false)
                }
            }
            // Imply run/stop cycle enabled
            Operation::RunUntil {
                instant,
                run_duration_millis,
                stop_duration_millis,
                once,
            } => {
                if now < instant {
                    self
                } else if once {
                    Operation::Halting
                } else {
                    Operation::halt_until(run_duration_millis, stop_duration_millis)
                }
            }
            Operation::Halting | Operation::TemporaryHalting { .. } | Operation::Running => self,
        }
    }

    #[inline]
    fn halt_until(run_duration_millis: u64, stop_duration_millis: u64) -> Operation {
        Operation::HaltUntil {
            instant: Instant::now() + Duration::from_millis(stop_duration_millis),
            run_duration_millis,
            stop_duration_millis,
        }
    }

    #[inline]
    pub fn run_until(run_duration_millis: u64, stop_duration_millis: u64, once: bool) -> Operation {
        Operation::RunUntil {
            instant: Instant::now() + Duration::from_millis(run_duration_millis),
            run_duration_millis,
            stop_duration_millis,
            once,
        }
    }
}

#[derive(Debug, Default)]
#[cfg(debug_assertions)]
pub struct Debug {
    auto_save: RefCell<bool>,
    last_rune_detector: RefCell<Option<Box<dyn Detector>>>,
    last_rune_result: RefCell<Option<[(Rect, KeyKind); 4]>>,
}

#[cfg(debug_assertions)]
impl Debug {
    pub fn auto_save_rune(&self) -> bool {
        *self.auto_save.borrow()
    }

    pub fn set_auto_save_rune(&self, auto_save: bool) {
        *self.auto_save.borrow_mut() = auto_save;
    }

    pub fn save_last_rune_result(&self) {
        if !*self.auto_save.borrow() {
            return;
        }
        if let Some((detector, result)) = self
            .last_rune_detector
            .borrow()
            .as_ref()
            .zip(*self.last_rune_result.borrow())
        {
            save_rune_for_training(detector.mat(), result);
        }
    }

    pub fn set_last_rune_result(&self, detector: Box<dyn Detector>, result: [(Rect, KeyKind); 4]) {
        *self.last_rune_detector.borrow_mut() = Some(detector);
        *self.last_rune_result.borrow_mut() = Some(result);
    }
}

/// A struct containing shared resources.
#[derive(Debug)]
pub struct Resources {
    /// A resource to hold debugging information.
    #[cfg(debug_assertions)]
    pub debug: Debug,
    /// A resource to send inputs.
    pub input: Box<dyn Input>,
    /// A resource for generating random values.
    pub rng: Rng,
    /// A resource for sending notifications through web hook.
    pub notification: DiscordNotification,
    /// A resource to detect game information.
    ///
    /// This is [`None`] when no frame as ever been captured.
    pub detector: Option<Box<dyn Detector>>,
    /// A resource indicating current operation state.
    pub operation: Operation,
    /// A resource indicating current tick.
    pub tick: u64,
}

impl Resources {
    #[cfg(test)]
    pub fn new(input: Option<MockInput>, detector: Option<MockDetector>) -> Self {
        Self {
            #[cfg(debug_assertions)]
            debug: Debug::default(),
            input: Box::new(input.unwrap_or_default()),
            rng: Rng::new(rand::random()),
            notification: DiscordNotification::new(Rc::new(RefCell::new(Settings::default()))),
            detector: detector.map(|detector| Box::new(detector) as Box<dyn Detector>),
            operation: Operation::Running,
            tick: 0,
        }
    }

    /// Retrieves a reference to a [`Detector`] for the latest captured frame.
    ///
    /// # Panics
    ///
    /// Panics if no frame has ever been captured.
    #[inline]
    pub fn detector(&self) -> &dyn Detector {
        self.detector
            .as_ref()
            .expect("detector is not available because no frame has ever been captured")
            .as_ref()
    }

    /// Same as [`Self::detector`] but cloned.
    #[inline]
    pub fn detector_cloned(&self) -> Box<dyn Detector> {
        clone_box(self.detector())
    }
}

/// Different game-related events.
#[derive(Debug, Clone, Copy)]
pub enum WorldEvent {
    CycledToHalt,
    PlayerDied,
    MinimapChanged,
    CaptureFailed,
}

/// A container for entities.
#[derive(Debug)]
pub struct World {
    pub minimap: MinimapEntity,
    pub player: PlayerEntity,
    pub skills: SkillEntities,
    pub buffs: BuffEntities,
}
