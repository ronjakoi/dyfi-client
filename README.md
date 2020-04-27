# Dy.fi client

This a dynamic DNS updater client for the Finnish [dy.fi](https://www.dy.fi/) service.
It is written in the Rust language.

## Configuration

Configuration options are read from environment variables.
If a `.env` file exists in the current working directory, those are also read,
but they will not overwrite environment variables already set.

The variables are:

* `DYFI_USER`
* `DYFI_PASSWORD`
* `DYFI_HOSTNAMES` -- a comma-separated list of hostnames associated with the selected username

## Building

Make sure you have the Rust toolchain installed and run `cargo build --release`.
The resulting binary will be in `./target/release/`.
