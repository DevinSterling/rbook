use clap::Subcommand;

mod debug;

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Print the debugged contents of rbook::Epub.
    Debug(debug::DebugCommand),
}
