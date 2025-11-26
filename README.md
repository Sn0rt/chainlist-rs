# chainlist-rs

[![CI](https://github.com/Sn0rt/chainlist-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/Sn0rt/chainlist-rs/actions/workflows/ci.yml)
[![Coverage](https://codecov.io/gh/Sn0rt/chainlist-rs/branch/main/graph/badge.svg)](https://codecov.io/gh/Sn0rt/chainlist-rs)

Typed access to EVM chain metadata generated from `chainid.network/chains.json`. The build script downloads the JSON on every build and turns it into a `Chain` enum with helpers for IDs, names, native currency, RPC URLs, and block times.

## Quick start

Add to your `Cargo.toml`:

```toml
chainlist-rs = "0.1"
```

Use the enum:

```rust
use chainlist_rs::Chain;

fn main() {
    let mainnet = Chain::Mainnet;
    println!(
        "{} (id {}) uses {}",
        mainnet.name(),
        mainnet.id(),
        mainnet.native_currency().1
    );
}
```

Examples:

- `cargo run --example print_chain`
- `cargo run --example wallet_params`
- `cargo run --example list_chains`
- Access full chain table: `let chains = chainlist_rs::all_chains();`

## Data source

- `chains.json` is downloaded at build time from <https://chainid.network/chains.json> (network required).
- Override with `CHAINS_JSON_URL` to point to your mirror, or `CHAINS_JSON_PATH` if you want to supply a local file explicitly.
- The downloaded file is kept in the build output dir and is ignored by git.

## Developing & releasing

- Builds require network access to fetch `chains.json` unless you provide `CHAINS_JSON_PATH`.
- Quality gates: `cargo fmt`, `cargo clippy --all-targets --all-features`, `cargo test`.
- CI: PRs/pushes run fmt/clippy/tests with `CHAINS_JSON_PATH=data/chains.json` to stay offline.
- Publish check: `cargo package --dry-run` (or `cargo publish --dry-run`) to verify the crate contents and metadata.
- Automated release: pushing a tag `vX.Y.Z` runs checks (fmt/clippy/tests) and publishes with `cargo publish --locked` when `CARGO_REGISTRY_TOKEN` is set in repo secrets; the tag must match the crate version.
- Coverage: CI runs `cargo llvm-cov --lcov` with `CHAINS_JSON_PATH=data/chains.json` and uploads to Codecov (set `CODECOV_TOKEN` if the repo is private).

## License

MIT OR Apache-2.0 (dual license).
