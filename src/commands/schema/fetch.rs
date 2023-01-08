use super::{confirm, Args, CommandExecutor};
use crate::{utils::load_config, RemoteRepo};
use clap_utils::prelude::*;

#[derive(Parser, Debug, Clone)]
pub struct SchemaFetchCommand {}

#[async_trait]
impl CommandExecutor for SchemaFetchCommand {
    async fn execute(&self, _args: &Args) -> Result<(), Error> {
        let config = load_config().await?;
        let repo = RemoteRepo::new(&config.url);

        if confirm("This will overwrite the local schema files. Continue?") {
            repo.fetch().await?;
        }
        Ok(())
    }
}
