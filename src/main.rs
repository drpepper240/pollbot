mod commands;
mod utils;

use std::collections::HashMap;
use std::env;
use std::vec;

use serenity::all::ChannelId;
use serenity::all::ChannelType;
use serenity::all::CreateInteractionResponseFollowup;
use serenity::all::PartialChannel;
use serenity::all::User;
use serenity::all::UserId;
use serenity::all::GuildId;
use serenity::async_trait;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use serenity::model::channel::Reaction;
use serenity::utils::MessageBuilder;
use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::model::application::{Command, Interaction};


struct Handler;

static REACTION_P: char = '✅';
static REACTION_N: char = '❌';
static REACTION_T: char = '❔';

enum ReactionChangeType {
    ADD,
    REMOVE,
    REMOVEEMOJI,
}


#[async_trait]
impl EventHandler for Handler {
    // Set a handler for the `message` event. This is called whenever a new message is received.
    //
    // Event handlers are dispatched through a threadpool, and so multiple events can be
    // dispatched simultaneously.
    // async fn message(&self, ctx: Context, msg: Message) {
    //     if msg.content == "!ping" {
    //         println!("msg.author.id: {}", msg.author.id);
    //         println!("msg.channel_id: {}", msg.channel_id);
    //         // Sending a message can fail, due to a network error, an authentication error, or lack
    //         // of permissions to post in the channel, so log to stdout when some error happens,
    //         // with a description of it.
    //         if let Err(why) = msg.channel_id.say(&ctx.http, "```Pong!```").await {
    //             println!("Error sending message: {why:?}");
    //         }
    //     } else if msg.content == "!new" {
    //         match send_msg_with_reactions(&ctx, msg.channel_id, msg.author.id,).await
    //         {
    //             Ok(s) => println!("!new: {}", s),
    //             Err(e) => println!("!new error: {}", e),
    //         };
    //     }
    // }

    // reaction add handler
    async fn reaction_add(&self, ctx: Context, reaction: Reaction)
    {
        match utils::handle_reaction_change(ctx, reaction, ReactionChangeType::ADD).await {
            Ok(s) => println!("reaction_add: {}", s),
            Err(e) => println!("reaction_add error: {}", e),
        }

    }

    //reaction remove handler
    async fn reaction_remove(&self, ctx: Context, reaction: Reaction)
    {
        match utils::handle_reaction_change(ctx, reaction, ReactionChangeType::REMOVE).await {
            Ok(s) => println!("reaction_remove: {}", s),
            Err(e) => println!("reaction_remove error: {}", e),
        }
    }

    // async fn reaction_remove_all(&self, _ctx: Context, channel_id: ChannelId, removed_from_message_id: MessageId)
    // {
    //TODO
    // }

    async fn reaction_remove_emoji(&self, ctx: Context, reaction: Reaction)
    {
        match utils::handle_reaction_change(ctx, reaction, ReactionChangeType::REMOVEEMOJI).await {
            Ok(s) => println!("reaction_remove_emoji: {}", s),
            Err(e) => println!("reaction_remove_emoji error: {}", e),
        }
    }

