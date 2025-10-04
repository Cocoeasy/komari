use tokio::sync::mpsc::Receiver;

use crate::{
    Settings,
    bot::{BotCommand, DiscordBot},
};

#[derive(Debug)]
pub struct BotService {
    bot: DiscordBot,
    bot_command_receiver: Receiver<BotCommand>,
}

impl Default for BotService {
    fn default() -> Self {
        let (bot, bot_command_receiver) = DiscordBot::new();
        Self {
            bot,
            bot_command_receiver,
        }
    }
}

impl BotService {
    pub fn poll(&mut self) -> Option<BotCommand> {
        self.bot_command_receiver.try_recv().ok()
    }

    pub fn update(&mut self, settings: &Settings) {
        if !settings.discord_bot_access_token.is_empty() {
            let _ = self.bot.start(settings.discord_bot_access_token.clone());
        }
    }
}
