use serenity::{all::{CommandInteraction, Context, GuildId}, builder::CreateCommand};

use crate::utils::{self, UserComparison};

pub async fn run(ctx: &Context, ci: &CommandInteraction, g_id: GuildId){
        utils::compare_channel_members_to_poll_and_respond(
            ctx, 
            ci, 
            g_id, 
            UserComparison::MembersSelectedOption, 
            Some(2),
            None)
        .await;
}

pub fn register() -> CreateCommand {
    CreateCommand::new("get_tentative")
    .description("Get the list of all users (mentionable) who selected \"❔\".")
    .description_localized("ru", "Получить список всех пользователей (для упоминания), кто выбрал \"❔\".")
}