# Repository Guidelines

## Project Structure & Module Organization
- `node/` is the Substrate entrypoint; `output/` collects staged binaries after make targets run.
- `runtimes/` defines FastChain and AnchorChain logic; Wasm artifacts land under `target/release/wbuild/`.
- `pallets/` hosts custom pallets such as `spin-anchoring` exposing the `SecureUpTo` watermark; common types live in `primitives/`.
- `client/consensus-spin` and `primitives/consensus-spin` implement SPIN consensus plumbing; `docs/`, `docker/`, and `zombienet/` support deep dives, container flows, and network experiments.

## Build, Test, and Development Commands
- `make qf-node` / `make qf-node-release` compile debug or release nodes and copy binaries into `output/`.
- `make qf-runtime` refreshes runtime Wasm artifacts before packaging, anchoring, or chain-spec work.
- `make qf-run` launches a dev FastChain with tmp state; `make qf-run-wasm` loads runtime overrides from `output/`.
- `make clippy`, `make fmt`, and `make qf-test` wrap `cargo` lint, format (+nightly), and test with `SKIP_WASM_BUILD`.
- Run `cargo check` locally before sending reviews to catch compile-time regressions.

## Coding Style & Naming Conventions
- Use Rust defaults: 4-space indentation, snake_case modules/functions, UpperCamelCase types, SCREAMING_SNAKE constants.
- Prefix new crates with `qf-`; place consensus additions under `client/` or `primitives/` to match the existing layout.
- Run `make fmt` and `make clippy`; avoid `unwrap` in runtime or consensus code—prefer structured error handling and typed errors.

## Testing Guidelines
- Run `make qf-test` for the suite; co-locate integration cases in each crate’s `tests/` folder.
- Model FastChain versus AnchorChain behavior in zombienet scenarios under `zombienet/`; log best/finalized/secure_up_to metrics.
- When touching SecureUpTo or authority transitions, add negative, reorg, and weight-edge cases.

## Commit & Pull Request Guidelines
- Keep commit subjects imperative (e.g., `Add spin-anchoring weight checks`) and wrap bodies near 72 characters when explaining context.
- Branch names follow `your-handle/summary`; rebase onto `main` before submitting PRs.
- PR descriptions should state motivation, test evidence (`make qf-test`, zombienet runs), linked issues/specs, and any telemetry or log artefacts for consensus/runtime changes.

## Architecture & Security Notes
- SPIN pairs a low-latency FastChain with AnchorChain finality; keep the SecureUpTo watermark monotonic and authenticated.
- Regenerate chain specs with `make qf-chainspec` after runtime updates and commit the JSON alongside refreshed Wasm.
- Treat FastChain heights beyond SecureUpTo as probabilistic; never publish runtime keys and follow `SECURITY.md` for disclosure process.
