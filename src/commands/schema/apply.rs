use super::{generate_plan, git_commit, git_dirty, Args, CommandExecutor};
use crate::{utils::load_config, DatabaseRepo};
use clap_utils::{
    dialoguer::{theme::ColorfulTheme, Confirm},
    prelude::*,
};

#[derive(Parser, Debug, Clone)]
pub struct SchemaApplyCommand {
    #[clap(long, value_parser, default_value = "false")]
    remote: bool,
}

#[async_trait]
impl CommandExecutor for SchemaApplyCommand {
    async fn execute(&self, _args: &Args) -> Result<(), Error> {
        let plan = generate_plan(self.remote).await?;
        if plan.is_empty() {
            return Ok(());
        }
        let config = load_config().await?;
        let db_repo = DatabaseRepo::new(&config);

        if git_dirty()? && !confirm("\nYour repo is dirty. Do you want to continue?") {
            bail!("Your repo is dirty. Please commit the changes before applying.");
        }

        if confirm("Do you want to perform this update?") {
            db_repo.apply(plan, self.remote).await?;
            git_commit("automatically retrieved most recent schema from remote server")?;
            println!(
                "Successfully applied migration to {}.\nYour repo is updated with the latest schema. See `git diff HEAD~1` for details.",
                config.url
            );
        } else {
            println!("Database schema update has been cancelled.");
        }

        Ok(())
    }
}

pub(crate) fn confirm(prompt: &'static str) -> bool {
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt(prompt)
        .interact()
        .expect("confirm UI should work")
}
