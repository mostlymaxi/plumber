# plumber
### unix process pipelines made easy

The goal of this project was to create simple, modular, and powerful pipelines using the standard tools of the unix operating system.

Namely, the idea was born when I had to process various streams of data with only slightly requirements so I decided to just make some simple programs that take text as input, transform it, and push it to output. This let me setup simple daemons that looked like this:

```pull_from_stream | transformer_1 | transformer_2 | transformer_3 | push_to_stream```

Unfortunately, daemonizing something this simple is never, well, simple. So here we are.

### example
create a test file with a pipeline (or pipelines) of processes:
```
tail -f some_log_file | grep '1' | wc
tail -f some_other_log_file | grep '2' | wc
```

run the pipeline with ```plumber <PATH>```.

hit ctrl-c (or any generic kill signal) to gracefully stop the pipline(s). This will send a kill signal to the FIRST process in the pipeline(s) and then wait for all the other processes to finish. As long as the first process handles the signal gracefully, this ensures that end-to-end no data ever gets lost!
