use serenity::builder::CreateCommand;
use serenity::all::{CommandInteraction, Context, CreateCommandOption, GuildId, MessageBuilder,};

use crate::utils;

const SQUAD_ICONS:[&str; 8] = ["❤️", "💚", "💙", "💛", "🤍", "🖤",
 "💜", "🧡",];

pub async fn run(ctx: &Context, ci: &CommandInteraction, g_id: GuildId){
    let num_squads: Option<usize>;
    if ci.data.options.len() > 0 && ci.data.options[0].name == "number" { //TODO find a better way to validate this
        num_squads = ci.data.options[0].value.as_i64().map(|s| s as usize);
    } else {
        num_squads = None;
    }
    get_lineup_template(ctx, ci, g_id, num_squads).await;
}


async fn get_lineup_template(ctx: &Context, ci: &CommandInteraction, g_id: GuildId, num_squads: Option<usize>){
    let possibly_in_voice = utils::get_all_members_in_voice_cached(ctx, &g_id);
    let mut in_voice_uids = String::new();
    if let Some(in_voice) = &possibly_in_voice {
        for (uid, _state) in in_voice {
            in_voice_uids += format!("<@{}>\n", uid.to_string()).as_str();
        }
    }
    let num_squads: usize = match num_squads {
        Some(n) if (n >= 1 && n <= SQUAD_ICONS.len()) => n,
        _ => 4,
    };
    let mut reply = MessageBuilder::new();
    reply.push("```Lineup:\n");
    for i in 0..num_squads {
        reply.push(SQUAD_ICONS[i])
            .push("\n\n");
    }
    reply.push("\n\n")
        .push(in_voice_uids)
        .push("```");
    let t = reply.build();
    utils::send_ephemeral_followup(ctx, &t, ci).await;
}

pub fn register() -> CreateCommand {
    let option = CreateCommandOption::new(
        serenity::all::CommandOptionType::Integer,
        "number",
        "Number of color-coded squads in the template (1-8, default 4)")
        .min_int_value(1)
        .max_int_value(SQUAD_ICONS.len() as u64)
        .required(false)
        .name_localized("ru", "количество")
        .description_localized("ru", "Количество отрядов (1-8, по умолчанию 4)");
    CreateCommand::new("lineup")
        .description("Get lineup template with all members currently in voice channels 💙💚💛.")
        .description_localized("ru","Получить шаблон для расписывания по отрядам тех, кто сейчас в голосовых каналах 💙💚💛.")
        .add_option(option)
}
