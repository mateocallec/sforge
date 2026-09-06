import { getDomain } from "tldts";

/**
 * The registrable domain (eTLD+1) of `url`, or `null` for a non-http(s)
 * URL (chrome://, about:, file://, the new-tab page, ...) or one tldts
 * can't resolve to a registrable domain.
 *
 * `allowPrivateDomains: true` matters for correctness, not just
 * robustness: without it, a target hosted on a PSL *private*-section
 * platform (e.g. `acmecorp.some-saas-platform.io`, alongside real entries
 * like `herokuapp.com`/`github.io`) would incorrectly widen to the
 * platform's own apex domain rather than the tenant's actual boundary.
 * With it, `some-saas-platform.io`-style entries are themselves treated as
 * the effective suffix, correctly yielding `acmecorp.some-saas-platform.io`.
 */
export function computeBaseDomain(url: string): string | null {
  let parsed: URL;
  try {
    parsed = new URL(url);
  } catch {
    return null;
  }
  if (parsed.protocol !== "http:" && parsed.protocol !== "https:") {
    return null;
  }
  return getDomain(url, { allowPrivateDomains: true });
}
