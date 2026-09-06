export type Os = "windows" | "macos" | "linux";

interface NavigatorUAData {
  platform?: string;
}

/**
 * `navigator.userAgentData` (Chrome/Edge) is checked first; Firefox and
 * older Chrome fall back to parsing `navigator.userAgent`.
 */
export function detectOs(): Os {
  const uaData = (navigator as Navigator & { userAgentData?: NavigatorUAData }).userAgentData;
  const platformHint = uaData?.platform?.toLowerCase();
  if (platformHint) {
    if (platformHint.includes("win")) return "windows";
    if (platformHint.includes("mac")) return "macos";
    if (platformHint.includes("linux")) return "linux";
  }

  const ua = navigator.userAgent.toLowerCase();
  if (ua.includes("windows")) return "windows";
  if (ua.includes("mac os") || ua.includes("macintosh")) return "macos";
  return "linux";
}
