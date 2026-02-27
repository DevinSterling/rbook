use clap::Parser;
use rbook::ebook::errors::EbookResult;
use rbook_cli::Cli;
use rbook_cli::command::Commands;

fn main() -> EbookResult<()> {
    let cli = Cli::parse();

    match cli.commands {
        Commands::Debug(debug) => debug.debug()?,
    }

    Ok(())
}
