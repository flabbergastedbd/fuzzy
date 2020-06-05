# Fuzzy Profile

This is source of truth for how your fuzz task is evaluated and what steps are done as part of it. In this doc
let us see different sections of the config. An updated collection of fuzz profiles is present in [samples].

## Common Structures

### Execution

In many places, there is a need to run a process. All those places accept same yaml structure.

- `executor`: Type of executor, current `Native` or `Docker`.
- `cpus`: Decides number of cpus for this process.
- `image`: This parameter will be in case of `Docker` executor.
- `executable`: Preferrably absolute path to executable that should be launched.
- `args`: Arguments to pass to `executable`.
- `envs`: Environment variables.
- `cwd`: Working directory where executable will be launched.

## Fuzz Driver

Core for everything, list of supported options will be in samples.

## Execution 

Decides on how the fuzzing should take place. Same structure as [Execution](#execution).

## Corpus

Configuration parameters on how corpus should be handled.

- `path`: Path to corpus directory, relative to fuzzing's `cwd`.
- `label`: A string identifier which decides what kind of corpus to download. Any new corpus will be
  uploaded with same label.
- `refresh_interval`: Time in seconds in which corpus should be synced (both upload & download).
- `upload`: Boolean, if corpus should be uploaded.
- `upload_filter`: A rust regex, to upload filenames matching certain pattern.

## Crash

Crash handling and verification.

- `path`: Path to crashes directory, relative to fuzzing's `cwd`. Incase of this being same to cwd, just use `.`.
- `label`: An identifier to be attached to crashes that are found.
- `filter`: A rust regex, to filter out crashes incase of fuzzer not being able to save crashes to a separate directory.
- `validate`: Can be skipped if crash validation is not required.
- `deduplicate`: Can be skipped if crash validation is not required.

### Validate

Parameters used to validate crashes, same as [Execution](#execution) above.

> Output to stdout & stderr are saved and a non zero exit code is treated as verified crash.

*Changes*

- `args`: Arguments to pass to validator process. *Path to crash file will be added an last parameter*.

### Deduplicate

Parameters used to deduplicate crashes, same as [Execution](#execution) above.

> Zero exit code will mark the second path be marked as crash as first.

*Changes*

- `args`: Two paths containing crash outputs are passed as args, exit code `0` indicates that they are duplicates (like `diff`).

``` bash
#!/bin/bash
#
# A simple dedup script that removes hex values from output before comparing which generally leaves call trace.
#
diff <(cat $1 | sed -e "s/0x[0-9a-fA-F]*//g") <(cat $2 | sed -e "s/0x[0-9a-fA-F]*//g")
```

## Fuzz Stat

Can be `null` in which case, custom driver coverage will be used like log parsing.

In case of not being `null`, it takes following parameters.

- `collector`: Type of collector to use. This will be elaborated below.
- `execution`: Similar to [Execution](#execution) structure.

### Collectors

#### LCov

This collector will place corpus files in current directory & expects one `*.lcov` file after
execution.

A sample config for this kind of collector for a program compiled with llvm.

``` yaml
fuzz_stat:
  collector: LCov
  execution:
    cpus: 1
    executor: Docker
    image: "snappy:fuzzy"
    executable: /bin/generate_lcov
    cwd: /profiling
```

**generate_lcov**

``` bash
#!/bin/bash

#
# 1. Since, corpus is present in cwd, run the program compiled with profile instrumentation on each of those files.
#
for i in $(ls); do
	LLVM_PROFILE_FILE="$i.profraw" /workspace/snappy/profiled/snappy_uncompress_fuzzer $i
done

#
# 2. Since only one *.lcov file is taken, combine this coverage & convert it into lcov info format and save with right
#    extension.
#
llvm-profdata merge -o "snappy.profdata" *.profraw
llvm-cov export --format=lcov /workspace/snappy/profiled/snappy_uncompress_fuzzer -instr-profile "snappy.profdata" > "fuzzy.lcov"
```

[samples]: ../samples/profiles/task/
