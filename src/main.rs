use clap::Parser;

use vz::cli::{self, Cli};

fn main() {
    let mut cli = Cli::parse();
    vz::apply_output_shorthands(&mut cli);
    let json_errors = cli.output == Some(cli::OutputFormat::Json);
    if let Err(e) = vz::run(&cli, json_errors) {
        // `vz::run` exits on dispatch errors; this is a defensive fallback.
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}
