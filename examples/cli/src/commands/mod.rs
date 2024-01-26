use clap::Parser;

mod configure;
mod remove;

#[derive(Clone, Parser)]
pub enum Resource {
    All,
    KeycloakRealm,
    Mongodb,
    S3,
}

#[derive(Parser)]
pub struct RemoveCommand {
    #[clap(long)]
    pub force: bool,
    #[clap(subcommand)]
    pub resource: Resource,
}

#[derive(Parser)]
pub struct ConfigureCommand {
    #[clap(long)]
    pub reset: bool,
    #[clap(subcommand)]
    pub resource: Resource,
}

#[derive(Parser)]
pub enum SubCommand {
    /// remove
    Remove(RemoveCommand),
    /// configure
    Configure(ConfigureCommand),
}

#[derive(Parser)]
#[clap(version, author = "Jürgen S. <juergen.seitz@h-d-gmbh.de>")]
pub struct Opts {
    #[clap(short, long)]
    pub quiet: bool,
    #[clap(subcommand)]
    pub subcmd: SubCommand,
}
