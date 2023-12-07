# plumber
## unix process pipelines made easy

The goal of this project was to create simple, modular, and powerful pipelines using the standard tools of the unix operating system.

```pull_from_stream | transformer_1 | transformer_2 | transformer_3 | push_to_stream```

Shell users may recognize the process pipeline language used and ask why even use plumber?

The purpose of plumber is to ensure that pipelines are robust against reboots and interrupts, easily defined and managed through files (for IaC use cases), and are highly configurable.

## features
- modular pipeline building
- robust signal handling
- extremely simple implementation

## behavior
- pipes imply that stdout is redirected to stdin of following program
- plumber run defaults stderr logs to ```/tmp/plumber/log/<plumber file name>/<cmd>.stderr.log```
- termination signals will be caught, sent to the FIRST program in the pipeline, and wait for completion

## example
create a test file with a pipeline of processes:
```
echo "exec = 'tail -n 100 -f /usr/share/dict/words | grep a | wc'" > test_pipeline.plumb
```
run the pipeline with ```plumber run <PATH>```
```
plumber run test_pipeline.plumb
```
```
[2023-12-07T19:31:43Z INFO  plumber::pipeline] (test_pipeline) executing pipeline: 'tail -n 100 -f /usr/share/dict/words | grep a | wc'
[2023-12-07T19:31:43Z INFO  plumber::pipeline] (test_pipeline) logging command stderr to: '/tmp/plumber/log/test_pipeline/*.stderr.log'
[2023-12-07T19:31:43Z INFO  plumber::pipeline] (test_pipeline) pid of first job in pipeline is: 30251
[2023-12-07T19:31:43Z INFO  plumber::pipeline] (test_pipeline) gracefully stop pipeline with: 'plumber stop test_pipeline'
```
gracefully stop the pipeline with ```plumber stop <NAME>``` or hitting Ctrl+C in the main process window
```
^C[2023-12-07T19:31:57Z INFO  plumber::pipeline] (test_pipeline) exiting gracefully...
```
stdout of the last command is streamed to the stdout of plumber to allow for chaining of pipelines. In this case we see the output of the ```wc``` command:
```
      28      28     334
```
find stderr logs in ```/tmp/plumber/log/test_pipeline/tail.stderr.log, grep.stderr.log, wc.stderr.log```

try rerunning the pipeline simply through your regular shell and hitting ctrl-c.
```
tail -n 100 -f /usr/share/dict/words | grep 'a' | wc
```
notice how there is no output as all the commands received the interrupt. With plumber you can be confident that data held in the buffers of intermediate processes will never be lost like this.

## modules
Building a module / transformer / process for the pipeline is easy in almost any programming language (performance is up to the programmer of course). All you need is a program that reads from stdin, does a transformation, and outputs to stdout.

Note: This is different for the first process in a pipeline - it is expected that it will handle SIGTERM and SIGINT in a responsible way.

## daemonizing
use your system's daemon / service manager to daemonize plumber pipelines. Here is an example systemd unit file:

```
[Unit]
Description=Example Plumber Pipeline

[Service]
Environment=RUST_LOG=debug
ExecStart=plumber run /opt/plumber-example
ExecStop=plumber stop /opt/plumber-example
User=plumber
Restart=always

[Install]
WantedBy=default.target
```
