mod commands;

use std::collections::HashMap;
use std::env;
use std::vec;

use serenity::all::ChannelId;
use serenity::all::ChannelType;
use serenity::all::CommandInteraction;
use serenity::all::CreateInteractionResponseFollowup;
use serenity::all::EditMessage;
use serenity::all::Member;
use serenity::all::PartialChannel;
use serenity::all::ReactionType;
use serenity::all::User;
use serenity::all::UserId;
use serenity::all::GuildId;
use serenity::async_trait;
use serenity::futures::StreamExt;
use serenity::model::channel::Message;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

use serenity::model;
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

// creates new poll, returns a message that can be presented to the user requesting new poll
async fn create_new_poll(ctx: &Context, channel_id: ChannelId, g_id: &GuildId, u: &User) -> Result<String, serenity::Error>
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
    match log_to_thread(&ctx, &log_message, g_id, &channel_id, &msg.id.to_string()).await
    {
        Err(e) => println!("{e}"),
        _ => {},
    }
    
    //adding initial reactions sequentially
    msg.react(&ctx, REACTION_P).await?;
    msg.react(&ctx, REACTION_N).await?;
    msg.react(&ctx, REACTION_T).await?;
    Ok("Successfully created a poll.".to_string())
}


// replaces the contents of the message with lists of users who reacted to this message with predefined reactions
// if supplied with both UserId and the reaction they added, removes the user from other reaction lists 
// and removes corresponding emoji reactions from the message
async fn edit_msg_with_reactions(ctx: &Context, mut msg: Message, g_id: &GuildId, u_id_added: Option<UserId>, 
    added_reaction: Option<char>) -> Result<String, serenity::Error> {
    
    use std::time::Instant;
    let now = Instant::now();

    //creates the following:
    // ✅ Accepted (14):
    // Nickname1
    // ServerNick2
    async fn create_text_for_reaction(ctx: &Context, msg: &Message, react: char, react_descr: String, own_id: &UserId, 
        g_id: &GuildId, u_id_reacted: Option<UserId>, added_reaction: Option<char>)
        -> Result<String, serenity::Error>
    {
        let users_p = msg.reaction_users(&ctx, react, Some(100u8), None).await?;
        let mut cnt = 0;
        let mut text= String::from("");
        for u in &users_p {
            if &u.id == own_id {continue;} //skipping own reactions
            if let Some(uidr) = u_id_reacted
            {
                if let Some(ar) = added_reaction {
                    if u.id == uidr && ar != react {
                        msg.delete_reaction(&ctx, u_id_reacted, react).await?;
                        continue;   //preemptively removing the user from other reactions lists
                    } 
                }                                               
            }
            text += MessageBuilder::new()
            .push_line( match nick_in_from_cache(&ctx, &u.id, g_id)
            {
                Some(n) => n.to_string(),
                None => u.display_name().to_string(),
            })
            .build().as_str();
            cnt+=1;
        }
        let cnt_str = {if cnt == 0 {"".to_string()} else {format!(" ({})", cnt)}};
            
        let header = MessageBuilder::new()
        .push(react)
        .push_bold_line_safe(format!(" __{}__{}:", react_descr, cnt_str))
        .build();

        Ok(format!("{header}{text}"))
    }
    let own_id = ctx.cache.current_user().id;

    let now1 = Instant::now();

    // concurrency
    let (text_a, text_d, text_t) = tokio::join!(
        create_text_for_reaction(ctx, &msg, REACTION_P, "Accepted".to_string(), &own_id, g_id, u_id_added, added_reaction),
        create_text_for_reaction(ctx, &msg, REACTION_N, "Declined".to_string(), &own_id, g_id, u_id_added, added_reaction),
        create_text_for_reaction(ctx, &msg, REACTION_T, "Tentative".to_string(), &own_id, g_id, u_id_added, added_reaction),
    );

    let text_a = text_a?;
    let text_d = text_d?;
    let text_t = text_t?;

    let fulltext = MessageBuilder::new()
    .push_line("_ _")
    .push_line(text_a)
    .push_line(text_d)
    .push_line(text_t)
    .push_line("_ _")
    .build();

    let elapsed1 = now1.elapsed();
    println!("edit_msg_with_reactions() - a-d-t: {:.2?}", elapsed1);
    
    // replace message contents
    let builder = EditMessage::new().content(fulltext);
    msg.edit(&ctx, builder).await?;

    let elapsed = now.elapsed();
    println!("edit_msg_with_reactions(): {:.2?}", elapsed);

    Ok("ok".to_string())
}


// Find an active guild thread by its parent_id
async fn find_thread_by_parent_id(ctx: &Context, guild_id: &GuildId, parent_id: &ChannelId) -> Option<ChannelId>
{
    let td = guild_id.get_active_threads(&ctx).await.ok()?;
    println!("Found {} active threads", td.threads.len());
    for t in td.threads {
        if let Some(tpid) = t.parent_id
        {
            if &tpid == parent_id {
                println!("Found {}", tpid.to_string());
                return Some(t.id);
            }
        }
    }
    return None;
}

