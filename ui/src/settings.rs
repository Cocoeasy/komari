use std::{fmt::Display, fs::File, io::BufReader};

use backend::{
    CaptureMode, CycleRunStopMode, FamiliarRarity, Familiars, InputMethod, IntoEnumIterator,
    KeyBinding, KeyBindingConfiguration, Notifications, Settings as SettingsData,
    SwappableFamiliars, query_capture_handles, query_settings, refresh_capture_handles,
    select_capture_handle, upsert_settings,
};
use dioxus::prelude::*;
use futures_util::StreamExt;
use rand::distr::{Alphanumeric, SampleString};

use crate::{
    AppState,
    button::{Button, ButtonKind},
    icons::{EyePasswordHideIcon, EyePasswordShowIcon},
    inputs::{Checkbox, KeyBindingInput, MillisInput, TextInput},
    select::{EnumSelect, Select},
};

#[derive(Debug)]
enum SettingsUpdate {
    Update(SettingsData),
}

#[component]
pub fn Settings() -> Element {
    let mut settings = use_context::<AppState>().settings;
    let settings_view = use_memo(move || settings().unwrap_or_default());

    // Handles async operations for settings-related
    let coroutine = use_coroutine(
        move |mut rx: UnboundedReceiver<SettingsUpdate>| async move {
            while let Some(message) = rx.next().await {
                match message {
                    SettingsUpdate::Update(new_settings) => {
                        settings.set(Some(upsert_settings(new_settings).await));
                    }
                }
            }
        },
    );
    let save_settings = use_callback(move |new_settings: SettingsData| {
        coroutine.send(SettingsUpdate::Update(new_settings));
    });

    use_future(move || async move {
        if settings.peek().is_none() {
            settings.set(Some(query_settings().await));
        }
    });

    rsx! {
        div { class: "flex flex-col h-full overflow-y-auto scrollbar",
            SectionCapture { settings_view, save_settings }
            SectionInput { settings_view, save_settings }
            SectionFamiliars { settings_view, save_settings }
            SectionControlAndNotifications { settings_view, save_settings }
            SectionHotkeys { settings_view, save_settings }
            SectionRunStopCycle { settings_view, save_settings }
            SectionOthers { settings_view, save_settings }
        }
    }
}

#[component]
fn Section(name: &'static str, children: Element) -> Element {
    rsx! {
        div { class: "flex flex-col pr-4 pb-3",
            div { class: "flex items-center title-xs h-10", {name} }
            {children}
        }
    }
}

#[component]
fn SectionCapture(
    settings_view: Memo<SettingsData>,
    save_settings: EventHandler<SettingsData>,
) -> Element {
    let mut selected_handle_index = use_signal(|| None);
    let mut handle_names = use_resource(move || async move {
        let (names, selected) = query_capture_handles().await;
        selected_handle_index.set(selected);
        names
    });
    let handle_names_with_default = use_memo(move || {
        let default = vec!["默认".to_string()];
        let names = handle_names().unwrap_or_default();

        [default, names].concat()
    });

    rsx! {
        Section { name: "捕获",
            div { class: "grid grid-cols-2 gap-3",
                SettingsSelect {
                    label: "句柄",
                    options: handle_names_with_default(),
                    on_select: move |(index, _)| async move {
                        if index == 0 {
                            selected_handle_index.set(None);
                            select_capture_handle(None).await;
                        } else {
                            selected_handle_index.set(Some(index - 1));
                            select_capture_handle(Some(index - 1)).await;
                        }
                    },
                    selected: selected_handle_index().map(|index| index + 1).unwrap_or_default(),
                }
                SettingsEnumSelect::<CaptureMode> {
                    label: "模式",
                    on_select: move |capture_mode| {
                        save_settings(SettingsData {
                            capture_mode,
                            ..settings_view.peek().clone()
                        });
                    },
                    selected: settings_view().capture_mode,
                }
            }
            Button {
                label: "刷新句柄",
                kind: ButtonKind::Secondary,
                on_click: move |_| async move {
                    refresh_capture_handles().await;
                    handle_names.restart();
                },
                class: "mt-2",
            }
        }
    }
}