    // Set a handler to be called on the `ready` event. This is called when a shard is booted, and
    // a READY payload is sent by Discord. This payload contains data like the current user's guild
    // Ids, current user data, private channels, and more.
    //
    // In this case, just print what the current user's username is.
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);

        //registering commands (guild-specific for now)
        // let guild_id = GuildId::from(330410844854943745);

        // let _commands = guild_id
        //     .set_commands(&ctx, vec![
        //         commands::new_poll::register(),
        //         commands::get_accepted::register(),
        //         commands::get_tentative::register(),
        //         commands::get_not_voted::register(),
        //         commands::get_not_in_voice::register(),
        //     ])
        //     .await;
        
        //println!("I now have the following guild slash commands: {commands:#?}");

        let g_commands = Command::set_global_commands(&ctx, vec![
                commands::new_poll::register(),
                commands::get_accepted::register(),
                commands::get_tentative::register(),
                commands::get_no_vote::register(),
                commands::get_not_in_voice::register(),
            ])
            .await;
        println!("I now have the following global slash commands: {g_commands:#?}");
    }

    async fn interaction_create(&self, ctx: Context, inter: Interaction) {
        if let Interaction::Command(cmd) = inter {
            //println!("Received command interaction: {cmd:#?}");

            if !utils::do_we_have_to_listen_to_this_guy(&ctx, &cmd).await {
                let resp_msg = CreateInteractionResponseMessage::new()
                .content("To use this command you have to be in the guild text channel while having at least one common role with the bot.")
                .ephemeral(true);
                let builder = CreateInteractionResponse::Message(resp_msg);
                if let Err(why) = cmd.create_response(&ctx.http, builder).await {
                    println!("Cannot respond to slash command: {why}");
                }
                return;
            }

            let d_msg = CreateInteractionResponseMessage::new()
            .content("Running command...")
            .ephemeral(true);
            let builder = CreateInteractionResponse::Defer(d_msg);
            if let Err(why) = cmd.create_response(&ctx.http, builder).await {
                println!("Cannot respond to slash command: {why}");
            }

            let mut response_str = "Default response".to_string();
            let mut response_eph = true;

            let g_id = match cmd.guild_id
            {
                Some(g_id) => g_id,
                None => {println!("No guild in the message"); return},
            };

            match cmd.data.name.as_str() {
                "new_poll" => {
                    response_str = match create_new_poll(&ctx, cmd.channel_id, &g_id, &cmd.user).await {
                        Ok(s) => format!("{}", s),
                        Err(e) => format!("'/new_poll' error: {}", e),
                    };
                },
                "get_not_voted" => {
                    if let Some(ch) = &cmd.channel {
                        if let Some(g_id) = &cmd.guild_id {
                            match mention_all_who_not_voted(&ctx, ch, g_id, &cmd.user).await {
                                Ok(s) => {response_str = s; response_eph = true},
                                Err(e) => {response_str = e.to_string(); response_eph = true},
                            }
                        }
                    };                  
                },
                "get_tentative" => {
                    if let Some(ch) = &cmd.channel {
                        if let Some(g_id) = &cmd.guild_id {
                            match mention_all_who_voted_emoji(&ctx, ch, g_id, REACTION_T, &cmd.user).await {
                                Ok(s) => {response_str = s; response_eph = true},
                                Err(e) => {response_str = e.to_string(); response_eph = true},
                            }
                        }
                    };                  
                },
                "get_accepted" => {
                    if let Some(ch) = &cmd.channel {
                        if let Some(g_id) = &cmd.guild_id {
                            match mention_all_who_voted_emoji(&ctx, ch, g_id, REACTION_P, &cmd.user).await {
                                Ok(s) => {response_str = s; response_eph = true},
                                Err(e) => {response_str = e.to_string(); response_eph = true},
                            }
                        }
                    };                  
                },
                "get_not_in_voice" => {
                    if let Some(ch) = &cmd.channel {
                        if let Some(g_id) = &cmd.guild_id {
                            match mention_all_who_voted_emoji_not_in_voice(&ctx, ch, g_id, REACTION_P, &cmd.user).await {
                                Ok(s) => {response_str = s; response_eph = true},
                                Err(e) => {response_str = e.to_string(); response_eph = true},
                            }
                        }
                    };
                },
                _ => response_str = "Not implemented :(".to_string(),
            };        

            let followup_msg = CreateInteractionResponseFollowup::new()
            .content(response_str)
            .ephemeral(response_eph);
            if let Err(why) = cmd.create_followup(&ctx.http, followup_msg).await {
                println!("Cannot respond to slash command: {why}");
            }

        }
    }
}


// creates new poll, returns a message that can be presented to the user requesting new poll
pub async fn create_new_poll(ctx: &Context, channel_id: ChannelId, g_id: &GuildId, u: &User) -> Result<String, serenity::Error>
{
    let g_ch = match g_id.to_guild_cached(&ctx).and_then(| g|g.channels.get(&channel_id).cloned()){
        Some(guild_channel) => guild_channel.to_owned(),
        None => {   // failed to get it from cache, trying API
            let ch = channel_id.to_channel(&ctx).await?;
            match ch.guild() {
                Some(guild_channel) => guild_channel,
                None => return Ok("Tried to create a poll not in a guild channel. Aborted.".to_string()),
            }
        },
    }; 

    //checking if we're in a proper channel
    // let ch = channel_id.to_channel(&ctx).await?; //TODO do this with cache as in 
    // let g_ch = match ch.guild() {
    //     Some(guild_channel) => guild_channel,
    //     None => return Ok("Tried to create a poll not in a guild channel. Aborted.".to_string()),
    // };
    if g_ch.kind != ChannelType::Text {
        return Ok("Tried to create a poll in a channel which is not a text channel. Aborted.".to_string());
    }
    
    //creating message
    let msg = channel_id.say(&ctx.http, "Creating new poll...").await?;

    //creating thread
    // let builder = serenity::builder::CreateThread::new(format!("log-{}", msg.id.to_string()))
    // .kind(ChannelType::PrivateThread);
    // let thr_id = channel_id.create_thread(&ctx, builder).await?;
    // let thr_msg_text = MessageBuilder::new()
    // .mention(&u)
    // .push(" created a poll.")
    // .build();
    // thr_id.say(&ctx.http, thr_msg_text).await?;

    let log_message = MessageBuilder::new()
    .mention(u)
    .push(" created a poll.")
    .build();
    match utils::log_to_thread(&ctx, &log_message, g_id, &channel_id, &msg.id.to_string()).await
    {
        Err(e) => println!("{e}"),
        _ => {},
    }
    
    //adding initial reactions sequentially
    msg.react(&ctx, crate::REACTION_P).await?;
    msg.react(&ctx, crate::REACTION_N).await?;
    msg.react(&ctx, crate::REACTION_T).await?;
    Ok("Successfully created a poll.".to_string())
}


