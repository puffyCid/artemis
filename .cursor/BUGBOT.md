# Artemis Repository Rules

This repository is a Rust workspace for a cross-platform DFIR collector. Keep changes aligned with the existing collection pipeline and multi-crate architecture instead of building parallel entry points, config models, or parser stacks.

## Workspace Architecture

```text
artemis/
├── cli/        # Thin CLI adapter; translate clap args into ArtemisToml
├── daemon/     # Enrollment, polling, remote collection orchestration
├── forensics/  # Core parser, runtime, output, reporting, filesystem helpers
├── timeline/   # Timeline normalization for supported artifacts
├── common/     # Shared cross-crate structs and helpers
├── .github/    # CI, release, coverage, audit, and deny workflows
└── .packages/  # Packaging assets for rpm, deb, pkg, and msi
```

## Collection Flow

```text
CLI args / TOML / base64 TOML / JS / daemon job
        ↓
forensics::structs::toml::ArtemisToml
        ↓
forensics::core::{parse_toml_file, parse_toml_data, parse_js_file, artemis_collection}
        ↓
forensics::artifacts::collection::collect
        ↓
OS-specific parser in forensics/src/artifacts/os/...
        ↓
forensics/src/output/{formats,local,remote}
        ↓
optional timeline conversion in timeline/
```

## Crate Boundaries

- `cli/` stays thin. It should only parse flags, build `Artifacts` and `Output`, and call `artemis_collection`.
- `daemon/` owns remote enrollment, polling, backoff, and collection scheduling. It should reuse `forensics` for execution.
- `forensics/` owns parser dispatch, filesystem access, runtime behavior, output handling, markers, logging, and reports.
- `timeline/` works on parsed `serde_json::Value` data and should preserve metadata such as `collection_metadata`.
- `common/` is for genuinely shared helpers and types, not a staging area for new artifact logic.

## Artifact Wiring Pattern

When adding or changing an artifact, wire the full path:

1. Define or extend option structs in `forensics/src/structs/artifacts/...`.
2. Add the field to `forensics/src/structs/toml.rs`. `ArtemisToml`, `Artifacts`, and `Output` are the canonical collection contract.
3. Implement parsing under the correct `forensics/src/artifacts/os/{windows,macos,linux}` area, or the adjacent runtime/system helpers when it is not OS-specific.
4. Register dispatch in `forensics/src/artifacts/collection.rs` using the existing `artifact_name` convention.
5. If exposed via CLI, add the clap subcommand in `cli/src/collector/commands.rs` and map it in `cli/src/collector/system.rs`.
6. If timeline support exists, add the enum case and transformer in `timeline/src/timeline.rs` and the relevant module in `timeline/src/artifacts/`.
7. Add fixture-backed tests under `forensics/tests/` and `timeline/tests/` as appropriate.

Keep naming consistent across TOML keys, `artifact_name`, CLI command names, test files, and timeline variants.

## Rust Coding Standards

These rules ensure maintainability, safety, and compatibility across the Artemis workspace.
- **MUST**: Enforced by CI or required by project architecture
- **SHOULD**: Strong recommendation
- **CAN**: Allowed when justified

## 1 — Before Coding

- **BP-1 (MUST)** Confirm the target flow before writing code: CLI, TOML, JS runtime, daemon, output, and optional timeline behavior.
- **BP-2 (MUST)** For new artifacts, identify the full wiring path up front: struct, dispatcher, parser, output, tests, and timeline support.
- **BP-3 (SHOULD)** State the target OS scope explicitly when behavior is Windows-only, macOS-only, Linux-only, or cross-platform.
- **BP-4 (SHOULD)** Define the validation plan before editing: focused `just` target, `cargo test --release`, and any fixture updates.

## 2 — Workspace & Dependencies

- **WD-1 (MUST)** Keep parser logic in `forensics/`; do not move it into `cli/`, `daemon/`, or `common/`.
- **WD-2 (SHOULD)** Prefer existing workspace crates and utilities before adding new abstractions.
- **WD-3 (MUST)** Limit new dependencies. Add a crate only when the payoff clearly outweighs the dependency and maintenance cost.
- **WD-4 (SHOULD)** Put dependencies in `[workspace.dependencies]` when shared by multiple crates.
- **WD-5 (MUST)** Keep `common/` small and truly shared. Artifact-specific types belong in `forensics/`.

