use std::path::PathBuf;
use log::error;
use clap::Parser;

mod pipeline;
use crate::pipeline::Pipeline;

/// unix pipelines made easy!
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Subargs,
}

#[derive(clap::Subcommand)]
enum Subargs {
    /// run pipelines from a plumber file
    Run {
        /// path to plumber file
        path: PathBuf,
    },
    /// execute a pipeline from a string input
    Exec {
        /// raw pipeline string
        pipeline: String,
        /// name to use for logging and metadata
        #[arg(short, long)]
        name: String,
    },
}

fn exec(name: String, pipeline: String) {
    if pipeline.trim().is_empty() {
        error!("tried to execute empty pipeline");
        return;
    }

    let pipeline = Pipeline::new(name, pipeline);
    pipeline.run();
}


fn run(path: PathBuf) {
    let pipeline = Pipeline::new_from_file(&path);
    pipeline.run();
}

fn main() {
    let args = Args::parse();
    env_logger::init();

    match &args.command {
        Subargs::Exec { pipeline, name  } => {
            exec(name.to_string(), pipeline.to_string());
        },
        Subargs::Run { path } => {
            run(path.into());
        }
    }
}
