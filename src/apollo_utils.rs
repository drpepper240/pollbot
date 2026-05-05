// Some features to work with polls made by Apollo-bot

use {
    std::collections::HashSet,
    serenity::all::Member,
    serenity::all::VoiceState,
    crate::utils::get_members_from_channelid_cached,
};

use std::collections::HashMap;
use serenity::all::UserId;
use serenity::utils::MessageBuilder;
use serenity::all::ChannelId;
use serenity::all::GuildId;
use serenity::all::Context;
use serenity::all::Message;
use crate::utils;
use serenity::futures::StreamExt;


static APOLLO_A: &str = "<:accepted:713124484436983971>";
static APOLLO_D: &str = "<:declined:713124484688642068>";
static APOLLO_T: &str = "<:tentative:713214962641666109>";

static MSG_LEN_LIMIT: usize = 1996;

static APOLLO_ICONS: [char; 3] = [crate::REACTION_A, crate::REACTION_D, crate::REACTION_T];


// a function to pass to compare_apollo_to_channel_members
// returns a formatted message listing every member who selected the specific poll option in Apollo's poll (both names and mention-ready code),
// with an additional list of names who selected the option but weren't found among the members
pub fn compare_voted_to_members_internal(
    poll_results: [Vec<String>; 3], //poll results for all three opts
    members: HashMap<String, Member>, //members from the channel
    a_i: usize, //apollo option index we want
    _possibly_in_voice: Option<HashMap<UserId, VoiceState>>) 
    -> String   //message that can be shown to the command user
{
    let mut voted_but_not_in_channel = String::from("");
    let mut voted_but_not_in_channel_descr: String = String::from("");
    let mut voted_names = String::from("");
    let mut voted_mentions = String::from("");
    for voted_n in poll_results[a_i].clone() {
        if let Some(m) = members.get(voted_n.as_str()){
            voted_names += MessageBuilder::new()
            .push_line_safe(format!("{} ({})", m.user.name, m.display_name()))
            .build().as_str();
            voted_mentions += MessageBuilder::new()
            .mention(m)
            .build().as_str();
        } else {
            voted_but_not_in_channel += voted_n.as_str();
        }
    }

    if voted_mentions.len() == 0 {return format!("Nobody selected {}.", APOLLO_ICONS[a_i])}
    if voted_but_not_in_channel.len() > 0 {
        voted_but_not_in_channel_descr = format!("Unable to find these members. Could they have changed their display name?\n");
    }

    return truncate_response_message([
        format!("These members selected {}:\n", APOLLO_ICONS[a_i]),
        voted_names,
        "```".to_string(),
        voted_mentions,
        "```".to_string(),
        voted_but_not_in_channel_descr.to_string(),
        voted_but_not_in_channel,
    ], MSG_LEN_LIMIT);
}



// a function to pass to compare_apollo_to_channel_members
// returns a formatted message listing every member not found among the voters of the Apollo's poll (both names and mention-ready code),
pub fn compare_non_vote_to_members_internal(
    poll_results: [Vec<String>; 3], //poll results for all three opts
    members: HashMap<String, Member>, //members from the channel
    _a_i: usize, //apollo option index we want
    _possibly_in_voice: Option<HashMap<UserId, VoiceState>>) 
    -> String   //message that can be shown to the command user
{
    let mut no_vote_names = String::from("");
    let mut no_vote_mentions = String::from("");
    let mut voters: HashSet<String> = HashSet::new();
    let mut nv_cnt = 0;

    for i in poll_results{
        for r in i {
            voters.insert(r); //TODO find a better way to do this
        }
    }
    
    for m in &members{
        if !voters.contains(m.0){
            no_vote_names += MessageBuilder::new()
            .push_line_safe(format!("{} ({})", m.1.user.name, m.1.display_name()))
            .build().as_str();
            no_vote_mentions += MessageBuilder::new()
            .mention(m.1)
            .build().as_str();
            nv_cnt +=1;
        }
    }
    if no_vote_mentions.len() == 0 {return format!("Everyone's participated 👌")}
    return truncate_response_message([
        format!("These members forgot to participate in the poll ({}/{}):\n", nv_cnt, members.len()),
        no_vote_names,
        "```".to_string(),
        no_vote_mentions,
        "```".to_string(),
        "".to_string(),
        "".to_string(),
    ], MSG_LEN_LIMIT);
}


