use std::fs::{self};

use backend::{
    GameTemplate, Localization as LocalizationData, convert_image_to_base64, query_localization,
    query_template, upsert_localization,
};
use dioxus::prelude::*;
use futures_util::{StreamExt, future::OptionFuture};
use rand::distr::{Alphanumeric, SampleString};

use crate::{
    AppState,
    button::{Button, ButtonKind},
};

#[derive(Debug)]
enum LocalizationUpdate {
    Update(LocalizationData),
}

#[component]
pub fn Localization() -> Element {
    let mut localization = use_context::<AppState>().localization;
    let localization_view = use_memo(move || localization().unwrap_or_default());

    // Handles async operations for localization-related
    let coroutine = use_coroutine(
        move |mut rx: UnboundedReceiver<LocalizationUpdate>| async move {
            while let Some(message) = rx.next().await {
                match message {
                    LocalizationUpdate::Update(new_localization) => {
                        localization.set(Some(upsert_localization(new_localization).await));
                    }
                }
            }
        },
    );
    let save_localization = use_callback(move |new_localization: LocalizationData| {
        coroutine.send(LocalizationUpdate::Update(new_localization));
    });

    use_future(move || async move {
        if localization.peek().is_none() {
            localization.set(Some(query_localization().await));
        }
    });

    rsx! {
        div { class: "flex flex-col h-full overflow-y-auto scrollbar",
            SectionInfo {}
            SectionPopups { localization_view, save_localization }
            SectionFamiliars { localization_view, save_localization }
            SectionOthers { localization_view, save_localization }
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
fn SectionInfo() -> Element {
    #[component]
    fn Header(title: &'static str) -> Element {
        rsx! {
            th { class: "title-xs text-left border-b border-gray-600", {title} }
        }
    }

    #[component]
    fn Data(description: &'static str) -> Element {
        rsx! {
            td { class: "label border-b border-gray-700 text-xs pt-2", {description} }
        }
    }

    rsx! {
        Section { name: "Info",
            table { class: "table-fixed",
                thead {
                    tr {
                        Header { title: "Function" }
                        Header { title: "Template(s)" }
                    }
                }
                tbody {
                    tr {
                        Data { description: "Unstuck player through closing menu, popup, dialog, etc." }
                        Data { description: "All popups." }
                    }
                    tr {
                        Data { description: "Go to town confirmation and save familiars setup." }
                        Data { description: "Confirm popup." }
                    }
                    tr {
                        Data { description: "Respawn on player death." }
                        Data { description: "Ok (new) popup." }
                    }
                    tr {
                        Data { description: "Sort familiar cards by level before swapping." }
                        Data { description: "Familiar menu setup tab's setup level sort button." }
                    }
                    tr {
                        Data { description: "Save familiars setup after swapping." }
                        Data { description: "Familiar menu setup tab's save button." }
                    }
                    tr {
                        Data { description: "Open setup tab in familiar menu." }
                        Data { description: "Familiar menu's setup button." }
                    }
                    tr {
                        Data { description: "Detect whether change channel menu is opened." }
                        Data { description: "Change channel text." }
                    }
                    tr {
                        Data { description: "Detect whether player entered cash shop." }
                        Data { description: "Cash shop text." }
                    }
                    tr {
                        Data { description: "Detect whether VIP/HEXA booster is in use." }
                        Data { description: "Timer text." }
                    }
                }
            }
        }
    }
}

#[component]
fn SectionPopups(
    localization_view: Memo<LocalizationData>,
    save_localization: EventHandler<LocalizationData>,
) -> Element {
    rsx! {
        Section { name: "Popups",
            div { class: "grid grid-cols-2  gap-4",
                LocalizationTemplateInput {
                    label: "Confirm",
                    template: GameTemplate::PopupConfirm,
                    on_value: move |image: Option<Vec<u8>>| async move {
                        save_localization(LocalizationData {
                            popup_confirm_base64: to_base64(image, true).await,
                            ..localization_view()
                        });
                    },
                    value: localization_view().popup_confirm_base64,
                }
                LocalizationTemplateInput {
                    label: "Yes",
                    template: GameTemplate::PopupYes,
                    on_value: move |image: Option<Vec<u8>>| async move {
                        save_localization(LocalizationData {
                            popup_yes_base64: to_base64(image, true).await,
                            ..localization_view()
                        });
                    },
                    value: localization_view().popup_yes_base64,
                }
                LocalizationTemplateInput {
                    label: "Next",
                    template: GameTemplate::PopupNext,
                    on_value: move |image: Option<Vec<u8>>| async move {
                        save_localization(LocalizationData {
                            popup_next_base64: to_base64(image, true).await,
                            ..localization_view()
                        });
                    },
                    value: localization_view().popup_next_base64,
                }
                LocalizationTemplateInput {
                    label: "End chat",
                    template: GameTemplate::PopupEndChat,
                    on_value: move |image: Option<Vec<u8>>| async move {
                        save_localization(LocalizationData {
                            popup_end_chat_base64: to_base64(image, true).await,
                            ..localization_view()
                        });
                    },
                    value: localization_view().popup_end_chat_base64,
                }
                LocalizationTemplateInput {
                    label: "Ok (new)",
                    template: GameTemplate::PopupOkNew,
                    on_value: move |image: Option<Vec<u8>>| async move {
                        save_localization(LocalizationData {
                            popup_ok_new_base64: to_base64(image, true).await,
                            ..localization_view()
                        });
                    },
                    value: localization_view().popup_ok_new_base64,
                }
                LocalizationTemplateInput {
                    label: "Ok (old)",
                    template: GameTemplate::PopupOkOld,
                    on_value: move |image: Option<Vec<u8>>| async move {
                        save_localization(LocalizationData {
                            popup_ok_old_base64: to_base64(image, true).await,
                            ..localization_view()
                        });
                    },
                    value: localization_view().popup_ok_old_base64,
                }
                LocalizationTemplateInput {
                    label: "Cancel (new)",
                    template: GameTemplate::PopupCancelNew,
                    on_value: move |image: Option<Vec<u8>>| async move {
                        save_localization(LocalizationData {
                            popup_cancel_new_base64: to_base64(image, true).await,
                            ..localization_view()
                        });
                    },
                    value: localization_view().popup_cancel_new_base64,
                }
                LocalizationTemplateInput {
                    label: "Cancel (old)",
                    template: GameTemplate::PopupCancelOld,
                    on_value: move |image: Option<Vec<u8>>| async move {
                        save_localization(LocalizationData {
                            popup_cancel_old_base64: to_base64(image, true).await,
                            ..localization_view()
                        });
                    },
                    value: localization_view().popup_cancel_old_base64,
                }
            }
        }
    }
}

#[component]
fn SectionFamiliars(
    localization_view: Memo<LocalizationData>,
    save_localization: EventHandler<LocalizationData>,
) -> Element {
    rsx! {
        Section { name: "Familiars",
            div { class: "grid grid-cols-2 gap-4",
                LocalizationTemplateInput {
                    label: "Level sort button",
                    template: GameTemplate::FamiliarsLevelSort,
                    on_value: move |image: Option<Vec<u8>>| async move {
                        save_localization(LocalizationData {
                            familiar_level_button_base64: to_base64(image, false).await,
                            ..localization_view()
                        });
                    },
                    value: localization_view().familiar_level_button_base64,
                }
                LocalizationTemplateInput {
                    label: "Save button",
                    template: GameTemplate::FamiliarsSaveButton,
                    on_value: move |image: Option<Vec<u8>>| async move {
                        save_localization(LocalizationData {
                            familiar_save_button_base64: to_base64(image, false).await,
                            ..localization_view()
                        });
                    },
                    value: localization_view().familiar_save_button_base64,
                }
                LocalizationTemplateInput {
                    label: "Setup button (unselected)",
                    template: GameTemplate::FamiliarsSetupButton,
                    on_value: move |image: Option<Vec<u8>>| async move {
                        save_localization(LocalizationData {
                            familiar_setup_button_base64: to_base64(image, false).await,
                            ..localization_view()
                        });
                    },
                    value: localization_view().familiar_setup_button_base64,
                }
            }
        }
    }
}

#[component]
fn SectionOthers(
    localization_view: Memo<LocalizationData>,
    save_localization: EventHandler<LocalizationData>,
) -> Element {
    rsx! {
        Section { name: "Others",
            div { class: "grid grid-cols-2 gap-4",
                LocalizationTemplateInput {
                    label: "Cash shop",
                    template: GameTemplate::CashShop,
                    on_value: move |image: Option<Vec<u8>>| async move {
                        save_localization(LocalizationData {
                            cash_shop_base64: to_base64(image, true).await,
                            ..localization_view()
                        });
                    },
                    value: localization_view().cash_shop_base64,
                }
                LocalizationTemplateInput {
                    label: "Change channel",
                    template: GameTemplate::ChangeChannel,
                    on_value: move |image: Option<Vec<u8>>| async move {
                        save_localization(LocalizationData {
                            change_channel_base64: to_base64(image, true).await,
                            ..localization_view()
                        });
                    },
                    value: localization_view().change_channel_base64,
                }
                LocalizationTemplateInput {
                    label: "Timer",
                    template: GameTemplate::Timer,
                    on_value: move |image: Option<Vec<u8>>| async move {
                        save_localization(LocalizationData {
                            timer_base64: to_base64(image, true).await,
                            ..localization_view()
                        });
                    },
                    value: localization_view().timer_base64,
                }
            }
        }
    }
}

#[component]
fn LocalizationTemplateInput(
    label: &'static str,
    template: GameTemplate,
    on_value: EventHandler<Option<Vec<u8>>>,
    value: Option<String>,
) -> Element {
    let id = use_memo(|| Alphanumeric.sample_string(&mut rand::rng(), 8));
    let select_file = use_callback(move |_| {
        let js = format!(
            r#"
            const element = document.getElementById("{}");
            if (element === null) {{
                return;
            }}
            element.click();
            "#,
            id()
        );
        document::eval(js.as_str());
    });
    let read_file = use_callback(move |file: String| {
        on_value(fs::read(file).ok());
    });
    let mut base64 = use_signal(String::default);

    use_effect(use_reactive!(|value| {
        if let Some(value) = value {
            base64.set(value);
        } else {
            spawn(async move {
                base64.set(query_template(template).await);
            });
        }
    }));

    rsx! {
        div { class: "flex gap-2",
            div { class: "flex-grow",
                div { class: "flex flex-col gap-1 w-full",
                    label { class: "label inline-block whitespace-nowrap overflow-hidden text-ellipsis",
                        {label}
                    }
                    div { class: "h-6 border-b border-gray-600 pb-0.5",
                        img {
                            src: format!("data:image/png;base64,{}", base64()),
                            class: "h-full",
                        }
                    }
                }
            }
            div { class: "flex items-end",
                Button {
                    label: "Reset",
                    class: "w-14",
                    kind: ButtonKind::Primary,
                    on_click: move |_| {
                        on_value(None);
                    },
                }
            }
            div { class: "flex items-end",
                input {
                    id: id(),
                    class: "w-0 h-0 invisible",
                    r#type: "file",
                    accept: ".png",
                    name: "Image",
                    onchange: move |e| {
                        if let Some(file) = e
                            .data
                            .files()
                            .and_then(|engine| engine.files().into_iter().next())
                        {
                            read_file(file);
                        }
                    },
                }
                Button {
                    label: "Replace",
                    class: "w-14",
                    kind: ButtonKind::Primary,
                    on_click: move |_| {
                        select_file(());
                    },
                }
            }
        }
    }
}

async fn to_base64(image: Option<Vec<u8>>, is_grayscale: bool) -> Option<String> {
    OptionFuture::from(image.map(|image| convert_image_to_base64(image, is_grayscale)))
        .await
        .flatten()
}
