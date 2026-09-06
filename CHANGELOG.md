# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/). The CLI (`cli/`) and the browser extension
(`extension/`) version independently; entries below are tagged `[cli]` or `[extension]`
accordingly.

## [Unreleased]

## [1.0.0] - 2026-09-06

Initial public release.

### Added

- `[cli]` A byte-exact HTTP/1.1 engine: header order and casing are sent exactly as given, useful
  for smuggling and parser-differential testing rather than just convenience requests. TLS via
  rustls, HTTP and SOCKS5 proxying (plain relay and `CONNECT` tunneling), redirects with per-hop
  cookie re-matching, chunked transfer encoding, gzip/deflate/br decompression, multipart form
  bodies, and `--retry`.
- `[cli]` Basic, Bearer, and Digest (RFC 7616) authentication.
- `[cli]` Named sessions: `session setup` (inline replace or `$EDITOR`), `session list`,
  `session delete`, and an interactive `session` TUI for browsing, filtering, editing, and
  deleting sessions, with secrets masked by default.
- `[cli]` A curl-compatible flag set (`-X`, `-H`, `-d`/`--data*`, `--json`, `-F`, `-T`, `-b`,
  `--set-cookie`, `-u`, `-x`, `--resolve`, `-L`, `-v`, `-o`/`-O`, `-i`/`-I`, `--compressed`, and
  more), colorized output, and automatic JSON highlighting.
- `[extension]` A popup with a one-click "copy setup command" flow and a settings view (session
  name, OS override, User-Agent toggle), scoped to the browser profile via `storage.local`.
- `[extension]` Cookie capture across a target's registrable domain and all of its subdomains —
  including subdomains never visited in the active tab — with a command builder that matches the
  CLI's `--set-cookie` grammar exactly.
- `[extension]` Chrome, Edge, and Firefox support (Manifest V3) from a single source tree.
