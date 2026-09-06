import { api } from "./lib/browser-api";

// No lifecycle setup is needed today; this is registered as the extension's
// background context so one exists to extend later without a manifest change.
api.runtime.onInstalled.addListener(() => {});
