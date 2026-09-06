# SessionForge extension

A Chrome/Edge/Firefox extension that captures the active browser profile's cookies into a
ready-to-paste `sforge session setup ...` command. See the repo-root [README](../README.md) for
the project overview and [`cli/`](../cli/) for the command-line tool this pairs with.

## Building

Requires Node.js 20+.

```sh
npm ci
npm run typecheck
npm run lint
npm run build
```

`npm run build` writes two independent, ready-to-load extensions:

- `dist/chrome/` — Chrome and Edge (Manifest V3, `background.service_worker`)
- `dist/firefox/` — Firefox (Manifest V3, `background.scripts`, plus the
  `browser_specific_settings.gecko` block AMO requires)

Both are built from the same `src/` — only `manifest/chrome.json` and `manifest/firefox.json`
differ, merged onto the shared `manifest/base.json`.

## Loading for manual testing

**Chrome / Edge**: open `chrome://extensions` (or `edge://extensions`), enable Developer mode,
click "Load unpacked", and select `dist/chrome`.

**Firefox**: open `about:debugging#/runtime/this-firefox`, click "Load Temporary Add-on", and
select any file inside `dist/firefox` (e.g. `manifest.json`). Or run it directly:

```sh
npx web-ext run --source-dir=dist/firefox
```

Lint the Firefox build before every submission:

```sh
npx web-ext lint --source-dir=dist/firefox
```

## Packaging

```sh
npm run package
```

Writes `dist/sessionforge-chrome.zip` and `dist/sessionforge-firefox.zip`, ready to upload to the
Chrome Web Store Developer Dashboard and the Firefox Add-on Developer Hub respectively. Edge uses
the Chrome package unmodified — no separate Edge build.

## Firefox AMO source submission

The build is bundled (esbuild) but deliberately **not minified**, specifically so AMO reviewers
can reproduce the exact submitted output from source. When submitting to AMO, attach the source
alongside the package:

- `src/`, `manifest/`, `scripts/`
- `../icons/` (repo-root, shared across projects)
- `package.json` and `package-lock.json` (exact-pinned versions)
- `tsconfig.json`, `eslint.config.js`

Reproduction steps for the reviewer: `npm ci && npm run build`, then compare `dist/firefox/` byte
for byte against the submitted package. Node version: see `package.json` engines once pinned, or
whatever LTS was current at submission time.

## Architecture notes

- **No network requests.** The extension never talks to any server — see [`PRIVACY.md`](../PRIVACY.md).
  Cookies are read locally and the generated command only ever goes to the clipboard.
- **`storage.local` only**, never `storage.sync` — settings are scoped to the one browser profile
  they were configured in, deliberately not synced across devices.
- **Cookie capture** matches the active tab's registrable domain _and its subdomains_, including
  subdomains never visited in that tab, via `cookies.getAll({ domain })`'s own filter semantics.
  The registrable domain itself is computed with `tldts` (`allowPrivateDomains: true`, so a target
  on a PSL private-section platform like `*.some-saas-platform.io` resolves to the tenant's actual
  boundary rather than the platform's apex).
- **The setup-command grammar** (`src/lib/command-builder.ts`) must stay byte-for-byte in sync with
  the CLI's parser (`cli/src/session/cookie.rs` and `cli/src/args/session_args.rs`) — in particular
  the `Expires=` HTTP-date format both sides use.

## Icons

Sized PNGs (`icon-16.png`, `icon-32.png`, `icon-48.png`, `icon-128.png`) live in `../icons/` at
the repo root, shared across projects rather than duplicated under `extension/`. `npm run build`
copies them into `dist/<target>/icons/`. `../icons/icon-512.png` is store-listing artwork only —
not referenced by the manifest, so it isn't copied into the build.

## Known gaps

- `src/icons/sforge.svg` (a master vector source, for re-exporting the sized PNGs above if the
  artwork changes) is still a zero-byte placeholder — the PNGs themselves were supplied directly.
- Firefox for Android requires a higher `strict_min_version` (142) than desktop Firefox (140) for
  the `data_collection_permissions` manifest key; not addressed, since Android isn't a target
  platform for this tool.
