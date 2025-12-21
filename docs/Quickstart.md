# Quickstart

## Build & Test

- `cargo test`

## Compile an example

- `cargo run -p mdfs_cli -- compile examples/minimal.mdfs -o /tmp/minimal.mdf.json`

## Load the compiled .mdf (runner-side)

- `cargo run -p mdf_runner --example print_meta -- /tmp/minimal.mdf.json`
