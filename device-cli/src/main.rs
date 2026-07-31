use crate::ipc::Ipc;
use anyhow::{Ok, Result};
use clap::Parser;
use device_common::Request;

mod ipc;

#[derive(Parser)]
#[command(version, disable_help_subcommand = true)]
struct Args {
    #[command(subcommand)]
    request: Request,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut ipc = Ipc::new()?;

    if let Some(message) = ipc.request(args.request)? {
        println!("{}", message);
    }

    Ok(())
}
