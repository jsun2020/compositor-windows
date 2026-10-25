import { describe, expect, it } from "vitest";
import { dropTarget, layerRows } from "../../src/panels/layer-rows";
import type { DocumentState, LayerState } from "../../src/engine/types";

function layer(id: string, o: Partial<LayerState> = {}): LayerState {
  return { id, name: id, visible: true, isGroup: false, parentId: null, opacity: 1, blendMode: "Normal", transform: { origin: [0, 0], size: [10, 10], rotation: 0, flipX: false, flipY: false, sampling: "High quality" },
    pixelsWidth: 0, pixelsHeight: 0, pixelsRevision: 1, hasPixels: false, hasMask: false, maskWidth: 0, maskHeight: 0, maskRevision: 0, maskEnabled: true, maskLinked: true, maskSourceId: null, maskPlacement: null, maskBackground: 255, ...o };
}
const state = (layers: LayerState[]): DocumentState => ({ id: "D", documentId: "D", width: 10, height: 10, resolution: 72, activeLayerId: null, canUndo: false, canRedo: false, isModified: false, undoDepth: 0, undoEntryId: null, path: null, guides: [], undrawn: [], selection: null, layers });

describe("layer rows", () => {
  // Bottom-to-top array: A, F(group) { B, C }, D
  const d = state([layer("A"), layer("F", { isGroup: true }), layer("B", { parentId: "F" }), layer("C", { parentId: "F" }), layer("D")]);
  it("lists top-first with depth and collapses folders", () => {
    expect(layerRows(d, []).map((r) => `${r.layer.id}:${r.depth}`)).toEqual(["D:0", "F:0", "C:1", "B:1", "A:0"]);
    expect(layerRows(d, ["F"]).map((r) => r.layer.id)).toEqual(["D", "F", "A"]);
    expect(layerRows(d, ["F"])[1].collapsed).toBe(true);
  });
  it("computes drop targets", () => {
    const rows = layerRows(d, []);
    expect(dropTarget(rows, 0, "above")).toEqual({ parent: null, above: "D", atBottom: false });
    expect(dropTarget(rows, 1, "into")).toEqual({ parent: "F", above: null, atBottom: false });
    expect(dropTarget(rows, 2, "below")).toEqual({ parent: "F", above: "B", atBottom: false });
    expect(dropTarget(rows, 3, "below")).toEqual({ parent: "F", above: null, atBottom: true });
    expect(dropTarget(rows, 4, "below")).toEqual({ parent: null, above: null, atBottom: true });
    expect(dropTarget(rows, 1, "below")).toEqual({ parent: null, above: "A", atBottom: false });
  });
});

import { filteredLayerRows } from "../../src/panels/layer-rows";
describe("Mac 1.5.6 layer filtering", () => {
  const d=state([layer("photo"),layer("folder",{isGroup:true,name:"Trip"}),layer("inner",{parentId:"folder",isGroup:true}),layer("leaf",{parentId:"inner",name:"Beach"}),layer("caption",{shape:{kind:"rectangle"}})]);
  it("finds descendants in collapsed folders without changing collapse state",()=>{
    const collapsed=["folder","inner"];
    expect(filteredLayerRows(d,collapsed,"  BEACH ",[],[]).map(r=>r.layer.id)).toEqual(["leaf"]);
    expect(collapsed).toEqual(["folder","inner"]);
    expect(filteredLayerRows(d,collapsed,"",[],[]).map(r=>r.layer.id)).toEqual(["caption","folder","photo"]);
  });
  it("includes the whole subtree of a matching folder and retains document depths",()=>{
    expect(filteredLayerRows(d,["folder"],"trip",[],[]).map(r=>[r.layer.id,r.depth])).toEqual([["folder",0],["inner",1],["leaf",2]]);
  });
  it("combines name with kind and treats Selected as another matching kind",()=>{
    expect(filteredLayerRows(d,[],"",["shape","selected"],["photo"]).map(r=>r.layer.id)).toEqual(["caption","photo"]);
    expect(filteredLayerRows(d,[],"beach",["shape"],[])).toEqual([]);
    expect(filteredLayerRows(d,[],"beach",["selected"],["leaf"]).map(r=>r.layer.id)).toEqual(["leaf"]);
  });
});
