use std::path::{Path, PathBuf};
use std::fs;
use std::process::{Child, Stdio, Command};
use std::os::unix::process::CommandExt;
use log::error;

struct PipelineCommand {
    name: String,
    args: Vec<String>,
}

impl PipelineCommand {
    pub fn new(mut cmd: Vec<String>) -> PipelineCommand {
        let name = cmd.remove(0);
        let args = cmd;

        PipelineCommand {
            name,
            args
        }
    }
}

pub struct Pipeline {
    _name: String,
    raw_pipeline: String,
    commands: Vec<PipelineCommand>,
    jobs: Vec<Child>,
    _metadata_dir: PathBuf,
    logging_dir: PathBuf,
}

impl Pipeline {
    fn parse_raw_pipeline(raw_pipeline: &str) -> Vec<PipelineCommand> {
        let split_on_pipe = raw_pipeline.split('|'); // split pipes

        let split_on_whitespace: Vec<Vec<String>> = split_on_pipe.map(|cmd_string|
            shlex::split(cmd_string)
            .unwrap_or_default())
            .collect();

        let commands: Vec<PipelineCommand> = split_on_whitespace
            .into_iter().map(|cmd|
            PipelineCommand::new(cmd))
            .collect();

        assert!(!commands.is_empty(), "unable to parse commands - empty list: {}", raw_pipeline);

        commands
    }

    pub fn new(name: String, raw_pipeline: String) -> Self {
        let commands = Pipeline::parse_raw_pipeline(&raw_pipeline);
        let metadata_dir = Path::new("/var/lib/plumber").join(&name);
        let logging_dir = Path::new("/var/log/plumber").join(&name);
        create_dir_with_nice_error(&metadata_dir);
        create_dir_with_nice_error(&logging_dir);

        Pipeline {
            _name: name,
            raw_pipeline,
            commands,
            jobs: Vec::new(),
            _metadata_dir: metadata_dir,
            logging_dir
        }
    }

    pub fn new_from_file(path: &Path) -> Self {
        let name = path.file_stem()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();

        let raw_pipeline = fs::read_to_string(path).unwrap();

        Self::new(name, raw_pipeline)
    }

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

    fn spawn_all(&mut self) {
        let mut prev_stdout = Stdio::inherit();

        let commands_except_last = &self.commands[..self.commands.len() - 1];
        for cmd in commands_except_last.iter() {
            let stderr_out = fs::File::create(&self.logging_dir
                        .join(&cmd.name)
                        .with_extension("stderr.log"))
                        .unwrap();

            let stderr_out = Stdio::from(stderr_out);

            let mut child = Self::spawn_process(
                &cmd.name, &cmd.args,
                prev_stdout, Stdio::piped(), stderr_out
            );
            prev_stdout = Stdio::from(child.stdout.take().unwrap());
            self.jobs.push(child);
        }

        // this is to pipe the stdout of the last command to the parent process
        let last_cmd = self.commands.last().unwrap();

        let stderr_out = fs::File::create(&self.logging_dir
            .join(&last_cmd.name)
            .with_extension("stderr.log")
        ).unwrap();

        let stderr_out = Stdio::from(stderr_out);

        let child = Self::spawn_process(
            &last_cmd.name, &last_cmd.args,
            prev_stdout, Stdio::inherit(), stderr_out
        );
        self.jobs.push(child);
    }

    pub fn run(mut self) {
        log::info!("executing pipeline: {}", &self.raw_pipeline.trim());
        log::info!("logging command stderr to: {}", &self.logging_dir.join("*.stderr.log").display());
        self.spawn_all();

        let first_job_pid = self.jobs.first()
            .unwrap()
            .id()
            .to_string();

        log::debug!("pid of first job in pipeline is {}", &first_job_pid);

        // let mut pid_file = fs::File::create(&self.metadata_dir.join(".pid")).unwrap();
        // pid_file.write_all(first_job_pid.as_bytes()).unwrap();
        // pid_file.flush();

        ctrlc::set_handler(move || {
            log::debug!("caught termination signal... attempting graceful exit");
            log::debug!("executing: kill -SIGTERM {first_job_pid}");
            let _ = Command::new("kill")
                .arg("-SIGTERM")
                .arg(&first_job_pid)
                .status()
                .unwrap();
        }).unwrap();

        for jobs in &mut self.jobs {
            jobs.wait().unwrap();
        }
    }
}

fn create_dir_with_nice_error(dir: &Path) {
    match fs::create_dir_all(dir) {
        Ok(_) => {},
        Err(e) => match e.kind() {
            std::io::ErrorKind::PermissionDenied => {
                error!("plumber requires permission to write in {}",
                       dir.parent().unwrap().display());
                error!("recommended to have user that executes plumber to own this directory");
                panic!("{e}");
            },
            k => panic!("{e} {k}"),
        }
    }
}