## 3 — Code Style

- **CS-1 (MUST)** `cargo fmt` and clippy must pass.
- **CS-2 (MUST)** Follow Rust naming conventions: modules and functions `snake_case`, types and enums `PascalCase`, constants `SCREAMING_SNAKE_CASE`.
- **CS-3 (SHOULD)** Prefer enums and typed structs over raw strings and boolean-heavy APIs for finite states and parser options.
- **CS-4 (MUST)** Avoid `unwrap` and `expect` in production paths. They are acceptable in tests and tightly controlled setup code only when failure is intentional.
- **CS-5 (SHOULD)** Borrow first; clone only when ownership transfer is required.
- **CS-6 (MUST)** Keep OS-scoped code in the existing module layout under `forensics/src/artifacts/os/` and `forensics/src/runtime/{windows,macos,linux}`.
- **CS-7 (SHOULD)** Extract small parsing helpers when a function becomes hard to follow, especially for binary formats and record decoding.
- **CS-8 (MUST)** When a function accumulates several booleans or too many positional arguments, replace them with a typed options struct or enum.
- **CS-9 (SHOULD)** Preserve existing artifact naming conventions such as `users-macos`, `sudologs-linux`, and `rawfiles-ext4`.

## 4 — Errors

- **ERR-1 (MUST)** Return typed errors with `Result`; prefer crate-local error enums over ad hoc strings.
- **ERR-2 (MUST)** Add context at the failing boundary so logs identify artifact, file, or parser stage.
- **ERR-3 (MUST)** Use pattern matching for error control flow; do not rely on string matching.
- **ERR-4 (SHOULD)** Keep parser failures local to the artifact when possible and preserve overall collection/report behavior.
- **ERR-5 (SHOULD)** Distinguish malformed input, unsupported format, IO failure, and platform limitation when they affect operator understanding.

## 5 — Concurrency & Async

- **CC-1 (MUST)** Use synchronous code by default unless async or concurrency clearly improves the workflow.
- **CC-2 (MUST)** Tie spawned threads or async tasks to explicit lifecycle control; avoid orphaned work.
- **CC-3 (MUST)** Preserve daemon timeout, polling, jitter, and completion semantics when modifying remote collection behavior.
- **CC-4 (SHOULD)** Protect shared mutable state with explicit synchronization and keep the ownership story obvious.
- **CC-5 (MUST)** Do not block inside async code without a deliberate boundary.

## 6 — Testing

- **T-1 (MUST)** Prefer deterministic, fixture-backed tests using `forensics/tests/test_data/` and `timeline/tests/test_data/`.
- **T-2 (MUST)** Gate platform-specific tests with `#[cfg(target_os = "...")]` following the existing pattern.
- **T-3 (SHOULD)** Add end-to-end artifact tests through `parse_toml_file` or `artemis_collection` when wiring a new artifact.
- **T-4 (SHOULD)** Add focused unit tests for tricky decoders, binary parsing, filtering, and timeline conversion.
- **T-5 (MUST)** Preserve `collection_metadata` behavior when touching timeline transforms.
- **T-6 (MUST)** Keep tests compatible with macOS, Linux, Windows, and Windows ARM CI.

## 7 — Logging & Reporting

- **OBS-1 (MUST)** Use the existing `log`-based logging patterns; keep messages artifact-aware and operationally useful.
- **OBS-2 (MUST)** Never log secrets, API keys, or sensitive remote credentials.
- **OBS-3 (MUST)** Preserve report generation, log upload, marker handling, compression, and output counting in the collection pipeline.
- **OBS-4 (SHOULD)** Log enough context to diagnose which artifact or stage failed without dumping unnecessary raw data.

## 8 — Performance

- **PERF-1 (MUST)** Be conservative with allocations and JSON cloning on hot parser paths.
- **PERF-2 (SHOULD)** Stream or iterate large inputs when the format allows it instead of loading entire sources eagerly.
- **PERF-3 (SHOULD)** Prefer focused decoding and filtering over full-structure materialization when only a subset is needed.
- **PERF-4 (CAN)** Add or extend benchmarks for parser hotspots when changing expensive artifact logic.

