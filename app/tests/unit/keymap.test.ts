import { describe, expect, it } from "vitest";
import { matchShortcut } from "../../src/shortcuts/keymap";

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
    expect(matchShortcut(ev("x"))).toBeNull();
  });
});
