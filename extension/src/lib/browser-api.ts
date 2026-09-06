/**
 * A minimal, hand-typed slice of the WebExtension API surface — just the
 * handful of calls this extension actually makes. Both Chrome's `chrome.*`
 * (promise-based since Manifest V3) and Firefox's native `browser.*` match
 * this shape, so no `webextension-polyfill` dependency is needed.
 */

export interface CookieDetails {
  name: string;
  value: string;
  domain: string;
  path: string;
  secure: boolean;
  httpOnly: boolean;
  sameSite?: "no_restriction" | "lax" | "strict" | "unspecified";
  expirationDate?: number;
}

export interface Tab {
  id?: number;
  url?: string;
}

interface CookiesApi {
  getAll(details: { domain?: string }): Promise<CookieDetails[]>;
}

interface TabsApi {
  query(queryInfo: { active: boolean; currentWindow: boolean }): Promise<Tab[]>;
}

interface StorageArea {
  get(keys?: string | string[] | null): Promise<Record<string, unknown>>;
  set(items: Record<string, unknown>): Promise<void>;
}

interface StorageApi {
  local: StorageArea;
}

interface RuntimeApi {
  onInstalled: { addListener(callback: () => void): void };
}

export interface BrowserApi {
  cookies: CookiesApi;
  tabs: TabsApi;
  storage: StorageApi;
  runtime: RuntimeApi;
}

function resolveApi(): BrowserApi {
  const globals = globalThis as unknown as Record<string, BrowserApi | undefined>;
  const found = globals.browser ?? globals.chrome;
  if (!found) {
    throw new Error("no WebExtension API found on globalThis (browser/chrome)");
  }
  return found;
}

export const api: BrowserApi = resolveApi();
