use std::time::Duration;
use std::{path::PathBuf, process::exit, fs, vec};
use std::thread;
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
        /// path to plumber file or directory of files
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
    /// stop pipelines using a plumber file path
    Stop {
        /// path to plumber file or directory of files
        path: PathBuf,
        /// shutdown timeout in seconds
        #[arg(short, long, default_value_t=30)]
        timeout: u32,
    }
}

fn exec(name: String, pipeline: String) {
    if pipeline.trim().is_empty() {
        error!("tried to execute empty pipeline");
        return;
    }

    let Ok(pipeline) = Pipeline::new(name.clone(), pipeline) else { return };

    ctrlc::set_handler(move || {
        if let Err(_) = Pipeline::stop(&name) {
            log::error!("something went very wrong with the termination signal handler");
            log::error!("this may cause the pipeline to continue running in the background!");
            log::error!("you may be able to still gracefully kill the pipeline by finding the pid of the first \
                        process in the pipeline and killing it manually");
            exit(1);
        }
    }).unwrap();

    pipeline.run();
}

fn stop(path: PathBuf, timeout: u32) {
    let names = match path.is_dir() {
        true => {
            let mut plumb_files = Vec::new();
            for file in fs::read_dir(&path).unwrap() {
                let Ok(file) = file else { continue };
                let file = file.path();
                if file.is_dir() { continue }
                let Some(ext) = file.extension() else { continue };
                if ext.eq_ignore_ascii_case("plumb") {
                    let name = file.file_stem()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .to_owned();

                    plumb_files.push(name);
                }
            }
            plumb_files
        },
        false => {
            let name = path.file_stem()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();

            vec![name]
        }
    };

    for name in &names {
        if let Err(e) = Pipeline::stop(&name) {
            match e {
                pipeline::PipelineError::FileNotFound => log::warn!("unabled to find pid for name '{}'", name),
                pipeline::PipelineError::Other => log::error!("{:#?}", e),
            }
        }
    }

    for _ in 0..=timeout {
        if !names.iter().any(|n| path.join(n).join(".pid").exists()) {
            break;
        }
        thread::sleep(Duration::from_secs(1));
    }
}

fn run(path: PathBuf) {

    let files = match path.is_dir() {
        true => {
            let mut plumb_files = Vec::new();
            for file in fs::read_dir(path).unwrap() {
                let Ok(file) = file else { continue };
                let file = file.path();
                if file.is_dir() { continue }
                let Some(ext) = file.extension() else { continue };
                if ext.eq_ignore_ascii_case("plumb") {
                    plumb_files.push(file);
                }
            }
            plumb_files
        },
        false => vec![path]
    };

    let mut handles = Vec::new();
    let mut names = Vec::new();
    for f in files {
        let Ok(pipeline) = Pipeline::new_from_file(&f) else { continue };
        let name = pipeline.get_name();
        handles.push(thread::spawn(move || pipeline.run()));
        names.push(name);
    }

    ctrlc::set_handler(move || {
        for name in &names {
            if let Err(e) = Pipeline::stop(&name) {
                log::error!("something went very wrong with the termination signal handler");
                log::error!("this may cause the pipeline to continue running in the background!");
                log::error!("you may be able to still gracefully kill the pipeline by finding the pid of the first \
                            process in the pipeline and killing it manually");
                log::error!("{:?}", e);
            }
        }
    }).unwrap();

    for handle in handles {
        handle.join().unwrap();
    }

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
        },
        Subargs::Stop { path , timeout} => {
            stop(path.into(), *timeout);
        }
    }
}
