use std::{
    mem,
    ops::{Index, IndexMut},
};

use anyhow::Result;
use log::debug;
use opencv::core::{MatTraitConst, Point, Rect, Vec4b};
use strum::{Display, EnumIter};

use crate::{
    ecs::Resources,
    player::Player,
    task::{Task, Update, update_detection_task},
    transition, transition_if, try_ok_transition,
};

/// An entity that contains skill-related data.
#[derive(Debug)]
pub struct SkillEntity {
    pub state: Skill,
    pub context: SkillContext,
}

pub type SkillEntities = [SkillEntity; SkillKind::COUNT];

#[derive(Debug)]
pub struct SkillContext {
    kind: SkillKind,
    task: Option<Task<Result<(Point, Vec4b)>>>,
}

impl SkillContext {
    pub fn new(kind: SkillKind) -> Self {
        Self { kind, task: None }
    }
}

#[derive(Clone, Copy, Debug, Display)]
pub enum Skill {
    Detecting,
    Idle(Point, Vec4b),
    Cooldown,
}

#[derive(Clone, Copy, Debug, EnumIter)]
pub enum SkillKind {
    ErdaShower,
    // TODO: Sol Janus?
}

impl SkillKind {
    pub const COUNT: usize = mem::variant_count::<SkillKind>();
}

impl Index<SkillKind> for SkillEntities {
    type Output = SkillEntity;

    fn index(&self, index: SkillKind) -> &Self::Output {
        self.get(index as usize).unwrap()
    }
}

impl IndexMut<SkillKind> for SkillEntities {
    fn index_mut(&mut self, index: SkillKind) -> &mut Self::Output {
        self.get_mut(index as usize).unwrap()
    }
}

pub fn run_system(resources: &Resources, skill: &mut SkillEntity, player_state: Player) {
    transition_if!(matches!(player_state, Player::CashShopThenExit(_)));

    match skill.state {
        Skill::Detecting => update_detection_state(resources, skill),
        Skill::Idle(anchor_point, anchor_pixel) => {
            update_idle_state(resources, skill, anchor_point, anchor_pixel);
        }
        Skill::Cooldown => update_detection_state(resources, skill),
    }
}

fn update_idle_state(
    resources: &Resources,
    skill: &mut SkillEntity,
    anchor_point: Point,
    anchor_pixel: Vec4b,
) {
    let result = resources.detector().mat().at_pt::<Vec4b>(anchor_point);
    let pixel = try_ok_transition!(skill, Skill::Detecting, result);

    transition_if!(
        skill,
        Skill::Idle(anchor_point, anchor_pixel),
        Skill::Cooldown,
        anchor_match(*pixel, anchor_pixel)
    );
}

#[inline]
fn update_detection_state(resources: &Resources, skill: &mut SkillEntity) {
    let kind = skill.context.kind;
    let task = &mut skill.context.task;
    let update = update_detection_task(resources, 1000, task, move |detector| {
        let bbox = match kind {
            SkillKind::ErdaShower => detector.detect_erda_shower()?,
        };
        Ok(get_anchor(detector.mat(), bbox))
    });

    match update {
        Update::Ok((point, pixel)) => transition!(skill, Skill::Idle(point, pixel)),
        Update::Err(err) => transition_if!(
            skill,
            Skill::Detecting,
            err.downcast::<f64>().unwrap() < 0.52
        ),
        Update::Pending => (),
    };
}

#[inline]
fn anchor_match(anchor: Vec4b, pixel: Vec4b) -> bool {
    const ANCHOR_ACCEPTABLE_ERROR_RANGE: u32 = 45;

    let b = anchor[0].abs_diff(pixel[0]) as u32;
    let g = anchor[1].abs_diff(pixel[1]) as u32;
    let r = anchor[2].abs_diff(pixel[2]) as u32;
    let avg = (b + g + r) / 3; // Average for grayscale
    avg <= ANCHOR_ACCEPTABLE_ERROR_RANGE
}

