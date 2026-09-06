# SessionForge

A curl-equivalent HTTP client with named, reusable browser sessions — built for penetration testers who juggle multiple accounts across multiple browser profiles.

[![Crates.io](https://img.shields.io/crates/v/sforge.svg)](https://crates.io/crates/sforge)
[![CI](https://github.com/mateocallec/sforge/actions/workflows/cli-ci.yml/badge.svg)](https://github.com/mateocallec/sforge/actions/workflows/cli-ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

## What it is

Testing a complex SaaS product usually means several browser profiles, each authenticated as a different account. SessionForge is two pieces that close the gap between "logged in as this account in Chrome" and "a working curl command for that account":

- **`sforge`** (in [`cli/`](cli/)) — a Rust CLI that behaves like `curl`, with an added `--session` concept: a named, persisted set of cookies/headers/auth you can attach to any request. The HTTP layer is hand-rolled for byte-exact control — header order and casing are sent exactly as given, useful for smuggling and parser-differential testing, not just convenience requests.
- **The SessionForge extension** (in [`extension/`](extension/)) — a Chrome/Edge/Firefox extension. One click copies a ready-to-run `sforge session setup ...` command carrying the active browser profile's cookies, so creating or refreshing a session is a paste, not a manual devtools cookie hunt.

## Install

```sh
cargo install sforge
```

The browser extension will be available on the Chrome Web Store and Firefox Add-ons once published; until then, see [`extension/README.md`](extension/README.md) for loading it unpacked.

## Quickstart

```sh
# Capture a session: click the SessionForge extension icon in your target
# browser profile, click Copy, then paste the command it gives you.
sforge session setup "acme-admin" --set-cookie 'session_id=abc123; Domain=app.acme.test; Path=/; Secure; HttpOnly'

# Use it on any request.
sforge --session acme-admin https://app.acme.test/api/users -v

# Sessions are just curl-like flags — everything works without one too.
sforge -X POST https://app.acme.test/api/login -d '{"user":"a"}' --json

# List, inspect, or manage sessions interactively.
sforge session list
sforge session
```

See [`cli/README.md`](cli/README.md) for the full flag reference.

## Repository layout

- [`cli/`](cli/) — the Rust CLI, published to crates.io as `sforge`.
- [`extension/`](extension/) — the browser extension, published to the Chrome Web Store and Firefox Add-ons.

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md). Participation is governed by the
[Code of Conduct](CODE_OF_CONDUCT.md).

## Security

See [`SECURITY.md`](SECURITY.md) for how to report a vulnerability, and [`PRIVACY.md`](PRIVACY.md) for what the browser extension does and does not access.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

## Citation

If SessionForge is useful in your work, citation details are in [`CITATION.cff`](CITATION.cff)
(also available via GitHub's "Cite this repository" button).
