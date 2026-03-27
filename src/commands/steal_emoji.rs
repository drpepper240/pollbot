use serenity::builder::CreateCommand;


pub fn register() -> CreateCommand {
    CreateCommand::new("steal_emoji").description("from another Discord server")
}