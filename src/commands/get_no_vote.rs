use serenity::builder::CreateCommand;

// pub fn run(_options: &[ResolvedOption]) -> String {
//     "get_not_voted.run".to_string()
// }

pub fn register() -> CreateCommand {
    CreateCommand::new("get_not_voted").description("Get the list of all users (mentionable) who have access to the channel, but haven't voted 👀")
}