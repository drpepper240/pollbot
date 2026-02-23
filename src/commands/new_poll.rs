use serenity::all::{Context, Interaction};
use serenity::builder::CreateCommand;
use serenity::model::application::ResolvedOption;

pub fn run(ctx: &Context, _options: &[ResolvedOption]) -> String {
    "new_poll.run".to_string()
}

pub fn register() -> CreateCommand {
    CreateCommand::new("new_poll").description("Create new poll")
}