import type { Os } from "./os-detect";

/** Single-quoted, embedded `'` escaped as `'\''` — POSIX shell. */
export function quotePosix(value: string): string {
  return `'${value.split("'").join("'\\''")}'`;
}

/**
 * Single-quoted, embedded `'` doubled — PowerShell. Single quotes are
 * specifically the safe choice here: PowerShell double-quoted strings
 * interpret `$` for variable expansion, which an arbitrary cookie value
 * could otherwise trigger.
 */
export function quotePowerShell(value: string): string {
  return `'${value.split("'").join("''")}'`;
}

export function quoteForOs(value: string, os: Os): string {
  return os === "windows" ? quotePowerShell(value) : quotePosix(value);
}
