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
* `DYFI_HOSTNAMES` – a comma-separated list of hostnames associated with the selected username

To control the log level, you may also set the `RUST_LOG` variable.
By default only errors are logged, but `RUST_LOG=dyfi_client=info` enables logging successes as well.

## Exit statuses

| Status  | Meaning                                                      |
| ------- | ------------------------------------------------------------ |
| 0       | OK                                                           |
| 1       | Bad authentication.                                          |
| 2       | No hostname given or hostname not allocated for user.        |
| 3       | Not a valid FQDN.                                            |
| 4       | IP not valid or not registered to a Finnish organisation.    |
| 5       | Request failed due to technical problem.                     |
| 6       | Request denied due to abuse.                                 |
| 10      | Initialization error - usually due to environment variables. |

## Building

Easiest is to build an image from the included `Dockerfile`.

### Building conventionally on host OS

Install these prerequisites:

* Development packages for OpenSSL:
    * `libssl-dev` on Debian and Ubuntu
	* `openssl-dev` on Fedora
* The Rust toolchain

Run `cargo build --release`.
The resulting binary will be in `./target/release/`.

## Running

The dy.fi client is a daemon that runs on a loop and keeps track of some state.
State is not saved on disk, so starting the daemon always performs an update first
and then sleeps.

Options:

1. Build and start a container from the included `Dockerfile`.
A `docker-compose.yml` is also provided.

2. Run the daemon on your host system, e.g. as a systemd unit.

## Dy.fi documentation

* https://www.dy.fi/page/clients
* https://www.dy.fi/page/specification

## TODO

* Tests.
* Maybe save last performed update on disk. This would require a volume in Docker.
* Handle multiple hostnames a bit better.
* Pre-built Docker image on e.g. Dockerhub.