// a function to pass to compare_apollo_to_channel_members
// returns a formatted message listing every member who selected the specific poll option in Apollo's poll (both names and mention-ready code),
// but was not found in the voice channels of the guild
// with an additional list of names who selected the option but weren't found among the members
pub fn compare_voted_to_in_voice_internal(
    poll_results: [Vec<String>; 3], //poll results for all three opts
    members: HashMap<String, Member>, //members from the channel
    a_i: usize, //apollo option index we want
    possibly_in_voice: Option<HashMap<UserId, VoiceState>>) 
    -> String   //message that can be shown to the command user
{
    let mut voted_but_not_in_channel = String::from("");
    let mut voted_but_not_in_channel_descr: String = String::from("");
    let mut niv_names = String::from("");
    let mut niv_mentions = String::from("");
    let mut v_cnt = 0;
    let mut niv_cnt = 0;
    for voted_n in poll_results[a_i].clone() {
        if let Some(m) = members.get(voted_n.as_str()){
           if let Some(in_voice) = &possibly_in_voice {
                if !in_voice.contains_key(&m.user.id) {
                    niv_names += MessageBuilder::new()
                    .push_line_safe(format!("{} ({})", m.user.name, m.display_name()))
                    .build().as_str();
                    niv_mentions += MessageBuilder::new()
                    .mention(m)
                    .build().as_str();   
                    niv_cnt +=1;
                }
            } // else nobody in voice
           v_cnt +=1;
        } else {
            voted_but_not_in_channel += voted_n.as_str();
        }
    }
    if voted_but_not_in_channel.len() > 0 {
        voted_but_not_in_channel_descr = format!("Unable to find these members. Could they have changed their display name?\n");
    }

    if niv_mentions.len() == 0 {
        return truncate_response_message([
        format!("Everyone's in the voice channels 👌\n"),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        "".to_string(),
        voted_but_not_in_channel_descr.to_string(),
        voted_but_not_in_channel,
    ], MSG_LEN_LIMIT);
    } else {
        return truncate_response_message([
        format!("These members selected {}, but were not found in the voice channels ({}/{}):\n", APOLLO_ICONS[a_i], niv_cnt, v_cnt),
        niv_names,
        "```".to_string(),
        niv_mentions,
        "```".to_string(),
        voted_but_not_in_channel_descr.to_string(),
        voted_but_not_in_channel,
    ], MSG_LEN_LIMIT);
    }
}


// prepares everything to run the supplied function which compares poll results to current channel members or members active in voice channels
// returns the string to be used as a reply from the bot
pub async fn compare_apollo_to_channel_members(
    ctx: &Context, 
    ch_id: &ChannelId, 
    g_id: &GuildId, 
    internal_fn: fn([Vec<String>; 3], HashMap<String, Member>, usize, Option<HashMap<UserId, VoiceState>>) -> String,
    apollo_option_index: usize) -> String
{
    // get list of channel members
    if let Ok (members_vec) = get_members_from_channelid_cached(ctx, ch_id, g_id)
    {
        // UserId to DisplayName for each member of the channel
        let mut members: HashMap<String, Member> = HashMap::new();
        for m in members_vec{
            // skip bots
            if m.user.id == UserId::from(475744554910351370u64) {continue;} //Apollo bot
            if m.user.id == UserId::from(235148962103951360u64) {continue;}  //Carl bot
            if m.user.id == UserId::from(310848622642069504u64) {continue;}  //Juniper bot
            if m.user.id == UserId::from(1470761370944405695u64) {continue;} //Pollbot //TODO get from cache //TODO cache somewhere closer
            members.insert(m.display_name().to_string(), m.clone()); //identical display names can be caught here
        }
        //get users in voice
        let possibly_in_voice = utils::get_all_members_in_voice_cached(ctx, g_id);
        // get list of people from the poll
        if let Some(poll_results) = get_and_parse_apollo_poll(&ctx, ch_id).await {
            // compare
            internal_fn(poll_results, members, apollo_option_index, possibly_in_voice) //TODO check index validity here
        } else {
            //TODO signal error
            return "Unable to find the poll.".to_string();
        }
    } else {
        //TODO signal error
        return "Unable to get members from the current channel.".to_string();
    }
}


