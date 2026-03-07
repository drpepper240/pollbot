use serenity::builder::CreateCommand;

// pub fn run(_options: &[ResolvedOption]) -> String {
//     "get_not_voted.run".to_string()
// }

pub fn register() -> CreateCommand {
    CreateCommand::new("apollo_get_tentative").description("Get the list of all users who selected ❔ in Apollo's poll.")
}