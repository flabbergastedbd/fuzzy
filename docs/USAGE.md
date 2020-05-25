# Usage

## Pre-Requisites

- `fuzzy` binary or a docker container with fuzzy.
- Remote url to connect to, also called as `--server-url`.
- CA cert path aka `--cert-authority`.
- Client cert to talk to api aka `--client-idenity`.

These can be easily listed by running help for global options

``` bash
Command line interface to interact with master

USAGE:
    fuzzy cli [FLAGS] [OPTIONS] [SUBCOMMAND]

FLAGS:
    -h, --help       Prints help information
    -v, --debug      Enable debug logging
    -V, --version    Prints version information

OPTIONS:
        --cert-authority <ca>             CA cert file (Default: ca.crt)
        --server-url <connect_addr>       Server address to connect (Default: https://localhost:12700)
        --logfile <logfile>               Logfile to write logs (Default: fuzzy.log)
        --client-identity <worker_pem>    Client identity to talk to server, necessary even for cli (Default:
                                          worker.pem)

SUBCOMMANDS:
    corpora    Access/Edit/Remove corpus
    crashes    Access crashes
    help       Prints this message or the help of the given subcommand(s)
    profile    Test fuzz profiles
    tasks      Access/Edit/Remove task information
```

> It is easy to keep the defaults in working directory to avoid passing most of these parameters. Like `ca.crt`,
> `worker.pem`.

## Use Cases

### Tasks

Tasks are the main building block for fuzzy & can be accessed under `tasks` subcommand. Please read `--help` carefully
for all subcommands as sometimes flags might not be intuitive.

> Like, when editing a task not passing of `--active` flag will disable the task.

#### New Task

A task needs a [fuzzy profile][Fuzzy Profile]. Copy paste from one of the [samples][Fuzzy Profile Samples] and edit
as needed.

[Fuzzy Profile]: ./PROFILE.md
[Fuzzy Profile Samples]: ../samples/profiles/task/
