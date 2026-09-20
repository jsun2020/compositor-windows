// @vitest-environment jsdom
import { describe, expect, it } from "vitest";
import { isEditableTarget } from "../../src/shortcuts/target";

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
