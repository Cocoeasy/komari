use std::{
    fmt::Display,
    mem::{discriminant, swap},
    ops::Range,
};

use backend::{
    Action, ActionCondition, ActionKey, ActionKeyDirection, ActionKeyWith, ActionMove, AutoMobbing,
    Bound, IntoEnumIterator, KeyBinding, LinkKeyBinding, Minimap, MobbingKey, PingPong, Platform,
    Position, RotationMode, key_receiver, update_minimap, upsert_map,
};
use dioxus::prelude::*;
use futures_util::StreamExt;
use tokio::task::spawn_blocking;

use crate::{
    AppState,
    button::{Button, ButtonKind},
    icons::{DownArrowIcon, PositionIcon, UpArrowIcon, XIcon},
    inputs::{Checkbox, KeyBindingInput, MillisInput, NumberInputI32, NumberInputU32},
    select::{EnumSelect, TextSelect},
};

const ITEM_TEXT_CLASS: &str =
    "text-center inline-block pt-1 text-ellipsis overflow-hidden whitespace-nowrap";
const ITEM_BORDER_CLASS: &str = "border-r-2 border-gray-700";

#[derive(Debug)]
enum ActionUpdate {
    SetPreset,
    CreatePreset(String),
    DeletePreset,
    SaveMinimap,
    EditMobbingKey(MobbingKey),
    EditMobbingBound(Bound),
    AddPlatform(Platform),
    EditPlatform(Platform, usize),
    DeletePlatform(usize),
    Add(Action, ActionCondition),
    Edit(Action, usize),
    Delete(usize),
    Move(usize, ActionCondition, bool),
}

#[derive(Clone, Copy, Debug)]
enum PopupInputKind {
    Action(ActionInputKind),
    Bound(Bound),
    Platform(Platform, Option<usize>),
}

#[derive(Clone, Copy, Debug)]
enum ActionInputKind {
    Add(Action),
    Edit(Action, usize),
    PingPongOrAutoMobbing(MobbingKey),
}

