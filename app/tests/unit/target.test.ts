// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import { isEditableTarget } from "../../src/shortcuts/target";
import { isShortcutBlocked } from "../../src/shortcuts/useShortcuts";

describe("isEditableTarget", () => {
  it("is true for form controls and contentEditable elements", () => {
    expect(isEditableTarget(document.createElement("select"))).toBe(true);
    expect(isEditableTarget(document.createElement("button"))).toBe(true);
    expect(isEditableTarget(document.createElement("input"))).toBe(true);
    expect(isEditableTarget(document.createElement("textarea"))).toBe(true);
    const editable = document.createElement("div");
    editable.contentEditable = "true";
    expect(isEditableTarget(editable)).toBe(true);
  });
  it("is false for a plain element or null", () => {
    expect(isEditableTarget(document.createElement("div"))).toBe(false);
    expect(isEditableTarget(null)).toBe(false);
  });
});

function keydown(target: EventTarget, key: string): KeyboardEvent {
  return { key, target } as unknown as KeyboardEvent;
}

describe("isShortcutBlocked", () => {
  it("lets a focused button through for every key except Enter and Space", () => {
    const button = document.createElement("button");
    expect(isShortcutBlocked(keydown(button, "Enter"))).toBe(true);
    expect(isShortcutBlocked(keydown(button, " "))).toBe(true);
    expect(isShortcutBlocked(keydown(button, "z"))).toBe(false);
    expect(isShortcutBlocked(keydown(button, "5"))).toBe(false);
    expect(isShortcutBlocked(keydown(button, "Delete"))).toBe(false);
  });
  it("still blocks every key for real editable targets", () => {
    expect(isShortcutBlocked(keydown(document.createElement("input"), "z"))).toBe(true);
    expect(isShortcutBlocked(keydown(document.createElement("select"), "5"))).toBe(true);
    expect(isShortcutBlocked(keydown(document.createElement("textarea"), "Delete"))).toBe(true);
    const editable = document.createElement("div");
    editable.contentEditable = "true";
    expect(isShortcutBlocked(keydown(editable, "z"))).toBe(true);
  });
  it("lets a plain element through", () => {
    expect(isShortcutBlocked(keydown(document.createElement("div"), "z"))).toBe(false);
  });
});
