export async function persistThemePreference({
  preferences,
  nextTheme,
  applyTheme,
  renderThemeSelector,
  savePreferences,
}) {
  const previousTheme = preferences.theme;

  preferences.theme = nextTheme;
  applyTheme();
  renderThemeSelector();

  try {
    await savePreferences();
  } catch (error) {
    preferences.theme = previousTheme;
    applyTheme();
    renderThemeSelector();
    throw error;
  }
}
