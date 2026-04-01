import test from "node:test";
import assert from "node:assert/strict";

import {
  createShellViewState,
  deriveCanvasInspectorStateFromView,
  deriveChatMockStateFromView,
  resolveChatTransitionView,
} from "./desktop-shell.js";

test("draft and active views derive expected chat mock state", () => {
  assert.equal(deriveChatMockStateFromView("chatDraft"), "draft");
  assert.equal(deriveChatMockStateFromView("chatActive"), "active");
  assert.equal(deriveChatMockStateFromView("canvasGlobal"), "draft");
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

test("shell view state tracks selected chat session through transitions", () => {
  const shell = createShellViewState();

  assert.equal(shell.currentView, "chatDraft");
  assert.equal(shell.selectedSession, "draft");

  let snapshot = shell.transition("chatActive");
  assert.equal(snapshot.currentView, "chatActive");
  assert.equal(snapshot.selectedSession, "active");

  snapshot = shell.transition("settingsGeneral");
  assert.equal(snapshot.currentView, "settingsGeneral");
  assert.equal(snapshot.selectedSession, "active");

  snapshot = shell.transition("chatDraft");
  assert.equal(snapshot.currentView, "chatDraft");
  assert.equal(snapshot.selectedSession, "draft");
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
