# Wombat

A new product inspired by TreeSheets: documents are grids of cells, and any cell can hold another
grid. This folder holds the proof of concept, a Rust core with no interface.

- The plan and its progress: [`docs/PLAN.md`](docs/PLAN.md)
- Design decisions: [`docs/DECISIONS.md`](docs/DECISIONS.md)
- Rules for changing the code: [`AGENTS.md`](AGENTS.md)

```sh
cargo test --workspace      # run all tests
cargo run -p wombat-cli     # run the command-line tool
```

"Wombat" is a working name. TreeSheets is (c) Wouter van Oortmerssen, zlib licence.
