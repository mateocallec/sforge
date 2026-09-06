# Privacy policy — SessionForge browser extension

This policy covers the SessionForge browser extension (Chrome, Edge, Firefox). It does not cover the `sforge` CLI, which runs locally on your machine and is covered by its own documentation in [`cli/README.md`](cli/README.md).

## What the extension accesses

- **Cookies** for the domain of your active browser tab, including its subdomains — read via the browser's `cookies` API only when you open the extension popup or click Copy. The extension never reads cookies for sites you haven't got open.
- **The active tab's URL**, to determine which domain's cookies to read.
- **The active tab's User-Agent string**, only if you enable the "include User-Agent" setting.

## What the extension stores

Your session name, OS preference, and the User-Agent toggle are stored locally via the browser's `storage.local` API, scoped to the single browser profile the extension is installed in. This is a deliberate choice, not an oversight: settings do not sync across devices or browser profiles, because a session name is meaningful only for the one profile/account it was configured for.

## What the extension sends

Nothing. The extension makes no network requests of its own — there is no telemetry, no analytics, and no server component. The only thing that leaves the popup is whatever you explicitly copy to your clipboard by clicking the Copy button, and that only goes wherever you paste it.

## What the extension does not do

- It does not transmit cookies, session names, or settings anywhere.
- It does not track your browsing.
- It does not read cookies or page content for any site other than the active tab's domain, and only on your explicit action.

## Changes

Any change to this policy will be reflected in this file, in the repository's version history.

## Contact

See [`SECURITY.md`](SECURITY.md) for how to reach maintainers regarding this extension.
