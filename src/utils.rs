use serenity::all::CacheHttp;
use serenity::all::ChannelId;
use serenity::all::ChannelType;
use serenity::all::CommandInteraction;
use serenity::all::Context;
use serenity::all::EditMessage;
use serenity::all::GuildId;
use serenity::all::Member;
use serenity::all::Mentionable;
use serenity::all::Message;
use serenity::all::MessageBuilder;
use serenity::all::PartialChannel;
use serenity::all::Reaction;
use serenity::all::ReactionType;
use serenity::all::UserId;
use serenity::futures::StreamExt;

use crate::ReactionChangeType;


// Find an active guild thread by its parent_id
pub async fn find_thread_by_parent_id(ctx: &Context, guild_id: &GuildId, parent_id: &ChannelId) -> Option<ChannelId>
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


// Sends the log_message string to the log thread as a normal message. If there is no log thread, creates one attached to gch_id channel.
// Names the thread with a number (poll msg id by default)
pub async fn log_to_thread(ctx: &Context, log_message: &String, g_id: &GuildId, gch_id: &ChannelId, 
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


// given partial channel, either returns its id (if the channel kind is Text) or parent_id (if it's a text Thread)
// can be used to find a channel with the poll (suitable for the poll)
pub fn find_suitable_channel (pch: &PartialChannel) -> Option<ChannelId>
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
pub fn nick_in_from_cache(ctx: &Context, u_id: &UserId, g_id: &GuildId) -> Option<String>{
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


pub fn get_all_members_in_voice_cached(ctx: &Context,  g_id: &GuildId) -> Option<std::collections::HashMap<UserId, serenity::model::voice::VoiceState>>
{
    if let Some(g) = g_id.to_guild_cached(&ctx)
    {
        return Some(g.voice_states.clone());
    } else {
        println!("Can't get guild from cache.");
    }
    return None;
}


pub fn get_userids_from_channelid_cached(ctx: &Context, ch_id: &ChannelId, g_id: &GuildId) -> Result<Vec<Member>, serenity::Error>
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
pub async fn find_last_own_message(ctx: &Context, ch_id: ChannelId) -> Option<Message>
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
pub async fn do_we_have_to_listen_to_this_guy(ctx: &Context, command: &CommandInteraction) -> bool
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


// makes all the checks and decides whether or not to do anything on reaction add event 
pub async fn handle_reaction_change(ctx: Context, reaction: Reaction, change: ReactionChangeType) -> Result<String, serenity::Error>{
    
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
    if r_emoji != crate::REACTION_P.to_string()
        && r_emoji != crate::REACTION_N.to_string()
        && r_emoji != crate::REACTION_T.to_string() {
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
        create_text_for_reaction(ctx, &msg, crate::REACTION_P, "Accepted".to_string(), &own_id, g_id, u_id_added, added_reaction),
        create_text_for_reaction(ctx, &msg, crate::REACTION_N, "Declined".to_string(), &own_id, g_id, u_id_added, added_reaction),
        create_text_for_reaction(ctx, &msg, crate::REACTION_T, "Tentative".to_string(), &own_id, g_id, u_id_added, added_reaction),
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