#[component]
pub fn Actions() -> Element {
    let mut minimap = use_context::<AppState>().minimap;
    let mut minimap_preset = use_context::<AppState>().minimap_preset;
    // Non-null view of minimap
    let minimap_view = use_memo(move || minimap().unwrap_or_default());
    // Maps currently selected `minimap` to presets
    let minimap_presets = use_memo(move || {
        minimap()
            .map(|minimap| minimap.actions.into_keys().collect::<Vec<String>>())
            .unwrap_or_default()
    });
    // Maps currently selected `minimap_preset` to actions
    let minimap_preset_actions = use_memo(move || {
        minimap()
            .zip(minimap_preset())
            .and_then(|(minimap, preset)| minimap.actions.get(&preset).cloned())
            .unwrap_or_default()
    });
    // Maps currently selected `minimap_preset` to the index in `minimap_presets`
    let minimap_preset_index = use_memo(move || {
        let presets = minimap_presets();
        minimap_preset().and_then(|preset| {
            presets
                .into_iter()
                .enumerate()
                .find(|(_, p)| &preset == p)
                .map(|(i, _)| i)
        })
    });

    // Handles async operations for action-related
    // TODO: Split into functions
    let coroutine = use_coroutine(move |mut rx: UnboundedReceiver<ActionUpdate>| async move {
        let mut save_minimap = async move |current_minimap: Minimap| {
            let mut save_minimap = current_minimap.clone();
            spawn_blocking(move || {
                upsert_map(&mut save_minimap).expect("failed to upsert minimap actions");
            })
            .await
            .unwrap();
            minimap.set(Some(current_minimap));
        };

        while let Some(message) = rx.next().await {
            match message {
                ActionUpdate::SetPreset => {
                    if let Some(minimap) = minimap() {
                        update_minimap(minimap_preset(), minimap).await;
                    }
                }
                ActionUpdate::CreatePreset(preset) => {
                    let Some(mut current_minimap) = minimap() else {
                        continue;
                    };

                    if current_minimap.actions.try_insert(preset, vec![]).is_ok() {
                        save_minimap(current_minimap).await;
                    }
                }
                ActionUpdate::DeletePreset => {
                    let Some(mut current_minimap) = minimap() else {
                        continue;
                    };
                    let Some(preset) = minimap_preset() else {
                        continue;
                    };

                    if current_minimap.actions.remove(&preset).is_some() {
                        minimap_preset.set(None);
                        save_minimap(current_minimap).await;
                    }
                }
                ActionUpdate::SaveMinimap => {
                    if let Some(minimap) = minimap() {
                        save_minimap(minimap).await;
                    }
                }
                ActionUpdate::EditMobbingKey(key) => {
                    let Some(mut current_minimap) = minimap() else {
                        continue;
                    };
                    let mode = match current_minimap.rotation_mode {
                        RotationMode::StartToEnd | RotationMode::StartToEndThenReverse => continue,
                        RotationMode::AutoMobbing(mobbing) => {
                            RotationMode::AutoMobbing(AutoMobbing { key, ..mobbing })
                        }
                        RotationMode::PingPong(ping_pong) => {
                            RotationMode::PingPong(PingPong { key, ..ping_pong })
                        }
                    };

                    current_minimap.rotation_mode = mode;
                    save_minimap(current_minimap).await;
                }
                ActionUpdate::EditMobbingBound(bound) => {
                    let Some(mut current_minimap) = minimap() else {
                        continue;
                    };
                    let mode = match current_minimap.rotation_mode {
                        RotationMode::StartToEnd | RotationMode::StartToEndThenReverse => continue,
                        RotationMode::AutoMobbing(mobbing) => {
                            RotationMode::AutoMobbing(AutoMobbing { bound, ..mobbing })
                        }
                        RotationMode::PingPong(ping_pong) => {
                            RotationMode::PingPong(PingPong { bound, ..ping_pong })
                        }
                    };

                    current_minimap.rotation_mode = mode;
                    save_minimap(current_minimap).await;
                }
                ActionUpdate::AddPlatform(platform) => {
                    let Some(mut current_minimap) = minimap() else {
                        continue;
                    };

                    current_minimap.platforms.push(platform);
                    save_minimap(current_minimap).await;
                }
                ActionUpdate::EditPlatform(platform, index) => {
                    let Some(mut current_minimap) = minimap() else {
                        continue;
                    };

                    *current_minimap.platforms.get_mut(index).unwrap() = platform;
                    save_minimap(current_minimap).await;
                }
                ActionUpdate::DeletePlatform(index) => {
                    let Some(mut current_minimap) = minimap() else {
                        continue;
                    };

                    current_minimap.platforms.remove(index);
                    save_minimap(current_minimap).await;
                }
                ActionUpdate::Add(action, condition) => {
                    let Some(mut current_minimap) = minimap() else {
                        continue;
                    };
                    let Some(preset) = minimap_preset() else {
                        continue;
                    };
                    let Some(actions) = current_minimap.actions.get_mut(&preset) else {
                        continue;
                    };
                    let index = if matches!(action.condition(), ActionCondition::Linked) {
                        find_last_linked_action_index(actions, condition)
                            .map(|index| index + 1)
                            .unwrap_or(actions.len())
                    } else {
                        actions.len()
                    };

                    actions.insert(index, action);
                    save_minimap(current_minimap).await;
                }
                ActionUpdate::Edit(action, index) => {
                    let Some(mut current_minimap) = minimap() else {
                        continue;
                    };
                    let Some(preset) = minimap_preset() else {
                        continue;
                    };
                    let Some(actions) = current_minimap.actions.get_mut(&preset) else {
                        continue;
                    };

                    actions[index] = action;
                    save_minimap(current_minimap).await;
                }
                ActionUpdate::Delete(index) => {
                    let Some(mut current_minimap) = minimap() else {
                        continue;
                    };
                    let Some(preset) = minimap_preset() else {
                        continue;
                    };
                    let Some(actions) = current_minimap.actions.get_mut(&preset) else {
                        continue;
                    };
                    let action = actions[index];

                    // Replaces the first linked action to this `action` condition
                    // TODO: Maybe replace find_linked_action_range with a simple lookahead
                    if !matches!(action.condition(), ActionCondition::Linked)
                        && find_linked_action_range(actions, index).is_some()
                    {
                        actions[index + 1] = actions[index + 1].with_condition(action.condition());
                    }
                    actions.remove(index);
                    save_minimap(current_minimap).await;
                }
                ActionUpdate::Move(index, condition, up) => {
                    let Some(mut current_minimap) = minimap() else {
                        continue;
                    };
                    let Some(preset) = minimap_preset() else {
                        continue;
                    };
                    let Some(actions) = current_minimap.actions.get_mut(&preset) else {
                        continue;
                    };
                    let filtered = filter_actions(actions.clone(), condition);
                    if (up && index <= filtered.first().expect("cannot be empty").1)
                        || (!up && index >= filtered.last().expect("cannot be empty").1)
                    {
                        continue;
                    }

                    // Finds the action index of `filtered` before or after `index`
                    let filtered_index = filtered
                        .iter()
                        .enumerate()
                        .find_map(|(filtered_index, (_, actions_index))| {
                            if *actions_index == index {
                                if up {
                                    Some(filtered_index - 1)
                                } else {
                                    Some(filtered_index + 1)
                                }
                            } else {
                                None
                            }
                        })
                        .expect("must be valid index");
                    let filtered_condition = filtered[filtered_index].0.condition();
                    let action_condition = actions[index].condition();
                    match (action_condition, filtered_condition) {
                        // Simple case - swapping two linked actions
                        (ActionCondition::Linked, ActionCondition::Linked) => {
                            actions.swap(index, filtered[filtered_index].1);
                            save_minimap(current_minimap).await;
                            continue;
                        }
                        // Disallows moving up/down if `index` is a linked action and
                        // `filtered_index` is a non-linked action
                        (ActionCondition::Linked, _) => continue,
                        _ => (),
                    }

                    // Finds the first non-linked action index of `filtered` before or after `index`
                    let mut filtered_non_linked_index = filtered_index;
                    while (up && filtered_non_linked_index > 0)
                        || (!up && filtered_non_linked_index < filtered.len() - 1)
                    {
                        let condition = filtered[filtered_non_linked_index].0.condition();
                        if !matches!(condition, ActionCondition::Linked) {
                            break;
                        }
                        if up {
                            filtered_non_linked_index -= 1;
                        } else {
                            filtered_non_linked_index += 1;
                        }
                    }
                    let condition = filtered[filtered_non_linked_index].0.condition();
                    if matches!(condition, ActionCondition::Linked) {
                        continue;
                    }

                    let actions_non_linked_index = filtered[filtered_non_linked_index].1;
                    let first_range = find_linked_action_range(actions, actions_non_linked_index);
                    let mut first_range = if let Some(range) = first_range {
                        actions_non_linked_index..range.end
                    } else {
                        actions_non_linked_index..actions_non_linked_index + 1
                    };

                    let second_range = find_linked_action_range(actions, index);
                    let mut second_range = if let Some(range) = second_range {
                        index..range.end
                    } else {
                        index..index + 1
                    };

                    if !up {
                        swap(&mut first_range, &mut second_range);
                    }

                    debug_assert!(
                        first_range.end <= second_range.start
                            || second_range.end <= first_range.start
                    );
                    let second_start = second_range.start;
                    let second_actions = actions.drain(second_range).collect::<Vec<_>>();
                    let first_actions = actions[first_range.clone()].to_vec();
                    for action in first_actions.into_iter().rev() {
                        actions.insert(second_start, action);
                    }

                    let first_start = first_range.start;
                    let _ = actions.drain(first_range);
                    for action in second_actions.into_iter().rev() {
                        actions.insert(first_start, action);
                    }

                    save_minimap(current_minimap).await;
                }
            }
        }
    });
    let save_minimap = use_callback(move |new_minimap: Minimap| {
        minimap.set(Some(new_minimap));
        coroutine.send(ActionUpdate::SaveMinimap);
        coroutine.send(ActionUpdate::SetPreset);
    });
    let mut popup_input_kind = use_signal(|| None);
    let actions_list_disabled = use_memo(move || minimap().is_none() || minimap_preset().is_none());

    // Sets a preset if there is not one
    use_effect(move || {
        if let Some(minimap) = minimap() {
            if !minimap.actions.is_empty() && minimap_preset.peek().is_none() {
                minimap_preset.set(minimap.actions.into_keys().next());
                coroutine.send(ActionUpdate::SetPreset);
            }
        } else {
            minimap_preset.set(None);
            coroutine.send(ActionUpdate::SetPreset);
        }
    });

    rsx! {
        div { class: "flex flex-col pb-15 h-full gap-3 overflow-y-auto scrollbar pr-2",
            SectionRotation {
                popup_input_kind,
                minimap_view,
                disabled: minimap().is_none(),
                save_minimap,
            }
            SectionPlatforms {
                popup_input_kind,
                minimap_view,
                disabled: minimap().is_none(),
                save_minimap,
            }
            SectionActions {
                popup_input_kind,
                actions_list_disabled,
                minimap_preset_actions,
            }
            SectionLegends {}
        }
        if let Some(kind) = popup_input_kind() {
            match kind {
                PopupInputKind::Action(_) => rsx! {
                    PopupActionInput { popup_input_kind, actions: minimap_preset_actions }
                },
                PopupInputKind::Bound(bound) => rsx! {
                    PopupBoundInput {
                        on_cancel: move |_| {
                            popup_input_kind.take();
                        },
                        on_value: move |bound| {
                            popup_input_kind.take();
                            coroutine.send(ActionUpdate::EditMobbingBound(bound));
                            coroutine.send(ActionUpdate::SetPreset);
                        },
                        value: bound,
                    }
                },
                PopupInputKind::Platform(platform, index) => {
                    rsx! {
                        PopupPlatformInput {
                            index,
                            on_cancel: move |_| {
                                popup_input_kind.take();
                            },
                            on_value: move |(mut platform, index): (Platform, Option<usize>)| {
                                platform.x_end = if platform.x_end <= platform.x_start {
                                    platform.x_start + 1
                                } else {
                                    platform.x_end
                                };
                                popup_input_kind.take();
                                if let Some(index) = index {
                                    coroutine.send(ActionUpdate::EditPlatform(platform, index));
                                } else {
                                    coroutine.send(ActionUpdate::AddPlatform(platform));
                                }
                                coroutine.send(ActionUpdate::SetPreset);
                            },
                            value: platform,
                        }
                    }
                }
            }
        }
        div { class: "flex items-center w-full h-10 pr-2 bg-gray-950 absolute bottom-0",
            TextSelect {
                class: "flex-grow",
                options: minimap_presets(),
                disabled: minimap().is_none(),
                placeholder: "Create an actions preset...",
                on_create: move |name| {
                    coroutine.send(ActionUpdate::CreatePreset(name));
                    coroutine.send(ActionUpdate::SetPreset);
                },
                on_delete: move |_| {
                    coroutine.send(ActionUpdate::DeletePreset);
                    coroutine.send(ActionUpdate::SetPreset);
                },
                on_select: move |(_, preset)| {
                    minimap_preset.set(Some(preset));
                    coroutine.send(ActionUpdate::SetPreset);
                },
                selected: minimap_preset_index(),
            }
        }
    }
}

