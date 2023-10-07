use std::process::{Command, Stdio, Child};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::os::unix::process::CommandExt;
use std::{thread, fs};
use std::time::Duration;

use signal_hook::consts::TERM_SIGNALS;

use clap::Parser;

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
        path: String
    },
    /// execute a pipeline from a string input
    Exec {
        /// raw pipeline string (must be in quotes)
        pipeline: String,
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

// Convenience structure to split command vector (cat a_file) into a
// command name (cat) and arguments ([a_file]).
struct PipelineCommand {
    name: String,
    args: Vec<String>,
}

impl PipelineCommand {
    fn new(mut cmd: Vec<String>) -> PipelineCommand {
        let name = cmd.remove(0);
        let args = cmd;

        PipelineCommand {
            name,
            args
        }
    }
}

// Future proof if pipeline needs more than a vector of commands as input.
// Maybe some sort of settings in the pipeline file?
struct PipelineInput {
    _input_string: String,
    commands: Vec<PipelineCommand>,
}

impl PipelineInput {
    fn new(input_string: String) -> PipelineInput {
        let split_on_pipe = input_string.split('|'); // split pipes

        let split_on_whitespace: Vec<Vec<String>> = split_on_pipe.map(|cmd_string|
            shlex::split(cmd_string)
            .unwrap_or_default())
            .collect();

        let commands: Vec<PipelineCommand> = split_on_whitespace
            .into_iter().map(|cmd|
            PipelineCommand::new(cmd))
            .collect();

        PipelineInput { _input_string: input_string, commands}
    }
}

struct Pipeline {
    shutdown: Arc<AtomicBool>,
    jobs: Vec<Child>,
}

impl Pipeline {
    fn spawn_process(
        name: &String,
        args: &Vec<String>,
        stdin: Stdio,
        stdout: Stdio,
        stderr: Stdio) -> Child {
        let mut child = Command::new(name);

        child.args(args);

        child
            .stdin(stdin)
            .stdout(stdout)
            .stderr(stderr)
            .process_group(0)
            .spawn()
            .expect(&format!("Failed to spawn command: {} {}", name, args.join(" ")))
    }

    fn new(input: &PipelineInput, shutdown: Arc<AtomicBool>) -> Pipeline {
        let mut jobs = Vec::new();
        let mut prev_stdout = Stdio::null();

        let commands_exc_last = &input.commands[..input.commands.len() - 1];

        if !commands_exc_last.is_empty() {
            for cmd in commands_exc_last.iter() {
                let mut child = Self::spawn_process(
                    &cmd.name, &cmd.args,
                    prev_stdout, Stdio::piped(), Stdio::piped()
                );
                prev_stdout = Stdio::from(child.stdout.take().unwrap());
                jobs.push(child);
            }
        }

        // this is to pipe the stdout of the last command to the parent process
        let last_cmd = input.commands.last().unwrap();
        let child = Self::spawn_process(
            &last_cmd.name, &last_cmd.args,
            prev_stdout, Stdio::inherit(), Stdio::piped()
        );

        jobs.push(child);

        Pipeline { shutdown: shutdown, jobs: jobs }
    }

    fn cleanup(mut self) {
        // if we get term signal, kill ONLY the first job.
        // this ensures all data in the pipeline is processed to the end.
        if self.shutdown.load(Ordering::Relaxed) {
            self.jobs.first_mut()
                .unwrap()
                .kill()
                .expect("Something went wrong killing first process");
        }

        for mut jobs in self.jobs {
            jobs.wait().unwrap();
        }
    }

    fn run(mut self) {
        while !self.shutdown.load(Ordering::Relaxed) {
            // busy wait is the best way I could think of
            match self.jobs.last_mut().unwrap().try_wait() {
                Ok(Some(_)) => { break; },
                Ok(None) => { },
                Err(e) => { eprintln!("error attempting to wait: {e}"); break; },
            };
            thread::sleep(Duration::from_secs(1));
        }

        self.cleanup();
    }
}

fn main() {
    let args = Args::parse();
    let shutdown = register_shutdown();

    let input = match &args.command {
        Subargs::Exec { pipeline } => pipeline.to_owned(),
        Subargs::Run { path } => fs::read_to_string(path).unwrap(),
    };

    let input: Vec<&str> = input.split("\n").collect();
    let input: Vec<String> = input.iter()
        .map(|s| s.to_string())
        .collect();

    let mut handles = Vec::new();
    for cmd in input {
        if cmd.is_empty() { continue }

        let input = PipelineInput::new(cmd);
        let pipeline = Pipeline::new(&input, shutdown.clone());
        handles.push(thread::spawn(move || pipeline.run()));
    }

    for handle in handles {
        handle.join().unwrap();
    }
}