#[component]
fn SectionInput(
    settings_view: Memo<SettingsData>,
    save_settings: EventHandler<SettingsData>,
) -> Element {
    rsx! {
        Section { name: "输入",
            div { class: "grid grid-cols-3 gap-3",
                SettingsEnumSelect::<InputMethod> {
                    label: "方法",
                    on_select: move |input_method| async move {
                        save_settings(SettingsData {
                            input_method,
                            ..settings_view.peek().clone()
                        });
                    },
                    selected: settings_view().input_method,
                }
                SettingsTextInput {
                    text_label: "RPC服务器URL",
                    button_label: "更新",
                    on_value: move |input_method_rpc_server_url| {
                        save_settings(SettingsData {
                            input_method_rpc_server_url,
                            ..settings_view.peek().clone()
                        });
                    },
                    value: settings_view().input_method_rpc_server_url,
                }
            }
        }
    }
}

#[component]
fn SectionFamiliars(
    settings_view: Memo<SettingsData>,
    save_settings: EventHandler<SettingsData>,
) -> Element {
    let familiars_view = use_memo(move || settings_view().familiars);

    rsx! {
        Section { name: "宠物",
            SettingsCheckbox {
                label: "启用交换",
                on_value: move |enable_familiars_swapping| {
                    save_settings(SettingsData {
                        familiars: Familiars {
                            enable_familiars_swapping,
                            ..familiars_view.peek().clone()
                        },
                        ..settings_view.peek().clone()
                    });
                },
                value: familiars_view().enable_familiars_swapping,
            }
            div { class: "grid grid-cols-2 gap-3 mt-2",
                SettingsEnumSelect::<SwappableFamiliars> {
                    label: "可交换槽位",
                    disabled: !familiars_view().enable_familiars_swapping,
                    on_select: move |swappable_familiars| async move {
                        save_settings(SettingsData {
                            familiars: Familiars {
                                swappable_familiars,
                                ..familiars_view.peek().clone()
                            },
                            ..settings_view.peek().clone()
                        });
                    },
                    selected: familiars_view().swappable_familiars,
                }
                SettingsEnumSelect::<FamiliarRarity> {
                    label: "稀有度",
                    disabled: !familiars_view().enable_familiars_swapping,
                    on_select: move |familiar_rarity| async move {
                        let mut rarities = familiars_view().swappable_rarities.clone();
                        if rarities.contains(&familiar_rarity) {
                            rarities.remove(&familiar_rarity);
                        } else {
                            rarities.insert(familiar_rarity);
                        }
                        save_settings(SettingsData {
                            familiars: Familiars {
                                swappable_rarities: rarities,
                                ..familiars_view.peek().clone()
                            },
                            ..settings_view.peek().clone()
                        });
                    },
                    selected: familiars_view().swappable_rarities.iter().next().copied().unwrap_or_default(),
                }
            }
        }
    }
}

