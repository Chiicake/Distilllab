import {
  createTranslator,
  DEFAULT_LOCALE,
  loadLocaleDictionaries,
  normalizeLocale,
} from "./i18n/translator.js";
import { persistThemePreference } from "./theme-preference.js";

const DEFAULT_PREFERENCES = {
  theme: "system",
  locale: DEFAULT_LOCALE,
  showDebugPanel: true,
};

const DEFAULT_VIEW_STATE = {
  currentView: "chatDraft",
  selectedSession: "draft",
  selectedCanvasScope: "global",
  selectedSettingsSection: "general",
};

const VIEW_DEFINITIONS = {
  chatDraft: { family: "chat", topTab: "chat", rail: "chat", inspector: "chat", session: "draft" },
  chatActive: { family: "chat", topTab: "chat", rail: "chat", inspector: "chat", session: "active" },
  canvasGlobal: { family: "canvas", topTab: "canvas", rail: "canvas", inspector: "canvas", canvasScope: "global" },
  canvasDetail: { family: "canvas", topTab: "canvas", rail: "canvas", inspector: "canvas", canvasScope: "detail" },
  settingsGeneral: { family: "settings", topTab: null, rail: "settings", inspector: "settings", settingsSection: "general" },
  settingsDebug: { family: "settings", topTab: null, rail: "settings", inspector: "settings", settingsSection: "debug" },
};

const state = {
  dictionaries: {},
  preferences: { ...DEFAULT_PREFERENCES },
  view: { ...DEFAULT_VIEW_STATE },
};

let translateImpl = function t(key) {
  throw new Error(`Missing translator for ${key}`);
};

const HAS_DOCUMENT = typeof document !== "undefined";

function createUiStub() {
  return {
    root: null,
    tabChat: null,
    tabCanvas: null,
    settingsButton: null,
    railButtons: [],
    railSections: [],
    inspectorSections: [],
    viewPanels: [],
    chatInspectorStates: [],
    chatTransitionButtons: [],
    debugUnavailable: null,
    localeSelector: null,
    themeSelector: null,
    debugShell: null,
    runButton: null,
    sessionButton: null,
    sourceButton: null,
    chunkSourceButton: null,
    extractWorkItemsButton: null,
    groupProjectButton: null,
    buildAssetsButton: null,
    listRunsButton: null,
    listSessionsButton: null,
    listSourcesButton: null,
    listWorkItemsButton: null,
    listProjectsButton: null,
    listAssetsButton: null,
    listChunksButton: null,
    sourceIdInput: null,
    configProviderInput: null,
    configModelInput: null,
    configProviderNameInput: null,
    configProviderNpmInput: null,
    configBaseUrlInput: null,
    configApiKeyInput: null,
    configProviderJsonInput: null,
    configImportPathInput: null,
    configLoadButton: null,
    configNewButton: null,
    configSaveButton: null,
    configDeleteButton: null,
    configImportButton: null,
    configTestButton: null,
    configResult: null,
    timelineSessionIdInput: null,
    timelineSessionSelector: null,
    timelineMessageInput: null,
    timelineAttachmentsInput: null,
    timelineCreateSessionButton: null,
    timelineRefreshSessionsButton: null,
    timelineSendButton: null,
    timelineRefreshButton: null,
    timelineResult: null,
    result: null,
  };
}