#[component]
fn Section(
    name: String,
    #[props(default = String::default())] class: String,
    children: Element,
) -> Element {
    rsx! {
        div { class: "flex flex-col gap-2 {class}",
            div { class: "flex flex-none items-center title-xs h-10", {name} }
            {children}
        }
    }
}

#[component]
fn SectionRotation(
    popup_input_kind: Signal<Option<PopupInputKind>>,
    minimap_view: Memo<Minimap>,
    disabled: bool,
    save_minimap: EventHandler<Minimap>,
) -> Element {
    let update_mobbing_button_disabled = use_memo(move || {
        !matches!(
            minimap_view().rotation_mode,
            RotationMode::AutoMobbing(_) | RotationMode::PingPong(_)
        )
    });

    rsx! {
        Section { name: "Rotation",
            div { class: "grid grid-cols-2 gap-3",
                ActionsSelect::<RotationMode> {
                    label: "Mode",
                    disabled,
                    on_select: move |rotation_mode| {
                        save_minimap(Minimap {
                            rotation_mode,
                            ..minimap_view.peek().clone()
                        })
                    },
                    selected: minimap_view().rotation_mode,
                }
                div {}
                Button {
                    text: "Update mobbing key",
                    kind: ButtonKind::Primary,
                    disabled: disabled | update_mobbing_button_disabled(),
                    on_click: move |_| {
                        let key = match minimap_view.peek().rotation_mode {
                            RotationMode::StartToEnd | RotationMode::StartToEndThenReverse => {
                                unreachable!()
                            }
                            RotationMode::AutoMobbing(auto_mobbing) => auto_mobbing.key,
                            RotationMode::PingPong(ping_pong) => ping_pong.key,
                        };
                        let kind = ActionInputKind::PingPongOrAutoMobbing(key);
                        popup_input_kind.set(Some(PopupInputKind::Action(kind)));
                    },
                }
                Button {
                    text: "Update mobbing bound",
                    kind: ButtonKind::Primary,
                    disabled: disabled | update_mobbing_button_disabled(),
                    on_click: move |_| {
                        let bound = match minimap_view.peek().rotation_mode {
                            RotationMode::StartToEnd | RotationMode::StartToEndThenReverse => {
                                unreachable!()
                            }
                            RotationMode::AutoMobbing(auto_mobbing) => auto_mobbing.bound,
                            RotationMode::PingPong(ping_pong) => ping_pong.bound,
                        };
                        popup_input_kind.set(Some(PopupInputKind::Bound(bound)));
                    },
                }
                ActionsCheckbox {
                    label: "Reset normal actions on Erda Shower condition",
                    disabled,
                    on_value: move |actions_any_reset_on_erda_condition| {
                        save_minimap(Minimap {
                            actions_any_reset_on_erda_condition,
                            ..minimap_view.peek().clone()
                        })
                    },
                    value: minimap_view().actions_any_reset_on_erda_condition,
                }
            }
        }
    }
}

#[component]
fn SectionPlatforms(
    popup_input_kind: Signal<Option<PopupInputKind>>,
    minimap_view: Memo<Minimap>,
    disabled: bool,
    save_minimap: EventHandler<Minimap>,
) -> Element {
    #[component]
    fn PlatformItem(
        platform: Platform,
        on_item_click: EventHandler,
        on_item_delete: EventHandler,
    ) -> Element {
        const ICON_CONTAINER_CLASS: &str = "w-4 h-6 flex justify-center items-center";
        const ICON_CLASS: &str = "w-[11px] h-[11px] fill-current";

        rsx! {
            div { class: "relative group",
                div {
                    class: "grid grid-cols-2 h-6 paragraph-xs gap-2 !text-gray-400 group-hover:bg-gray-900",
                    onclick: move |e| {
                        e.stop_propagation();
                        on_item_click(());
                    },
                    div { class: "{ITEM_BORDER_CLASS} {ITEM_TEXT_CLASS}",
                        {format!("X / {} - {}", platform.x_start, platform.x_end)}
                    }
                    div { class: "{ITEM_TEXT_CLASS}", {format!("Y / {}", platform.y)} }
                }
                div { class: "absolute invisible group-hover:visible top-0 right-1 flex",
                    div {
                        class: ICON_CONTAINER_CLASS,
                        onclick: move |e| {
                            e.stop_propagation();
                            on_item_delete(());
                        },
                        XIcon { class: "{ICON_CLASS} text-red-500" }
                    }
                }
            }
        }
    }

    let coroutine = use_coroutine_handle::<ActionUpdate>();
    let settings = use_context::<AppState>().settings;
    let position = use_context::<AppState>().position;

    use_future(move || async move {
        let mut platform = Platform::default();
        let mut key_receiver = key_receiver().await;
        loop {
            let Ok(key) = key_receiver.recv().await else {
                continue;
            };
            let Some(settings) = &*settings.peek() else {
                continue;
            };

            if settings.platform_start_key.enabled && settings.platform_start_key.key == key {
                platform.x_start = position.peek().0;
                platform.y = position.peek().1;
                continue;
            }

            if settings.platform_end_key.enabled && settings.platform_end_key.key == key {
                platform.x_end = position.peek().0;
                platform.x_end = if platform.x_end <= platform.x_start {
                    platform.x_start + 1
                } else {
                    platform.x_end
                };
                platform.y = position.peek().1;
                continue;
            }

            if settings.platform_add_key.enabled && settings.platform_add_key.key == key {
                coroutine.send(ActionUpdate::AddPlatform(platform));
                coroutine.send(ActionUpdate::SetPreset);
                continue;
            }
        }
    });

    rsx! {
        Section { name: "Platforms",
            div { class: "grid grid-cols-3 gap-3",
                ActionsCheckbox {
                    label: "Rune pathing",
                    disabled,
                    on_value: move |rune_platforms_pathing| {
                        save_minimap(Minimap {
                            rune_platforms_pathing,
                            ..minimap_view.peek().clone()
                        })
                    },
                    value: minimap_view().rune_platforms_pathing,
                }
                ActionsCheckbox {
                    label: "Up jump only",
                    disabled: disabled || !minimap_view().rune_platforms_pathing,
                    on_value: move |rune_platforms_pathing_up_jump_only| {
                        save_minimap(Minimap {
                            rune_platforms_pathing_up_jump_only,
                            ..minimap_view.peek().clone()
                        })
                    },
                    value: minimap_view().rune_platforms_pathing_up_jump_only,
                }
                div {}
                ActionsCheckbox {
                    label: "Auto-mobbing pathing",
                    disabled,
                    on_value: move |auto_mob_platforms_pathing| {
                        save_minimap(Minimap {
                            auto_mob_platforms_pathing,
                            ..minimap_view.peek().clone()
                        })
                    },
                    value: minimap_view().auto_mob_platforms_pathing,
                }
                ActionsCheckbox {
                    label: "Up jump only",
                    disabled: disabled || !minimap_view().auto_mob_platforms_pathing,
                    on_value: move |auto_mob_platforms_pathing_up_jump_only| {
                        save_minimap(Minimap {
                            auto_mob_platforms_pathing_up_jump_only,
                            ..minimap_view.peek().clone()
                        })
                    },
                    value: minimap_view().auto_mob_platforms_pathing_up_jump_only,
                }
                ActionsCheckbox {
                    label: "Bound by platforms",
                    disabled,
                    on_value: move |auto_mob_platforms_bound| {
                        save_minimap(Minimap {
                            auto_mob_platforms_bound,
                            ..minimap_view.peek().clone()
                        })
                    },
                    value: minimap_view().auto_mob_platforms_bound,
                }
            }
            if !minimap_view().platforms.is_empty() {
                div { class: "mt-2" }
            }
            for (index , platform) in minimap_view().platforms.into_iter().enumerate() {
                PlatformItem {
                    platform,
                    on_item_click: move |_| {
                        popup_input_kind.set(Some(PopupInputKind::Platform(platform, Some(index))));
                    },
                    on_item_delete: move |_| {
                        coroutine.send(ActionUpdate::DeletePlatform(index));
                        coroutine.send(ActionUpdate::SetPreset);
                    },
                }
            }
            Button {
                text: "Add platform",
                kind: ButtonKind::Secondary,
                on_click: move |_| {
                    let kind = PopupInputKind::Platform(Platform::default(), None);
                    popup_input_kind.set(Some(kind));
                },
                disabled,
                class: "label mt-2",
            }
        }
    }
}

