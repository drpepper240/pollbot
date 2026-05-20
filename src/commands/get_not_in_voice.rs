use serenity::{all::{CommandInteraction, Context, GuildId}, builder::CreateCommand};

use crate::utils::{self, UserComparison};

pub async fn run(ctx: &Context, ci: &CommandInteraction, g_id: GuildId){
        utils::compare_channel_members_to_poll_and_respond(ctx, ci, g_id, UserComparison::MembersSelectedOptionNotInVoice, Some(0)).await;
}

pub fn register() -> CreateCommand {
    CreateCommand::new("get_not_in_voice")
        .description("Get the list of users who selected \"✅\" but are not present in any of the voice channels right now 🔇.")
        .description_localized("ru", "Получить список всех пользователей, кто выбрал \"✅\", но отсутствует в голосовых каналах 🔇.")
}