#[component]
fn SectionControlAndNotifications(
    settings_view: Memo<SettingsData>,
    save_settings: EventHandler<SettingsData>,
) -> Element {
    let notifications_view = use_memo(move || settings_view().notifications);

    rsx! {
        Section { name: "控制和通知",
            div { class: "grid grid-cols-2 gap-3 mb-2",
                SettingsTextInput {
                    text_label: "Discord机器人访问令牌",
                    button_label: "更新",
                    sensitive: true,
                    on_value: move |discord_bot_access_token| {
                        save_settings(SettingsData {
                            discord_bot_access_token,
                            ..settings_view.peek().clone()
                        });
                    },
                    value: settings_view().discord_bot_access_token,
                }
                SettingsTextInput {
                    text_label: "Discord Webhook URL",
                    button_label: "更新",
                    sensitive: true,
                    on_value: move |discord_webhook_url| {
                        save_settings(SettingsData {
                            notifications: Notifications {
                                discord_webhook_url,
                                ..notifications_view.peek().clone()
                            },
                            ..settings_view.peek().clone()
                        });
                    },
                    value: notifications_view().discord_webhook_url,
                }
                SettingsTextInput {
                    text_label: "Discord用户ID",
                    button_label: "更新",
                    on_value: move |discord_user_id| {
                        save_settings(SettingsData {
                            notifications: Notifications {
                                discord_user_id,
                                ..notifications_view.peek().clone()
                            },
                            ..settings_view.peek().clone()
                        });
                    },
                    value: notifications_view().discord_user_id,
                }
            }
            div { class: "grid grid-cols-3 gap-3",
                SettingsCheckbox {
                    label: "符文刷新",
                    on_value: move |notify_on_rune_appear| {
                        save_settings(SettingsData {
                            notifications: Notifications {
                                notify_on_rune_appear,
                                ..notifications_view.peek().clone()
                            },
                            ..settings_view.peek().clone()
                        });
                    },
                    value: notifications_view().notify_on_rune_appear,
                }
                SettingsCheckbox {
                    label: "精英BOSS刷新",
                    on_value: move |notify_on_elite_boss_appear| {
                        save_settings(SettingsData {
                            notifications: Notifications {
                                notify_on_elite_boss_appear,
                                ..notifications_view.peek().clone()
                            },
                            ..settings_view.peek().clone()
                        });
                    },
                    value: notifications_view().notify_on_elite_boss_appear,
                }
                SettingsCheckbox {
                    label: "玩家死亡",
                    on_value: move |notify_on_player_die| {
                        save_settings(SettingsData {
                            notifications: Notifications {
                                notify_on_player_die,
                                ..notifications_view.peek().clone()
                            },
                            ..settings_view.peek().clone()
                        });
                    },
                    value: notifications_view().notify_on_player_die,
                }
                SettingsCheckbox {
                    label: "公会成员出现",
                    on_value: move |notify_on_player_guildie_appear| {
                        save_settings(SettingsData {
                            notifications: Notifications {
                                notify_on_player_guildie_appear,
                                ..notifications_view.peek().clone()
                            },
                            ..settings_view.peek().clone()
                        });
                    },
                    value: notifications_view().notify_on_player_guildie_appear,
                }
                SettingsCheckbox {
                    label: "陌生人出现",
                    on_value: move |notify_on_player_stranger_appear| {
                        save_settings(SettingsData {
                            notifications: Notifications {
                                notify_on_player_stranger_appear,
                                ..notifications_view.peek().clone()
                            },
                            ..settings_view.peek().clone()
                        });
                    },
                    value: notifications_view().notify_on_player_stranger_appear,
                }
                SettingsCheckbox {
                    label: "好友出现",
                    on_value: move |notify_on_player_friend_appear| {
                        save_settings(SettingsData {
                            notifications: Notifications {
                                notify_on_player_friend_appear,
                                ..notifications_view.peek().clone()
                            },
                            ..settings_view.peek().clone()
                        });
                    },
                    value: notifications_view().notify_on_player_friend_appear,
                }
                SettingsCheckbox {
                    label: "检测失败或地图变更",
                    on_value: move |notify_on_fail_or_change_map| {
                        save_settings(SettingsData {
                            notifications: Notifications {
                                notify_on_fail_or_change_map,
                                ..notifications_view.peek().clone()
                            },
                            ..settings_view.peek().clone()
                        });
                    },
                    value: notifications_view().notify_on_fail_or_change_map,
                }
            }
        }
    }
}

