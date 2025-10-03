use dioxus::prelude::*;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ButtonKind {
    Primary,
    Secondary,
    Danger,
}

#[component]
pub fn Button(
    label: String,
    kind: ButtonKind,
    on_click: EventHandler,
    #[props(default = false)] disabled: bool,
    #[props(default = String::default())] class: String,
) -> Element {
    let style = match kind {
        ButtonKind::Primary => "button-primary",
        ButtonKind::Secondary => "button-secondary",
        ButtonKind::Danger => "button-danger",
    };

    rsx! {
        button {
            class: "{style} h-6 {class}",
            disabled,
            onclick: move |e| {
                e.stop_propagation();
                on_click(());
            },
            {label}
        }
    }
}
