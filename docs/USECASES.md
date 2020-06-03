# Corpora

## Upload Corpus

Corpus is identified with a label, so to upload corpus files with label `test`.

``` bash
fuzzy cli corpora add test corpus/*
```

## Download Corpus

Download corpus to a `new_corpus` folder.

``` bash
fuzzy cli corpora download test new_corpus
```

## Delete Corpus

Delete old corpus with label `old`.

``` bash
fuzzy cli corpora delete old
```

## Minimize Corpus

> It is recommended to stop any running tasks that is using this corpus while doing this.

- Download corpus that needs to be minimized to a `new_corpus` folder.

``` bash
fuzzy cli corpora download test new_corpus
```

- Apply any minimizations as necessary with appropriate tools and write to `minimized_corpus/`.
- Delete old corpus with same label `test`.

``` bash
fuzzy cli corpora delete test
```

- Upload new minimized corpus with same label.

``` bash
fuzzy cli corpora add test minimized_corpus/*
```

## Download Latest Corpus

To download latest `100` new corpus that is being saved with label `new`

``` bash
fuzzy cli corpora download new new_corpus/ --latest 100
```

This can generally be used to generate lcov html reports locally to see how coverage is proceeding.

# Tasks

## Add Task

``` bash
fuzzy cli tasks add specialTask <path to profile.yaml>
```

## List Tasks

``` bash
fuzzy cli tasks list
```

## Stop Task

To stop a task `snappy` with id `1`.

``` bash
fuzzy cli tasks edit 1
```

## Restart Task

To restart a task `snappy` with id `1` with a new profile.

``` bash
fuzzy cli tasks edit 1 --active --profile <path_to_profile.yaml>
```

# Crashes

## Download Crashes

To download crashes with a label `snappy_uncompress`

``` bash
fuzzy cli crashes snappy_uncompress new_crashes/
```

To download only verified crashes for a particular task alone

``` bash
fuzzy cli crashes snappy_uncompress new_crashes/ --task-id 1 --verified
```

To download crashes that match a particular crash pattern

``` bash
fuzzy cli crashes snappy_uncompress new_crashes/ --output "%SIGSEGV%"
```

## Revalidate Crashes

If you have changed fuzz profile and need to validate your crashes again, let us say for task id `2`

``` bash
fuzzy cli crashes revalidate 2 --all
```

`--all` flag revalidates verified crashes again with new config.
