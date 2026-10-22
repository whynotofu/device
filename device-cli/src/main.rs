use crate::ipc::Ipc;
use anyhow::{Ok, Result};
use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{ColorChoice, Parser};
use device_common::Request;

mod ipc;

fn styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Green.on_default().effects(Effects::BOLD))
        .usage(AnsiColor::Green.on_default().effects(Effects::BOLD))
        .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
        .placeholder(AnsiColor::Cyan.on_default())
        .error(AnsiColor::Red.on_default().effects(Effects::BOLD))
        .valid(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
        .invalid(AnsiColor::Yellow.on_default().effects(Effects::BOLD))
}

#[derive(Parser)]
#[command(version, disable_help_subcommand = true, color = ColorChoice::Auto, styles = styles())]
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
