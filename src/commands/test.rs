use core::num;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::string;

use serenity::builder::CreateCommand;
use serenity::all::{CommandInteraction, Context, CreateCommandOption, CreateInteractionResponseFollowup, GuildId, Member, Message, MessageBuilder, User, UserId};

use crate::utils;

const SQUAD_ICONS:[&str; 8] = ["❤️", "💚", "💙", "💛", "🤍", "🖤",
 "💜", "🧡",];

pub async fn run(ctx: &Context, ci: &CommandInteraction, g_id: GuildId){
    // compare_channel_members_to_poll_and_respond(ctx, ci, g_id, UserComparison::MembersNotSelectedOption, Some(0)).await;
    get_4_squad_template(ctx, ci, g_id,Some(4)).await;
}


async fn get_4_squad_template(ctx: &Context, ci: &CommandInteraction, g_id: GuildId, num_squads: Option<usize>){
    let possibly_in_voice = utils::get_all_members_in_voice_cached(ctx, &g_id);
    let mut in_voice_uids = String::new();
    if let Some(in_voice) = &possibly_in_voice {
        for (uid, _state) in in_voice {
            in_voice_uids += format!("<@{}>\n", uid.to_string()).as_str();
        }
    }
    let num_squads: usize = match num_squads {
        Some(n) if (n > 1 && n <= 7) => n,
        _ => 4,
    };
    let mut reply = MessageBuilder::new();
    reply.push("```Лайнап:");
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
        .max_int_value(8)
        .required(false);
    CreateCommand::new("test").description("Get 4-squad template with all members currently in voice.").add_option(option)
}