## 9 — Configuration & Output

- **CFG-1 (MUST)** `ArtemisToml` and `Output` are the canonical config and output contract for CLI, TOML, JS, and daemon flows.
- **CFG-2 (MUST)** Do not create parallel config models for the same collection behavior.
- **CFG-3 (MUST)** Timeline output is always `jsonl`.
- **CFG-4 (MUST)** Keep output backends under `forensics/src/output/`; local and remote paths must share the same parsed artifact model.
- **CFG-5 (SHOULD)** Keep JavaScript execution and filter behavior inside `forensics/src/runtime/`.

## 10 — APIs & Boundaries

- **API-1 (MUST)** Keep public surfaces minimal; expose only what other crates actually need.
- **API-2 (MUST)** `timeline/` transforms parsed JSON output; it should not become a second raw parser stack.
- **API-3 (SHOULD)** Keep filesystem helpers, parser logic, and output logic separated as they are today.
- **API-4 (MUST)** If behavior already exists in `forensics`, reuse it from `cli/` and `daemon/` rather than duplicating it.

## 11 — Security & Project Constraints

- **SEC-1 (MUST)** `unsafe` is not allowed across the workspace unless maintainers explicitly approve an exception.
- **SEC-2 (MUST)** Shelling out to external tools is not allowed in product code.
- **SEC-3 (MUST)** Submitting collected data to third-party services is not allowed.
- **SEC-4 (SHOULD)** Avoid system APIs when a native Rust parser is viable. The main exceptions are volatile artifacts such as processes and live network connections.
- **SEC-5 (MUST)** Treat remote upload, enrollment, and credential handling as sensitive paths.

## 12 — CI/CD

- **CI-1 (MUST)** CI must continue to pass across the existing PR workflows: fmt, clippy, tests, coverage, audit, and cargo-deny.
- **CI-2 (MUST)** Keep cross-platform support intact; do not introduce changes that silently work on only one host OS.
- **CI-3 (SHOULD)** If dependency posture changes, update `deny.toml` and validate the cargo-deny workflow behavior.
- **CI-4 (SHOULD)** Preserve packaging and release assumptions used by `.justfile`, `.packages/`, and GitHub Actions.

## 13 — Tooling

- **TL-1 (MUST)** Use `just` targets for focused local development when possible.
- **TL-2 (SHOULD)** Use `cargo test --no-run --release` for fast compile validation when changing broad code paths.
- **TL-3 (CAN)** Use targeted `just` commands such as `just registry`, `just runtime`, `just timeline`, `just linux`, `just macos`, and `just windows` during development.

## 14 — Tooling Gates

- **G-1 (MUST)** `cargo fmt -- --check` passes.
- **G-2 (MUST)** `just` passes.
- **G-3 (MUST)** `cargo test --release` passes for relevant changes.
- **G-4 (SHOULD)** `just _coverage` is run when changing core parser, output, or timeline behavior.

## Writing Parser Functions Best Practices

1. Is the parser easy to follow without mentally simulating every branch?
2. Does the function mix format decoding, filtering, and output shaping in one place?
3. Can repeated binary parsing logic be extracted into a helper with a typed return?
4. Are there needless clones, temporary `Value` allocations, or ownership moves?
5. Would an enum, options struct, or named helper make the parser behavior clearer?

## Do

- Keep `cli/` and `daemon/` thin over `forensics`.
- Follow the current OS-first module layout.
- Reuse existing structs, helpers, and fixture patterns.
- Wire artifacts through config, dispatch, tests, and timeline support together.
- Preserve reporting, marker, compression, and output behavior.

## Don't

- Add parser logic directly to `cli/`, `daemon/`, or `common/`.
- Introduce duplicate config models next to `ArtemisToml` and `Output`.
- Bypass existing collection, logging, or report flow in `forensics/src/artifacts/collection.rs`.
- Treat Artemis as single-platform; the repo and CI are explicitly multi-platform.
- Add one-off artifacts without the full pipeline and tests.
```