// returns a string with non-menton mentions of all the people who left the selected emoji under the last vote post in the current
// or parent text channel
async fn mention_all_who_voted_emoji(ctx: &Context, pch: &PartialChannel, g_id: &GuildId, react: char, u: &User) 
-> Result<String, serenity::Error>
{
    if let Some (ch_id) = utils::find_suitable_channel( &pch){
        if let Some(msg) = utils::find_last_own_message(ctx, ch_id).await
        {
            let own_id = ctx.cache.current_user().id;
            let users_p = msg.reaction_users(&ctx, react, Some(100u8), None).await?;
            let mut names= String::from("");
            let mut mentions= String::from("");
            let mut cnt = 0;
            for u in &users_p {
                if u.id == own_id {continue;} //skipping own reactions
                names += MessageBuilder::new()
                .push_line_safe( match u.nick_in(&ctx, g_id).await
                {
                    Some(n) => n.to_string(),
                    None => u.display_name().to_string(),
                })
                .build().as_str();
                mentions += MessageBuilder::new()
                .mention(u)
                .build().as_str();
                cnt+=1;
            }
            let log_message = MessageBuilder::new()
                .mention(u)
                .push_line_safe(format!(" requested the list of all members who voted \"{react}\" ({cnt}):"))
                .push_line(match mentions.len() {0 => "".to_string(), _ => format!("{names}```{mentions}```"),})
                .build();
            utils::log_to_thread(&ctx, &log_message, g_id, &ch_id, &msg.id.to_string()).await?;
            if cnt > 0 {
                return Ok(format!("The following members selected \"{react}\" ({cnt}):\n{names}```{mentions}```").to_string());
            } else {
                return Ok(format!("Nobody selected \"{react}\".").to_string());
            }
        }
        return Ok("Unable to find any members.".to_string());
    } else {
        return Ok("Unable to find the poll.".to_string());
    }
}

async fn mention_all_who_not_voted(ctx: &Context, pch: &PartialChannel, g_id: &GuildId, u: &User) -> Result<String, serenity::Error>
{
    if let Some (ch_id) = utils::find_suitable_channel( &pch)
    {
        if let Some(msg) = utils::find_last_own_message(ctx, ch_id).await
        {
            if let Ok(ch_members) = utils::get_userids_from_channelid_cached(&ctx, &ch_id, g_id)
            {
                let own_id = ctx.cache.current_user().id;
                let (reacted_p, reacted_n, reacted_t) = tokio::join!(
                msg.reaction_users(&ctx, REACTION_P, Some(100u8), None),
                msg.reaction_users(&ctx, REACTION_N, Some(100u8), None),
                msg.reaction_users(&ctx, REACTION_T, Some(100u8), None),
                );
                let mut reacted: Vec<User>= vec![];
                reacted.extend(reacted_p?);
                reacted.extend(reacted_n?);
                reacted.extend(reacted_t?);
                let reacted_map: HashMap<UserId, User> = reacted
                .into_iter()
                .map(|user|(user.id, user))
                .collect();

                let mut names= String::from("");
                let mut mentions= String::from("");
                let mut cnt_not_v = 0;
                let mut cnt = 0;

                for m in ch_members{
                    if m.user.id == own_id { continue; }
                    if !reacted_map.contains_key(&m.user.id) {
                        names += MessageBuilder::new()
                        .push_line_safe( match &m.nick
                        {
                            Some(n) => n.to_string(),
                            None => m.display_name().to_string(),
                        })
                        .build().as_str();
                        mentions += MessageBuilder::new()
                        .mention(&m)
                        .build().as_str();
                        cnt_not_v+=1;
                    }
                    cnt+=1;
                }
                let log_message = MessageBuilder::new()
                .mention(u)
                .push_line_safe(format!(" requested the list of all members who have not voted yet ({cnt_not_v}/{cnt}):"))
                .push_line(match mentions.len() {0 => "".to_string(), _ => format!("{names}```{mentions}```"),})
                .build();
                utils::log_to_thread(&ctx, &log_message, g_id, &ch_id, &msg.id.to_string()).await?;
                if cnt_not_v > 0 
                {
                    return Ok(format!("The following members have not voted yet ({cnt_not_v}/{cnt}):\n{names}```{mentions}```").to_string());
                } else if cnt_not_v == 0 && cnt > 0 {
                    return Ok(format!("Everyone's voted ({cnt_not_v}/{cnt}) 👌"));
                }
                return Ok("Unable to find any members.".to_string());
            } else {
                return Ok("No cached members found in the channel.".to_string());
            }
        }
    }
    Ok("mention_all_who_not_voted()".to_string())
}


