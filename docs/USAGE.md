# Usage

## Pre-Requisites

- `fuzzy` binary or a docker container with fuzzy.
- Remote url to connect to, also called as `--server-url`. This can be set as environment variable `FUZZY_CONNECT_URL`.
- CA cert path aka `--cert-authority` or present in working directory as `ca.crt`.
- Client cert to talk to api aka `--client-idenity` or present in working directory as `worker.pem`.

These can be easily listed by running help for global options, always use `--help`

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

Detailed usecases are documented [here](./USECASES.md) with examples.

## Fuzz Profiles

[Fuzzy Profile][] & [Fuzzy Profile Samples][].

[Fuzzy Profile]: ./PROFILE.md
[Fuzzy Profile Samples]: ../samples/profiles/task/
