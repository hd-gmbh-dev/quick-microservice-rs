//! # qm-cli
//!
//! This is a example cli that let's you configure or remove infrastructure components.
//!

use clap::Parser;

use commands::{Opts, SubCommand};
mod commands;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let opts: Opts = Opts::parse();
    match opts.subcmd {
        SubCommand::Configure(cmd) => cmd.run().await?,
        SubCommand::Remove(cmd) => cmd.run().await?,
    }
    Ok(())
}
