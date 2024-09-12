// disable unused code warnings for dev builds
#![cfg_attr(debug_assertions, allow(dead_code, unused_imports, unused_variables))]

use std::{
    fs::File,
    io::{prelude::*, stdout, BufWriter},
    path::Path,
};

use anyhow::Result;
use clap::Parser;

#[macro_use]
extern crate log;
extern crate env_logger;

mod call;
mod duplicates;
mod generate_index;
mod cli;
use cli::{Cli, Commands};
// mod ordered_rayon;

fn get_writer(output: &Option<String>) -> Result<impl Write> {
    // get output as a BufWriter - equal to stdout if None
    let writer = BufWriter::new(match output {
        Some(ref x) => {
            let file = File::create(Path::new(x))?;
            Box::new(file) as Box<dyn Write + Send>
        }
        None => Box::new(stdout()) as Box<dyn Write + Send>,
    });
    Ok(writer)
}

fn try_main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_target(false)
        .init();

    let cli = Cli::parse();

    println!("nailpolish v{}", cli::VERSION);

    match &cli.command {
        Commands::Summary { index } => {
            info!("Summarising index at {index}");
            let (_, statistics) = duplicates::get_duplicates(index)?;

            println!(
                "{}",
                serde_json::to_string_pretty(&statistics).expect("Should be serialisable")
            );
        }
        Commands::GenerateIndex { file, index } => {
            generate_index::construct_index(file, index);
            info!("Completed index generation to {index}");
        }
        Commands::Call {
            index,
            input,
            output,
            threads,
            duplicates_only,
            report_original_reads,
        } => {
            info!("Collecting duplicates... {}", duplicates_only);
            let (duplicates, _statistics) =
                duplicates::get_duplicates(index).expect("Could not parse index.");
            info!("Iterating through individual duplicates");

            let mut writer = get_writer(output)?;

            call::consensus(
                input,
                &mut writer,
                duplicates,
                *threads,
                *duplicates_only,
                *report_original_reads,
            )?;

            info!("Completed successfully.")
        }
        Commands::Group {
            index,
            input,
            output,
            threads,
            shell,
            command,
        } => {
            let command_str = command.join(" ");
            info!(
                "Executing `{}` for every group using {}",
                command_str, shell
            );
            info!(
                "Multithreading is {}",
                if *threads != 1 { "enabled" } else { "disabled" }
            );

            info!("Collecting duplicates...");
            let (duplicates, _statistics) =
                duplicates::get_duplicates(index).expect("Could not parse index.");
            info!("Iterating through individual duplicates");

            let mut writer = get_writer(output)?;

            call::custom_command(input, &mut writer, duplicates, *threads, shell, &command_str)?;
        }
    };
    Ok(())
}

fn main() {
    if let Err(err) = try_main() {
        error!("{}", err);
        err.chain()
            .skip(1)
            .for_each(|cause| error!("  because: {}", cause));
    }
}
