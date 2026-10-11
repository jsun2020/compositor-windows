"""Operate only native dialogs belonging to the isolated acceptance process."""
import argparse
import json
import os
import time
from pathlib import Path

import psutil
from pywinauto import Desktop

parser = argparse.ArgumentParser()
parser.add_argument("pid", type=int)
parser.add_argument("exe")
parser.add_argument("action", choices=["inspect", "cancel", "select"])
parser.add_argument("--path")
parser.add_argument("--output", required=True)
args = parser.parse_args()
assert os.path.normcase(os.path.realpath(psutil.Process(args.pid).exe())) == os.path.normcase(os.path.realpath(args.exe)), "Acceptance process executable mismatch"
desktop = Desktop(backend="win32")
deadline = time.monotonic() + 20
dialogs = []
while time.monotonic() < deadline:
    dialogs = [w for w in desktop.windows(process=args.pid) if w.class_name() == "#32770" and w.is_visible()]
    if dialogs:
        break
    time.sleep(0.1)
assert len(dialogs) == 1, f"Expected exactly one native dialog owned by {args.pid}; found {len(dialogs)}"
dialog = dialogs[0]
assert dialog.window_text() in ("Open", "Save As", "Open Compositor Project (.comp folder)"), "Unexpected native dialog; do not interact"
controls = dialog.descendants()
snapshot = [{"handle": c.handle, "class": c.class_name(), "id": c.control_id(), "text": c.window_text(), "visible": c.is_visible(), "enabled": c.is_enabled()} for c in controls]
result = {"pid": args.pid, "exe": args.exe, "dialog": {"handle": dialog.handle, "title": dialog.window_text()}, "controls": snapshot, "action": args.action, "inputMethod": "Win32 control messages through pywinauto; no mouse coordinates, screenshot or mocked picker response"}
with Path(args.output).open("x", encoding="utf-8") as receipt:
    json.dump(result, receipt, ensure_ascii=False, indent=2)
if args.action != "inspect":
    if args.action == "cancel":
        candidates = [c for c in controls if c.class_name() == "Button" and c.control_id() == 2 and c.is_visible() and c.is_enabled()]
    else:
        assert args.path and os.path.isabs(args.path), "Absolute test path required"
        # Import uses the legacy ComboBox 1148; the observed Save As picker
        # instead exposes a visible Edit 1001 inside an unnamed ComboBox.
        combos = [c for c in controls if c.class_name() == "ComboBox" and c.is_visible()]
        edits = [c for combo in combos for c in combo.descendants() if c.class_name() == "Edit"]
        edits = [c for c in edits if c.is_visible() and c.is_enabled()]
        if dialog.window_text() == "Open Compositor Project (.comp folder)":
            # The independently observed folder picker has a standalone Edit
            # 1152 next to "Folder:", rather than a file-name ComboBox.
            edits = [c for c in controls if c.class_name() == "Edit" and c.control_id() == 1152 and c.is_visible() and c.is_enabled()]
        assert len(edits) == 1, f"Expected one observed file-name edit, found {len(edits)}"
        target = os.path.normpath(args.path)
        edits[0].set_edit_text(target)
        assert edits[0].window_text() == target, "Native file-name control did not retain path"
        result["selectedPath"] = target
        candidates = [c for c in controls if c.class_name() == "Button" and c.control_id() == 1 and c.is_visible() and c.is_enabled()]
    assert len(candidates) == 1, f"Expected one action button, found {len(candidates)}"
    result["button"] = {"id": candidates[0].control_id(), "text": candidates[0].window_text()}
    action = Desktop(backend="uia").window(handle=candidates[0].handle)
    assert action.process_id() == args.pid and action.handle == candidates[0].handle
    result["inputMethod"] = "PID-scoped native edit control and UI Automation Invoke for the observed action button"
    action.invoke()
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline and desktop.window(handle=dialog.handle).exists():
        time.sleep(0.1)
    result["dialogClosed"] = not desktop.window(handle=dialog.handle).exists()
    Path(args.output).write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding="utf-8")
    assert result["dialogClosed"], "Native dialog did not close"
print(json.dumps({k: v for k, v in result.items() if k != "controls"}, ensure_ascii=False))
