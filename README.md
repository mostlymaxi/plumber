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
- pipes (|) imply that stdout is redirected to stdin of following program
- all programs stderr are merged to parent stderr (!not working!)
- termination signals will be caught, sent to the FIRST program in the pipeline, and wait for completion

## modules
Building a module / transformer / process for the pipeline is easy in almost any programming language (performance is up to the programmer of course). All you need is a program that reads from stdin, does a transormation, and outputs to stdout.

Note: This is different for the first process in a pipeline - it is expected that it will handle SIGTERM and SIGINT in a responsible way.

## example
create a test file with a pipeline (or pipelines) of processes:
```
tail -f some_log_file | grep '1' | wc
tail -f some_other_log_file | grep '2' | wc
```

run the pipeline with ```plumber <PATH>```.

hit ctrl-c (or any generic kill signal) to gracefully stop the pipline(s). This will send a kill signal to the FIRST process in the pipeline(s) and then wait for all the other processes to finish. As long as the first process handles the signal gracefully, this ensures that end-to-end no data ever gets lost!