const ui = HAS_DOCUMENT
  ? {
      root: document.documentElement,
      tabChat: getElement("tab-chat"),
      tabCanvas: getElement("tab-canvas"),
      settingsButton: getElement("open-settings-button"),
      railButtons: Array.from(document.querySelectorAll(".rail-item[data-view-target]")),
      railSections: Array.from(document.querySelectorAll("[data-rail-view]")),
      inspectorSections: Array.from(document.querySelectorAll("[data-inspector-view]")),
      viewPanels: Array.from(document.querySelectorAll("[data-view]")),
      chatInspectorStates: Array.from(document.querySelectorAll("[data-chat-inspector-state]")),
      chatTransitionButtons: Array.from(document.querySelectorAll("[data-chat-transition]")),
      debugUnavailable: getElement("debug-unavailable"),
      localeSelector: getElement("locale-selector"),
      themeSelector: getElement("theme-selector"),
      debugShell: getElement("debug-shell"),
      runButton: getElement("create-run-button"),
      sessionButton: getElement("create-session-button"),
      sourceButton: getElement("create-source-button"),
      chunkSourceButton: getElement("chunk-source-button"),
      extractWorkItemsButton: getElement("extract-work-items-button"),
      groupProjectButton: getElement("group-project-button"),
      buildAssetsButton: getElement("build-assets-button"),
      listRunsButton: getElement("list-runs-button"),
      listSessionsButton: getElement("list-sessions-button"),
      listSourcesButton: getElement("list-sources-button"),
      listWorkItemsButton: getElement("list-work-items-button"),
      listProjectsButton: getElement("list-projects-button"),
      listAssetsButton: getElement("list-assets-button"),
      listChunksButton: getElement("list-chunks-button"),
      sourceIdInput: getElement("source-id-input"),
      configProviderInput: getElement("config-provider-input"),
      configModelInput: getElement("config-model-input"),
      configProviderNameInput: getElement("config-provider-name-input"),
      configProviderNpmInput: getElement("config-provider-npm-input"),
      configBaseUrlInput: getElement("config-base-url-input"),
      configApiKeyInput: getElement("config-api-key-input"),
      configProviderJsonInput: getElement("config-provider-json-input"),
      configImportPathInput: getElement("config-import-path-input"),
      configLoadButton: getElement("config-load-button"),
      configNewButton: getElement("config-new-button"),
      configSaveButton: getElement("config-save-button"),
      configDeleteButton: getElement("config-delete-button"),
      configImportButton: getElement("config-import-button"),
      configTestButton: getElement("config-test-button"),
      configResult: getElement("config-result"),
      timelineSessionIdInput: getElement("timeline-session-id-input"),
      timelineSessionSelector: getElement("timeline-session-selector"),
      timelineMessageInput: getElement("timeline-message-input"),
      timelineAttachmentsInput: getElement("timeline-attachments-input"),
      timelineCreateSessionButton: getElement("timeline-create-session-button"),
      timelineRefreshSessionsButton: getElement("timeline-refresh-sessions-button"),
      timelineSendButton: getElement("timeline-send-button"),
      timelineRefreshButton: getElement("timeline-refresh-button"),
      timelineResult: getElement("timeline-result"),
      result: getElement("result"),
    }
  : createUiStub();

const {
  tabChat,
  tabCanvas,
  settingsButton,
  railButtons,
  railSections,
  inspectorSections,
  viewPanels,
  chatInspectorStates,
  chatTransitionButtons,
  debugUnavailable,
  localeSelector,
  themeSelector,
  debugShell,
  runButton,
  sessionButton,
  sourceButton,
  chunkSourceButton,
  extractWorkItemsButton,
  groupProjectButton,
  buildAssetsButton,
  listRunsButton,
  listSessionsButton,
  listSourcesButton,
  listWorkItemsButton,
  listProjectsButton,
  listAssetsButton,
  listChunksButton,
  sourceIdInput,
  configProviderInput,
  configModelInput,
  configProviderNameInput,
  configProviderNpmInput,
  configBaseUrlInput,
  configApiKeyInput,
  configProviderJsonInput,
  configImportPathInput,
  configLoadButton,
  configNewButton,
  configSaveButton,
  configDeleteButton,
  configImportButton,
  configTestButton,
  configResult,
  timelineSessionIdInput,
  timelineSessionSelector,
  timelineMessageInput,
  timelineAttachmentsInput,
  timelineCreateSessionButton,
  timelineRefreshSessionsButton,
  timelineSendButton,
  timelineRefreshButton,
  timelineResult,
  result,
} = ui;

if (configResult && timelineResult && result) {
  configResult.dataset.defaultState = "true";
  timelineResult.dataset.defaultState = "true";
  result.dataset.defaultState = "true";
}

function t(key, replacements) {
  return translateImpl(key, replacements);
}

function getElement(id) {
  const element = document.getElementById(id);

  if (!element) {
    throw new Error(`Expected element #${id}`);
  }

  return element;
}

function normalizeTheme(theme) {
  return ["system", "light", "dark"].includes(theme) ? theme : DEFAULT_PREFERENCES.theme;
}

function normalizeView(viewId) {
  return VIEW_DEFINITIONS[viewId] ? viewId : DEFAULT_VIEW_STATE.currentView;
}

function deriveChatMockStateFromView(viewId) {
  return getViewDefinition(viewId).session === "active" ? "active" : "draft";
}

