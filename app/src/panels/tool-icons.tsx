// Tool rail icons. The Mac draws its tools with SF Symbols, which Apple licenses for Apple
// platforms only, so this port uses the closest Lucide icons (lucide-static 1.48.0), copied here
// rather than added as a dependency. The polygonal lasso is drawn for this port in the same style,
// as the Mac draws its own (PolygonalLassoToolIcon).
//
// Lucide icons: ISC License, Copyright (c) 2026 Lucide Icons and Contributors.
// Permission to use, copy, modify, and/or distribute this software for any purpose with or without
// fee is hereby granted, provided that the above copyright notice and this permission notice appear
// in all copies. THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH REGARD
// TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL
// THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT,
// NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR PERFORMANCE OF
// THIS SOFTWARE.
import type { ReactNode } from "react";
import type { Tool } from "../state/store";
import type { LassoKind, MarqueeKind } from "../tools/selection-draft";

/** Which icon a tool shows: the Marquee and the Lasso follow their mode, as on the Mac
 *  (ContentView.swift: circle.dashed in Ellipse mode, its own icon for the polygonal lasso). */
export type ToolIconName = "move" | "marquee-rectangle" | "marquee-ellipse" | "lasso-freehand" | "lasso-polygonal"
  | "wand" | "crop" | "hand" | "zoom";

export function toolIconName(tool: Tool, marquee: MarqueeKind, lasso: LassoKind): ToolIconName {
  switch (tool) {
    case "marquee": return marquee === "Ellipse" ? "marquee-ellipse" : "marquee-rectangle";
    case "lasso": return lasso === "Polygonal" ? "lasso-polygonal" : "lasso-freehand";
    default: return tool;
  }
}

const SHAPES: Record<ToolIconName, ReactNode> = {
  // lucide move-diagonal-2 (SF arrow.up.left.and.arrow.down.right)
  "move": <><path d="M19 13v6h-6" /><path d="M5 11V5h6" /><path d="m5 5 14 14" /></>,
  // lucide square-dashed (SF rectangle.dashed)
  "marquee-rectangle": <>
    <path d="M5 3a2 2 0 0 0-2 2" /><path d="M19 3a2 2 0 0 1 2 2" /><path d="M21 19a2 2 0 0 1-2 2" /><path d="M5 21a2 2 0 0 1-2-2" />
    <path d="M9 3h1" /><path d="M9 21h1" /><path d="M14 3h1" /><path d="M14 21h1" />
    <path d="M3 9v1" /><path d="M21 9v1" /><path d="M3 14v1" /><path d="M21 14v1" />
  </>,
  // lucide circle-dashed (SF circle.dashed)
  "marquee-ellipse": <>
    <path d="M10.1 2.182a10 10 0 0 1 3.8 0" /><path d="M13.9 21.818a10 10 0 0 1-3.8 0" />
    <path d="M17.609 3.721a10 10 0 0 1 2.69 2.7" /><path d="M2.182 13.9a10 10 0 0 1 0-3.8" />
    <path d="M20.279 17.609a10 10 0 0 1-2.7 2.69" /><path d="M21.818 10.1a10 10 0 0 1 0 3.8" />
    <path d="M3.721 6.391a10 10 0 0 1 2.7-2.69" /><path d="M6.391 20.279a10 10 0 0 1-2.69-2.7" />
  </>,
  // lucide lasso (SF lasso)
  "lasso-freehand": <><path d="M3.704 14.467a10 8 0 1 1 3.115 2.375" /><path d="M7 22a5 5 0 0 1-2-3.994" /><circle cx="5" cy="16" r="2" /></>,
  // Drawn for this port: lucide's lasso with its loop made of straight sides.
  "lasso-polygonal": <><path d="M3.9 14.2 4.6 6 12 2.8l8.4 2.4.9 7.3-7.2 4.4-7.3.3" /><path d="M7 22a5 5 0 0 1-2-3.994" /><circle cx="5" cy="16" r="2" /></>,
  // lucide wand-sparkles (SF wand.and.stars)
  "wand": <>
    <path d="m21.64 3.64-1.28-1.28a1.21 1.21 0 0 0-1.72 0L2.36 18.64a1.21 1.21 0 0 0 0 1.72l1.28 1.28a1.2 1.2 0 0 0 1.72 0L21.64 5.36a1.2 1.2 0 0 0 0-1.72" />
    <path d="m14 7 3 3" /><path d="M5 6v4" /><path d="M19 14v4" /><path d="M10 2v2" /><path d="M7 8H3" /><path d="M21 16h-4" /><path d="M11 3H9" />
  </>,
  // lucide crop (SF crop)
  "crop": <><path d="M6 2v14a2 2 0 0 0 2 2h14" /><path d="M18 22V8a2 2 0 0 0-2-2H2" /></>,
  // lucide hand (SF hand.draw)
  "hand": <>
    <path d="M18 11V6a2 2 0 0 0-2-2a2 2 0 0 0-2 2" /><path d="M14 10V4a2 2 0 0 0-2-2a2 2 0 0 0-2 2v2" />
    <path d="M10 10.5V6a2 2 0 0 0-2-2a2 2 0 0 0-2 2v8" />
    <path d="M18 8a2 2 0 1 1 4 0v6a8 8 0 0 1-8 8h-2c-2.8 0-4.5-.86-5.99-2.34l-3.6-3.6a2 2 0 0 1 2.83-2.82L7 15" />
  </>,
  // lucide search (SF magnifyingglass)
  "zoom": <><path d="m21 21-4.34-4.34" /><circle cx="11" cy="11" r="8" /></>,
};

export function ToolIcon({ name }: { name: ToolIconName }) {
  return (
    <svg data-icon={name} width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75"
      strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" focusable="false">
      {SHAPES[name]}
    </svg>
  );
}
