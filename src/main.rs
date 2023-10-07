use std::io::{BufReader, BufRead};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::{fs, thread};
use rand::seq::IteratorRandom;

use signal_hook::consts::TERM_SIGNALS;
use clap::Parser;

mod pipeline;
use crate::pipeline::{Pipeline, PipelineInput};

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
        path: String,
        /// name to use for output metadata
        #[arg(short, long)]
        name: Option<String>,
    },
    /// execute a pipeline from a string input
    Exec {
        /// raw pipeline string
        pipeline: Vec<String>,
        /// name to use for output metadata
        #[arg(short, long)]
        name: Option<String>,
    },
}

fn register_shutdown() -> Arc<AtomicBool> {
    let shutdown = Arc::new(AtomicBool::new(false));

    for sig in TERM_SIGNALS {
        signal_hook::flag::register(*sig, Arc::clone(&shutdown))
            .expect("Failed to register shutdown");
    }

    shutdown
}

fn setup_directories(name: &Option<String>) -> String {

    let name = match name {
        Some(name) => name.to_owned(),
        None => {
            let words = fs::File::open("/usr/share/dict/words").unwrap();
            let words = BufReader::new(words).lines();
            words.choose(&mut rand::thread_rng())
                .unwrap()
                .unwrap()
        }
    };

    let _ = fs::create_dir_all(format!("/tmp/plumber/{}", name));

    eprintln!("{name}");

    name
}

fn main() {
    let args = Args::parse();
    let shutdown = register_shutdown();

    let (input, name) = match &args.command {
        Subargs::Exec { pipeline , name} => {
            let name = setup_directories(name);
            let input = pipeline.join(" ");
            (input, name)
        },
        Subargs::Run { path, name } => {
            let name = setup_directories(name);
            let input = fs::read_to_string(path).unwrap();
            (input, name)
        }
    };

    let input = input.trim().to_string();
    eprintln!("{input}");

    let mut handles = Vec::new();

    let input = PipelineInput::new(input.clone(), format!("/tmp/plumber/{name}"));
    let pipeline = Pipeline::new(&input, shutdown.clone());
    handles.push(thread::spawn(move || pipeline.run()));

    for handle in handles {
        handle.join().unwrap();
    }
}
