import test from "node:test";
import assert from "node:assert/strict";

import { persistThemePreference } from "./theme-preference.js";

test("reverts theme state and UI hooks when persistence fails", async () => {
  const preferences = {
    theme: "system",
    locale: "en",
    showDebugPanel: true,
  };
  const appliedThemes = [];
  const renderedThemes = [];

  await assert.rejects(
    persistThemePreference({
      preferences,
      nextTheme: "dark",
      applyTheme: () => {
        appliedThemes.push(preferences.theme);
      },
      renderThemeSelector: () => {
        renderedThemes.push(preferences.theme);
      },
      savePreferences: async () => {
        throw new Error("save failed");
      },
    }),
    /save failed/
  );

  assert.equal(preferences.theme, "system");
  assert.deepEqual(appliedThemes, ["dark", "system"]);
  assert.deepEqual(renderedThemes, ["dark", "system"]);
});

test("keeps the new theme when persistence succeeds", async () => {
  const preferences = {
    theme: "system",
    locale: "en",
    showDebugPanel: true,
  };
  const appliedThemes = [];
  const renderedThemes = [];

  await persistThemePreference({
    preferences,
    nextTheme: "light",
    applyTheme: () => {
      appliedThemes.push(preferences.theme);
    },
    renderThemeSelector: () => {
      renderedThemes.push(preferences.theme);
    },
    savePreferences: async () => {},
  });

  assert.equal(preferences.theme, "light");
  assert.deepEqual(appliedThemes, ["light"]);
  assert.deepEqual(renderedThemes, ["light"]);
});
