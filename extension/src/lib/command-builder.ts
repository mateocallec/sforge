import type { CookieDetails } from "./browser-api";
import type { Os } from "./os-detect";
import { quoteForOs } from "./shell-quote";

const WEEKDAYS = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/**
 * Must match `cli/src/session/cookie.rs`'s `HTTP_DATE` format exactly:
 * `"[weekday repr:short], [day] [month repr:short] [year] [hour]:[minute]:[second] GMT"`.
 * Both sides of the setup-command contract need to agree on this format
 * byte-for-byte, since the CLI parses whatever this produces.
 */
function formatHttpDate(epochSeconds: number): string {
  const date = new Date(epochSeconds * 1000);
  const weekday = WEEKDAYS[date.getUTCDay()];
  const day = String(date.getUTCDate()).padStart(2, "0");
  const month = MONTHS[date.getUTCMonth()];
  const year = date.getUTCFullYear();
  const hours = String(date.getUTCHours()).padStart(2, "0");
  const minutes = String(date.getUTCMinutes()).padStart(2, "0");
  const seconds = String(date.getUTCSeconds()).padStart(2, "0");
  return `${weekday}, ${day} ${month} ${year} ${hours}:${minutes}:${seconds} GMT`;
}

function mapSameSite(value: CookieDetails["sameSite"]): "Strict" | "Lax" | "None" | null {
  switch (value) {
    case "strict":
      return "Strict";
    case "lax":
      return "Lax";
    case "no_restriction":
      return "None";
    default:
      return null;
  }
}

/**
 * Renders one cookie as a `--set-cookie` fragment reusing real Set-Cookie
 * attribute syntax verbatim — a format pentesters already know, not a
 * bespoke one.
 */
export function cookieToSetCookieFragment(cookie: CookieDetails): string {
  let fragment = `${cookie.name}=${cookie.value}; Domain=${cookie.domain}; Path=${cookie.path}`;
  if (cookie.secure) fragment += "; Secure";
  if (cookie.httpOnly) fragment += "; HttpOnly";
  const sameSite = mapSameSite(cookie.sameSite);
  if (sameSite) fragment += `; SameSite=${sameSite}`;
  if (typeof cookie.expirationDate === "number") {
    fragment += `; Expires=${formatHttpDate(cookie.expirationDate)}`;
  }
  return fragment;
}

export interface BuildCommandOptions {
  sessionName: string;
  cookies: CookieDetails[];
  userAgent?: string;
  os: Os;
}

/**
 * The canonical `sforge session setup ...` one-liner. One `--set-cookie`
 * per captured cookie; the CLI's inline-replace path treats that set as a
 * full replace of the session's cookie array, which is what lets this
 * single generated line serve as both create and update.
 */
export function buildSetupCommand(options: BuildCommandOptions): string {
  const parts = ["sforge", "session", "setup", quoteForOs(options.sessionName, options.os)];

  for (const cookie of options.cookies) {
    parts.push("--set-cookie", quoteForOs(cookieToSetCookieFragment(cookie), options.os));
  }

  if (options.userAgent) {
    parts.push("-H", quoteForOs(`User-Agent: ${options.userAgent}`, options.os));
  }

  return parts.join(" ");
}
