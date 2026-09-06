import { api, type CookieDetails } from "./browser-api";

/**
 * `cookies.getAll({ domain })` matches the given domain *and its
 * subdomains* per the API's own documented filter semantics, over the
 * whole profile's cookie store — this is what captures cookies for
 * subdomains the active tab never visited, with no extra logic needed.
 */
export async function collectCookies(baseDomain: string): Promise<CookieDetails[]> {
  return api.cookies.getAll({ domain: baseDomain });
}