// makes all the checks and decides whether or not to do anything on reaction add event 
async fn handle_reaction_change(ctx: Context, reaction: Reaction, change: ReactionChangeType) -> Result<String, serenity::Error>{
    
    use std::time::Instant;
    let now = Instant::now();
    
    // get message that was reacted to
    let msg = reaction.message(&ctx).await?;
    let msgidstring = msg.id.to_string();

    // ignoring custom reactions
    let r_emoji = match reaction.emoji {
        ReactionType::Unicode(s) => s,
        _ => return Ok("custom reaction".to_string()),
    };

    // ignoring other reactions
    if r_emoji != REACTION_P.to_string()
        && r_emoji != REACTION_N.to_string()
        && r_emoji != REACTION_T.to_string() {
            return Ok("ignored reaction".to_string())
        }
 
    // return if the bot is not the author
    if msg.author.id != ctx.cache.current_user().id { return Ok("Reacted on someone else's message".to_string()) }

    // get GuildId from reaction (faster)
    let g_id = match reaction.guild_id {
        None => {return Ok("Nothing in reaction.guild_id".to_string());},
        Some(g_id) => g_id,
    };

    let u_id_added = match change {
        ReactionChangeType::ADD => reaction.user_id,
        _ => None,        
    };

    let r_emoji_char = r_emoji.chars().next();

    println!("edit_msg_with_reactions: {}", edit_msg_with_reactions(&ctx, msg, &g_id, u_id_added, r_emoji_char).await?); //TODO run concurrently with the rest of this fn
    
        
    // name the user that reacted
    let user_string = match reaction.user_id {
        Some(r_user_id) => {
            let r_u = r_user_id.to_user(&ctx).await?; //uses the cache first
            // get their nickname 
            let r_name = (r_u.display_name()).to_string();
            // try for server-specific
            let r_g_name = match nick_in_from_cache(&ctx, &r_user_id, &g_id)
            {
                Some(n) => format!(" ({n})"),
                None => "".to_string(),
            };
            let u_mention = r_user_id.mention();
            format!("{r_name}{r_g_name} `{u_mention}`").to_string()
        },
        None => "Someone (no user_id)".to_string(),
    };

    let log_message = match change {
        ReactionChangeType::ADD => format!("{user_string} reacted with {r_emoji}"),
        ReactionChangeType::REMOVE => format!("{user_string} removed {r_emoji}"),
        ReactionChangeType::REMOVEEMOJI => format!("{user_string} removed emoji {r_emoji}"),
        _ => format!("{user_string} did something else with {r_emoji}"),        
    };
    log_to_thread(&ctx, &log_message, &g_id, &reaction.channel_id, &msgidstring).await?;
    
    let elapsed = now.elapsed();
    println!("handle_reaction_change(): {:.2?}", elapsed);

    Ok(format!("{log_message}"))
}


