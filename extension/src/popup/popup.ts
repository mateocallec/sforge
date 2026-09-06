import { computeBaseDomain } from "../lib/base-domain";
import { api } from "../lib/browser-api";
import { buildSetupCommand } from "../lib/command-builder";
import { collectCookies } from "../lib/cookies";
import { detectOs, type Os } from "../lib/os-detect";
import { loadSettings, saveSettings, type OsOverride } from "../lib/storage";

const mainView = requireElement<HTMLElement>("main-view");
const settingsView = requireElement<HTMLElement>("settings-view");
const settingsButton = requireElement<HTMLButtonElement>("settings-button");
const doneButton = requireElement<HTMLButtonElement>("done-button");
const sessionNameDisplay = requireElement<HTMLElement>("session-name-display");
const copyButton = requireElement<HTMLButtonElement>("copy-button");
const mainMessage = requireElement<HTMLElement>("main-message");
const includeUaMainCheckbox = requireElement<HTMLInputElement>("include-ua-checkbox");
const sessionNameInput = requireElement<HTMLInputElement>("session-name-input");
const sessionNameError = requireElement<HTMLElement>("session-name-error");
const osSelect = requireElement<HTMLSelectElement>("os-select");

function requireElement<T extends HTMLElement>(id: string): T {
  const el = document.getElementById(id);
  if (!el) throw new Error(`missing #${id} in popup.html`);
  return el as T;
}

function showMessage(text: string): void {
  mainMessage.textContent = text;
  mainMessage.hidden = false;
}

function clearMessage(): void {
  mainMessage.hidden = true;
  mainMessage.textContent = "";
}

function resolveOs(override: OsOverride): Os {
  return override === "auto" ? detectOs() : override;
}

async function showView(hasSessionName: boolean): Promise<void> {
  mainView.hidden = hasSessionName === false;
  settingsView.hidden = hasSessionName === true;
}

async function handleDone(): Promise<void> {
  const name = sessionNameInput.value.trim();
  if (!name) {
    sessionNameError.hidden = false;
    sessionNameInput.focus();
    return;
  }
  sessionNameError.hidden = true;

  await saveSettings({ sessionName: sessionNameInput.value });
  await refreshMainView();
  await showView(true);
}

async function refreshMainView(): Promise<void> {
  const settings = await loadSettings();
  sessionNameDisplay.textContent = settings.sessionName;
  includeUaMainCheckbox.checked = settings.includeUserAgent;
}

async function handleCopy(): Promise<void> {
  clearMessage();

  const settings = await loadSettings();
  const [tab] = await api.tabs.query({ active: true, currentWindow: true });
  const url = tab?.url;
  if (!url) {
    showMessage("No active tab URL available.");
    return;
  }

  const baseDomain = computeBaseDomain(url);
  if (!baseDomain) {
    showMessage("This tab isn't an http(s) page — nothing to capture.");
    return;
  }

  const cookies = await collectCookies(baseDomain);
  if (cookies.length === 0) {
    showMessage(`No cookies found for ${baseDomain}.`);
    return;
  }

  const os = resolveOs(settings.osOverride);
  const command = buildSetupCommand({
    sessionName: settings.sessionName,
    cookies,
    userAgent: settings.includeUserAgent ? navigator.userAgent : undefined,
    os,
  });

  await navigator.clipboard.writeText(command);
  showMessage(`Copied (${cookies.length} cookie${cookies.length === 1 ? "" : "s"}).`);
}

settingsButton.addEventListener("click", () => {
  void showView(false);
});

doneButton.addEventListener("click", () => {
  void handleDone();
});

copyButton.addEventListener("click", () => {
  void handleCopy();
});

includeUaMainCheckbox.addEventListener("change", () => {
  void saveSettings({ includeUserAgent: includeUaMainCheckbox.checked });
});

sessionNameInput.addEventListener("input", () => {
  if (sessionNameInput.value.trim()) {
    sessionNameError.hidden = true;
  }
  void saveSettings({ sessionName: sessionNameInput.value });
});

sessionNameInput.addEventListener("keydown", (event) => {
  if (event.key === "Enter") {
    void handleDone();
  }
});

osSelect.addEventListener("change", () => {
  void saveSettings({ osOverride: osSelect.value as OsOverride });
});

void (async () => {
  const settings = await loadSettings();
  sessionNameInput.value = settings.sessionName;
  osSelect.value = settings.osOverride;

  const hasSessionName = settings.sessionName.trim().length > 0;
  if (hasSessionName) {
    await refreshMainView();
  }
  await showView(hasSessionName);
})();
