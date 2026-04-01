import test from "node:test";
import assert from "node:assert/strict";

import {
  createShellViewState,
  deriveCanvasInspectorStateFromView,
  deriveChatStateFromView,
  deriveChatMockStateFromView,
  isDebugPanelVisible,
  parseTimelineEntries,
  reconcileSelectedSessionId,
  resolveChatTransitionView,
} from "./desktop-shell.js";

test("draft and active views derive expected chat mock state", () => {
  assert.equal(deriveChatMockStateFromView("chatDraft"), "draft");
  assert.equal(deriveChatMockStateFromView("chatActive"), "active");
  assert.equal(deriveChatMockStateFromView("canvasGlobal"), "draft");
});

test("chat state stays draft until a real session id exists", () => {
  assert.equal(deriveChatStateFromView("chatDraft", ""), "draft");
  assert.equal(deriveChatStateFromView("chatActive", ""), "draft");
  assert.equal(deriveChatStateFromView("chatActive", "session-123"), "active");
  assert.equal(deriveChatStateFromView("settingsGeneral", "session-123"), "draft");
});

test("canvas views derive expected inspector state", () => {
  assert.equal(deriveCanvasInspectorStateFromView("canvasGlobal"), "global");
  assert.equal(deriveCanvasInspectorStateFromView("canvasDetail"), "detail");
  assert.equal(deriveCanvasInspectorStateFromView("chatDraft"), "global");
});

test("chat transition helper maps requested modes to chat views", () => {
  assert.equal(resolveChatTransitionView("active"), "chatActive");
  assert.equal(resolveChatTransitionView("draft"), "chatDraft");
  assert.equal(resolveChatTransitionView("other"), "chatDraft");
});

test("timeline parser keeps blank lines inside a message body", () => {
  const entries = parseTimelineEntries([
    "[User]",
    "  First line",
    "  ",
    "  Second paragraph",
    "[Assistant]",
    "  Reply line",
  ].join("\n"));

  assert.deepEqual(entries, [
    { header: "[User]", body: "First line\n\nSecond paragraph" },
    { header: "[Assistant]", body: "Reply line" },
  ]);
});

test("selected session falls back to draft when refreshed options no longer include it", () => {
  assert.equal(
    reconcileSelectedSessionId("session-missing", [
      { sessionId: "session-1" },
      { sessionId: "session-2" },
    ]),
    "",
  );
});

test("selected session stays active when refreshed options still include it", () => {
  assert.equal(
    reconcileSelectedSessionId("session-2", [
      { sessionId: "session-1" },
      { sessionId: "session-2" },
    ]),
    "session-2",
  );
});

test("shell view state preserves a real selected chat session through transitions", () => {
  const shell = createShellViewState({
    currentView: "chatActive",
    selectedSession: "active",
    selectedSessionId: "session-123",
  });

  assert.equal(shell.currentView, "chatActive");
  assert.equal(shell.selectedSession, "active");
  assert.equal(shell.selectedSessionId, "session-123");

  let snapshot = shell.transition("chatActive");
  assert.equal(snapshot.currentView, "chatActive");
  assert.equal(snapshot.selectedSession, "active");
  assert.equal(snapshot.selectedSessionId, "session-123");

  snapshot = shell.transition("settingsGeneral");
  assert.equal(snapshot.currentView, "settingsGeneral");
  assert.equal(snapshot.selectedSession, "active");

  snapshot = shell.transition("chatDraft");
  assert.equal(snapshot.currentView, "chatDraft");
  assert.equal(snapshot.selectedSession, "active");
});

test("shell view state keeps chat draft default when no real session id is selected", () => {
  const shell = createShellViewState({
    currentView: "chatActive",
    selectedSession: "active",
    selectedSessionId: "",
  });

  const snapshot = shell.transition("chatActive");

  assert.equal(snapshot.currentView, "chatActive");
  assert.equal(snapshot.selectedSession, "draft");
  assert.equal(snapshot.selectedSessionId, "");
});

test("shell view state tracks selected canvas scope through transitions", () => {
  const shell = createShellViewState();

  let snapshot = shell.transition("canvasGlobal");
  assert.equal(snapshot.currentView, "canvasGlobal");
  assert.equal(snapshot.selectedCanvasScope, "global");

  snapshot = shell.transition("canvasDetail");
  assert.equal(snapshot.currentView, "canvasDetail");
  assert.equal(snapshot.selectedCanvasScope, "detail");

  snapshot = shell.transition("settingsGeneral");
  assert.equal(snapshot.currentView, "settingsGeneral");
  assert.equal(snapshot.selectedCanvasScope, "detail");
});

test("shell view state preserves the most recent canvas scope when returning", () => {
  const shell = createShellViewState({ currentView: "canvasDetail", selectedCanvasScope: "detail" });

  const snapshot = shell.transition("canvasGlobal");

  assert.equal(snapshot.currentView, "canvasGlobal");
  assert.equal(snapshot.selectedCanvasScope, "global");
});

test("shell view state tracks selected settings section through transitions", () => {
  const shell = createShellViewState();

  let snapshot = shell.transition("settingsDebug");
  assert.equal(snapshot.currentView, "settingsDebug");
  assert.equal(snapshot.selectedSettingsSection, "debug");

  snapshot = shell.transition("chatDraft");
  assert.equal(snapshot.currentView, "chatDraft");
  assert.equal(snapshot.selectedSettingsSection, "debug");

  snapshot = shell.transition("settingsGeneral");
  assert.equal(snapshot.currentView, "settingsGeneral");
  assert.equal(snapshot.selectedSettingsSection, "general");
});

test("shell view state honors an initial debug settings selection", () => {
  const shell = createShellViewState({
    currentView: "settingsDebug",
    selectedSettingsSection: "debug",
  });

  assert.equal(shell.currentView, "settingsDebug");
  assert.equal(shell.selectedSettingsSection, "debug");
});

test("debug panel visibility follows preference in non-development contexts", () => {
  assert.equal(isDebugPanelVisible({ showDebugPanel: true }, false), true);
  assert.equal(isDebugPanelVisible({ showDebugPanel: false }, false), false);
});

test("debug panel visibility stays enabled in development contexts", () => {
  assert.equal(isDebugPanelVisible({ showDebugPanel: false }, true), true);
});
