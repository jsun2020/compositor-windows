/**
 * True when a keyboard event's target is a form control (or contentEditable
 * element) that should keep its native key handling - Enter/Escape shortcuts
 * that apply document-level actions must not fire while the user is
 * interacting with an `<input>`, `<select>`, a button, or an editable region.
 */
export function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target instanceof HTMLInputElement) return true;
  if (target instanceof HTMLTextAreaElement) return true;
  if (target instanceof HTMLSelectElement) return true;
  if (target instanceof HTMLButtonElement) return true;
  // `isContentEditable` requires computing the editing-host algorithm, which jsdom does
  // not implement (it always reports false in tests); `contentEditable` is a plain
  // reflected attribute and works in both jsdom and real browsers.
  return target.isContentEditable || target.contentEditable === "true";
}
