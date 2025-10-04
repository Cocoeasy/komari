use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::time::Duration;
use std::time::Instant;

use crate::CycleRunStopMode;
use crate::RotateKind;

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

    pub fn update_from_rotate_kind_and_mode(
        self,
        kind: RotateKind,
        mode: CycleRunStopMode,
        run_duration_millis: u64,
        stop_duration_millis: u64,
    ) -> Operation {
        match (kind, mode) {
            (RotateKind::TemporaryHalt, CycleRunStopMode::None) | (RotateKind::Halt, _) => {
                Operation::Halting
            }
            (RotateKind::TemporaryHalt, _) => {
                if let Operation::RunUntil {
                    instant,
                    run_duration_millis,
                    stop_duration_millis,
                    once,
                } = self
                {
                    Operation::TemporaryHalting {
                        resume: instant.saturating_duration_since(Instant::now()),
                        run_duration_millis,
                        stop_duration_millis,
                        once,
                    }
                } else {
                    Operation::Halting
                }
            }
            (RotateKind::Run, CycleRunStopMode::Once | CycleRunStopMode::Repeat) => {
                if let Operation::TemporaryHalting {
                    resume,
                    run_duration_millis,
                    stop_duration_millis,
                    once,
                } = self
                {
                    Operation::RunUntil {
                        instant: Instant::now() + resume,
                        run_duration_millis,
                        stop_duration_millis,
                        once,
                    }
                } else {
                    Operation::run_until(
                        run_duration_millis,
                        stop_duration_millis,
                        matches!(mode, CycleRunStopMode::Once),
                    )
                }
            }
            (RotateKind::Run, CycleRunStopMode::None) => Operation::Running,
        }
    }

    pub fn update_from_mode(
        self,
        mode: CycleRunStopMode,
        run_duration_millis: u64,
        stop_duration_millis: u64,
    ) -> Operation {
        match self {
            Operation::HaltUntil {
                instant,
                stop_duration_millis: current_stop_duration_millis,
                ..
            } => match mode {
                CycleRunStopMode::None | CycleRunStopMode::Once => Operation::Halting,
                CycleRunStopMode::Repeat => {
                    if current_stop_duration_millis == stop_duration_millis {
                        Operation::HaltUntil {
                            instant,
                            stop_duration_millis,
                            run_duration_millis,
                        }
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
                    || matches!(mode, CycleRunStopMode::None)
                {
                    Operation::Halting
                } else {
                    self
                }
            }
            Operation::Halting => Operation::Halting,
            Operation::Running | Operation::RunUntil { .. } => match mode {
                CycleRunStopMode::None => Operation::Running,
                CycleRunStopMode::Once | CycleRunStopMode::Repeat => Operation::run_until(
                    run_duration_millis,
                    stop_duration_millis,
                    matches!(mode, CycleRunStopMode::Once),
                ),
            },
        }
    }

    pub fn update_tick(self) -> Operation {
        let now = Instant::now();
        match self {
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
    fn run_until(run_duration_millis: u64, stop_duration_millis: u64, once: bool) -> Operation {
        Operation::RunUntil {
            instant: Instant::now() + Duration::from_millis(run_duration_millis),
            run_duration_millis,
            stop_duration_millis,
            once,
        }
    }
}

impl Display for Operation {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            Operation::HaltUntil { instant, .. } => {
                write!(f, "Halting for {}", duration_from_instant(instant))
            }
            Operation::TemporaryHalting { resume, .. } => write!(
                f,
                "Halting temporarily with {} remaining",
                duration_from(resume)
            ),
            Operation::Halting => write!(f, "Halting"),
            Operation::Running => write!(f, "Running"),
            Operation::RunUntil { instant, .. } => {
                write!(f, "Running for {}", duration_from_instant(instant))
            }
        }
    }
}

#[inline]
fn duration_from_instant(instant: Instant) -> String {
    duration_from(instant.saturating_duration_since(Instant::now()))
}

#[inline]
fn duration_from(duration: Duration) -> String {
    let seconds = duration.as_secs() % 60;
    let minutes = (duration.as_secs() / 60) % 60;
    let hours = (duration.as_secs() / 60) / 60;

    format!("{hours:0>2}:{minutes:0>2}:{seconds:0>2}")
}
