use clap::Parser;

/// Parser for latex log files
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub(crate) struct Args {
    /// Latex log file
    #[clap(index = 1)]
    pub(crate) file: String,
}