#[component]
fn SectionLegends() -> Element {
    rsx! {
        Section { name: "Action Legends", class: "paragraph-xs",
            p { "⟳ - Repeat" }
            p { "⏱︎  - Wait" }
            p { "ㄨ - No position" }
            p { "⇈ - Queue to front" }
            p { "⇆ - Any direction" }
            p { "← - Left direction" }
            p { "→ - Right direction" }
            p { "A ~ B - Random range between A and B" }
            p { "A ↝ B - Use A key then B key" }
            p { "A ↜ B - Use B key then A key" }
            p { "A ↭ B - Use A and B keys at the same time" }
            p { "A ↷ B - Use A key then B key while A is held down" }
        }
    }
}

#[component]
fn SectionActions(
    popup_input_kind: Signal<Option<PopupInputKind>>,
    actions_list_disabled: Memo<bool>,
    minimap_preset_actions: Memo<Vec<Action>>,
) -> Element {
    let coroutine = use_coroutine_handle::<ActionUpdate>();
    let mut popup_input = move |action_input_kind| {
        let popup_kind = PopupInputKind::Action(action_input_kind);
        popup_input_kind.set(Some(popup_kind));
    };

    rsx! {
        Section { name: "Normal actions",
            ActionList {
                on_add_click: move |_| {
                    popup_input(ActionInputKind::Add(Action::Key(ActionKey::default())));
                },
                on_item_click: move |(action, index)| {
                    popup_input(ActionInputKind::Edit(action, index));
                },
                on_item_move: move |(index, condition, up)| {
                    coroutine.send(ActionUpdate::Move(index, condition, up));
                    coroutine.send(ActionUpdate::SetPreset);
                },
                on_item_delete: move |index| {
                    coroutine.send(ActionUpdate::Delete(index));
                    coroutine.send(ActionUpdate::SetPreset);
                },
                condition_filter: ActionCondition::Any,
                disabled: actions_list_disabled(),
                actions: minimap_preset_actions(),
            }
        }
        Section { name: "Erda Shower off cooldown priority actions",
            ActionList {
                on_add_click: move |_| {
                    let action = Action::Key(ActionKey {
                        condition: ActionCondition::ErdaShowerOffCooldown,
                        ..ActionKey::default()
                    });
                    popup_input(ActionInputKind::Add(action));
                },
                on_item_click: move |(action, index)| {
                    popup_input(ActionInputKind::Edit(action, index));
                },
                on_item_move: move |(index, condition, up)| {
                    coroutine.send(ActionUpdate::Move(index, condition, up));
                    coroutine.send(ActionUpdate::SetPreset);
                },
                on_item_delete: move |index| {
                    coroutine.send(ActionUpdate::Delete(index));
                    coroutine.send(ActionUpdate::SetPreset);
                },
                condition_filter: ActionCondition::ErdaShowerOffCooldown,
                disabled: actions_list_disabled(),
                actions: minimap_preset_actions(),
            }
        }
        Section { name: "Every milliseconds priority actions",
            ActionList {
                on_add_click: move |_| {
                    let action = Action::Key(ActionKey {
                        condition: ActionCondition::EveryMillis(0),
                        ..ActionKey::default()
                    });
                    popup_input(ActionInputKind::Add(action));
                },
                on_item_click: move |(action, index)| {
                    popup_input(ActionInputKind::Edit(action, index));
                },
                on_item_move: move |(index, condition, up)| {
                    coroutine.send(ActionUpdate::Move(index, condition, up));
                    coroutine.send(ActionUpdate::SetPreset);
                },
                on_item_delete: move |index| {
                    coroutine.send(ActionUpdate::Delete(index));
                    coroutine.send(ActionUpdate::SetPreset);
                },
                condition_filter: ActionCondition::EveryMillis(0),
                disabled: actions_list_disabled(),
                actions: minimap_preset_actions(),
            }
        }
    }
}