#[inline]
fn get_anchor(mat: &impl MatTraitConst, bbox: Rect) -> (Point, Vec4b) {
    let point = (bbox.tl() + bbox.br()) / 2;
    let pixel = mat.at_pt::<Vec4b>(point).unwrap();
    let anchor = (point, *pixel);
    debug!(target: "skill", "detected at {bbox:?} with anchor {anchor:?}");
    anchor
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;
    use std::time::Duration;

    use anyhow::{Context as AnyhowContext, anyhow};
    use opencv::core::{CV_8UC4, Mat, MatExprTraitConst, MatTrait, Rect, Vec4b};
    use tokio::time::advance;

    use super::*;
    use crate::detect::MockDetector;
    use crate::ecs::Resources;

    fn create_test_mat_bbox(center_pixel: u8) -> (Mat, Rect) {
        let mut mat = Mat::zeros(100, 100, CV_8UC4).unwrap().to_mat().unwrap();
        let rect = Rect::new(0, 0, 100, 100);
        let center = (rect.tl() + rect.br()) / 2;
        *mat.at_pt_mut::<Vec4b>(center).unwrap() = Vec4b::all(center_pixel);
        (mat, rect)
    }

    fn create_mock_detector(center_pixel: u8, error: Option<f64>) -> (MockDetector, Rect) {
        let mut detector = MockDetector::new();
        let (mat, rect) = create_test_mat_bbox(center_pixel);

        detector.expect_mat().return_const(mat.into());
        detector
            .expect_clone()
            .returning(move || create_mock_detector(center_pixel, error).0);
        detector.expect_detect_erda_shower().returning(move || {
            if let Some(error) = error {
                Err(anyhow!("error")).context(error)
            } else {
                Ok(rect)
            }
        });

        (detector, rect)
    }

    async fn run_system_until_task_completed(resources: &Resources, skill: &mut SkillEntity) {
        while !skill
            .context
            .task
            .as_ref()
            .map_or(false, |task| task.completed())
        {
            run_system(resources, skill, Player::Idle);
            advance(Duration::from_millis(1000)).await;
        }
    }

    #[tokio::test(start_paused = true)]
    async fn run_system_detecting_to_idle() {
        let (detector, rect) = create_mock_detector(255, None);
        let resources = Resources::new(None, Some(detector));
        let mut skill = SkillEntity {
            state: Skill::Detecting,
            context: SkillContext::new(SkillKind::ErdaShower),
        };

        run_system_until_task_completed(&resources, &mut skill).await;

        match skill.state {
            Skill::Idle(point, pixel) => {
                assert_eq!(point, (rect.tl() + rect.br()) / 2);
                assert_eq!(pixel, Vec4b::all(255));
            }
            _ => panic!(),
        }
    }

    #[test]
    fn run_system_idle_to_cooldown() {
        let (detector, rect) = create_mock_detector(200, None);
        let resources = Resources::new(None, Some(detector));
        let mut skill = SkillEntity {
            state: Skill::Idle((rect.tl() + rect.br()) / 2, Vec4b::all(255)),
            context: SkillContext::new(SkillKind::ErdaShower),
        };

        run_system(&resources, &mut skill, Player::Idle);

        assert_matches!(skill.state, Skill::Cooldown);
    }

    #[tokio::test(start_paused = true)]
    async fn run_system_cooldown_to_detecting() {
        let (detector, _) = create_mock_detector(255, Some(0.51));
        let resources = Resources::new(None, Some(detector));
        let mut skill = SkillEntity {
            state: Skill::Cooldown,
            context: SkillContext::new(SkillKind::ErdaShower),
        };

        run_system_until_task_completed(&resources, &mut skill).await;

        assert_matches!(skill.state, Skill::Detecting);
    }

    #[tokio::test(start_paused = true)]
    async fn run_system_cooldown_to_idle() {
        let (detector, rect) = create_mock_detector(255, None);
        let resources = Resources::new(None, Some(detector));
        let mut skill = SkillEntity {
            state: Skill::Cooldown,
            context: SkillContext::new(SkillKind::ErdaShower),
        };

        run_system_until_task_completed(&resources, &mut skill).await;

        match skill.state {
            Skill::Idle(point, pixel) => {
                assert_eq!(point, (rect.tl() + rect.br()) / 2);
                assert_eq!(pixel, Vec4b::all(255));
            }
            _ => panic!(),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn run_system_cooldown_to_cooldown() {
        let (detector, _) = create_mock_detector(255, Some(0.52));
        let resources = Resources::new(None, Some(detector));
        let mut skill = SkillEntity {
            state: Skill::Cooldown,
            context: SkillContext::new(SkillKind::ErdaShower),
        };

        run_system_until_task_completed(&resources, &mut skill).await;

        assert_matches!(skill.state, Skill::Cooldown);
    }
}
