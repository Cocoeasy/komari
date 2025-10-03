use std::{
    cell::RefCell,
    env,
    rc::Rc,
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

use opencv::{
    core::{Vector, VectorToVec},
    imgcodecs::imencode_def,
};
use platforms::Error;
use strum::IntoEnumIterator;
use tokio::sync::broadcast::channel;

#[cfg(debug_assertions)]
use crate::ecs::Debug;
use crate::{
    bridge::Capture,
    buff::{self, Buff, BuffContext, BuffEntity, BuffKind},
    database::{query_seeds, query_settings},
    detect::CachedDetector,
    ecs::{Operation, Resources, World, WorldEvent},
    mat::OwnedMat,
    minimap::{self, Minimap, MinimapContext, MinimapEntity},
    navigator::{DefaultNavigator, Navigator},
    notification::DiscordNotification,
    player::{self, Player, PlayerContext, PlayerEntity},
    rng::Rng,
    rotator::{DefaultRotator, Rotator},
    services::{DefaultService, PollArgs},
    skill::{self, Skill, SkillContext, SkillEntity, SkillKind},
};

/// The FPS the bot runs at.
///
/// This must **not** be changed as it affects other ticking systems.
pub const FPS: u32 = 30;

/// Milliseconds per tick as an [`u64`].
pub const MS_PER_TICK: u64 = MS_PER_TICK_F32 as u64;

/// Milliseconds per tick as an [`f32`].
pub const MS_PER_TICK_F32: f32 = 1000.0 / FPS as f32;

pub fn init() {
    static LOOPING: AtomicBool = AtomicBool::new(false);

    if LOOPING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::Acquire)
        .is_ok()
    {
        let dll = env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .join("onnxruntime.dll");

        ort::init_from(dll.to_str().unwrap()).commit().unwrap();
        platforms::init();
        thread::spawn(|| {
            let tokio_rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap();
            let _tokio_guard = tokio_rt.enter();
            tokio_rt.block_on(async {
                systems_loop();
            });
        });
    }
}
fn systems_loop() {
    let settings = Rc::new(RefCell::new(query_settings()));
    let seeds = query_seeds(); // Fixed, unchanged
    let rng = Rng::new(seeds.seed); // Create one for Context
    let (event_tx, event_rx) = channel::<WorldEvent>(5);
    let (mut service, input, mut capture) =
        DefaultService::new(seeds, settings.clone(), event_tx.subscribe());
    let mut rotator = DefaultRotator::default();
    let mut navigator = DefaultNavigator::new(event_rx);
    let notification = DiscordNotification::new(settings.clone());
    let mut resources = Resources {
        #[cfg(debug_assertions)]
        debug: Debug::default(),
        input: Box::new(input),
        rng,
        notification,
        detector: None,
        operation: Operation::Halting,
        tick: 0,
    };

    let minimap = MinimapEntity {
        state: Minimap::Detecting,
        context: MinimapContext::default(),
    };
    let player = PlayerEntity {
        state: Player::Idle,
        context: PlayerContext::default(),
    };
    let skills = SkillKind::iter()
        .map(SkillContext::new)
        .map(|context| SkillEntity {
            state: Skill::Detecting,
            context,
        })
        .collect::<Vec<_>>()
        .try_into()
        .expect("matching size");

    let buffs = BuffKind::iter()
        .map(BuffContext::new)
        .map(|context| BuffEntity {
            state: Buff::No,
            context,
        })
        .collect::<Vec<_>>()
        .try_into()
        .expect("matching size");
    let mut world = World {
        minimap,
        player,
        skills,
        buffs,
    };
    let mut is_capturing_normally = false;

    loop_with_fps(FPS, || {
        let detector = capture
            .grab()
            .map(OwnedMat::new_from_frame)
            .map(CachedDetector::new);
        let was_capturing_normally = is_capturing_normally;
        let player_in_cash_shop = matches!(world.player.state, Player::CashShopThenExit(_));

        is_capturing_normally = detector.is_ok()
            || (!player_in_cash_shop
                && !matches!(
                    detector,
                    Err(Error::WindowNotFound | Error::WindowInvalidSize)
                ));
        resources.tick += 1;
        if let Ok(detector) = detector {
            let was_running_cycle = matches!(resources.operation, Operation::RunUntil { .. });
            let was_player_alive = !world.player.context.is_dead();
            let was_minimap_idle = matches!(world.minimap.state, Minimap::Idle(_));

            resources.detector = Some(Box::new(detector));
            resources.operation = resources.operation.update();

            minimap::run_system(&resources, &mut world.minimap, world.player.state);
            player::run_system(&resources, &mut world.player, &world.minimap, &world.buffs);
            for skill in world.skills.iter_mut() {
                skill::run_system(&resources, skill, world.player.state);
            }
            for buff in world.buffs.iter_mut() {
                buff::run_system(&resources, buff, world.player.state);
            }

            if navigator.navigate_player(&resources, &mut world.player.context, world.minimap.state)
            {
                rotator.rotate_action(&resources, &mut world);
            }

            let did_cycled_to_stop = resources.operation.halting();
            // Go to town on stop cycle
            if was_running_cycle && did_cycled_to_stop {
                let _ = event_tx.send(WorldEvent::CycledToHalt);
            }

            let player_died = was_player_alive && world.player.context.is_dead();
            if player_died {
                let _ = event_tx.send(WorldEvent::PlayerDied);
            }

            let minimap_detecting = matches!(world.minimap.state, Minimap::Detecting);
            if was_minimap_idle && minimap_detecting {
                let _ = event_tx.send(WorldEvent::MinimapChanged);
            }
        }

        if was_capturing_normally && !is_capturing_normally {
            let _ = event_tx.send(WorldEvent::CaptureFailed);
        }

        resources.input.update(resources.tick);
        resources
            .notification
            .update(|| to_png(resources.detector.as_ref().map(|detector| detector.mat())));
        service.poll(PollArgs {
            resources: &mut resources,
            world: &mut world,
            rotator: &mut rotator,
            navigator: &mut navigator,
            capture: &mut capture,
        });
    });
}

#[inline]
fn loop_with_fps(fps: u32, mut on_tick: impl FnMut()) {
    #[cfg(debug_assertions)]
    const LOG_INTERVAL_SECS: u64 = 5;

    let nanos_per_frame = (1_000_000_000 / fps) as u128;
    #[cfg(debug_assertions)]
    let mut last_logged_instant = Instant::now();

    loop {
        let start = Instant::now();

        on_tick();

        let now = Instant::now();
        let elapsed_duration = now.duration_since(start);
        let elapsed_nanos = elapsed_duration.as_nanos();
        if elapsed_nanos <= nanos_per_frame {
            thread::sleep(Duration::new(0, (nanos_per_frame - elapsed_nanos) as u32));
        } else {
            #[cfg(debug_assertions)]
            if now.duration_since(last_logged_instant).as_secs() >= LOG_INTERVAL_SECS {
                use log::debug;

                last_logged_instant = now;
                debug!(target: "context", "ticking running late at {}ms", elapsed_duration.as_millis());
            }
        }
    }
}

#[inline]
fn to_png(frame: Option<&OwnedMat>) -> Option<Vec<u8>> {
    frame.and_then(|image| {
        let mut bytes = Vector::new();
        imencode_def(".png", image, &mut bytes).ok()?;
        Some(bytes.to_vec())
    })
}
