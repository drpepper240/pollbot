use serenity::{all::{CommandInteraction, Context, CreateCommandOption, GuildId}, builder::CreateCommand};

use crate::utils::{self, UserComparison};

pub async fn run(ctx: &Context, ci: &CommandInteraction, g_id: GuildId){
    //TODO parse option
    utils::compare_channel_members_to_poll_and_respond(ctx, ci, g_id, UserComparison::MembersNotSelectedOption, None).await;
}

pub fn register() -> CreateCommand {
    let option = CreateCommandOption::new(
        serenity::all::CommandOptionType::Role,
        "role",
        "Role to narrow the members down to (optional)")
        .required(false);
    CreateCommand::new("get_no_vote")
        .description("Get the list of all members (mentionable) who have access to the channel, but haven't voted 👀")
        .description_localized("ru", "Получить список всех пользователей, кто видит опрос, но не выбрал никакой вариант 👀.")
        .add_option(option)
}