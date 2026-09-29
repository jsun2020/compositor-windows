export type ActionId = "new" | "open" | "save" | "save-as" | "export-png" | "export-jpeg" | "close" | "undo" | "redo" | "new-layer"
  | "canvas-size" | "image-size" | "zoom-in" | "zoom-out" | "fit" | "actual" | "tool-move" | "tool-hand" | "tool-zoom" | "tool-crop" | "apply" | "cancel"
  | "nudge-left" | "nudge-right" | "nudge-up" | "nudge-down"
  | "new-folder" | "duplicate" | "group" | "merge" | "clip" | "layer-up" | "layer-down" | "blend-next" | "blend-prev" | "delete-layer"
  | "opacity-0" | "opacity-1" | "opacity-2" | "opacity-3" | "opacity-4" | "opacity-5" | "opacity-6" | "opacity-7" | "opacity-8" | "opacity-9"
  | "levels" | "curves" | "hue-saturation" | "invert"
  | "tool-marquee" | "tool-lasso" | "tool-wand" | "select-all" | "deselect" | "select-inverse" | "cycle-tool-mode"
  | "swap-colors" | "default-colors" | "tool-eyedropper" | "fill-foreground" | "fill-background" | "tool-gradient";

export interface Shortcut { key: string; ctrl?: boolean; shift?: boolean; alt?: boolean; }

export const SHORTCUTS = {
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
  "new-folder": [], "duplicate": [{ key: "j", ctrl: true }], "group": [{ key: "g", ctrl: true }], "merge": [{ key: "e", ctrl: true }],
  "clip": [{ key: "g", ctrl: true, alt: true }], "layer-up": [{ key: "]", ctrl: true }], "layer-down": [{ key: "[", ctrl: true }],
  "blend-next": [{ key: "=", shift: true }, { key: "+", shift: true }], "blend-prev": [{ key: "-", shift: true }, { key: "_", shift: true }],
  "delete-layer": [{ key: "Delete" }, { key: "Backspace" }],
  "opacity-0": [{ key: "0" }], "opacity-1": [{ key: "1" }], "opacity-2": [{ key: "2" }], "opacity-3": [{ key: "3" }], "opacity-4": [{ key: "4" }],
  "opacity-5": [{ key: "5" }], "opacity-6": [{ key: "6" }], "opacity-7": [{ key: "7" }], "opacity-8": [{ key: "8" }], "opacity-9": [{ key: "9" }],
  "levels": [{ key: "l", ctrl: true }], "curves": [{ key: "m", ctrl: true }],
  "hue-saturation": [{ key: "u", ctrl: true }], "invert": [{ key: "i", ctrl: true }],
  // The selection tools and the Select menu (KeyboardShortcuts.swift; CompositorApp.swift:194-230).
  "tool-marquee": [{ key: "m" }], "tool-lasso": [{ key: "l" }], "tool-wand": [{ key: "w" }],
  "select-all": [{ key: "a", ctrl: true }], "deselect": [{ key: "d", ctrl: true }], "select-inverse": [{ key: "i", ctrl: true, shift: true }],
  // Tab switches the current tool's kind (EditorCanvas.swift:1823-1827).
  "cycle-tool-mode": [{ key: "Tab" }],
  // The palette (EditorCanvas.swift:1835-1836).
  "swap-colors": [{ key: "x" }], "default-colors": [{ key: "d" }],
  "tool-eyedropper": [{ key: "i" }], "tool-gradient": [{ key: "g" }],
  // Photoshop's fill keys (CompositorApp.swift:175-186: Option-Delete and Command-Delete on the Mac).
  "fill-foreground": [{ key: "Backspace", alt: true }, { key: "Delete", alt: true }],
  "fill-background": [{ key: "Backspace", ctrl: true }, { key: "Delete", ctrl: true }],
} satisfies Record<ActionId, Shortcut[]>;

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
