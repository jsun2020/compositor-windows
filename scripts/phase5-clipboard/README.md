# Phase 5 clipboard safety and diagnostics

These Windows helpers support acceptance verification. They do not replace
the production clipboard implementation or turn failed protocols into passes.
The original acceptance helper, assertions, deadlines and retry count stay
separate and frozen. Build output and clipboard evidence belong under ignored
`build-artifacts`, never in Git.

From the repository root, build into a fresh directory:

```powershell
powershell.exe -NoProfile -ExecutionPolicy Bypass -File .\scripts\phase5-clipboard\build-safety-tools.ps1
```

The builder reports the output directory and records source/executable SHA256s.
It refuses an existing output directory. Compilation does not access clipboard.

`ClipboardOwnershipProbe.exe OUTPUT_JSONL SECONDS` records open-window and
last-writer process metadata and clipboard sequence changes. Seconds must be
1 through 600; creating `OUTPUT_JSONL.stop` ends capture early. The probe never
opens the clipboard, reads its contents or modifies it. Zero open-window does
not prove the clipboard is free: an anonymous opener can also return zero.
The last writer is not necessarily the current locker.

`ClipboardSnapshotGuard.exe snapshot NEW_DIRECTORY` captures supported
restorable formats, checks sequence stability and writes `original.dpapi`
encrypted using Windows DPAPI CurrentUser. Strings, string arrays, byte arrays,
streams and verified lossless 24/32-bit bitmaps are supported. Unsupported
types, changed sequence, failed encryption or failed decrypt/roundtrip prevent
the caller from starting the clipboard protocol. Snapshot does not modify the
clipboard. The backup is private user data; retain it locally.

`ClipboardSnapshotGuard.exe check-sequence SNAPSHOT_READY_JSON` verifies the
current sequence still matches the saved snapshot. The acceptance runner must
also compare its original helper's format set with the snapshot format set
before running any mutation. Never continue on a failed precondition.

`ClipboardSnapshotGuard.exe verify ORIGINAL_DPAPI` decrypts and validates a
backup without accessing the clipboard. A synthetic self-test exercises all
supported kinds, lossless pixel/string/byte roundtrips, tamper rejection and
refusal to overwrite a retained backup; it never accesses the clipboard:

```powershell
.\ClipboardSnapshotGuard.exe self-test .\new-synthetic-check
```

The explicit `restore ORIGINAL_DPAPI` operation writes the saved data to the
clipboard. It is a recovery operation, not an automatic acceptance retry.
Only restore when that backup is the intended original and replacing the
current clipboard is wanted. A recovery can never erase an earlier failed
assertion or be counted as a passing original protocol. Do not print decrypted
data or commit encrypted payloads, user returns or runtime profiles.

`ClipboardTextNativeRecovery.exe prepare ORIGINAL_DPAPI` prepares the four
standard Text, UnicodeText, OEMText and Locale formats in memory without
accessing the system clipboard. It uses the local WinForms COM serializer
and copies its HGLOBAL bytes exactly; it does not guess ANSI/OEM encodings.
Unsupported formats, duplicate names, embedded NULs and invalid Locale data
are refused before any mutation. `self-test` verifies synthetic local memory
copies, Unicode and Locale preservation and invalid input rejection without
accessing the system clipboard.

Its explicit `restore ORIGINAL_DPAPI` operation publishes immediate Win32
data using a hidden owner window, after all formats have been prepared.
Clipboard acquisition is bounded to 250 ms. It does not repeat publication
or use delayed OLE rendering. Before this recovery, separately back up and
verify the current clipboard; afterwards snapshot it and compare the exact
decrypted format/data entries in memory. Never claim success solely from
the write exit code. Both backups must be retained if publication or the
read-only comparison fails. This narrow recovery is separate from acceptance
and cannot substitute for a passing original restoration assertion.

`ClipboardReadOnlyLocker.exe EVIDENCE_DIRECTORY [MILLISECONDS]` holds the
clipboard open using its own hidden window, then closes it. The default is
five seconds; explicit intervals must be 50 through 5000 milliseconds. Create
the evidence directory first. This is a bounded controlled contention experiment, not a production tool. It never reads,
empties or writes clipboard contents, but temporarily blocks other clipboard
operations. Use it only during a notified diagnostic interval. The frozen
acceptance helper can then demonstrate the original `0x800401D0` error while
the ownership probe records this known lock. Check the sequence is unchanged
and keep diagnostic failures separate from native acceptance results.


The read-only native acquisition regression uses the production Tauri bridge
through an owned CDP endpoint. With the app launched using a fresh WebView
profile and the compiled helpers in a separate directory:

```powershell
$env:COMPOSITOR_QA_CDP_ENDPOINT='http://127.0.0.1:<OWNED_PORT>'
$env:COMPOSITOR_QA_HELPER_DIRECTORY='<COMPILED_HELPER_DIRECTORY>'
node .\scripts\phase5-clipboard\native-acquisition-control.cjs '<FRESH_EVIDENCE_DIRECTORY>' fixed
```

Create the evidence directory first. The runner requires the production bridge
and excludes the development API. A 160 ms read-only lock must release before
the native read completes; a 500 ms lock must produce the bounded availability
error after 200 through 999 ms. Each case checks the clipboard sequence is
unchanged. An unavailable initial clipboard aborts before the controlled locks.
The baseline mode expects the original immediate error instead. Neither mode
repeats a failed case or proves the complete image/text clipboard protocol.
