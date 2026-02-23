use serenity::builder::CreateCommand;
use serenity::model::application::ResolvedOption;

pub fn run(_options: &[ResolvedOption]) -> String {
    "get_not_voted.run".to_string()
}

pub fn register() -> CreateCommand {
    CreateCommand::new("get_tentative").description("Get the list of all users (mentionable) who selected ❔.")
}