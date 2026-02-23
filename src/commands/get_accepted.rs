use serenity::builder::CreateCommand;

// pub fn run(_options: &[ResolvedOption]) -> String {
//     "get_accepted.run".to_string()
// }

pub fn register() -> CreateCommand {
    CreateCommand::new("get_accepted").description("Get a list of all users (mentionable) who selected ✅.")
}