// returns a string with non-menton mentions of all the people who left the selected emoji under the last vote post in the current
// or parent text channel
async fn mention_all_who_voted_emoji_not_in_voice(ctx: &Context, pch: &PartialChannel, g_id: &GuildId, react: char, u: &User) 
    -> Result<String, serenity::Error>
{
    if let Some (ch_id) = utils::find_suitable_channel(&pch){
        if let Some(msg) = utils::find_last_own_message(ctx, ch_id).await
        {
            let own_id = ctx.cache.current_user().id;
            let users_p = msg.reaction_users(&ctx, react, Some(100u8), None).await?;
            let mut names= String::from("");
            let mut mentions= String::from("");
            let mut cnt = 0;
            let mut cnt_not_in_v = 0;
            let mut cnt_in_v = 0;
            let mut names_in_v= String::from("");


            //get users in voice
            let possibly_in_voice = utils::get_all_members_in_voice_cached(ctx, g_id);            

            for u in &users_p {
                if u.id == own_id {continue;} //skipping own reactions
                cnt+=1;
                let u_name = MessageBuilder::new()
                    .push_line_safe( match utils::nick_in_from_cache(&ctx, &u.id, g_id)
                    {
                        Some(n) => n,
                        None => u.display_name().to_string(),
                    })
                    .build();
                if let Some(in_voice) = &possibly_in_voice {
                    if in_voice.contains_key(&u.id) {
                        println!("Found {} in voice channel!", u.name);
                        names_in_v += u_name.as_str();
                        cnt_in_v += 1;
                        continue;
                    }
                }
                names += u_name.as_str();
                mentions += MessageBuilder::new()
                .mention(u)
                .build().as_str();
                cnt_not_in_v+=1;
            }
            let log_message = MessageBuilder::new()
            .mention(u)
            .push_line_safe(format!(" requested the list of all members who voted \"{react}\" but are not in voice ({cnt_not_in_v}/{cnt}):"))
            .push_line(match mentions.len() {0 => "".to_string(), _ => format!("{names}```{mentions}```"),})
            .push_line(format!("Present in voice channels right now ({cnt_in_v}/{cnt}):"))
            .push_line(names_in_v)
            .build();
            utils::log_to_thread(&ctx, &log_message, g_id, &ch_id, &msg.id.to_string()).await?;

            if cnt_not_in_v > 0 {
                return Ok(format!("The following members selected \"{react}\" and are not present in any of the voice channels right now ({cnt_not_in_v}/{cnt}):\n{names}```{mentions}```").to_string());
            } else if cnt > 0 && cnt_not_in_v == 0 {
                return Ok(format!("Everyone's in voice ({cnt_in_v}/{cnt}) 👌"));
            } else if cnt == 0 {
                return Ok(format!("Nobody selected \"{react}\".").to_string());
            }
        }
        return Ok("Unable to find any members.".to_string());
    } else {
        return Ok("Unable to find the poll.".to_string());
    }
}




#[tokio::main]
async fn main() {
    // Configure the client with your Discord bot token in the environment.
    let token = match env::var("DISCORD_TOKEN")
    {
        Ok(s) => s,
        Err(e) => {
            println!("Not found a DISCORD_TOKEN in the environment variables, trying the first argument...");
            match std::env::args().nth(1){
                Some(a) => a,
                None => panic!("Not found anything in the first argument, exiting. Have you forgotten to supply the token?"),
            }
        },
    };
    // Set gateway intents, which decides what events the bot will be notified about
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::DIRECT_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::DIRECT_MESSAGE_REACTIONS
        | GatewayIntents::GUILD_MESSAGE_REACTIONS
        | GatewayIntents::GUILD_PRESENCES           // needed for user caching to work
        // | GatewayIntents::GUILD_MEMBERS  
        | GatewayIntents::GUILD_VOICE_STATES        // needed for voice channel presence
        | GatewayIntents::GUILDS;                   // needed for voice channel presence

    // Create a new instance of the Client, logging in as a bot. This will automatically prepend
    // your bot token with "Bot ", which is a requirement by Discord for bot users.
    let mut client =
        Client::builder(&token, intents).event_handler(Handler).await.expect("Err creating client");

    // Finally, start a single shard, and start listening to events.
    //
    // Shards will automatically attempt to reconnect, and will perform exponential backoff until
    // it reconnects.
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}