#[component]
fn SectionHotkeys(
    settings_view: Memo<SettingsData>,
    save_settings: EventHandler<SettingsData>,
) -> Element {
    #[component]
    fn Hotkey(
        label: &'static str,
        on_value: EventHandler<KeyBindingConfiguration>,
        value: KeyBindingConfiguration,
    ) -> Element {
        rsx! {
            div { class: "flex gap-2",
                KeyBindingInput {
                    label,
                    div_class: "flex-grow",
                    on_value: move |new_value: Option<KeyBinding>| {
                        on_value(KeyBindingConfiguration {
                            key: new_value.expect("not optional"),
                            ..value
                        });
                    },
                    value: Some(value.key),
                }
                SettingsCheckbox {
                    label: "启用",
                    on_value: move |enabled| {
                        on_value(KeyBindingConfiguration {
                            enabled,
                            ..value
                        });
                    },
                    value: value.enabled,
                }
            }
        }
    }

    rsx! {
        Section { name: "热键",
            div { class: "grid grid-cols-2 gap-3",
                Hotkey {
                    label: "切换开始/停止操作",
                    on_value: move |toggle_actions_key| {
                        save_settings(SettingsData {
                            toggle_actions_key,
                            ..settings_view.peek().clone()
                        });
                    },
                    value: settings_view().toggle_actions_key,
                }
                Hotkey {
                    label: "添加平台",
                    on_value: move |platform_add_key| {
                        save_settings(SettingsData {
                            platform_add_key,
                            ..settings_view.peek().clone()
                        });
                    },
                    value: settings_view().platform_add_key,
                }
                Hotkey {
                    label: "标记平台起点",
                    on_value: move |platform_start_key| {
                        save_settings(SettingsData {
                            platform_start_key,
                            ..settings_view.peek().clone()
                        });
                    },
                    value: settings_view().platform_start_key,
                }
                Hotkey {
                    label: "标记平台终点",
                    on_value: move |platform_end_key| {
                        save_settings(SettingsData {
                            platform_end_key,
                            ..settings_view.peek().clone()
                        });
                    },
                    value: settings_view().platform_end_key,
                }
            }
        }
    }
}

#[component]
fn SectionRunStopCycle(
    settings_view: Memo<SettingsData>,
    save_settings: EventHandler<SettingsData>,
) -> Element {
    rsx! {
        Section { name: "运行/停止循环",
            div { class: "grid grid-cols-3 gap-3",
                MillisInput {
                    label: "运行时长",
                    on_value: move |cycle_run_duration_millis| {
                        save_settings(SettingsData {
                            cycle_run_duration_millis,
                            ..settings_view.peek().clone()
                        });
                    },
                    value: settings_view().cycle_run_duration_millis,
                }
                MillisInput {
                    label: "停止时长",
                    on_value: move |cycle_stop_duration_millis| {
                        save_settings(SettingsData {
                            cycle_stop_duration_millis,
                            ..settings_view.peek().clone()
                        });
                    },
                    value: settings_view().cycle_stop_duration_millis,
                }
                SettingsEnumSelect::<CycleRunStopMode> {
                    label: "模式",
                    on_select: move |cycle_run_stop| {
                        save_settings(SettingsData {
                            cycle_run_stop,
                            ..settings_view.peek().clone()
                        });
                    },
                    selected: settings_view().cycle_run_stop,
                }
            }
        }
    }
}

