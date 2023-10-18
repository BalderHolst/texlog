use clap::Parser;
use log::Log;

mod cli;
mod lexer;
mod log;
mod parser;
mod text;

fn main() {
    let args = cli::Args::parse();
    let log = Log::from_path(args.file.as_str());
    log.print_diagnostics()
}
