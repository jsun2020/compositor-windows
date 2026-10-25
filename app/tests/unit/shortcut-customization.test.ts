// @vitest-environment jsdom
import {afterEach,describe,expect,it} from "vitest";
import {assignShortcut,effectiveShortcuts,matchShortcut,readShortcutOverrides,saveShortcutOverrides,SHORTCUT_STORAGE_KEY} from "../../src/shortcuts/keymap";
afterEach(()=>localStorage.removeItem(SHORTCUT_STORAGE_KEY));
describe("custom shortcuts",()=>{
 it("persists explicit clearing without restoring defaults",()=>{
  saveShortcutOverrides(assignShortcut({},"delete-layer",null));
  expect(readShortcutOverrides()["delete-layer"]).toEqual([]);
  expect(matchShortcut(new KeyboardEvent("keydown",{key:"Delete"}))).toBeNull();
 });
 it("reassigns Delete and only removes the conflicting old binding",()=>{
  const overrides=assignShortcut({},"tool-brush",{key:"Delete"});saveShortcutOverrides(overrides);
  expect(matchShortcut(new KeyboardEvent("keydown",{key:"Delete"}))).toBe("tool-brush");
  expect(matchShortcut(new KeyboardEvent("keydown",{key:"Backspace"}))).toBe("delete-layer");
  expect(effectiveShortcuts(overrides)["tool-brush"]).toEqual([{key:"Delete"}]);
 });
 it("uses physical letters for non-Latin layouts while respecting composition and AltGraph",()=>{
  expect(matchShortcut(new KeyboardEvent("keydown",{key:"и",code:"KeyB"}))).toBe("tool-brush");
  expect(matchShortcut(new KeyboardEvent("keydown",{key:"и",code:"KeyB",isComposing:true}))).toBeNull();
  expect(matchShortcut({key:"b",code:"KeyB",getModifierState:()=>true} as unknown as KeyboardEvent)).toBeNull();
 });
 it("ignores malformed and unknown stored overrides",()=>{
  localStorage.setItem(SHORTCUT_STORAGE_KEY,JSON.stringify({unknown:[{key:"x"}],save:[{key:3}],undo:[]}));
  expect(readShortcutOverrides()).toEqual({undo:[]});
  localStorage.setItem(SHORTCUT_STORAGE_KEY,"broken");expect(readShortcutOverrides()).toEqual({});
 });
});
