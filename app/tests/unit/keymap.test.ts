import { describe, expect, it } from "vitest";
import { matchShortcut } from "../../src/shortcuts/keymap";
import { typeOpacityDigit } from "../../src/shortcuts/useShortcuts";

function ev(key: string, mods: Partial<{ ctrlKey: boolean; shiftKey: boolean; altKey: boolean }> = {}): KeyboardEvent {
  return { key, ctrlKey: false, shiftKey: false, altKey: false, metaKey: false, ...mods } as KeyboardEvent;
}

describe("keymap", () => {
  it("maps Photoshop-style shortcuts", () => {
    expect(matchShortcut(ev("s", { ctrlKey: true }))).toBe("save");
    expect(matchShortcut(ev("S", { ctrlKey: true, shiftKey: true }))).toBe("save-as");
    expect(matchShortcut(ev("z", { ctrlKey: true }))).toBe("undo");
    expect(matchShortcut(ev("Z", { ctrlKey: true, shiftKey: true }))).toBe("redo");
    expect(matchShortcut(ev("y", { ctrlKey: true }))).toBe("redo");
    expect(matchShortcut(ev("c"))).toBe("tool-crop");
    expect(matchShortcut(ev("c", { ctrlKey: true, altKey: true }))).toBe("canvas-size");
    expect(matchShortcut(ev("0", { ctrlKey: true }))).toBe("fit");
    expect(matchShortcut(ev("=", { ctrlKey: true }))).toBe("zoom-in");
    expect(matchShortcut(ev("+", { ctrlKey: true, shiftKey: true }))).toBe("zoom-in");
    expect(matchShortcut(ev("=", { ctrlKey: true, shiftKey: true }))).toBe("zoom-in");
    expect(matchShortcut(ev("Enter"))).toBe("apply");
    expect(matchShortcut(ev("Escape"))).toBe("cancel");
    expect(matchShortcut(ev("Enter", { ctrlKey: true }))).toBeNull();
    expect(matchShortcut(ev("x"))).toBeNull();
    expect(matchShortcut(ev("ArrowLeft"))).toBe("nudge-left");
    expect(matchShortcut(ev("ArrowRight", { shiftKey: true }))).toBe("nudge-right");
    expect(matchShortcut(ev("ArrowUp"))).toBe("nudge-up");
    expect(matchShortcut(ev("ArrowDown", { shiftKey: true }))).toBe("nudge-down");
  });

  it("maps layer shortcuts", () => {
    expect(matchShortcut(ev("j", { ctrlKey: true }))).toBe("duplicate");
    expect(matchShortcut(ev("g", { ctrlKey: true }))).toBe("group");
    expect(matchShortcut(ev("g", { ctrlKey: true, altKey: true }))).toBe("clip");
    expect(matchShortcut(ev("e", { ctrlKey: true }))).toBe("merge");
    expect(matchShortcut(ev("]", { ctrlKey: true }))).toBe("layer-up");
    expect(matchShortcut(ev("+", { shiftKey: true }))).toBe("blend-next");
    expect(matchShortcut(ev("_", { shiftKey: true }))).toBe("blend-prev");
    expect(matchShortcut(ev("5"))).toBe("opacity-5");
    expect(matchShortcut(ev("Delete"))).toBe("delete-layer");
  });
});

describe("typeOpacityDigit", () => {
  it("applies the first digit immediately, combines a second digit within 600ms, and restarts after the buffer expires", () => {
    const calls: number[] = [];
    const apply = (opacity: number) => calls.push(opacity);
    typeOpacityDigit(5, 1000, apply);
    expect(calls).toEqual([0.5]);
    typeOpacityDigit(2, 2000, apply);
    expect(calls).toEqual([0.5, 0.2]);
    typeOpacityDigit(5, 2300, apply);
    expect(calls).toEqual([0.5, 0.2, 0.25]);
    typeOpacityDigit(0, 5000, apply);
    expect(calls).toEqual([0.5, 0.2, 0.25, 1]);
  });
});