function resolveChatTransitionView(targetMode = "active") {
  return targetMode === "active" ? "chatActive" : "chatDraft";
}

function getViewDefinition(viewId) {
  return VIEW_DEFINITIONS[normalizeView(viewId)];
}

function setText(target, message) {
  target.textContent = message;
}

function setStatusText(target, key, replacements) {
  target.dataset.defaultState = "false";
  target.textContent = t(key, replacements);
}

function setDefaultText(target, key) {
  target.dataset.defaultState = "true";
  target.textContent = t(key);
}

function setRawText(target, message) {
  target.dataset.defaultState = "false";
  target.textContent = message;
}

function renderLocaleSelector() {
  const options = [
    ["en", t("settings.language.en")],
    ["zh-CN", t("settings.language.zhCN")],
  ];

  localeSelector.innerHTML = "";
  for (const [value, label] of options) {
    const option = document.createElement("option");
    option.value = value;
    option.textContent = label;
    localeSelector.appendChild(option);
  }

  localeSelector.value = state.preferences.locale;
}

function renderThemeSelector() {
  const options = [
    ["system", t("settings.appearance.theme.system")],
    ["light", t("settings.appearance.theme.light")],
    ["dark", t("settings.appearance.theme.dark")],
  ];

  themeSelector.innerHTML = "";
  for (const [value, label] of options) {
    const option = document.createElement("option");
    option.value = value;
    option.textContent = label;
    themeSelector.appendChild(option);
  }

  themeSelector.value = state.preferences.theme;
}

function renderDefaultStatusText() {
  if (configResult.dataset.defaultState !== "false") {
    setDefaultText(configResult, "status.noConfig");
  }

  if (timelineResult.dataset.defaultState !== "false") {
    setDefaultText(timelineResult, "status.noTimeline");
  }

  if (result.dataset.defaultState !== "false") {
    setDefaultText(result, "status.noResult");
  }
}

function renderTimelineSelectorPlaceholder() {
  const placeholderText = t("debug.timeline.placeholder.selectSession");
  const firstOption = timelineSessionSelector.options[0];

  if (firstOption && firstOption.value === "") {
    firstOption.textContent = placeholderText;
    return;
  }

  const placeholder = document.createElement("option");
  placeholder.value = "";
  placeholder.textContent = placeholderText;
  timelineSessionSelector.insertBefore(placeholder, timelineSessionSelector.firstChild);
}

function renderStaticTranslations() {
  document.documentElement.lang = state.preferences.locale;
  document.title = "Distilllab";

  for (const element of document.querySelectorAll("[data-i18n]")) {
    setText(element, t(element.dataset.i18n));
  }

  for (const element of document.querySelectorAll("[data-i18n-placeholder]")) {
    element.setAttribute("placeholder", t(element.dataset.i18nPlaceholder));
  }

  for (const element of document.querySelectorAll("[data-i18n-aria-label]")) {
    element.setAttribute("aria-label", t(element.dataset.i18nAriaLabel));
  }

  renderLocaleSelector();
  renderThemeSelector();
  renderTimelineSelectorPlaceholder();
  renderDefaultStatusText();
  renderShellView();
}

function updateTranslator() {
  translateImpl = createTranslator(state.dictionaries, state.preferences.locale);
}

