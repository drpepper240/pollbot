use serenity::builder::CreateCommand;

// pub fn run(_options: &[ResolvedOption]) -> String {
//     "get_not_in_voice.run".to_string()
// }

pub fn register() -> CreateCommand {
    CreateCommand::new("get_not_in_voice").description("Get the list of users who selected ✅ but are not present in any of the voice channels right now 🔇")
}