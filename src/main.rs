use std::fs::read_dir;

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::{fs, thread};

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
        /// path to plumber file or directory
        path: PathBuf,
    },
    /// execute a pipeline from a string input
    Exec {
        /// raw pipeline string
        pipeline: Vec<String>,
        /// path to metadata directory where stderr is logged
        #[arg(short, long, default_value = "/tmp/plumber/leaky")]
        metadata_dir: PathBuf,
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

fn parse_plumber_file(path: &PathBuf) -> String {
    let input = fs::read_to_string(path).unwrap();
    input
}

fn exec(pipeline: String, metadata_dir: PathBuf, shutdown: Arc<AtomicBool>) -> JoinHandle<()> {
    fs::create_dir_all(&metadata_dir).expect("Failed to create metadata directory");

    eprintln!("spawning pipeline: {}", pipeline);
    eprintln!("logging to => {}", metadata_dir.display());

    let input = PipelineInput::new(pipeline, metadata_dir);
    let pipeline = Pipeline::new(&input, shutdown.clone());
    thread::spawn(move || pipeline.run())
}


fn run(path: PathBuf, shutdown: Arc<AtomicBool>) -> Vec<JoinHandle<()>>{
    let mut handles = Vec::new();

    match path.is_dir() {
        true => {
            for file in read_dir(path).unwrap() {
                let file = file.unwrap().path();
                if !file.is_file() { continue }
                if !file.ends_with(".plumb") { continue }

                let pipeline = parse_plumber_file(&file);
                let mut metadata_dir = PathBuf::from("/var/log/");
                metadata_dir.push(file.file_stem().unwrap());

                handles.push(exec(pipeline, metadata_dir, shutdown.clone()));
            }
        },
        false => {
            let pipeline = parse_plumber_file(&path);
            let mut metadata_dir = PathBuf::from("/var/log/");
            metadata_dir.push(path.file_stem().unwrap());

            handles.push(exec(pipeline, metadata_dir, shutdown));
        }
    }

    handles
}

fn main() {
    let args = Args::parse();
    let shutdown = register_shutdown();

    match &args.command {
        Subargs::Exec { pipeline, metadata_dir } => {
            let pipeline = pipeline.join(" ");
            exec(pipeline, metadata_dir.into(), shutdown)
                .join()
                .expect("failed to join thread");
        },
        Subargs::Run { path } => {
            for thread in run(path.into(), shutdown) {
                thread.join().expect("failed to join thread");
            }
        }
    }
}
