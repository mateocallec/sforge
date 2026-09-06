# sforge

A curl-equivalent HTTP client with named, reusable browser sessions — built for penetration
testers juggling multiple accounts across multiple browser profiles. Byte-exact by design: header
order and casing are sent exactly as given, useful for smuggling and parser-differential testing,
not just convenience requests.

Part of [SessionForge](https://github.com/mateocallec/sforge); pairs with the
[browser extension](https://github.com/mateocallec/sforge/tree/main/extension) that captures a
profile's cookies into a ready-to-paste `sforge session setup ...` command.

## Install

```sh
cargo install sforge
```

## Quickstart

```sh
sforge session setup "acme-admin" --set-cookie 'session_id=abc123; Domain=app.acme.test; Path=/; Secure; HttpOnly'
sforge --session acme-admin https://app.acme.test/api/users -v

sforge -X POST https://app.acme.test/api/login -d '{"user":"a"}' --json

sforge session list
sforge session
```

Run `sforge --help` and `sforge session --help` for the full flag reference, or see the
[repository README](https://github.com/mateocallec/sforge#readme) for the complete picture.

## License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
