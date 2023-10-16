use clap::Parser;
use log::Log;

mod parser;
mod lexer;
mod text;
mod log;
mod cli;


fn main() {
    let args = cli::Args::parse();
    let log = Log::from_path(args.file.as_str());
    log.print_warnings_and_errors()
}
