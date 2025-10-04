use tokio::sync::mpsc::Receiver;

use crate::{
    Settings,
    control::{BotCommand, DiscordBot},
};

#[derive(Debug)]
pub struct ControlService {
    bot: DiscordBot,
    bot_command_rx: Receiver<BotCommand>,
}

impl Default for ControlService {
    fn default() -> Self {
        let (bot, bot_command_receiver) = DiscordBot::new();
        Self {
            bot,
            bot_command_rx: bot_command_receiver,
        }
    }
}

impl ControlService {
    pub fn poll(&mut self) -> Option<BotCommand> {
        self.bot_command_rx.try_recv().ok()
    }

    pub fn update(&mut self, settings: &Settings) {
        if !settings.discord_bot_access_token.is_empty() {
            let _ = self.bot.start(settings.discord_bot_access_token.clone());
        }
    }
}
