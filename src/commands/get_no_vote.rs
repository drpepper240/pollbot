use serenity::{all::{CommandDataOptionValue, CommandInteraction, Context, CreateCommandOption, GuildId, Role}, builder::CreateCommand};

use crate::utils::{self, UserComparison};

pub async fn run(ctx: &Context, ci: &CommandInteraction, g_id: GuildId){
    //TODO parse option
    let mut role = None;
    if ci.data.options.len() > 0 
        && ci.data.options[0].name == "role"{
            match ci.data.options[0].value {
                CommandDataOptionValue::Role(roleid) => {
                    role = ci.data.resolved.roles.get(&roleid);
                }
                _ =>{},
            }
        }

    utils::compare_channel_members_to_poll_and_respond(
        ctx, 
        ci, 
        g_id, 
        UserComparison::MembersNotSelectedOption, 
        None,
        role,
    ).await;
}

pub fn register() -> CreateCommand {
    let option = CreateCommandOption::new(
        serenity::all::CommandOptionType::Role,
        "role",
        "Role to narrow the members down to (optional)")
        .description_localized("ru", "Ограничить список пользователей конкретной ролью (необязательно)")
        .required(false);
    CreateCommand::new("get_no_vote")
        .description("Get the list of all members (mentionable) who have access to the channel, but haven't voted 👀")
        .description_localized("ru", "Получить список всех пользователей, кто видит опрос, но не выбрал никакой вариант 👀.")
        .add_option(option)
}