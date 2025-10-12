use serde::{Deserialize, Serialize};

use crate::impl_identifiable;

#[derive(Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Localization {
    #[serde(skip_serializing, default)]
    pub id: Option<i64>,
    pub cash_shop_base64: Option<String>,
    pub change_channel_base64: Option<String>,
    pub timer_base64: Option<String>,
    pub popup_confirm_base64: Option<String>,
    pub popup_yes_base64: Option<String>,
    pub popup_next_base64: Option<String>,
    pub popup_end_chat_base64: Option<String>,
    pub popup_ok_new_base64: Option<String>,
    pub popup_ok_old_base64: Option<String>,
    pub popup_cancel_new_base64: Option<String>,
    pub popup_cancel_old_base64: Option<String>,
    pub familiar_level_button_base64: Option<String>,
    pub familiar_save_button_base64: Option<String>,
    pub familiar_setup_button_base64: Option<String>,
}

impl_identifiable!(Localization);