#[component]
fn SectionOthers(
    settings_view: Memo<SettingsData>,
    save_settings: EventHandler<SettingsData>,
) -> Element {
    let export_element_id = use_memo(|| Alphanumeric.sample_string(&mut rand::rng(), 8));
    let export = use_callback(move |_| {
        let js = format!(
            r#"
            const element = document.getElementById("{}");
            if (element === null) {{
                return;
            }}
            const json = await dioxus.recv();

            element.setAttribute("href", "data:application/json;charset=utf-8," + encodeURIComponent(json));
            element.setAttribute("download", "settings.json");
            element.click();
            "#,
            export_element_id(),
        );
        let eval = document::eval(js.as_str());
        let Ok(json) = serde_json::to_string_pretty(&*settings_view.peek()) else {
            return;
        };
        let _ = eval.send(json);
    });

    let import_element_id = use_memo(|| Alphanumeric.sample_string(&mut rand::rng(), 8));
    let import = use_callback(move |_| {
        let js = format!(
            r#"
            const element = document.getElementById("{}");
            if (element === null) {{
                return;
            }}
            element.click();
            "#,
            import_element_id()
        );
        document::eval(js.as_str());
    });
    let import_settings = use_callback(move |file| {
        let Some(id) = settings_view.peek().id else {
            return;
        };
        let Ok(file) = File::open(file) else {
            return;
        };
        let reader = BufReader::new(file);
        let Ok(mut settings) = serde_json::from_reader::<_, SettingsData>(reader) else {
            return;
        };
        settings.id = Some(id);
        save_settings(settings);
    });

    rsx! {
        Section { name: "其他",
            div { class: "grid grid-cols-2 gap-3",
                SettingsCheckbox {
                    label: "启用符文解析",
                    on_value: move |enable_rune_solving| {
                        save_settings(SettingsData {
                            enable_rune_solving,
                            ..settings_view.peek().clone()
                        });
                    },
                    value: settings_view().enable_rune_solving,
                }
                div {}
                SettingsCheckbox {
                    label: "检测失败或地图变更时停止操作",
                    on_value: move |stop_on_fail_or_change_map| {
                        save_settings(SettingsData {
                            stop_on_fail_or_change_map,
                            ..settings_view.peek().clone()
                        });
                    },
                    value: settings_view().stop_on_fail_or_change_map,
                }
                SettingsCheckbox {
                    label: "启用紧急模式",
                    on_value: move |enable_panic_mode| {
                        save_settings(SettingsData {
                            enable_panic_mode,
                            ..settings_view.peek().clone()
                        });
                    },
                    value: settings_view().enable_panic_mode,
                }
                div {
                    a { id: export_element_id(), class: "w-0 h-0 invisible" }
                    Button {
                        class: "w-full",
                        label: "导出",
                        kind: ButtonKind::Primary,
                        on_click: move |_| {
                            export(());
                        },
                    }
                }
                div {
                    input {
                        id: import_element_id(),
                        class: "w-0 h-0 invisible",
                        r#type: "file",
                        accept: ".json",
                        name: "设置JSON",
                        onchange: move |e| {
                            if let Some(file) = e
                                .data
                                .files()
                                .and_then(|engine| engine.files().into_iter().next())
                            {
                                import_settings(file);
                            }
                        },
                    }
                    Button {
                        class: "w-full",
                        label: "导入",
                        kind: ButtonKind::Primary,
                        on_click: move |_| {
                            import(());
                        },
                    }
                }
            }
        }
    }
}

#[component]
fn SettingsSelect<T: 'static + Clone + PartialEq + Display>(
    label: &'static str,
    options: Vec<T>,
    on_select: EventHandler<(usize, T)>,
    selected: usize,
) -> Element {
    rsx! {
        Select {
            label,
            options,
            on_select,
            selected,
        }
    }
}

#[component]
fn SettingsEnumSelect<T: 'static + Clone + PartialEq + Display + IntoEnumIterator>(
    label: &'static str,
    #[props(default = false)] disabled: bool,
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
fn SettingsCheckbox(
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
fn SettingsTextInput(
    text_label: String,
    button_label: String,
    #[props(default = false)] sensitive: bool,
    on_value: EventHandler<String>,
    value: String,
) -> Element {
    const EYE_ICON_CLASS: &str = "text-gray-50 w-[16px] h-[16px] fill-current";

    let mut text = use_signal(String::default);
    let mut hidden = use_signal(|| sensitive);

    use_effect(use_reactive!(|value| text.set(value)));

    rsx! {
        div { class: "relative group",
            TextInput {
                label: text_label,
                hidden: hidden(),
                on_value: move |new_text| {
                    text.set(new_text);
                },
                value: text(),
            }
            if sensitive {
                div {
                    class: "absolute right-1 bottom-1 invisible group-hover:visible bg-gray-950",
                    onclick: move |_| {
                        hidden.toggle();
                    },
                    if hidden() {
                        EyePasswordShowIcon { class: EYE_ICON_CLASS }
                    } else {
                        EyePasswordHideIcon { class: EYE_ICON_CLASS }
                    }
                }
            }
        }
        div { class: "flex items-end",
            Button {
                label: button_label,
                kind: ButtonKind::Primary,
                on_click: move |_| {
                    on_value(text.peek().clone());
                },
                class: "w-full",
            }
        }
    }
}
