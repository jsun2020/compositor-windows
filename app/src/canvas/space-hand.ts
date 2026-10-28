/** Space must keep its typing/activation meaning on controls and in modal UI. */
const SPACE_CONTROL = 'input, textarea, select, button, a[href], [role="button"], [contenteditable]:not([contenteditable="false"]), [role="dialog"], .color-picker, .sheet, [data-testid="color-picker"]';
export function shouldIgnoreSpaceForHand(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  return !!el && (el.isContentEditable || (typeof el.closest === "function" && !!el.closest(SPACE_CONTROL)));
}

/** One listener for both pointer panning and the temporary-hand cursor. */
export function installSpaceHand(host: Window, blocked: () => boolean, change: (held: boolean) => void): () => void {
  const release = () => change(false);
  const key = (event: KeyboardEvent) => {
    if (event.code !== "Space") return;
    // Release even when focus moved into a field while the key was held.
    if (event.type === "keyup") { release(); return; }
    if (event.defaultPrevented || event.isComposing || blocked() || shouldIgnoreSpaceForHand(event.target)) { release(); return; }
    event.preventDefault();
    change(true);
  };
  host.addEventListener("keydown", key);
  host.addEventListener("keyup", key);
  host.addEventListener("blur", release);
  return () => {
    host.removeEventListener("keydown", key);
    host.removeEventListener("keyup", key);
    host.removeEventListener("blur", release);
    release();
  };
}