// Sends the log_message string to the log thread as a normal message. If there is no log thread, creates one attached to gch_id channel.
// Names the thread with a number (poll msg id by default)
async fn log_to_thread(ctx: &Context, log_message: &String, g_id: &GuildId, gch_id: &ChannelId, 
    thread_number: &String) -> Result<String, serenity::Error>
{
    let t_id = match find_thread_by_parent_id(&ctx, g_id, &gch_id).await
    {
        Some(t_id) => t_id,
        None => {
            //creating thread
            let builder = serenity::builder::CreateThread::new(format!("log-{}", thread_number))
            .kind(ChannelType::PrivateThread);
            let thr = gch_id.create_thread(&ctx, builder).await?;
            let thr_msg_text = MessageBuilder::new()
            .push("Can't find an existing log thread, created a new one.")
            .build();
            thr.say(&ctx.http, thr_msg_text).await?;
            thr.id
        },
    };
    if let Err(why) = t_id.say(&ctx.http, log_message).await {
            println!("Error sending message: {why:?}");
    }
    Ok("".to_string())
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
        match handle_reaction_change(ctx, reaction, ReactionChangeType::ADD).await {
            Ok(s) => println!("reaction_add: {}", s),
            Err(e) => println!("reaction_add error: {}", e),
        }

    }

    //reaction remove handler
    async fn reaction_remove(&self, ctx: Context, reaction: Reaction)
    {
        match handle_reaction_change(ctx, reaction, ReactionChangeType::REMOVE).await {
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
        match handle_reaction_change(ctx, reaction, ReactionChangeType::REMOVEEMOJI).await {
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
                commands::get_not_voted::register(),
                commands::get_not_in_voice::register(),
            ])
            .await;
        println!("I now have the following global slash commands: {g_commands:#?}");
    }

    async fn interaction_create(&self, ctx: Context, inter: Interaction) {
        if let Interaction::Command(cmd) = inter {
            //println!("Received command interaction: {cmd:#?}");

            if !do_we_have_to_listen_to_this_guy(&ctx, &cmd).await {
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


// given partial channel, either returns its id (if the channel kind is Text) or parent_id (if it's a text Thread)
// can be used to find a channel with the poll (suitable for the poll)
fn find_suitable_channel (pch: &PartialChannel) -> Option<ChannelId>
{
    Some(match pch.kind {
        ChannelType::Text => pch.id,
        ChannelType::PrivateThread | ChannelType::PublicThread => {
            match pch.parent_id {
                Some(id) => id,
                _ => return None,
            }
        },
        _ => return None,
    })
}

// returns guild-specific nickname for a user or none
fn nick_in_from_cache(ctx: &Context, u_id: &UserId, g_id: &GuildId) -> Option<String>{
    if let Some(cache) = ctx.cache() {
        if let Some(guild) = g_id.to_guild_cached(cache) {
            if let Some(member) = guild.members.get(u_id) {
                return member.nick.clone();
            }
        }
    }
    println!("No nickname in cache for {}", u_id);
    return None;
}


// returns a string with non-menton mentions of all the people who left the selected emoji under the last vote post in the current
// or parent text channel
async fn mention_all_who_voted_emoji(ctx: &Context, pch: &PartialChannel, g_id: &GuildId, react: char, u: &User) 
-> Result<String, serenity::Error>
{
    if let Some (ch_id) = find_suitable_channel( &pch){
        if let Some(msg) = find_last_own_message(ctx, ch_id).await
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
            log_to_thread(&ctx, &log_message, g_id, &ch_id, &msg.id.to_string()).await?;
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
    if let Some (ch_id) = find_suitable_channel( &pch)
    {
        if let Some(msg) = find_last_own_message(ctx, ch_id).await
        {
            if let Ok(ch_members) = get_userids_from_channelid_cached(&ctx, &ch_id, g_id)
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
                log_to_thread(&ctx, &log_message, g_id, &ch_id, &msg.id.to_string()).await?;
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
    if let Some (ch_id) = find_suitable_channel(&pch){
        if let Some(msg) = find_last_own_message(ctx, ch_id).await
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
            let possibly_in_voice = get_all_members_in_voice_cached(ctx, g_id);            

            for u in &users_p {
                if u.id == own_id {continue;} //skipping own reactions
                cnt+=1;
                let u_name = MessageBuilder::new()
                    .push_line_safe( match nick_in_from_cache(&ctx, &u.id, g_id)
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
            log_to_thread(&ctx, &log_message, g_id, &ch_id, &msg.id.to_string()).await?;

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


fn get_all_members_in_voice_cached(ctx: &Context,  g_id: &GuildId) -> Option<std::collections::HashMap<UserId, model::voice::VoiceState>>
{
    if let Some(g) = g_id.to_guild_cached(&ctx)
    {
        return Some(g.voice_states.clone());
    } else {
        println!("Can't get guild from cache.");
    }
    return None;
}


fn get_userids_from_channelid_cached(ctx: &Context, ch_id: &ChannelId, g_id: &GuildId) -> Result<Vec<Member>, serenity::Error>
{
    if let Some(g) = g_id.to_guild_cached(&ctx)
    {
        if let Some(g_ch) = g.channels.get(ch_id)
        {
            match g_ch.members(&ctx) {
                Ok(result) => return Ok(result),
                Err(e) => {println!("get_userids_from_partial_channel_cached error: {e}"); return Err(e)},
            }
        } else {
            println!("Can't get guild channel from cache.");
        }
    } else {
        println!("Can't get guild from cache.");
    }

    return Ok(Vec::new());
}


// find the last own message in the channel
async fn find_last_own_message(ctx: &Context, ch_id: ChannelId) -> Option<Message>
{
    let own_id = ctx.cache.current_user().id;
    let mut messages = ch_id.messages_iter(&ctx).boxed();
    while let Some(message_result) = messages.next().await {
        match message_result {
            Ok(msg) => if msg.author.id == own_id {return Some(msg)},
            Err(error) => {
                println!("Error getting last own message: {}", error);
                return  None;
            }
        }
    }
    return None;
}



// returns true if the user in question has at least one common role with ourselves in the guild
// returns false otherwise
async fn do_we_have_to_listen_to_this_guy(ctx: &Context, command: &CommandInteraction) -> bool
{
    //get own roles
    if let Some(member) = &command.member {
        let g_id = member.guild_id;
        let own_id = ctx.cache.current_user().id;
        // Get own roles in this guild
        match g_id.member(&ctx, own_id).await {
            Ok(own_member) => {
                // Check if there's any common role
                for role_id in &member.roles {
                    if own_member.roles.contains(role_id) {
                        return true;
                    }
                }
            },
            Err(e) => println!("Error fetching own member {e} within the guild {g_id}"),
        }
    }
    return false;
}

#[tokio::main]
async fn main() {
    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");
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
