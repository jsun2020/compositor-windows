/** @vitest-environment jsdom */
import { afterEach, describe, expect, it } from "vitest";
import { installSpaceHand } from "../../src/canvas/space-hand";
let cleanup = () => {};
afterEach(() => { cleanup(); document.body.replaceChildren(); });
function press(target: EventTarget, type = "keydown") {
  const event = new KeyboardEvent(type, { code: "Space", key: " ", bubbles: true, cancelable: true });
  target.dispatchEvent(event); return event;
}
describe("temporary hand keyboard lifecycle", () => {
  it("holds on the canvas, clears when keyup moves to an input or window loses focus", () => {
    let held = false;
    cleanup = installSpaceHand(window, () => false, v => { held = v; });
    const canvas = document.createElement("canvas"), input = document.createElement("input");
    document.body.append(canvas, input);
    expect(press(canvas).defaultPrevented).toBe(true); expect(held).toBe(true);
    press(input, "keyup"); expect(held).toBe(false);
    press(canvas); window.dispatchEvent(new Event("blur")); expect(held).toBe(false);
    cleanup(); expect(press(canvas).defaultPrevented).toBe(false);
  });
  it.each(["input", "textarea", "select", "button", 'a href="#"', 'div contenteditable', 'div role="dialog"'])
  ("preserves Space in %s including nested targets", tag => {
    let held = false;
    cleanup = installSpaceHand(window, () => false, v => { held = v; });
    const name = tag.split(" ")[0];
    document.body.innerHTML = `<${tag}><span>target</span></${name}>`;
    const target = document.body.firstElementChild!;
    expect(press(target.querySelector("span") ?? target).defaultPrevented).toBe(false);
    expect(held).toBe(false);
  });
  it("does not activate through a modal or color picker even if the target is outside it", () => {
    let blocked = true, held = false;
    cleanup = installSpaceHand(window, () => blocked, v => { held = v; });
    expect(press(document.body).defaultPrevented).toBe(false); expect(held).toBe(false);
    blocked = false; press(document.body); expect(held).toBe(true);
    blocked = true; press(document.body); expect(held).toBe(false);
  });
});
