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
- plumber run defaults stderr logs to ```/var/log/plumber/<plumber file name>/<cmd>.stderr.log```
- termination signals will be caught, sent to the FIRST program in the pipeline, and wait for completion

## modules
Building a module / transformer / process for the pipeline is easy in almost any programming language (performance is up to the programmer of course). All you need is a program that reads from stdin, does a transormation, and outputs to stdout.

Note: This is different for the first process in a pipeline - it is expected that it will handle SIGTERM and SIGINT in a responsible way.

## example
create a test file with a pipeline of processes:
```
$ echo "tail -n 100 -f /usr/share/dict/words | grep 'a' | wc" > test_pipeline.plumb
```
set the ```RUST_LOG``` environment variable to one of ```debug, info, warn, error``` to see logged messages in stderr
```
$ export RUST_LOG=debug
```
run the pipeline with ```plumber run <PATH>``` (the user executing the command will need permission to write to ```/var/log/plumber/```)
```
$ plumber run test_pipeline.plumb
spawning pipeline: tail -n 100 -f /usr/share/dict/words | grep 'a' | wc
logging to => /var/log/plumber/test_pipeline
```
hit ctrl-c (or send any generic term signal) to gracefully stop the pipeline and get the stdout of the final command.
```
$ ^Cexiting gracefully...
      28      28     334
```
find stderr logs in ```/var/log/plumber/test_pipeline/tail.stderr.log, grep.stderr.log, wc.stderr.log```

try rerunning the pipeline simply through your regular shell and hitting ctr-c.
```
$ tail -n 100 -f /usr/share/dict/words | grep 'a' | wc
^C
```
notice how there is no output as all the commands received the interrupt. With plumber you can be confident that data held in the buffers of intermediate processes will never be lost like this.

## daemonizing
I have a strong belief in letting programs do the things they are good at and nothing more, so plumber will never have a built in daemon feature. In order to daemonize plumber you have to use your operating system's daemonization mechanism to execute ```plumber run <PATH TO PLUMBER FILE>```. However, feel free to use it bravely as the signals and restarts will work as intended!
