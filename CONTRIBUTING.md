# Contributing

Thanks for considering a contribution to SessionForge. This project follows a
[Code of Conduct](CODE_OF_CONDUCT.md) — participation in any capacity means agreeing to abide by
it.

SessionForge is two independent projects sharing one repository:

- [`cli/`](cli/) — Rust
- [`extension/`](extension/) — TypeScript

Changes to one rarely require changes to the other. The one thing they share is the setup-command
grammar described in [`cli/README.md`](cli/README.md) and implemented in
`extension/src/lib/command-builder.ts` — if you change one side of that contract, update the
other.

## Reporting bugs

Open an [issue](https://github.com/mateocallec/sforge/issues/new/choose) using the bug report
template. Include the affected component, your OS/browser, and steps to reproduce. If the bug is
a security vulnerability, follow [`SECURITY.md`](SECURITY.md) instead of opening a public issue.

## Suggesting features

Open an issue using the feature request template. Explain the problem you're trying to solve, not
just the solution you have in mind — it makes it easier to evaluate alternatives.

## Development setup

### CLI (`cli/`)

Requires a recent stable Rust toolchain (see `cli/rust-toolchain.toml`).

```sh
cd cli
cargo build
cargo test
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
```

`cargo test` runs both unit tests and the `assert_cmd`-based integration tests in `cli/tests/`.
Integration tests point at an isolated temp directory via the `SFORGE_DATA_DIR` environment
variable and never touch a real session store.

### Extension (`extension/`)

Requires Node.js 20+.

```sh
cd extension
npm ci
npm run typecheck
npm run lint
npm run format:check
npm run build      # writes dist/chrome and dist/firefox
```

To load a build for manual testing:

- **Chrome/Edge**: `chrome://extensions` → enable Developer mode → Load unpacked → select
  `extension/dist/chrome`.
- **Firefox**: `about:debugging#/runtime/this-firefox` → Load Temporary Add-on → select any file
  in `extension/dist/firefox`. Or run `npx web-ext run` from `extension/`.

### Both, from the repo root

```sh
scripts/format.sh   # rustfmt + prettier
scripts/build.sh    # cargo build + npm run build
```

Each accepts an optional `cli` or `extension` argument to run against just one project.

## Commit style

Plain, descriptive commit messages in English, present tense ("add session index NFC
normalization", not "added" or "adds"). No AI-attribution boilerplate beyond what your own
tooling appends automatically.

## Pull requests

Keep CLI and extension changes in separate PRs unless a change genuinely spans both (e.g. a
change to the shared command grammar). Include how you tested the change — `cargo test` output, a
manual command transcript, or `web-ext lint` output as relevant.
