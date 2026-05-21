//Things to interact with third-party voting/polling bots such as Apollo or Pancake

use std::collections::HashMap;

use serenity::{all::{Member, Message, UserId},};


const APOLLO_ADT: [&str; 3] = ["<:accepted:713124484436983971>", "<:declined:713124484688642068>", "<:tentative:713214962641666109>"];
const PANCAKE_ADT: [&str; 3] = ["✅", "❌", "❔"];


// Parses the embed assuming it is a poll from compatible third-party bot, 
// tries to extract the lists of names for "accepted", "declined" and "tentative" options. 
// Some or all vecs might be empty.
pub fn parse_tp_bot_poll(msg: &Message) -> Result<[Vec<String>; 3], String>
{
    fn trim_and_split_names_apollo(s: &str) -> Option<Vec<String>>{
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

    fn trim_and_split_names_pancake(s: &str) -> Option<Vec<String>>{
        let mut v: Vec<String>= Vec::new();
        let s = s.replace("\\\\", "\\"); //the string received from the JSON embed appears to be double-serialized for some reason
        let mut l_iter = s.lines();
        while let Some(l) = l_iter.next() {
            if let Some(l) = l.strip_prefix("> "){
                v.push(l.to_string());
            }
        }
        if !v.is_empty() {return Some(v)};
        return None;
    }

    let trim_and_split_names = match msg.author.id.get() {
        crate::BOT_ID_APOLLO => trim_and_split_names_apollo,
        crate::BOT_ID_PANCAKE => trim_and_split_names_pancake,
        _ => return Err("Unsupported 3rd party bot.".to_string())
    };

    let adt_symbols = match msg.author.id.get() {
        crate::BOT_ID_APOLLO => APOLLO_ADT,
        _ => PANCAKE_ADT, //pancake's emojis look default enough
    };

    let mut adt_strings: [String; 3] = Default::default();
    let mut has_poll = false;

    match msg.embeds.get(0) {
        None => {
            println!("No embeds found. Message content in question:"); 
            println!("{}", msg.content); 
            return Err("No embeds found in the last 3rd party bot message.".to_string());},
        Some (e) => {
            for f in e.fields.clone() {
                for i in 0..3 {
                    if f.name.starts_with(adt_symbols[i]) {
                        has_poll = true;
                        adt_strings[i] = f.value.clone();
                    }
                }
            }
        },
    };
    if !has_poll {
        return Err("No poll found in the last 3rd party bot message.".to_string());
    }
    let mut result: [Vec<String>; 3] = Default::default();

    for i in 0..3 {
        if let Some(trimmed_split) = trim_and_split_names(adt_strings[i].as_str()) {
            if !(trimmed_split.len() == 1 && trimmed_split[0] == "-") {
                result[i] = trimmed_split;
            }
        }
    }
    
    return Ok(result);
}


// Tries to find channel members from the supplied hashmap for every position in the 3-vector of voters' names 
// Returns 3-vector of IDs (where found) and message with user-presentable warnings (might be empty)
pub fn convert_names_to_ids(names: [Vec<String>; 3], channel_members: &HashMap<String, Member>) -> ([Vec<UserId>; 3], String) {
    let mut result: [Vec<UserId>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    let mut not_found = String::new();
    for i in 0..=2 {
        for n in &names[i] {
            if let Some(m) = channel_members.get(n) {
                result[i].push(m.user.id);
            } else {
                not_found = format!("{not_found}\n{n}");
            }
        }
    }
    if not_found.len() > 0 {
        not_found = format!("Not found among channel members (by name):{not_found}");
    }
    return (result, not_found);
}