#[component]
fn PopupPlatformInput(
    index: Option<usize>,
    on_cancel: EventHandler,
    on_value: EventHandler<(Platform, Option<usize>)>,
    value: Platform,
) -> Element {
    const ICON_CONTAINER_CLASS: &str = "absolute invisible group-hover:visible top-5 right-1 w-4 h-6 flex justify-center items-center";
    const ICON_CLASS: &str = "w-3 h-3 text-gray-50 fill-current";

    let position = use_context::<AppState>().position;
    let mut platform = use_signal(|| value);
    let section_name = if index.is_some() {
        "Modify platform"
    } else {
        "Add platform"
    };
    let button_name = if index.is_some() { "Save" } else { "Add" };

    use_effect(use_reactive!(|value| platform.set(value)));

    rsx! {
        div { class: "px-16 py-42 w-full h-full absolute inset-0 z-1 bg-gray-950/80",
            div { class: "bg-gray-900 h-full px-2",
                Section { name: section_name, class: "relative h-full",
                    div { class: "grid grid-cols-3 gap-3",
                        div { class: "relative group",
                            ActionsNumberInputI32 {
                                label: "X start",
                                on_value: move |x| {
                                    platform.write().x_start = x;
                                },
                                value: platform().x_start,
                            }
                            div {
                                class: ICON_CONTAINER_CLASS,
                                onclick: move |_| {
                                    platform.write().x_start = position.peek().0;
                                },
                                PositionIcon { class: ICON_CLASS }
                            }
                        }
                        div { class: "relative group",
                            ActionsNumberInputI32 {
                                label: "X end",
                                on_value: move |x| {
                                    platform.write().x_end = x;
                                },
                                value: platform().x_end,
                            }
                            div {
                                class: ICON_CONTAINER_CLASS,
                                onclick: move |_| {
                                    platform.write().x_end = position.peek().0;
                                },
                                PositionIcon { class: ICON_CLASS }
                            }
                        }
                        div { class: "relative group",
                            ActionsNumberInputI32 {
                                label: "Y",
                                on_value: move |y| {
                                    platform.write().y = y;
                                },
                                value: platform().y,
                            }
                            div {
                                class: ICON_CONTAINER_CLASS,
                                onclick: move |_| {
                                    platform.write().y = position.peek().1;
                                },
                                PositionIcon { class: ICON_CLASS }
                            }
                        }
                    }
                    div { class: "flex w-full gap-3 absolute bottom-2",
                        Button {
                            class: "flex-grow border border-gray-600",
                            text: button_name,
                            kind: ButtonKind::Primary,
                            on_click: move |_| {
                                on_value((*platform.peek(), index));
                            },
                        }
                        Button {
                            class: "flex-grow border border-gray-600",
                            text: "Cancel",
                            kind: ButtonKind::Danger,
                            on_click: move |_| {
                                on_cancel(());
                            },
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn PopupBoundInput(
    on_cancel: EventHandler,
    on_value: EventHandler<Bound>,
    value: Bound,
) -> Element {
    let mut bound = use_signal(|| value);

    use_effect(use_reactive!(|value| bound.set(value)));

    rsx! {
        div { class: "px-16 py-35 w-full h-full absolute inset-0 z-1 bg-gray-950/80",
            div { class: "bg-gray-900 h-full px-2",
                Section { name: "Modify mobbing bound", class: "relative h-full",
                    div { class: "grid grid-cols-2 gap-3",
                        ActionsNumberInputI32 {
                            label: "X offset",
                            on_value: move |x| {
                                bound.write().x = x;
                            },
                            value: bound().x,
                        }
                        ActionsNumberInputI32 {
                            label: "Y offset",
                            on_value: move |y| {
                                bound.write().y = y;
                            },
                            value: bound().y,
                        }
                        ActionsNumberInputI32 {
                            label: "Width",
                            on_value: move |width| {
                                bound.write().width = width;
                            },
                            value: bound().width,
                        }
                        ActionsNumberInputI32 {
                            label: "Height",
                            on_value: move |height| {
                                bound.write().height = height;
                            },
                            value: bound().height,
                        }
                    }
                    div { class: "flex w-full gap-3 absolute bottom-2",
                        Button {
                            class: "flex-grow border border-gray-600",
                            text: "Save",
                            kind: ButtonKind::Primary,
                            on_click: move |_| {
                                on_value(*bound.peek());
                            },
                        }
                        Button {
                            class: "flex-grow border border-gray-600",
                            text: "Cancel",
                            kind: ButtonKind::Danger,
                            on_click: move |_| {
                                on_cancel(());
                            },
                        }
                    }
                }
            }
        }
    }
}

// TODO: Move popup_input_kind out and replace with callbacks
#[component]
fn PopupActionInput(
    popup_input_kind: Signal<Option<PopupInputKind>>,
    actions: ReadOnlySignal<Vec<Action>>,
) -> Element {
    #[derive(PartialEq, Clone, Debug)]
    struct State {
        action: Action,
        modifying: bool,
        switchable: bool,
        section_text: String,
        can_create_linked_action: bool,
    }

    let state = use_memo(move || {
        popup_input_kind()
            .map(|kind| match kind {
                PopupInputKind::Action(kind) => kind,
                PopupInputKind::Bound(_) | PopupInputKind::Platform(_, _) => unreachable!(),
            })
            .map(|kind| {
                let (action, index) = match kind {
                    ActionInputKind::PingPongOrAutoMobbing(key) => {
                        let key = ActionKey {
                            key: key.key,
                            link_key: key.link_key,
                            count: key.count,
                            with: key.with,
                            wait_before_use_millis: key.wait_before_millis,
                            wait_before_use_millis_random_range: key
                                .wait_before_millis_random_range,
                            wait_after_use_millis: key.wait_after_millis,
                            wait_after_use_millis_random_range: key.wait_after_millis_random_range,
                            ..ActionKey::default()
                        };
                        let action = Action::Key(key);

                        (action, None)
                    }
                    ActionInputKind::Add(action) => (action, None),
                    ActionInputKind::Edit(action, index) => (action, Some(index)),
                };
                let switchable = !matches!(kind, ActionInputKind::PingPongOrAutoMobbing(_));
                let modifying = matches!(
                    kind,
                    ActionInputKind::Edit(_, _) | ActionInputKind::PingPongOrAutoMobbing(_)
                );
                let can_create_linked_action = match kind {
                    ActionInputKind::Add(_) | ActionInputKind::Edit(_, _) => {
                        match action.condition() {
                            ActionCondition::EveryMillis(_)
                            | ActionCondition::ErdaShowerOffCooldown
                            | ActionCondition::Any => {
                                let actions = actions();
                                let filtered = filter_actions(actions, action.condition());
                                let is_not_empty = !filtered.is_empty();
                                let first_index = filtered.into_iter().next().map(|first| first.1);

                                is_not_empty && first_index != index
                            }
                            ActionCondition::Linked => false,
                        }
                    }
                    ActionInputKind::PingPongOrAutoMobbing(_) => false,
                };
                let section_text = match kind {
                    ActionInputKind::Add(_) | ActionInputKind::Edit(_, _) => {
                        let name = match action.condition() {
                            backend::ActionCondition::Any => "normal",
                            backend::ActionCondition::EveryMillis(_) => "every milliseconds",
                            backend::ActionCondition::ErdaShowerOffCooldown => {
                                "Erda Shower off cooldown"
                            }
                            backend::ActionCondition::Linked => "linked",
                        };
                        if modifying {
                            format!("Modify a {name} action")
                        } else {
                            format!("Add a new {name} action")
                        }
                    }
                    ActionInputKind::PingPongOrAutoMobbing(_) => "Modify mobbing skill".to_string(),
                };

                State {
                    action,
                    switchable,
                    modifying,
                    section_text,
                    can_create_linked_action,
                }
            })
    });
    let coroutine = use_coroutine_handle::<ActionUpdate>();

    rsx! {
        if let Some(
            State { action, switchable, modifying, section_text, can_create_linked_action },
        ) = state()
        {
            div { class: "p-8 w-full h-full absolute inset-0 z-1 bg-gray-950/80",
                ActionInput {
                    section_text,
                    switchable,
                    modifying,
                    can_create_linked_action,
                    can_have_position: switchable,
                    can_have_direction: switchable,
                    on_cancel: move |_| {
                        popup_input_kind.set(None);
                    },
                    on_value: move |(action, condition)| {
                        match popup_input_kind
                            .take()
                            .map(|kind| match kind {
                                PopupInputKind::Action(kind) => kind,
                                PopupInputKind::Bound(_) => unreachable!(),
                                PopupInputKind::Platform(_, _) => unreachable!(),
                            })
                            .expect("input kind must already be set")
                        {
                            ActionInputKind::Add(_) => {
                                coroutine.send(ActionUpdate::Add(action, condition));
                            }
                            ActionInputKind::Edit(_, index) => {
                                coroutine.send(ActionUpdate::Edit(action, index));
                            }
                            ActionInputKind::PingPongOrAutoMobbing(_) => {
                                let action = match action {
                                    Action::Move(_) => unreachable!(),
                                    Action::Key(action) => action,
                                };
                                coroutine
                                    .send(
                                        ActionUpdate::EditMobbingKey(MobbingKey {
                                            key: action.key,
                                            link_key: action.link_key,
                                            count: action.count,
                                            with: action.with,
                                            wait_before_millis: action.wait_before_use_millis,
                                            wait_before_millis_random_range: action
                                                .wait_before_use_millis_random_range,
                                            wait_after_millis: action.wait_after_use_millis,
                                            wait_after_millis_random_range: action
                                                .wait_after_use_millis_random_range,
                                        }),
                                    );
                            }
                        }
                        coroutine.send(ActionUpdate::SetPreset);
                    },
                    value: action,
                }
            }
        }
    }
}

#[component]
fn ActionInput(
    section_text: String,
    switchable: bool,
    modifying: bool,
    can_create_linked_action: bool,
    can_have_position: bool,
    can_have_direction: bool,
    on_cancel: EventHandler,
    on_value: EventHandler<(Action, ActionCondition)>,
    value: Action,
) -> Element {
    let mut action = use_signal(|| value);
    let button_text = use_memo(move || {
        if matches!(action(), Action::Move(_)) {
            "Switch to key"
        } else {
            "Switch to move"
        }
    });

    use_effect(use_reactive!(|value| action.set(value)));

    rsx! {
        div { class: "bg-gray-900 h-full px-2",
            Section { name: section_text, class: "relative h-full",
                if switchable {
                    Button {
                        text: button_text(),
                        kind: ButtonKind::Primary,
                        on_click: move |_| {
                            if discriminant(&value) != discriminant(&*action.peek()) {
                                action.set(value);
                            } else if matches!(value, Action::Move(_)) {
                                action
                                    .set(
                                        Action::Key(ActionKey {
                                            condition: value.condition(),
                                            ..ActionKey::default()
                                        }),
                                    );
                            } else {
                                action
                                    .set(
                                        Action::Move(ActionMove {
                                            condition: value.condition(),
                                            ..ActionMove::default()
                                        }),
                                    );
                            }
                        },
                        class: "flex-none label border-b border-gray-600",
                    }
                }
                match action() {
                    Action::Move(action) => rsx! {
                        ActionMoveInput {
                            modifying,
                            can_create_linked_action,
                            on_cancel,
                            on_value: move |(action, condition)| {
                                on_value((Action::Move(action), condition));
                            },
                            value: action,
                        }
                    },
                    Action::Key(action) => rsx! {
                        ActionKeyInput {
                            modifying,
                            can_create_linked_action,
                            can_have_position,
                            can_have_direction,
                            on_cancel,
                            on_value: move |(action, condition)| {
                                on_value((Action::Key(action), condition));
                            },
                            value: action,
                        }
                    },
                }
            }
        }
    }
}

#[component]
fn ActionMoveInput(
    modifying: bool,
    can_create_linked_action: bool,
    on_cancel: EventHandler,
    on_value: EventHandler<(ActionMove, ActionCondition)>,
    value: ActionMove,
) -> Element {
    const ICON_CONTAINER_CLASS: &str = "absolute invisible group-hover:visible top-5 right-1 w-4 h-6 flex justify-center items-center";
    const ICON_CLASS: &str = "w-3 h-3 text-gray-50 fill-current";

    let position = use_context::<AppState>().position;
    let mut action = use_signal(|| value);

    use_effect(use_reactive!(|value| { action.set(value) }));

    rsx! {
        div { class: "grid grid-cols-3 gap-3",
            // Position
            ActionsCheckbox {
                label: "Adjust",
                on_value: move |adjust: bool| {
                    let mut action = action.write();
                    action.position.allow_adjusting = adjust;
                },
                value: action().position.allow_adjusting,
            }
            div { class: "col-span-2" }
            div { class: "relative group",
                ActionsNumberInputI32 {
                    label: "X",
                    on_value: move |x| {
                        let mut action = action.write();
                        action.position.x = x;
                    },
                    value: action().position.x,
                }
                div {
                    class: ICON_CONTAINER_CLASS,
                    onclick: move |_| {
                        let mut action = action.write();
                        action.position.x = position.peek().0;
                    },
                    PositionIcon { class: ICON_CLASS }
                }
            }

            ActionsNumberInputI32 {
                label: "X random range",
                on_value: move |x| {
                    let mut action = action.write();
                    action.position.x_random_range = x;
                },
                value: action().position.x_random_range,
            }
            div { class: "relative group",
                ActionsNumberInputI32 {
                    label: "Y",
                    on_value: move |y| {
                        let mut action = action.write();
                        action.position.y = y;
                    },
                    value: action().position.y,
                }
                div {
                    class: ICON_CONTAINER_CLASS,
                    onclick: move |_| {
                        let mut action = action.write();
                        action.position.y = position.peek().1;
                    },
                    PositionIcon { class: ICON_CLASS }
                }
            }
            ActionsMillisInput {
                label: "Wait after move",
                on_value: move |millis| {
                    let mut action = action.write();
                    action.wait_after_move_millis = millis;
                },
                value: action().wait_after_move_millis,
            }
            if can_create_linked_action {
                ActionsCheckbox {
                    label: "Linked action",
                    on_value: move |is_linked: bool| {
                        let mut action = action.write();
                        action.condition = if is_linked {
                            ActionCondition::Linked
                        } else {
                            value.condition
                        };
                    },
                    value: matches!(action().condition, ActionCondition::Linked),
                }
            }
        }
        div { class: "flex w-full gap-3 absolute bottom-2",
            Button {
                class: "flex-grow border border-gray-600",
                text: if modifying { "Save" } else { "Add" },
                kind: ButtonKind::Primary,
                on_click: move |_| {
                    on_value((*action.peek(), value.condition));
                },
            }
            Button {
                class: "flex-grow border border-gray-600",
                text: "Cancel",
                kind: ButtonKind::Danger,
                on_click: move |_| {
                    on_cancel(());
                },
            }
        }
    }
}

#[component]
fn ActionKeyInput(
    modifying: bool,
    can_create_linked_action: bool,
    can_have_position: bool,
    can_have_direction: bool,
    on_cancel: EventHandler,
    on_value: EventHandler<(ActionKey, ActionCondition)>,
    value: ActionKey,
) -> Element {
    const ICON_CONTAINER_CLASS: &str = "absolute invisible group-hover:visible top-5 right-1 w-4 h-6 flex justify-center items-center";
    const ICON_CLASS: &str = "w-3 h-3 text-gray-50 fill-current";

    let position = use_context::<AppState>().position;
    let mut action = use_signal(|| value);

    use_effect(use_reactive!(|value| { action.set(value) }));

    rsx! {
        div { class: "grid grid-cols-3 gap-3 pb-10 pr-2 overflow-y-auto scrollbar",
            if can_have_position {
                ActionsCheckbox {
                    label: "Positioned",
                    on_value: move |has_position: bool| {
                        let mut action = action.write();
                        action.position = has_position.then_some(Position::default());
                    },
                    value: action().position.is_some(),
                }
                ActionsCheckbox {
                    label: "Adjust",
                    disabled: action().position.is_none(),
                    on_value: move |adjust: bool| {
                        let mut action = action.write();
                        action.position.as_mut().unwrap().allow_adjusting = adjust;
                    },
                    value: action().position.map(|pos| pos.allow_adjusting).unwrap_or_default(),
                }
                div {}


                // Position
                div { class: "relative group",
                    ActionsNumberInputI32 {
                        label: "X",
                        disabled: action().position.is_none(),
                        on_value: move |x| {
                            let mut action = action.write();
                            action.position.as_mut().unwrap().x = x;
                        },
                        value: action().position.map(|pos| pos.x).unwrap_or_default(),
                    }
                    if action().position.is_some() {
                        div {
                            class: ICON_CONTAINER_CLASS,
                            onclick: move |_| {
                                let mut action = action.write();
                                action.position.as_mut().unwrap().x = position.peek().0;
                            },
                            PositionIcon { class: ICON_CLASS }
                        }
                    }
                }
                ActionsNumberInputI32 {
                    label: "X random range",
                    disabled: action().position.is_none(),
                    on_value: move |x| {
                        let mut action = action.write();
                        action.position.as_mut().unwrap().x_random_range = x;
                    },
                    value: action().position.map(|pos| pos.x_random_range).unwrap_or_default(),
                }
                div { class: "relative group",
                    ActionsNumberInputI32 {
                        label: "Y",
                        disabled: action().position.is_none(),
                        on_value: move |y| {
                            let mut action = action.write();
                            action.position.as_mut().unwrap().y = y;
                        },
                        value: action().position.map(|pos| pos.y).unwrap_or_default(),
                    }
                    if action().position.is_some() {
                        div {
                            class: ICON_CONTAINER_CLASS,
                            onclick: move |_| {
                                let mut action = action.write();
                                action.position.as_mut().unwrap().y = position.peek().1;
                            },
                            PositionIcon { class: ICON_CLASS }
                        }
                    }
                }
            }

            // Key, count and link key
            ActionsKeyBindingInput {
                label: "Key",
                disabled: false,
                on_value: move |key: Option<KeyBinding>| {
                    let mut action = action.write();
                    action.key = key.expect("not optional");
                },
                value: Some(action().key),
            }
            ActionsNumberInputU32 {
                label: "Use count",
                on_value: move |count| {
                    let mut action = action.write();
                    action.count = count;
                },
                value: action().count,
            }
            if can_create_linked_action {
                ActionsCheckbox {
                    label: "Linked action",
                    on_value: move |is_linked: bool| {
                        let mut action = action.write();
                        action.condition = if is_linked {
                            ActionCondition::Linked
                        } else {
                            value.condition
                        };
                        action.queue_to_front = None;
                    },
                    value: matches!(action().condition, ActionCondition::Linked),
                }
            } else {
                div {} // Spacer
            }
            ActionsKeyBindingInput {
                label: "Link key",
                disabled: action().link_key.is_none(),
                on_value: move |key: Option<KeyBinding>| {
                    let mut action = action.write();
                    action.link_key = action
                        .link_key
                        .map(|link_key| link_key.with_key(key.expect("not optional")));
                },
                value: action().link_key.unwrap_or_default().key(),
            }
            ActionsSelect::<LinkKeyBinding> {
                label: "Link key type",
                disabled: action().link_key.is_none(),
                on_select: move |link_key: LinkKeyBinding| {
                    let mut action = action.write();
                    action.link_key = Some(
                        link_key.with_key(action.link_key.expect("has link key if selectable").key()),
                    );
                },
                selected: action().link_key.unwrap_or_default(),
            }
            ActionsCheckbox {
                label: "Has link key",
                on_value: move |has_link_key: bool| {
                    let mut action = action.write();
                    action.link_key = has_link_key.then_some(LinkKeyBinding::default());
                },
                value: action().link_key.is_some(),
            }

            // Use with, direction

            ActionsSelect::<ActionKeyWith> {
                label: "Use with",
                disabled: false,
                on_select: move |with| {
                    let mut action = action.write();
                    action.with = with;
                },
                selected: action().with,
            }
            if can_have_direction {
                ActionsSelect::<ActionKeyDirection> {
                    label: "Use direction",
                    disabled: false,
                    on_select: move |direction| {
                        let mut action = action.write();
                        action.direction = direction;
                    },
                    selected: action().direction,
                }
            } else {
                div {} // Spacer
            }
            if matches!(
                action().condition,
                ActionCondition::EveryMillis(_) | ActionCondition::ErdaShowerOffCooldown
            )
            {
                ActionsCheckbox {
                    label: "Queue to front",
                    on_value: move |queue_to_front: bool| {
                        let mut action = action.write();
                        action.queue_to_front = Some(queue_to_front);
                    },
                    value: action().queue_to_front.is_some(),
                }
            } else {
                div {} // Spacer
            }
            if let ActionCondition::EveryMillis(millis) = action().condition {
                ActionsMillisInput {
                    label: "Use every",
                    on_value: move |millis| {
                        let mut action = action.write();
                        action.condition = ActionCondition::EveryMillis(millis);
                    },
                    value: millis,
                }
                div { class: "col-span-2" }
            }

            // Wait before use
            ActionsMillisInput {
                label: "Wait before",
                on_value: move |millis| {
                    let mut action = action.write();
                    action.wait_before_use_millis = millis;
                },
                value: action().wait_before_use_millis,
            }
            ActionsMillisInput {
                label: "Wait random range",
                on_value: move |millis| {
                    let mut action = action.write();
                    action.wait_before_use_millis_random_range = millis;
                },
                value: action().wait_before_use_millis_random_range,
            }
            div {} // Spacer

            // Wait after use
            ActionsMillisInput {
                label: "Wait after",
                on_value: move |millis| {
                    let mut action = action.write();
                    action.wait_after_use_millis = millis;
                },
                value: action().wait_after_use_millis,
            }
            ActionsMillisInput {
                label: "Wait random range",
                on_value: move |millis| {
                    let mut action = action.write();
                    action.wait_after_use_millis_random_range = millis;
                },
                value: action().wait_after_use_millis_random_range,
            }
        }
        div { class: "flex w-full gap-3 absolute bottom-0 py-2 bg-gray-900",
            Button {
                class: "flex-grow border border-gray-600",
                text: if modifying { "Save" } else { "Add" },
                kind: ButtonKind::Primary,
                on_click: move |_| {
                    on_value((*action.peek(), value.condition));
                },
            }
            Button {
                class: "flex-grow border border-gray-600",
                text: "Cancel",
                kind: ButtonKind::Danger,
                on_click: move |_| {
                    on_cancel(());
                },
            }
        }
    }
}

#[component]
fn ActionList(
    on_add_click: EventHandler,
    on_item_click: EventHandler<(Action, usize)>,
    on_item_move: EventHandler<(usize, ActionCondition, bool)>,
    on_item_delete: EventHandler<usize>,
    condition_filter: ActionCondition,
    disabled: bool,
    actions: Vec<Action>,
) -> Element {
    #[component]
    fn Icons(
        condition_filter: ActionCondition,
        action: Action,
        index: usize,
        on_item_move: EventHandler<(usize, ActionCondition, bool)>,
        on_item_delete: EventHandler<usize>,
    ) -> Element {
        const ICON_CONTAINER_CLASS: &str = "w-4 h-6 flex justify-center items-center";
        const ICON_CLASS: &str = "w-[11px] h-[11px] fill-current";

        let container_margin = if matches!(action.condition(), ActionCondition::Linked) {
            ""
        } else {
            "mt-2"
        };
        rsx! {
            div { class: "absolute invisible group-hover:visible top-0 right-1 flex {container_margin}",
                div {
                    class: ICON_CONTAINER_CLASS,
                    onclick: move |e| {
                        e.stop_propagation();
                        on_item_move((index, condition_filter, true));
                    },
                    UpArrowIcon { class: "{ICON_CLASS} text-gray-50" }
                }
                div {
                    class: ICON_CONTAINER_CLASS,
                    onclick: move |e| {
                        e.stop_propagation();
                        on_item_move((index, condition_filter, false));
                    },
                    DownArrowIcon { class: "{ICON_CLASS} text-gray-50" }
                }
                div {
                    class: ICON_CONTAINER_CLASS,
                    onclick: move |e| {
                        e.stop_propagation();
                        on_item_delete(index);
                    },
                    XIcon { class: "{ICON_CLASS} text-red-500" }
                }
            }
        }
    }

    let filtered = filter_actions(actions, condition_filter);

    rsx! {
        div { class: "flex flex-col",
            for (action , index) in filtered {
                div {
                    class: "relative group",
                    onclick: move |e| {
                        e.stop_propagation();
                        on_item_click((action, index));
                    },
                    match action {
                        Action::Move(action) => rsx! {
                            ActionMoveItem { action }
                        },
                        Action::Key(action) => rsx! {
                            ActionKeyItem { action }
                        },
                    }
                    Icons {
                        condition_filter,
                        action,
                        index,
                        on_item_move,
                        on_item_delete,
                    }
                }
            }
            Button {
                text: "Add action",
                kind: ButtonKind::Secondary,
                on_click: move |_| {
                    on_add_click(());
                },
                disabled,
                class: "label mt-2",
            }
        }
    }
}

#[component]
fn ActionMoveItem(action: ActionMove) -> Element {
    let ActionMove {
        position:
            Position {
                x,
                x_random_range,
                y,
                allow_adjusting,
            },
        condition,
        wait_after_move_millis,
    } = action;

    let x_min = (x - x_random_range).max(0);
    let x_max = (x + x_random_range).max(0);
    let x = if x_min == x_max {
        format!("{x}")
    } else {
        format!("{x_min}~{x_max}")
    };
    let allow_adjusting = if allow_adjusting { " / Adjust" } else { "" };

    let position = format!("{x}, {y}{allow_adjusting}");
    let linked_action = if matches!(condition, ActionCondition::Linked) {
        ""
    } else {
        "mt-2"
    };
    let wait_secs = format!("⏱︎ {:.2}s", wait_after_move_millis as f32 / 1000.0);

    rsx! {
        div { class: "grid grid-cols-[140px_100px_auto] h-6 paragraph-xs !text-gray-400 group-hover:bg-gray-900 {linked_action}",
            div { class: "{ITEM_BORDER_CLASS} {ITEM_TEXT_CLASS}", "{position}" }
            div { class: "{ITEM_TEXT_CLASS}", "{wait_secs}" }
            div {}
        }
    }
}

#[component]
fn ActionKeyItem(action: ActionKey) -> Element {
    let ActionKey {
        key,
        link_key,
        count,
        position,
        condition,
        direction,
        with,
        queue_to_front,
        wait_before_use_millis,
        wait_after_use_millis,
        ..
    } = action;

    let position = if let Some(Position {
        x,
        y,
        x_random_range,
        allow_adjusting,
    }) = position
    {
        let x_min = (x - x_random_range).max(0);
        let x_max = (x + x_random_range).max(0);
        let x = if x_min == x_max {
            format!("{x}")
        } else {
            format!("{x_min}~{x_max}")
        };
        let allow_adjusting = if allow_adjusting { " / Adjust" } else { "" };

        format!("{x}, {y}{allow_adjusting}")
    } else {
        "ㄨ".to_string()
    };
    let queue_to_front = if queue_to_front.unwrap_or_default() {
        "⇈ / "
    } else {
        ""
    };
    let linked_action = if matches!(condition, ActionCondition::Linked) {
        ""
    } else {
        "mt-2"
    };
    let link_key = match link_key {
        Some(LinkKeyBinding::Before(key)) => format!("{key} ↝ "),
        Some(LinkKeyBinding::After(key)) => format!("{key} ↜ "),
        Some(LinkKeyBinding::AtTheSame(key)) => format!("{key} ↭ "),
        Some(LinkKeyBinding::Along(key)) => format!("{key} ↷ "),
        None => "".to_string(),
    };
    let millis = if let ActionCondition::EveryMillis(millis) = condition {
        format!("⟳ {:.2}s / ", millis as f32 / 1000.0)
    } else {
        "".to_string()
    };
    let wait_before_secs = if wait_before_use_millis > 0 {
        Some(format!("⏱︎ {:.2}s", wait_before_use_millis as f32 / 1000.0))
    } else {
        None
    };
    let wait_after_secs = if wait_after_use_millis > 0 {
        Some(format!("⏱︎ {:.2}s", wait_after_use_millis as f32 / 1000.0))
    } else {
        None
    };
    let wait_secs = match (wait_before_secs, wait_after_secs) {
        (Some(before), None) => format!("{before} - ⏱︎ 0.00s"),
        (None, None) => "".to_string(),
        (None, Some(after)) => format!("⏱︎ 0.00s - {after} / "),
        (Some(before), Some(after)) => format!("{before} - {after} / "),
    };
    let with = match with {
        ActionKeyWith::Any => "Any",
        ActionKeyWith::Stationary => "Stationary",
        ActionKeyWith::DoubleJump => "Double jump",
    };

    rsx! {
        div { class: "grid grid-cols-[140px_100px_30px_auto] h-6 paragraph-xs !text-gray-400 group-hover:bg-gray-900 {linked_action}",
            div { class: "{ITEM_BORDER_CLASS} {ITEM_TEXT_CLASS}", "{queue_to_front}{position}" }
            div { class: "{ITEM_BORDER_CLASS} {ITEM_TEXT_CLASS}", "{link_key}{key} × {count}" }
            div { class: "{ITEM_BORDER_CLASS} {ITEM_TEXT_CLASS}",
                match direction {
                    ActionKeyDirection::Any => "⇆",
                    ActionKeyDirection::Left => "←",
                    ActionKeyDirection::Right => "→",
                }
            }
            div { class: "pr-13 {ITEM_TEXT_CLASS}", "{millis}{wait_secs}{with}" }
        }
    }
}

#[component]
fn ActionsSelect<T: 'static + Clone + PartialEq + Display + IntoEnumIterator>(
    label: &'static str,
    disabled: bool,
    on_select: EventHandler<T>,
    selected: T,
) -> Element {
    rsx! {
        EnumSelect {
            label,
            disabled,
            on_select,
            selected,
        }
    }
}

#[component]
fn ActionsNumberInputI32(
    label: &'static str,
    #[props(default = false)] disabled: bool,
    on_value: EventHandler<i32>,
    value: i32,
) -> Element {
    rsx! {
        NumberInputI32 {
            label,
            disabled,
            on_value,
            value,
        }
    }
}

#[component]
fn ActionsNumberInputU32(
    label: &'static str,
    #[props(default = false)] disabled: bool,
    on_value: EventHandler<u32>,
    value: u32,
) -> Element {
    rsx! {
        NumberInputU32 {
            label,
            minimum_value: 1,
            disabled,
            on_value,
            value,
        }
    }
}

#[component]
fn ActionsMillisInput(label: &'static str, on_value: EventHandler<u64>, value: u64) -> Element {
    rsx! {
        MillisInput { label, on_value, value }
    }
}

#[component]
fn ActionsCheckbox(
    label: &'static str,
    #[props(default = false)] disabled: bool,
    on_value: EventHandler<bool>,
    value: bool,
) -> Element {
    rsx! {
        Checkbox {
            label,
            input_class: "w-6",
            disabled,
            on_value,
            value,
        }
    }
}

#[component]
fn ActionsKeyBindingInput(
    label: &'static str,
    disabled: bool,
    on_value: EventHandler<Option<KeyBinding>>,
    value: Option<KeyBinding>,
) -> Element {
    rsx! {
        KeyBindingInput {
            label,
            input_class: "border border-gray-600",
            disabled,
            optional: false,
            on_value: move |value: Option<KeyBinding>| {
                on_value(value);
            },
            value,
        }
    }
}

/// Finds the linked action index range where `action_index` is a non-linked action.
fn find_linked_action_range(actions: &[Action], action_index: usize) -> Option<Range<usize>> {
    if action_index + 1 >= actions.len() {
        return None;
    }
    let start = action_index + 1;
    if !matches!(actions[start].condition(), ActionCondition::Linked) {
        return None;
    }

    let mut end = start + 1;
    while end < actions.len() {
        if !matches!(actions[end].condition(), ActionCondition::Linked) {
            break;
        }
        end += 1;
    }

    Some(start..end)
}

/// Finds the last linked action index of the last action matching `condition_filter`.
fn find_last_linked_action_index(
    actions: &[Action],
    condition_filter: ActionCondition,
) -> Option<usize> {
    let condition_filter = discriminant(&condition_filter);
    let (mut last_index, _) = actions
        .iter()
        .enumerate()
        .rev()
        .find(|(_, action)| condition_filter == discriminant(&action.condition()))?;

    if let Some(range) = find_linked_action_range(actions, last_index) {
        last_index += range.count();
    }

    Some(last_index)
}

/// Filters `actions` to find action with condition matching `condition_filter` including linked
/// action(s) of that matching action.
///
/// Returns a [`Vec<(Action, usize)>`] where [`usize`] is the index of the action inside the
/// original `actions`.
fn filter_actions(actions: Vec<Action>, condition_filter: ActionCondition) -> Vec<(Action, usize)> {
    let condition_filter = discriminant(&condition_filter);
    let mut filtered = Vec::with_capacity(actions.len());
    let mut i = 0;
    while i < actions.len() {
        let action = actions[i];
        if condition_filter != discriminant(&action.condition()) {
            i += 1;
            continue;
        }

        filtered.push((action, i));
        if let Some(range) = find_linked_action_range(&actions, i) {
            filtered.extend(actions[range.clone()].iter().copied().zip(range.clone()));
            i += range.count();
        }
        i += 1;
    }

    filtered
}
