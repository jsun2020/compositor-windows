export type ActionId = "new" | "open" | "save" | "save-as" | "export-png" | "export-jpeg" | "close" | "undo" | "redo" | "new-layer"
  | "canvas-size" | "image-size" | "zoom-in" | "zoom-out" | "fit" | "actual" | "tool-move" | "tool-hand" | "tool-zoom" | "tool-crop" | "apply" | "cancel"
  | "nudge-left" | "nudge-right" | "nudge-up" | "nudge-down";

export interface Shortcut { key: string; ctrl?: boolean; shift?: boolean; alt?: boolean; }

export const SHORTCUTS: Record<ActionId, Shortcut[]> = {
  "new": [{ key: "n", ctrl: true }], "open": [{ key: "o", ctrl: true }], "save": [{ key: "s", ctrl: true }],
  "save-as": [{ key: "s", ctrl: true, shift: true }], "export-jpeg": [{ key: "s", ctrl: true, shift: true, alt: true }],
  "export-png": [{ key: "e", ctrl: true, shift: true }], "close": [{ key: "w", ctrl: true }],
  "undo": [{ key: "z", ctrl: true }], "redo": [{ key: "z", ctrl: true, shift: true }, { key: "y", ctrl: true }],
  "new-layer": [{ key: "n", ctrl: true, shift: true }], "canvas-size": [{ key: "c", ctrl: true, alt: true }], "image-size": [{ key: "i", ctrl: true, alt: true }],
  "zoom-in": [{ key: "=", ctrl: true }, { key: "+", ctrl: true, shift: true }, { key: "=", ctrl: true, shift: true }], "zoom-out": [{ key: "-", ctrl: true }], "fit": [{ key: "0", ctrl: true }], "actual": [{ key: "1", ctrl: true }],
  "tool-move": [{ key: "v" }], "tool-hand": [{ key: "h" }], "tool-zoom": [{ key: "z" }], "tool-crop": [{ key: "c" }],
  // Enter/Escape apply or cancel whatever pending edit owns the current tool (crop rect or
  // move-tool transform); runAction routes by tool and by whether an edit is pending.
  "apply": [{ key: "Enter" }], "cancel": [{ key: "Escape" }],
  "nudge-left": [{ key: "ArrowLeft" }, { key: "ArrowLeft", shift: true }],
  "nudge-right": [{ key: "ArrowRight" }, { key: "ArrowRight", shift: true }],
  "nudge-up": [{ key: "ArrowUp" }, { key: "ArrowUp", shift: true }],
  "nudge-down": [{ key: "ArrowDown" }, { key: "ArrowDown", shift: true }],
};

export function matchShortcut(e: KeyboardEvent): ActionId | null {
  const key = e.key.toLowerCase();
  const ctrl = e.ctrlKey || e.metaKey;
  for (const [id, list] of Object.entries(SHORTCUTS) as [ActionId, Shortcut[]][]) {
    for (const s of list) {
      if (s.key.toLowerCase() === key && !!s.ctrl === ctrl && !!s.shift === e.shiftKey && !!s.alt === e.altKey) return id;
    }
  }
  return null;
}