function resolveTheme(theme) {
  const normalizedTheme = normalizeTheme(theme);

  if (normalizedTheme !== "system") {
    return normalizedTheme;
  }

  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function renderTopTabs(viewDefinition) {
  const topTabs = [
    [tabChat, "chat"],
    [tabCanvas, "canvas"],
  ];

  for (const [button, tabId] of topTabs) {
    const active = viewDefinition.topTab === tabId;
    button.dataset.active = String(active);
    button.setAttribute("aria-selected", String(active));
  }

  settingsButton.dataset.active = String(viewDefinition.family === "settings");
  settingsButton.setAttribute("aria-expanded", String(viewDefinition.family === "settings"));
}

function renderViewPanels(currentView) {
  for (const panel of viewPanels) {
    panel.hidden = panel.dataset.view !== currentView;
  }
}

function renderSurfaceSections(attributeName, activeValue, sections) {
  for (const section of sections) {
    section.hidden = section.dataset[attributeName] !== activeValue;
  }
}

function renderChatInspectorState(chatState) {
  for (const section of chatInspectorStates) {
    section.hidden = section.dataset.chatInspectorState !== chatState;
  }
}

function renderRailSelection(currentView) {
  for (const button of railButtons) {
    const targetView = normalizeView(button.dataset.viewTarget);
    const active = targetView === currentView;
    button.dataset.active = String(active);
    button.setAttribute("aria-current", active ? "page" : "false");
  }
}

function renderShellView() {
  const currentView = normalizeView(state.view.currentView);
  const viewDefinition = getViewDefinition(currentView);
  const chatState = deriveChatMockStateFromView(currentView);

  renderTopTabs(viewDefinition);
  renderViewPanels(currentView);
  renderSurfaceSections("railView", viewDefinition.rail, railSections);
  renderSurfaceSections("inspectorView", viewDefinition.inspector, inspectorSections);
  renderRailSelection(currentView);

  if (viewDefinition.family === "chat") {
    renderChatInspectorState(chatState);
  }

  const showingDebugView = currentView === "settingsDebug";
  const debugAvailable = state.preferences.showDebugPanel;
  debugShell.hidden = !showingDebugView || !debugAvailable;
  debugUnavailable.hidden = !showingDebugView || debugAvailable;
}

function transitionToView(viewId) {
  const currentView = normalizeView(viewId);
  const definition = getViewDefinition(currentView);

  state.view.currentView = currentView;

  if (definition.session) {
    state.view.selectedSession = definition.session;
  }

  if (definition.canvasScope) {
    state.view.selectedCanvasScope = definition.canvasScope;
  }

  if (definition.settingsSection) {
    state.view.selectedSettingsSection = definition.settingsSection;
  }

  renderShellView();
}

function transitionChatMockSurface(targetMode = "active") {
  const nextView = resolveChatTransitionView(targetMode);
  transitionToView(nextView);
  return nextView;
}

async function invokeTauri(commandName, args) {
  const invoke = window.__TAURI_INTERNALS__?.invoke;

  if (!invoke) {
    throw new Error("Tauri bridge is not available in the current page context.");
  }

  return await invoke(commandName, args);
}

async function loadDesktopPreferences() {
  try {
    const preferencesText = await invokeTauri("load_desktop_ui_preferences_command");
    const preferences = JSON.parse(preferencesText);

    state.preferences = {
      theme: normalizeTheme(preferences?.theme),
      locale: normalizeLocale(preferences?.locale),
      showDebugPanel: typeof preferences?.showDebugPanel === "boolean"
        ? preferences.showDebugPanel
        : DEFAULT_PREFERENCES.showDebugPanel,
    };
  } catch (error) {
    state.preferences = { ...DEFAULT_PREFERENCES };
    console.warn("Failed to load desktop UI preferences.", error);
  }
}

async function saveDesktopPreferences() {
  const response = await invokeTauri("save_desktop_ui_preferences_command", {
    preferences: state.preferences,
  });
  const savedPreferences = JSON.parse(response);

  state.preferences = {
    theme: normalizeTheme(savedPreferences?.theme),
    locale: normalizeLocale(savedPreferences?.locale),
    showDebugPanel: typeof savedPreferences?.showDebugPanel === "boolean"
      ? savedPreferences.showDebugPanel
      : DEFAULT_PREFERENCES.showDebugPanel,
  };
}

function applyDesktopThemePreference() {
  const resolvedTheme = resolveTheme(state.preferences.theme);

  document.documentElement.dataset.theme = resolvedTheme;
  document.documentElement.dataset.themePreference = state.preferences.theme;
}

async function setTheme(theme, options = {}) {
  const persist = options.persist !== false;
  const nextTheme = normalizeTheme(theme);

  if (persist) {
    await persistThemePreference({
      preferences: state.preferences,
      nextTheme,
      applyTheme: applyDesktopThemePreference,
      renderThemeSelector,
      savePreferences: saveDesktopPreferences,
    });
    return;
  }

  state.preferences.theme = nextTheme;
  applyDesktopThemePreference();
  renderThemeSelector();
}

async function setLocale(locale, options = {}) {
  const persist = options.persist !== false;
  const nextLocale = normalizeLocale(locale);

  state.preferences.locale = nextLocale;
  updateTranslator();
  renderStaticTranslations();

  if (persist) {
    await saveDesktopPreferences();
    updateTranslator();
    renderStaticTranslations();
  }
}

async function refreshTimeline() {
  const sessionId = timelineSessionIdInput.value.trim();

  if (!sessionId) {
    setStatusText(timelineResult, "status.error.enterSessionId");
    return;
  }

  setStatusText(timelineResult, "status.loadingTimeline");

  try {
    const response = await invokeTauri("list_session_messages_command", { sessionId });
    setRawText(timelineResult, response);
  } catch (error) {
    setStatusText(timelineResult, "status.error.generic", { error });
  }
}

async function refreshSessionSelector() {
  try {
    const response = await invokeTauri("list_session_selector_options");
    const sessions = JSON.parse(response);
    const currentValue = timelineSessionSelector.value || timelineSessionIdInput.value.trim();

    timelineSessionSelector.innerHTML = "";
    renderTimelineSelectorPlaceholder();

    for (const session of sessions) {
      const option = document.createElement("option");
      option.value = session.sessionId;
      option.textContent = session.label;
      timelineSessionSelector.appendChild(option);
    }

    if (currentValue && sessions.some((session) => session.sessionId === currentValue)) {
      timelineSessionSelector.value = currentValue;
    }
  } catch (error) {
    setStatusText(result, "status.error.generic", { error });
  }
}

async function switchToSession(sessionId) {
  if (!sessionId) {
    timelineSessionIdInput.value = "";
    setStatusText(timelineResult, "status.noActiveSession");
    setStatusText(result, "status.noActiveSession");
    return;
  }

  timelineSessionIdInput.value = sessionId;
  transitionToView("chatActive");
  const selectedOption = timelineSessionSelector.selectedOptions?.[0];
  if (selectedOption && selectedOption.value) {
    setStatusText(result, "status.switchedToSession", {
      sessionId: selectedOption.textContent,
    });
  } else {
    setStatusText(result, "status.switchedToSession", { sessionId });
  }
  await refreshTimeline();
}

function rebuildProviderOptions(config, selectedProvider) {
  configProviderInput.innerHTML = "";

  const providerIds = Object.keys(config?.provider ?? {});
  for (const providerId of providerIds) {
    const option = document.createElement("option");
    option.value = providerId;
    option.textContent = providerId;
    configProviderInput.appendChild(option);
  }

  if (selectedProvider) {
    configProviderInput.value = selectedProvider;
  }
}

function rebuildModelOptions(config, selectedProvider, selectedModel) {
  configModelInput.innerHTML = "";

  const models = config?.provider?.[selectedProvider]?.models ?? {};
  for (const modelId of Object.keys(models)) {
    const option = document.createElement("option");
    option.value = modelId;
    option.textContent = modelId;
    configModelInput.appendChild(option);
  }

  if (selectedModel && models[selectedModel]) {
    configModelInput.value = selectedModel;
  } else if (configModelInput.options.length > 0) {
    configModelInput.selectedIndex = 0;
  }
}

function syncConfigFormFromJson(configJson) {
  const config = JSON.parse(configJson);
  const currentProvider = config?.distilllab?.currentProvider ?? "";
  const currentModel = config?.distilllab?.currentModel ?? "";
  const providerEntry = config?.provider?.[currentProvider] ?? null;

  rebuildProviderOptions(config, currentProvider);
  rebuildModelOptions(config, currentProvider, currentModel);
  configProviderNameInput.value = providerEntry?.name ?? "";
  configProviderNpmInput.value = providerEntry?.npm ?? "@ai-sdk/openai-compatible";
  configBaseUrlInput.value = providerEntry?.options?.baseURL ?? "";
  configApiKeyInput.value = providerEntry?.options?.apiKey ?? "";
  configProviderJsonInput.value = providerEntry ? JSON.stringify(providerEntry, null, 2) : "";
}

async function refreshProviderEditorFromCurrentSelection() {
  try {
    const rawJson = await invokeTauri("load_llm_config_json_command");
    const config = JSON.parse(rawJson);
    const currentProvider = configProviderInput.value.trim();
    const currentModel = configModelInput.value.trim();
    const providerEntry = config?.provider?.[currentProvider] ?? null;

    rebuildModelOptions(config, currentProvider, currentModel);
    configProviderNameInput.value = providerEntry?.name ?? "";
    configProviderNpmInput.value = providerEntry?.npm ?? "@ai-sdk/openai-compatible";
    configBaseUrlInput.value = providerEntry?.options?.baseURL ?? "";
    configApiKeyInput.value = providerEntry?.options?.apiKey ?? "";
    configProviderJsonInput.value = providerEntry ? JSON.stringify(providerEntry, null, 2) : "";
  } catch (error) {
    setStatusText(configResult, "status.error.generic", { error });
  }
}

async function invokeCommand(commandName, loadingMessageKey) {
  setStatusText(result, loadingMessageKey);

  try {
    const response = await invokeTauri(commandName);
    setRawText(result, response);
  } catch (error) {
    setStatusText(result, "status.error.generic", { error });
  }
}

async function loadConfigSummary() {
  setStatusText(configResult, "status.loadingConfig");

  try {
    const summary = await invokeTauri("load_llm_config_command");
    const rawJson = await invokeTauri("load_llm_config_json_command");
    setRawText(configResult, summary);
    syncConfigFormFromJson(rawJson);
  } catch (error) {
    setStatusText(configResult, "status.error.generic", { error });
  }
}

function bindShellViewEvents() {
  tabChat.addEventListener("click", () => {
    transitionToView(state.view.selectedSession === "active" ? "chatActive" : "chatDraft");
  });

  tabCanvas.addEventListener("click", () => {
    transitionToView(state.view.selectedCanvasScope === "detail" ? "canvasDetail" : "canvasGlobal");
  });

  settingsButton.addEventListener("click", () => {
    transitionToView(state.view.selectedSettingsSection === "debug" ? "settingsDebug" : "settingsGeneral");
  });

  for (const button of railButtons) {
    if (!button.classList.contains("rail-item")) {
      continue;
    }

    button.addEventListener("click", () => {
      transitionToView(button.dataset.viewTarget);
    });
  }

  for (const button of chatTransitionButtons) {
    button.addEventListener("click", () => {
      transitionChatMockSurface(button.dataset.chatTransition);
    });
  }
}

function bindShellEvents() {
  bindShellViewEvents();

  localeSelector.addEventListener("change", async () => {
    try {
      await setLocale(localeSelector.value);
    } catch (error) {
      setStatusText(result, "status.error.generic", { error });
    }
  });

  themeSelector.addEventListener("change", async () => {
    try {
      await setTheme(themeSelector.value);
    } catch (error) {
      setStatusText(result, "status.error.generic", { error });
    }
  });

  window.matchMedia("(prefers-color-scheme: dark)").addEventListener("change", () => {
    if (state.preferences.theme === "system") {
      applyDesktopThemePreference();
    }
  });

  configLoadButton.addEventListener("click", async () => {
    await loadConfigSummary();
  });

  configProviderInput.addEventListener("change", async () => {
    const providerId = configProviderInput.value.trim();

    try {
      const rawJson = await invokeTauri("load_llm_config_json_command");
      const config = JSON.parse(rawJson);
      rebuildModelOptions(config, providerId, null);

      const modelId = configModelInput.value.trim();
      const response = await invokeTauri("set_current_provider_model_command", {
        providerId,
        modelId,
      });
      setRawText(configResult, response);
      await refreshProviderEditorFromCurrentSelection();
    } catch (error) {
      setStatusText(configResult, "status.error.generic", { error });
    }
  });

  configModelInput.addEventListener("change", async () => {
    const providerId = configProviderInput.value.trim();
    const modelId = configModelInput.value.trim();

    try {
      const response = await invokeTauri("set_current_provider_model_command", {
        providerId,
        modelId,
      });
      setRawText(configResult, response);
      await refreshProviderEditorFromCurrentSelection();
    } catch (error) {
      setStatusText(configResult, "status.error.generic", { error });
    }
  });

  configSaveButton.addEventListener("click", async () => {
    const currentProvider = configProviderInput.value.trim();
    const currentModel = configModelInput.value.trim();

    if (!currentProvider || !currentModel) {
      setStatusText(configResult, "status.error.providerModelRequired");
      return;
    }

    setStatusText(configResult, "status.savingConfig");

    try {
      const response = await invokeTauri("save_llm_config_command", {
        form: {
          currentProvider,
          currentModel,
          providerName: configProviderNameInput.value.trim(),
          providerNpm: configProviderNpmInput.value.trim(),
          baseUrl: configBaseUrlInput.value.trim(),
          apiKey: configApiKeyInput.value,
          rawProviderJson: configProviderJsonInput.value,
        },
      });
      setRawText(configResult, response);
    } catch (error) {
      setStatusText(configResult, "status.error.generic", { error });
    }
  });

  configNewButton.addEventListener("click", async () => {
    const providerId = window.prompt(t("prompt.newProviderId"), "new-provider");
    if (!providerId || !providerId.trim()) {
      return;
    }

    setStatusText(configResult, "status.creatingProvider");

    try {
      const response = await invokeTauri("create_provider_command", {
        providerId: providerId.trim(),
      });
      setRawText(configResult, response);

      const rawJson = await invokeTauri("load_llm_config_json_command");
      syncConfigFormFromJson(rawJson);
    } catch (error) {
      setStatusText(configResult, "status.error.generic", { error });
    }
  });

  configDeleteButton.addEventListener("click", async () => {
    const providerId = configProviderInput.value.trim();
    if (!providerId) {
      setStatusText(configResult, "status.error.noProviderSelected");
      return;
    }

    const confirmed = window.confirm(t("confirm.deleteProvider", { providerId }));
    if (!confirmed) {
      return;
    }

    setStatusText(configResult, "status.deletingProvider");

    try {
      const response = await invokeTauri("delete_provider_command", {
        providerId,
      });
      setRawText(configResult, response);

      const rawJson = await invokeTauri("load_llm_config_json_command");
      syncConfigFormFromJson(rawJson);
    } catch (error) {
      setStatusText(configResult, "status.error.generic", { error });
    }
  });

  configImportButton.addEventListener("click", async () => {
    setStatusText(configResult, "status.importingProviders");

    try {
      const sourcePath = configImportPathInput.value.trim();
      const response = await invokeTauri("import_opencode_providers_command", {
        form: {
          sourcePath,
        },
      });
      setRawText(configResult, response);

      const rawJson = await invokeTauri("load_llm_config_json_command");
      syncConfigFormFromJson(rawJson);
    } catch (error) {
      setStatusText(configResult, "status.error.generic", { error });
    }
  });

  configTestButton.addEventListener("click", async () => {
    setStatusText(configResult, "status.testingProvider");

    try {
      const response = await invokeTauri("test_current_provider_command");
      setRawText(configResult, response);
    } catch (error) {
      setStatusText(configResult, "status.error.generic", { error });
    }
  });

  timelineCreateSessionButton.addEventListener("click", async () => {
    setStatusText(timelineResult, "status.creatingSession");

    try {
      const response = await invokeTauri("create_demo_session");
      const match = response.match(/created session: (session-[^\s]+)/);

      if (!match) {
        setStatusText(timelineResult, "status.createdSessionCouldNotParse", { response });
        return;
      }

      const sessionId = match[1];
      timelineSessionIdInput.value = sessionId;
      state.view.selectedSession = "active";
      setStatusText(timelineResult, "status.usingSession", { sessionId });
      await refreshSessionSelector();
      timelineSessionSelector.value = sessionId;
      transitionToView("chatActive");
      await refreshTimeline();
    } catch (error) {
      setStatusText(timelineResult, "status.error.generic", { error });
    }
  });

  timelineRefreshSessionsButton.addEventListener("click", async () => {
    setStatusText(result, "status.loadingSessions");
    await refreshSessionSelector();
    setStatusText(result, "status.sessionsRefreshed");
  });

  timelineSessionSelector.addEventListener("change", async () => {
    const sessionId = timelineSessionSelector.value.trim();
    await switchToSession(sessionId);
  });

  timelineSendButton.addEventListener("click", async () => {
    const sessionId = timelineSessionIdInput.value.trim();
    const userMessage = timelineMessageInput.value.trim();
    const attachmentPaths = timelineAttachmentsInput.value
      .split("\n")
      .map((value) => value.trim())
      .filter(Boolean);

    if (!sessionId || !userMessage) {
      setStatusText(timelineResult, "status.error.sessionIdMessageRequired");
      return;
    }

    state.view.selectedSession = "active";
    setStatusText(timelineResult, "status.sendingSessionMessage");
    setStatusText(result, "status.previewingSessionIntake");

    try {
      const preview = await invokeTauri("preview_session_intake_command", {
        form: {
          sessionId,
          userMessage,
          attachmentPaths,
        },
      });
      setRawText(result, preview);

      const response = await invokeTauri("send_session_message_command", {
        form: {
          sessionId,
          userMessage,
          attachmentPaths,
        },
      });
      setRawText(timelineResult, response);
      await refreshSessionSelector();
      timelineSessionSelector.value = sessionId;
      timelineMessageInput.value = "";
      transitionToView("chatActive");
    } catch (error) {
      setStatusText(timelineResult, "status.error.generic", { error });
      setStatusText(result, "status.error.generic", { error });
    }
  });

  timelineRefreshButton.addEventListener("click", async () => {
    await refreshTimeline();
  });

  runButton.addEventListener("click", async () => {
    await invokeCommand("create_demo_run", "status.creatingDemoRun");
  });

  sessionButton.addEventListener("click", async () => {
    await invokeCommand("create_demo_session", "status.creatingDemoSession");
  });

  sourceButton.addEventListener("click", async () => {
    await invokeCommand("create_demo_source", "status.creatingDemoSource");
  });

  chunkSourceButton.addEventListener("click", async () => {
    await invokeCommand("chunk_demo_source", "status.chunkingDemoSource");
  });

  extractWorkItemsButton.addEventListener("click", async () => {
    await invokeCommand("extract_demo_work_items", "status.extractingDemoWorkItems");
  });

  groupProjectButton.addEventListener("click", async () => {
    await invokeCommand("group_demo_project", "status.groupingDemoProject");
  });

  buildAssetsButton.addEventListener("click", async () => {
    await invokeCommand("build_demo_assets", "status.buildingDemoAssets");
  });

  listRunsButton.addEventListener("click", async () => {
    await invokeCommand("list_runs", "status.loadingRuns");
  });

  listSessionsButton.addEventListener("click", async () => {
    await invokeCommand("list_sessions", "status.loadingSessions");
  });

  listSourcesButton.addEventListener("click", async () => {
    await invokeCommand("list_sources", "status.loadingSources");
  });

  listWorkItemsButton.addEventListener("click", async () => {
    await invokeCommand("list_work_items", "status.loadingWorkItems");
  });

  listProjectsButton.addEventListener("click", async () => {
    await invokeCommand("list_projects", "status.loadingProjects");
  });

  listAssetsButton.addEventListener("click", async () => {
    await invokeCommand("list_assets", "status.loadingAssets");
  });

  listChunksButton.addEventListener("click", async () => {
    const sourceId = sourceIdInput.value.trim();

    if (!sourceId) {
      setStatusText(result, "status.error.enterSourceId");
      return;
    }

    setStatusText(result, "status.loadingChunks");

    try {
      const response = await invokeTauri("list_chunks_for_source", { sourceId });
      setRawText(result, response);
    } catch (error) {
      setStatusText(result, "status.error.generic", { error });
    }
  });
}

async function bootstrap() {
  state.dictionaries = await loadLocaleDictionaries();
  await loadDesktopPreferences();
  updateTranslator();
  applyDesktopThemePreference();
  renderStaticTranslations();
  bindShellEvents();
  await loadConfigSummary();
  await refreshSessionSelector();
}

if (
  typeof window !== "undefined"
  && typeof document !== "undefined"
  && window.__DISTILLLAB_SKIP_DESKTOP_BOOTSTRAP__ !== true
) {
  bootstrap().catch((error) => {
    console.error("Failed to bootstrap desktop shell.", error);
    result.textContent = String(error);
  });
}

export function createShellViewState(initial = {}) {
  const viewState = {
    ...DEFAULT_VIEW_STATE,
    ...initial,
  };

  return {
    get currentView() {
      return normalizeView(viewState.currentView);
    },
    get selectedSession() {
      return viewState.selectedSession;
    },
    get selectedCanvasScope() {
      return viewState.selectedCanvasScope;
    },
    get selectedSettingsSection() {
      return viewState.selectedSettingsSection;
    },
    transition(nextView) {
      const normalizedView = normalizeView(nextView);
      const definition = getViewDefinition(normalizedView);

      viewState.currentView = normalizedView;
      if (definition.session) {
        viewState.selectedSession = definition.session;
      }
      if (definition.canvasScope) {
        viewState.selectedCanvasScope = definition.canvasScope;
      }
      if (definition.settingsSection) {
        viewState.selectedSettingsSection = definition.settingsSection;
      }

      return {
        currentView: viewState.currentView,
        selectedSession: viewState.selectedSession,
        selectedCanvasScope: viewState.selectedCanvasScope,
        selectedSettingsSection: viewState.selectedSettingsSection,
      };
    },
  };
}

export {
  deriveChatMockStateFromView,
  resolveChatTransitionView,
  transitionChatMockSurface,
};