// Finds and parses the last Apollo's poll, tries to extract the lists of names for "accepted", "declined" and "tentative"
// options. Some vecs can be empty. Returns None on failure.
pub async fn get_and_parse_apollo_poll(ctx: &Context, ch_id: &ChannelId)  -> Option<[Vec<String>; 3]>
{
    if let Some(msg) = find_last_message_apollo_with_embed(ctx, ch_id).await {
        fn trim_and_split_names(s: &str) -> Option<Vec<String>>{
            if let Some(s) = s.strip_prefix(">>> "){
                let mut v: Vec<String>= Vec::new();
                let s = s.replace("\\\\", "\\"); //the string received from the JSON embed appears to be double-serialized for some reason
                let mut l_iter = s.lines();
                while let Some(l) = l_iter.next() {
                    v.push(l.to_string());
                }
                if !v.is_empty() {return Some(v)};
            }
            return None;
        }

        let mut a_str = String::new();
        let mut d_str = String::new();
        let mut t_str = String::new();
        match msg.embeds.get(0) {
            None => {println!("No embeds found. Message content in question:"); println!("{}", msg.content); return None;},
            Some (e) => {
                for f in e.fields.clone() {
                    if f.name.starts_with(APOLLO_A) { //TODO use APOLLO_OPTIONS tuple instead
                        a_str = f.value;
                    } else if f.name.starts_with(APOLLO_D) {
                        d_str = f.value;
                    } else if f.name.starts_with(APOLLO_T) {
                        t_str = f.value;
                    }
                }
            },
        };
       
        let mut v_a: Vec<String>= Vec::new();
        let mut v_d: Vec<String>= Vec::new();
        let mut v_t: Vec<String>= Vec::new();
        if let Some(a) = trim_and_split_names(&a_str) {v_a = a;}
        if let Some(d) = trim_and_split_names(&d_str) {v_d = d;}
        if let Some(t) = trim_and_split_names(&t_str) {v_t = t;}
        // return Some((v_a, v_d, v_t));
        return Some([v_a, v_d, v_t]);
    } else {
        println!("Apollo's message was not found!");
    }
    return None;
}


// find the last Apollo's message in the channel with embed
pub async fn find_last_message_apollo_with_embed(ctx: &Context, ch_id: &ChannelId) -> Option<Message>
{
    let apollo_id = UserId::from(475744554910351370);
    let mut messages = ch_id.messages_iter(&ctx).boxed();
    while let Some(message_result) = messages.next().await {
        match message_result {
            Ok(msg) => if msg.author.id == apollo_id && msg.embeds.len() > 0 {return Some(msg)},
            Err(error) => {
                println!("Error getting last Apollo's message: {}", error);
                return  None;
            }
        }
    }
    return None;
}

// truncates or omits parts of the message, returns concatenated result
// pt0 response descr
// pt1 name list
// pt2 ```
// pt3 mentions
// pt4 ```
// pt5 not in channel (opt)
// pt6 not in channel name list (opt)
pub fn truncate_response_message(mut msg: [String; 7], limit: usize) -> String{
    let placeholder = "...too long.";

    fn msg_size(msg: &[String; 7]) -> usize{msg.iter().fold(0, |mut acc, item| {acc += item.len(); acc}) }
    fn msg_concat(msg: &[String; 7]) -> String{msg.iter().fold("".to_string(), |mut acc, item| {acc += item; acc}) }
    fn shorten_string_w_placeholder(text: &String, shorten_by: usize, placeholder: &str) -> String { // output string is at least placeholder.len() long
        text[..text.floor_char_boundary(text.len().saturating_sub(shorten_by + placeholder.len()))].to_owned() + placeholder }
    
    if msg_size(&msg) <= limit { return msg_concat(&msg) }
    msg[1] = shorten_string_w_placeholder(&msg[1], msg_size(&msg) - limit, placeholder);
    if msg_size(&msg) <= limit { return msg_concat(&msg) }
    msg[6] = shorten_string_w_placeholder(&msg[6], msg_size(&msg) - limit, placeholder);
    if msg_size(&msg) <= limit { return msg_concat(&msg) }
    msg[1] = "".to_string();
    msg[5] = "".to_string();
    msg[6] = "".to_string();
    if msg_size(&msg) <= limit { return msg_concat(&msg) }
    msg[3] = shorten_string_w_placeholder(&msg[3], msg_size(&msg) - limit, placeholder);
    if msg_size(&msg) <= limit { return msg_concat(&msg) }
    
    return "truncate_response_message limit is too small.".to_string();
}