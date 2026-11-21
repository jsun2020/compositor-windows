import { describe, expect, it } from "vitest";
import { layerDistances } from "../../src/tools/layer-distances";

const canvas = { x: 0, y: 0, width: 100, height: 100 };
const active = { x: 30, y: 20, width: 20, height: 12 };

describe("Move tool layer distances", () => {
  it("measures the nearest edge sharing the perpendicular span", () => {
    const others = [{x:2,y:19,width:18,height:6}, {x:24,y:40,width:5,height:4}];
    const sides = layerDistances(active, others, canvas).filter(l => l.horizontal);
    expect(sides.map(({start,end}) => [start,end])).toEqual([[20,30],[50,100]]);
  });

  it("uses the hovered layer and can measure insets to the canvas", () => {
    const hovered = { x: 60, y: 22, width: 5, height: 6 };
    expect(layerDistances(active, [], canvas, hovered)).toEqual([{horizontal:true,start:50,end:60,cross:25}]);
    const canvasDistances = layerDistances(active, [], canvas, null);
    expect(canvasDistances.filter(l => !l.horizontal).map(({start,end}) => [start,end])).toEqual([[0,20],[32,100]]);
  });
});
