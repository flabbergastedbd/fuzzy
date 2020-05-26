# Development

## Toolchain

Have `rustup` installed. This can be done using many package managers including `Homebrew`. 

``` bash
rustup install stable
```

## Depdencies

One of our build dependencies that compiles `protobuf` will need `rustfmt`.

``` bash
rustup component add rustfmt
```

Have `protoc` (Protobuf compiler) & `libpq` installed.

## Build/Run

After having `~/.cargo/bin/` to your path, you should be able to do following. Any parameter passed after
`--` will be passed as parameter to fuzzy.

``` bash
cargo run -- --help
```
