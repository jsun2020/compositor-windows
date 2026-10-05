# Phase 5 acceptance, 2026-10-03

This tracks acceptance against the Phase 4 remaining / Phase 5 plan, using the
existing assertions and budgets. The user's explicit goal is full acceptance;
implementation completion and individual passing checks are not a substitute.
The user's Phase 3.5d sampling probes and `.workbuddy` remain untouched. The
full-acceptance goal now includes the pixel-copy compatibility work below.

## Verified unlocked desktop control, 2026-10-05

The 0054 source checkpoint and immutable release-WASM browser assets pass
all 199 functional cases (7.5 minutes), with the 29 original performance opt-in skips exercised
separately. The complete original performance run finishes 27 passed / two
failed (8.8 minutes), one worker and zero retries. The failures are a 24 MP
typed-width result frame at 34 ms / <33 ms and 100 MP Levels installation
at 543 ms / <450 ms. Both remain failed; no assertion or budget changes.

Two independent clipboard pairs pass all four original native protocols:
six groups per protocol, 16 complete RGBA comparisons, eight editable text
records, matching UTF-16/style and original restoration. Fresh profiles and
four distinct ports have verified native parent ownership. An additional
read-only DPAPI comparison confirms all original formats and data match all
four backups immediately after the protocols, before the user resumes copying.
No emergency recovery is used and no clipboard payload is published.

The full 1,096-second control has 4,753 unlocked-session samples and 4,734
GameViewer-main-process absence samples. Both monitors cover the complete
control and the installed launcher reopens afterwards. Sampling does not prove
state between observations. GameViewerService and system settings are unchanged.
These observations do not establish the session state of historical failures.

Two wrapper preflights abort before any original test: the first reads the
newly-created monitor file before its first flushed line, and the second uses
a reader sharing mode that conflicts with its active writer. Their evidence
and restored GameViewer state are retained. The corrected wrapper waits for a
complete line within the original five-second startup deadline, uses a
read-only shared stream, extends process monitoring to 30 minutes and rejects
early monitor completion. All 32 inputs match; original protocols have no retries.

Finite observation-only diagnostics retain their failures separately. Typed
width reaches an uncached reduction from level 2 to level 3; layerPixels takes
21.6 ms within a 27.7 ms CPU render. Levels installation spends most of its
CPU time appending output bytes; initial reservation is about 21 ms. No cause
is inferred solely from a diagnostic passing a particular timing.

A separate fixed six-copy memory diagnostic compares 400 MB first writes with
resident overwrites: plain buffers 296/40 ms, independent WASM memory 360/38 ms,
and the actual staged bridge 310/45 ms of CPU. Source preparation, allocations,
all cooperative yields and cancellation are recorded separately. It executes
no original acceptance case and does not establish the cause of every prior
543 ms installation. The GameViewer launcher is restored; all 112 sampled
session states are unlocked. Evidence: `kernel-copy-boundary-independent-review.json`.

The packed integer RGBA box-average candidate passes the complete native
workspace: 628 tests, zero failures, ten existing ignored across 85 suites,
including both new independent regressions and all 13 original peak-heap
budgets. Release WASM, all three type checks, 287 unit tests and fresh browser
assets also pass. Working and frozen-asset WASM hashes match; 137 source,
original-assertion and asset inputs are frozen for the candidate. Its full
functional run passes 199 cases with zero failures (8.0 minutes), with the
29 original opt-in performance skips exercised separately. The complete
29-case performance run passes all cases (8.0 minutes), one worker and zero
retries. Typed-width frames are 12 ms at 24 MP and 29 ms at 100 MP / <33 ms;
100 MP Levels installation is 320 ms / <450 ms. All 137 frozen inputs verify
afterwards, all 4,222 session samples are unlocked and the launcher reopens.
This is one complete original performance run on the changed candidate; it
does not establish a universal fix for prior intermittent copy outliers. The candidate
keeps the exact (sum + 2)/4 rule, edge behavior and channel separation; new
independent scalar-reference tests cover thin/odd grids, chained reductions
and 65,536 channel permutations covering carries and half-up rounding. It is not a validated release.
Fresh production `COMPOSITOR_BUILD_0.8.0_20261005-1025` builds successfully.
Its 4,746,697-byte ZIP contains exactly the executable, license and README;
all three CRCs and extracted-file comparisons pass. Source, fixed-test and
production WASM bytes match. The earlier production assets and packages remain
preserved. The 184 candidate/package/helper inputs are frozen; original
clipboard observed/protection helpers are unchanged, and native UI uses the
original helper. Native package protocols have not started because the
read-only session gate reports locked at 02:38:32 UTC. A new unlock request
is pending. This is a session requirement, not an approval request.

Full Phase 5 acceptance remains incomplete until the fresh 1025 package passes
the original native UI, Mac-return and independent protected clipboard gates. Local evidence:
`kernel-unlocked-control-independent-result.json`,
`kernel-unlocked-clipboard-independent-review.json`,
`kernel-unlocked-clipboard-backup-protection-review.json`,
`kernel-quiet-control-20261005-084723-186` and both diagnostic directories.

## Previous kernel optimization and environment checkpoint, 2026-10-05

Production `COMPOSITOR_BUILD_0.8.0_20261005-0054` specializes gray/RGBA paint
loops and caches only two decoded source rows plus interpolation columns for
spatial enlargement. It preserves the original coordinate calculations,
floating-point operation order, rounding and preview resolutions. Two complete
synthetic output references were generated from the validated e644fe3 release
library before either kernel changed. Both new regressions match every byte.
The initially attempted full float-image cache exceeded two original peak-heap
budgets and was rejected; the final row cache passes the unchanged budgets.
The complete native workspace passes 626 tests, zero failures, ten existing
ignored. Release native/WASM builds, three type checks and 287 unit tests pass.

The new complete functional run retains 198 passes, one failure and 29 original
performance opt-in skips, with one worker and zero retries. The failure occurs
while setting up the page, exceeding the original 30 seconds before the
100-megapixel project-limit test body executes. It remains failed. The targeted
original performance pair retains one spatial pass and one gradient failure:
100 MP gradient drag is 35 ms / <50 ms and motion-blur eyedropper is
1,012 ms / <2,000 ms, but settled gradient frames still exceed their unchanged
budget. These improvements do not establish full performance acceptance.

An erroneous diagnostic launcher invoked the original two-case runner again,
overwriting its fixed-path outputs. The first run's 34 ms drag, 1,057 ms
eyedropper and 555/564 ms settled totals survive only as recorded observations;
its raw log is lost. The unexpected second run also fails, and its outputs are
retained separately in `kernel-diagnostic-launcher-error-retained`. This error
is explicitly recorded in `kernel-diagnostic-launcher-error-audit.json`; no
repeat is treated as a pass or substituted for the first failed result.

Corrected independent passive and serialized GPU diagnostics retain their
failures separately. With explicit finishes after individual draws/uploads,
no wrapped finish exceeds 20 ms, but subsequent 1x1 readPixels still stalls
near 500 ms. A separate build that draws/presents through a dedicated screen
framebuffer also fails (529 ms / <150 ms), with readPixels stalls of
448.5–542.6 ms. Neither diagnostic changes production renderer code or proves
a driver/scheduling cause. GPU error is zero on Intel HD Graphics 520 / ANGLE
D3D11. The prior no-application minimal control also retains a similar stall.

The new portable passes all eight original native UI groups and the owned
Mac protocol: 11 reads, ten atomic saves, continued text/shape/effect edits and
reopened 7/14 effects. All nine complete Mac PNG exports match every RGBA byte.
Source, test and production WASM match. ZIP's three entries pass CRC and match
the staged portable. Native adapters differ only in package identities and
evidence destinations, verified by reversing those replacements.

The new clipboard control stops at the encrypted snapshot gate before any
protocol or clipboard mutation (`clipboard-protected-20261005-010814`).
The main GameViewer process is absent in 51 sampled observations over
11,825 ms and is reopened afterwards. The snapshot helper reports an external
clipboard exception. A separate read-only Win32 metadata check returns
OpenClipboard error 5, with accessible Default input desktop; a zero observed
open-window handle does not prove absence of a clipboard lock. The ordinary
native UI's earlier read-only clipboard probe also refuses access. Historical
2216 four-protocol passes remain valid for that earlier tested state; they do
not pass the new package's failed snapshot gate.

GameViewerService and its server/health children remain running after exiting
GameViewer.exe. Their role is unproven. A bounded service-control helper passed
read-only preflight, but the administrator request is superseded by more direct
session evidence. Windows 10 19045 WTSINFOEX Level1 reports current session 1,
state 0 and SessionFlags 0. Microsoft's
[WTS structure definition](https://learn.microsoft.com/en-us/windows/win32/api/wtsapi32/ns-wtsapi32-wtsinfoex_level1_w)
defines this as locked. An accessible Default input desktop is insufficient
to establish an unlocked session. This proves the present state, not the state
through every earlier failed test or the cause of every GPU/clipboard failure.

The old service-control command is disabled; its original source is retained
as superseded evidence. The service still reports Running / Auto and no
service/display/driver setting has been changed. The user is asked only to
unlock Windows and keep the desktop active, with no clipboard copying during
the original protocols. The public `Phase5SessionGate.cs` queries only session
metadata and refuses unknown, inactive or locked state. It reads no clipboard,
sends no input and changes no policy. Its Unicode WTS query confirms the
232-byte response, Level1/session identities and flags 0. A negative preflight
refuses before any original protocol, clipboard access or process mutation.
The prepared unlocked environment control verifies 32 frozen inputs and
records session samples at 200 ms alongside main GameViewer process samples.
It runs the original independent clipboard pairs, complete functional suite
and 29 performance cases without modifying bodies/assertions/budgets or adding
retries. All native/browser tests remain non-elevated; user unlock is pending.

Evidence: `kernel-specialization-row-cache-native-summary.json`,
`kernel-specialization-wasm-test-build-summary.json`,
`kernel-specialization-functional-summary.json`,
`kernel-gradient-gpu-passive.json`, `kernel-gradient-gpu-serialized.json`,
`kernel-screen-gpu-passive.json`, `kernel-package-integrity.json`,
`kernel-native-adapter-equivalence.json`, `native-kernel-ui-20261005-0054`,
`native-kernel-mac-20261005-0054`,
`kernel-native-owned-runtime-return-comparison.json`,
`kernel-input-desktop-and-clipboard-metadata.json`,
`kernel-readonly-wts-session-metadata.json`, `kernel-session-gate-preflight.json`,
`kernel-unlocked-control-negative-preflight.json`,
`kernel-service-request-superseded.json` and
`kernel-package-native-and-control-summary.json`. Full acceptance remains false.

Package identity:

- EXE: 11,979,264 bytes; SHA256
  `BA76044FD002A42E41DC94200ACB94CADF1D57B1E5EDE300600B555AC8327798`.
- ZIP: 4,746,599 bytes; SHA256
  `F5CCA10EF3F795170DE46EDEF79E4D65CA0E8995B2E40905DA097624CCCD07E8`.
- WASM: 3,264,517 bytes; SHA256
  `EF30FB8B91C999627551D9C9C5F7F9DFF9A240D2317CD741834F39B86BFC25B4`.

## File-operation status fix and local Windows checkpoint, 2026-10-04

The user confirms Windows local operation. GameViewer is temporarily exited
for a bounded environment control and reopened through its installed root
launcher afterwards. No service, Explorer, remote connection or system setting
is changed. The initial launcher check that observed a still-terminating
process is retained separately; it starts no original protocol.

The old candidate's GameViewer-absent control passes two complete protocols,
then fails Cut in `clipboard-protected-20261004-220239`: three layers remain
instead of two within the original 5,000 ms. Ctrl+X is received at 11,990 ms,
but no clipboard write follows. Recent-file registration spans
11,967.6–12,043.8 ms and keeps the file busy guard set, while the old status
indicator reports only edit working. A deterministic delayed-registration
regression reproduces the missing indicator with the frozen old assets.

`App.tsx` now displays the existing working status for either edit working or
file busy. The regression verifies that Save As remains visibly busy after
the package commit until recent-file registration finishes, then Cut and Undo
work. The file-operation guard and all original clipboard actions, assertions,
timeouts and performance budgets remain unchanged. This fixes the observed
missed Cut; it does not establish a cause for the earlier all-zero PNG or
retroactively pass any historical failure.

The new production marker is `COMPOSITOR_BUILD_0.8.0_20261004-2216`. Three
TypeScript checks, 287 unit tests, and the full fixed-asset functional suite
(199 passed, 29 original performance opt-in skips, one worker, zero retries)
pass. The unchanged native core retains its 624 passed / zero failures /
10 existing ignored coverage. Source, test and production WASM are identical.
The new portable passes eight original native UI groups and 11 Mac reads /
10 atomic saves. All nine complete Mac PNG exports match every RGBA byte;
continued text/shape edits and reopened Stroke 7 / Shadow Distance 14 pass.

Two independent original clipboard pairs on the new package pass all four
complete native protocols and strict readers, with no retry or repeated
gesture. Case directories are `clipboard-protected-20261004-223815`,
`223847`, `223927`, and `223953` (same full prefix). Each passes all six
operation groups, four exact RGBA comparisons, two editable UTF-16 records,
matching style, and mandatory snapshot restoration. Separate read-only
rechecks confirm all four strict results, valid encrypted backups, unique
owned native endpoints/profiles and complete current formats/data identical
to each pre-test snapshot after GameViewer restarts. The absence monitor
records 545 samples over 130,685 ms at a 200 ms interval, with no observed
GameViewer PID; this is sampled absence, not an assertion between samples.

The new package's complete original 29-case performance run finishes with
24 passed / five failed in 12.7 minutes, one worker and zero retries. Failures
are: Add Mask page-fixture setup exceeds 30 seconds before its body; 100 MP
typed-width frame 36 ms / <33 ms; 100 MP fit gradient drag 51 ms / <50 ms;
100 MP gradient installation 501 ms / <450 ms; and blank-layer gradient
worker frame gap 112 ms / <100 ms. All frozen inputs verify afterwards.
These failures remain failed, with no source or assertion change justified
solely by the environment hypothesis. The single distinct complete 29-case
GameViewer-absent environment control finishes with 27 passed / two failed
in 10.2 minutes, same fixed assets/config and zero retries. The remaining
failures are 100 MP gradient drag at 1:1 (58 ms / <50 ms, engine 51 ms and
frame 8 ms) and motion-blur eyedropper (2,122 ms / <2,000 ms). Source/assets
verify after both runs; sampled process absence passes and GameViewer is
reopened afterwards. Full acceptance remains false. The environment control
does not establish GameViewer as a cause or justify a retry or budget waiver.
The next work measures the existing gradient paint and spatial-enlargement
kernels and preserves their complete pixel outputs before any optimization. Clipboard
acceptance is established for the tested ordinary local Windows environment
with GameViewer absent during the protocol. Running GameViewer or another
external clipboard consumer remains a separate environment constraint; these
passes do not prove universal interoperability under external locks. The
recorded restoration locks, all-zero PNG, missed Cut, earlier Save As timeout
and historical performance failures remain retained with their original status.

The ZIP contains three CRC-verified entries identical to the staged portable:
4,744,584 bytes, SHA256
`6D7BA0B384C08F6260C7E849BADECB1B7D5768ED7CFE4E71C69E74E6F63A5D0F`.
The executable is 11,977,728 bytes, SHA256
`B5B5A6E0585752ED2DAEFA4DA316B681F3ACF7F883D21273FA6A37D2DD76C0A3`.
Private backups, user Mac returns, profiles and generated packages remain local.
Evidence is under `build-artifacts/phase5-acceptance`: `file-busy-*`,
`native-file-busy-ui-20261004-2216`, `native-file-busy-mac-20261004-2216`,
`local-file-busy-gameviewer-absence-20261004-223806-115` and
`clipboard-file-busy-gameviewer-absent-control-20261004-223811-394`.

## Earlier recovery and recognition-off control, 2026-10-04 21:51

The user disabled GameViewer's **remote-assistance clipboard recognition**
setting and explicitly authorized recovery. The screenshot describes invitation
recognition; it does not establish that remote clipboard synchronization is off.
The previously prepared sync-off control is retained as historical preparation,
not executed or accepted as proof of synchronization being disabled.

The first explicit recovery (`clipboard-recovery-210049-20261004-212826-449`)
backs up current contents successfully, then fails Windows Forms restoration.
That failure is retained; it does not start acceptance or repeat restoration.
A separate narrow native recovery helper is then built and checked offline.
It rehydrates the four standard text formats through local WinForms COM
serialization, copies their HGLOBAL bytes exactly before mutation, and uses
immediate Win32 publication with a hidden owner window. It rejects unsupported
formats and duplicates, verifies Unicode and Locale, and never guesses ANSI
or OEM encodings. All allocation and validation precede EmptyClipboard;
successful SetClipboardData transfers ownership to Windows. See
[Microsoft SetClipboardData documentation](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setclipboarddata).

The separately authorized native recovery succeeds in
`clipboard-native-recovery-210049-20261004-213943-372`. A fresh current-content
backup is verified before writing. Afterwards a new encrypted snapshot is
decrypted and compared in memory: **all four original formats and complete
data entries match exactly**. Earlier recovery and protocol failures remain
failed; recovery is explicitly not native acceptance.

The recognition-off original control
`clipboard-gameviewer-recognition-off-control-20261004-214032-773` stops at
its first case (`clipboard-protected-20261004-214037`). All six original
operation groups finish and original snapshot restoration succeeds. However,
the unchanged independent strict reader fails parsing native-copy-consumed.png.
That file is 7,057 bytes, **all zero**, with SHA256
`fea2ddfd594ede58d1b04e9e5dbc0d0f896125d7c0f619a63e26bb26add36105`.
The complete original gate remains failed. No second protocol or second pair
runs, no retry is added, and original assertions/budgets are unchanged. The
20 ms ownership probe does not identify an external locker for that read;
the zero-file cause cannot be assigned to GameViewer or product publication.

Two separate bounded diagnostics retain that uncertainty. In
`clipboard-png-consumer-contention-20261004-214605`, a known native PNG is
consumed byte-exactly without a lock; with a known 500 ms read-only lock,
the original consumer reports no stream. This does **not** reproduce the
all-zero file. In `clipboard-png-path-witness-20261004-214736`, two added
native reads surround the original independent consumer. All three paths
return the same valid 7,057-byte PNG with SHA256
`2f6401dbdd850abe974449f43cf61b5543ca9a4d250b0dc8bf366be386ae67c1`.
The strict four RGBA/two text/style/restoration reader passes, but these
additional read observations make this a diagnostic, not an original passing
acceptance session. Neither diagnostic justifies changing the product.

After both diagnostics, `clipboard-after-diagnostics-readonly-20261004-215128-797`
confirms the current clipboard still exactly matches the complete pre-test
encrypted snapshot. All private backups, returns and profiles remain local.
Product source and portable identity are unchanged. All 26 original frozen
inputs, both observed helpers and five protection inputs remain verified.

The new `verify-phase5-clipboard-gameviewer-absent-control.ps1` passes
CheckOnly. It refuses to start while GameViewer is running and never stops
a process, connection or service. Two original independent pairs retain
zero retries and stop on first failure. Process absence is checked at pair
boundaries; continuous absence is not falsely claimed. The current user
operation mode must be established before arranging this comparison without
interrupting their remote access. **Full Phase 5 acceptance remains false**;
624 native tests, eight packaged UI groups, nine exact Mac exports and 29/29
original performance coverage retain their applicable evidence.

## Earlier history-off receipt and captured GameViewer lock, 2026-10-04 21:00

The received `clipboard-history-off-control-20261004-205957-992` verifies
all original frozen inputs and begins with Clipboard history disabled
(EnableClipboardHistory=0). Its first two independent original protocols
and strict readers pass. The third finishes all application UI/data steps,
then fails at the original helper's final snapshot restoration, with
0x800401D0. The existing finally restoration fails as well. No fourth
protocol starts and no retry is added. History-off alone therefore does not
establish stable clipboard interoperability.

All four native reads and three native writes in the failed case return
success. Final application state is five layers, no error and working=false.
Independent artifact review confirms all four exact RGBA comparisons and
two editable UTF-16 records with matching style. This data-only review
explicitly reports nativeProtocolAccepted=false; restoration failure remains
a failed complete protocol. The first two strict protocol readers and all
three encrypted backups are independently verified again.

The metadata-only ownership probe captures **GameViewer PID 24508** holding
the open clipboard window during the restoration failures: the same window
and thread appear at 13:01:05.952, 13:01:07.005 and 13:01:08.070 UTC, spanning
2,118 ms. Main restore is requested at 13:01:05.861 and its error response is
written at 13:01:07.230; the finally error is written at 13:01:08.308, before
the next observed unlocked sample at 13:01:08.357. This supplies a named
external lock overlapping both failure intervals, consistent with the prior
controlled restoration experiment. It does not attribute every older failure
to this process or prove which GameViewer feature acquired the lock. No user
remote application, connection, Explorer or system service is stopped.

**The original clipboard is not confirmed restored.** The original helper
has exited, but its DPAPI CurrentUser backup decrypts and validates. It contains
the pre-test text/locale formats and remains private locally. Recovery has not
been attempted. The local `recover-phase5-clipboard-210049.ps1` defaults to
verification only and requires explicit `-RestoreOriginal` to write. It first
backs up the current clipboard into a fresh encrypted directory, performs one
recovery operation, then reads a new encrypted snapshot and compares the
complete saved formats/data in memory. Recovery can never turn the original
protocol failure into a pass. Its verification-only entry and read-only backup
comparison test pass, including rejection of a changed format.

The next environment control requires the user to disable GameViewer clipboard
synchronization while retaining the remote connection. This is an intervention
to test the captured blocker, not a claimed proven feature-specific fix. The
prepared `verify-phase5-clipboard-wait250-gameviewer-control.ps1` passes
CheckOnly and preserves all 26 original inputs, both observer hashes, five
protection inputs, the native executable/source hashes, original actions,
assertions, budgets and zero retries. It changes no setting or running process;
two independent pairs stop on the first failure. History and performance
need not be changed or rerun for this comparison. User action is pending.

Full Phase 5 acceptance remains false. Product commit `3299300`, its native
250 ms acquisition control, 624 native tests, eight packaged UI groups, nine
exact native Mac exports and the unchanged frontend's 29/29 original timing
evidence remain valid. This receipt does not justify another product patch.
Evidence remains local under `build-artifacts/phase5-acceptance`: the received
control and case `clipboard-protected-20261004-210049`,
`clipboard-history-off-205957-independent-review.json`,
`clipboard-history-off-210049-gameviewer-failure-review.json` and
`clipboard-history-off-205957-receipt-file-hashes.json`.

## Earlier clipboard-fix checkpoint, 2026-10-04 evening

The native candidate waits up to 250 ms to acquire a busy Windows clipboard
on its existing blocking worker, before reading or clearing
the clipboard. The previous single attempt classified error 5 as
a non-interactive desktop even though the same error occurs under a normal
read-only external lock. Persistent unavailability still returns an error; no
clipboard payload cache or repeated application gesture is added. Original
acceptance assertions, deadlines, budgets and zero case retries are unchanged.

The original 1527 baseline passed eight independent native protocols before
its ninth failed at editable-text paste: accepted Ctrl+V, a real native read
returning access denied in 12 ms, and four layers instead of five. The original
snapshot restored and its encrypted backup remains valid. The 20 ms metadata
probe does not identify the exact locker during that short failure. A separate
read-only external lock also reproduces the original helper's 0x800401D0
restoration failure without changing clipboard data; this establishes the
mechanism, not the cause of the historical 18:23 failure.

Controlled production-native checks distinguish the original and fixed
executables: the original refuses a 160 ms lock immediately; the candidate
completes after the short lock releases (155 ms measured read). A 500 ms lock
returns the new availability error after 255 ms, before release. Both cases
keep clipboard sequence 3609 unchanged. An earlier candidate-control preflight
was unavailable and is retained; a subsequent native UI read established access
had returned before the completed control. No user/system service is stopped.

The fresh native workspace run passes **624 tests, zero failures, 10 existing
ignored**. Seven frontend/WASM assets remain byte-identical to the verified
1527 assets: 287 unit checks, three type checks, 198 functional browser checks
and the received complete **29/29 original performance run** continue to apply
to those identical assets. Performance is not rerun for the Windows-only
clipboard acquisition change, and earlier timing failures remain retained.
The candidate independently passes all eight original native UI groups and
11 real native Mac reads / 10 atomic saves. All nine complete PNGs match the
independent Mac returns exactly; saved text, shapes and 7/14 effects remain
editable on reopen. Original action/assertion bodies are independently checked
byte-for-byte after normalizing only endpoint/evidence-directory plumbing.

Portable: `Compositor-portable-0.8.0-20261004-clipboard-wait250.zip`,
4,911,205 bytes, SHA256
`015F4C7B61C2E07EE6CE3477E52AB628EF670F971C899FD78BA3501CB2CE2278`.
EXE SHA256:
`64C74694D6D78208F238BA532FF6908A252FC65525F9FE7902B01C72BCBFC93B`.
All three archive entries pass CRC and exact staged-file comparison. The
frontend marker remains `COMPOSITOR_BUILD_0.8.0_20261004-1527`; candidate
identity is the new executable hash and native-source hash, not a new UI marker.

The first five independent complete candidate clipboard protocols pass their
original six groups and strict RGBA/text/style/restoration comparisons. The
sixth fails: the original external Windows Forms bitmap consumer returns no
image, after a successful 34.2 ms native Cut write. Ctrl+X is accepted, the
application is idle with two layers and no error. Explorer is sampled holding
the clipboard at 12:25:27.567 UTC; the consumer request is written at .608
and its error response at .718. The exact managed GetImage call time is not
recorded, so that sample alone does not prove the natural failure's exact locker.
The bounded series stops immediately; remaining pairs are not run. Original
formats restore, the encrypted backup validates and no recovery is attempted.

A new, separately guarded controlled experiment uses the unchanged original
helper and actual candidate native image publication. Unlocked consumption
passes; under a known 500 ms read-only external lock, the same GetImage call
returns the same no-image assertion before release. Sequence remains 3887
while held, and the original helper restores the snapshot after release. This
proves the original independent consumer can fail under external contention
even after successful native publication. It is diagnostic evidence, not a
substitute for passing complete protocols or proof of the historical locker.

Windows Clipboard history is currently enabled (read-only registry check).
Its involvement is a hypothesis, not a confirmed cause. The local hash-bound
`verify-phase5-clipboard-wait250-history-control.ps1` passes CheckOnly and
requires the user to temporarily turn that preference off before it runs two
independent original pairs. It never modifies Windows settings, retries a
case, relaxes assertions or repeats performance. The user can restore the
preference afterward. **Full acceptance remains false: external-consumer
stability is still unresolved.**

The published native acquisition regression is also run against the candidate
and freshly compiled public helpers, passing both short and persistent lock
cases with the sequence unchanged.

Reusable, reviewed QA source is in `scripts/phase5-clipboard`: the encrypted
backup guard, metadata-only ownership probe, read-only external locker, fresh
builder and native acquisition regression. Their compiled synthetic self-test
and read-only probe pass. Backups, user payloads, runtime profiles, screenshots,
packages and all historical failed evidence stay local and excluded from Git.

Evidence under `build-artifacts/phase5-acceptance`: `clipboard-protected-191051-failure-review.json`,
`clipboard-restoration-contention-20261004-190028-939`,
`native-clipboard-acquisition-baseline-20261004-192254-039`,
`native-clipboard-acquisition-fixed-20261004-201901-264`,
`clipboard-native-wait-rust-test-review.json`,
`clipboard-native-wait-package-integrity.json`,
`clipboard-wait-runtime-original-body-equivalence.json`,
`clipboard-wait-native-owned-runtime-return-comparison.json` and
`clipboard-wait250-protected-20261004-202041-865`,
`clipboard-wait250-stability-series-20261004-202253-009`,
`clipboard-wait250-202505-failure-review.json` and
`clipboard-consumer-contention-20261004-203058`.

## Earlier checkpoint, 2026-10-04 evening

The actual Mac-applied 7/14 return now passes production-native open/export,
save/reopen and editable text/shape checks. Its full PNG is byte-identical.
The separated GPU shader passes all 287 units, three type checks, all 198
functional cases and all nine independent Mac full-RGBA export comparisons.
The four alpha probes also match GPU/CPU bytes exactly; the other 800x600
Mac cases remain within one GPU byte. Saved transforms stay unchanged.

The received ordinary interactive 1527 run passes all 29 original timing
cases in 8.8 minutes, one worker and zero retries, with all 26 frozen inputs
unchanged. The earlier Codex-host full run remains 20 passed / nine failed
(10.0 minutes), including GPU waits in result/partial upload frames; its typed
W passes at 17/27 ms, and both brush/retouch sizes pass. No intermittent
performance cause is claimed fixed merely from the later complete pass.
The prior human-run 1256 verifier is 26/29; its clipboard preflight safely
aborts at OleGetClipboard. Two same-assets visible-window diagnostic cases
finish one pass / one failure; a minimal no-application WebGL program also
reproduces a 446.7 ms draw/read wait. The specific system cause remains unproven.
These diagnostic passes do not waive any complete-suite failure. Full Phase 5
acceptance remains incomplete. Production 1527 passes archive identity, eight
native UI groups and an independent nine-project native open/save/export run
(11 reads / 10 atomic saves), with all nine complete PNGs exact. The initial
native Save As timeout is retained and its cause is unproven. Current-package
clipboard from the Codex host safely aborts at the OLE snapshot; independent
Win32 OpenClipboard returns error 5. The subsequently received administrator
run records a complete GPU/CPU ETL and passes original F1, but its native
clipboard protocol never starts because the WebView debug endpoint is absent
for 45 seconds. The snapshot succeeds and its original format set is restored.
A non-elevated same-package endpoint control passes. The new hash-bound
ordinary PowerShell verifier produces the complete 29/29 timing evidence
and a first six-group native clipboard pass, independently verified by four
exact RGBA comparisons and UTF-16/style/restoration checks. Its second
clipboard session fails when Ctrl+V after Cut leaves two layers instead of
three within the unchanged five-second timeout. Both original clipboard
format sets are restored. The later observed run passes its first protocol
and strict reader; its second session instead fails before engine startup
because the observer mishandles fetch(URL). That diagnostic-only bug is
fixed, with startup/input compatibility and controlled full-UI/error-capture
checks passing. Each revised session has its own debug port, and endpoint
ownership must match the launched native PID and fresh profile. Original
actions/assertions/timeouts stay unchanged. An independently owned native
Mac rerun again passes 11 reads, 10 atomic saves and all nine full-PNG byte
comparisons. The corrected 18:23 interactive run passes its first native
protocol and strict reader. Its second session completes the clipboard UI
actions and saves; four independently checked RGBA comparisons and two
editable text records pass. It fails at the original snapshot restoration:
Windows Forms SetDataObject returns 0x800401D0, and the existing finally
fallback also fails. The helper exits without restoring its original snapshot;
that in-memory snapshot is no longer available. This second protocol is failed,
not accepted. Seven native clipboard IPC responses succeeded before that
failure, and the application reports five layers, no error and no work in
progress. The cause of the restoration failure and the earlier actual
Cut/Paste failure remain unproven. Full clipboard stability is not accepted.
Historical evidence below is retained.

## Earlier checkpoint, 2026-10-04 morning

The received Windows interactive run passes all 29 original performance cases
for production 0244. A later unchanged 0244 clipboard run passes all six
native groups and independent exact RGBA/UTF-16/style/restoration checks;
prior intermittent failures remain recorded.

The new measured byte interpolation makes all eight received Mac projects'
release-WASM exports exactly match every straight RGBA byte, including
Mac-created. The four alpha probes also match CPU/GPU bytes exactly.
The final candidate passes 623 complete native workspace cases, 287 units,
three TypeScript checks and nineteen render cases. The subsequently added
half-column regression passes alongside the exact grid and five existing
pixel-copy checks. All 198 functional browser cases pass, with the 29 original
opt-in performance skips. The separate complete timing run finishes 23 passed /
six failed (10.6 minutes), one worker and zero retries. The independent
preceding-version control also reproduces four responsiveness failures, plus
a setup assertion failure. Production 1256 passes all eight native UI groups,
nine native Mac reads/atomic saves, eight exact Mac PNG comparisons and the
six-group clipboard protocol with independent exact RGBA/text/restoration
checks. Source commit b452534 is published on the authorized branch.

Mac gesture records report PASS, but the returned blue rounded rectangle
contains Stroke 5 / Shadow Distance 12 instead of the reported applied 7/14.
The actually applied-and-reopened Mac project has been requested. Full Phase 5
acceptance is not yet complete. Later sections retain historical failures and
diagnostics; the exact-byte candidate and final verification sections describe
new work.

## Changes found during acceptance

Native Copy and Cut sent document coordinates in HTTP `Origin`. WebView2 owns
that header and replaced it with `http://tauri.localhost`, so the native command
rejected the write as `Invalid clipboard origin`. The production bridge and Rust
command now use `compositor-pixel-origin`; finite-coordinate validation remains.
A native unit check includes the actual browser Origin alongside the new header.

Pixel comparison of a real Windows Forms bitmap producer exposed a second bug:
the synthesized DIBV5 has a repeated RGB mask table after its V5 header. The
converter treated that table as pixels, dropping the final row and making the
two-row sample appear vertically reversed. Conversion now recognizes matching
mask tables only when the complete pixel span establishes their position.
Ordinary V4/V5 bitmaps with embedded masks keep their original offset. New native
tests cover both row directions, alpha and the synthesized extra table. The
Windows behavior is consistent with Microsoft's description of the
[BITMAPV5HEADER bmiColors masks](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-bitmapv5header).

Large worker edits now reuse the transferred input buffer for a same-sized
output. The main thread retains at most one spare of 4 MiB to 400 MB, moves
ownership out of retired callers and releases it after 30 seconds idle. This
avoids repeated 400 MB allocations without retaining every history buffer.
Different-sized outputs still allocate their required size.
Histogram and edit workers return unused input buffers as well: the
next edit can reuse them, including after cancellation or supersession. Retired
result buffers also return to the bounded pool when no caller can install them.
The spare is released on pagehide as well as idle timeout. Histogram
bins and result ownership are unchanged.

Staged installation copies each JavaScript chunk directly into the reserved
WASM destination. The old `Vec<u8>` binding copied into an intermediate vector
and then into the destination. Overflow, invalid planes and incomplete results
remain rejected before installation. A real-worker browser check verifies exact
pixels, repeated edits and undo/redo for a chunked layer.

## Production 1206 baseline

Evidence is retained in `build-artifacts/phase5-acceptance/`. Interrupted runs and
failed diagnostic attempts remain retained separately. Builds, performance,
native clipboard, native UI and Mac checks are distinct evidence layers.

- Native workspace: 616 passed, zero failed, 10 existing ignored;
  `native-workspace-bitmap.log`.
- Unit suite: 268 passed in 42 files; `unit-retired-results.log`.
- Production TypeScript/UI build passes; `ui-build-retired-results.log`.
- Full fixed-asset browser suite: 190 passed, zero failed, 29 unchanged opt-in
  performance skips (7.3 minutes); `e2e-retired-results.log`. These skips are
  not performance acceptance.
- Final release WASM SHA-256:
  `928BDB0293AB9D06E1FE0CC2983E633DAD9309860BAC8871C9A1D6EA8FD801EE`.
- Final bundled test WASM has the same SHA-256. The first attempt used an
  optimization-intermediate binary and was interrupted rather than accepted.

The final direct-install binary first passed all five targeted performance
cases. Its independent repetition failed 100 MP Levels input at 461 ms against
450 ms. Returning histogram input buffers subsequently reduced that input to
51 ms, but F1's 100 MP blank-layer fill still recorded a 142 ms gap against
100 ms. The other four cases passed, including 100 MP brush/blur/healing/content
fill (maximum gaps 47/22/20/41 ms), Add Mask and tab drag. F1 is not yet accepted
as stable. Logs: `perf-final-stable-first.log`,
`perf-final-stable-repeat.log`, `perf-histogram-recycle-first.log`.

A broader 11-case baseline passed 10 and failed F1's 24 MP Levels result frame
(409 ms / 100 ms). After unused-input and canceled-result retirement, the
Levels budgets pass, but the 100 MP blank-fill result frame still reaches
491 ms / 100 ms. This broader run finishes 7 passed / 4 failed: Add Mask also
exceeds its frame budget (24 MP Hide Selection 415 ms; 100 MP frames 466-522 ms),
and Content-Aware Fill reaches 104 ms at 24 MP and 314 ms at 100 MP against
100 ms. The 100 MP Delete case passes its original median-frame
budget while recording a 514 ms worst frame. This is retained as a limit,
not waived as jitter. Logs: `perf-page-lifecycle-baseline.log` and
`perf-retired-results.log`. A diagnostic separates render, texture upload and
GPU readback without changing the acceptance test. In the diagnostic, the original
Delete and F1 assertions both pass, but other/cold frames record 21 slow
`readPixels` calls (maximum 403 ms), 13 slow `texImage2D` calls (maximum 50 ms),
and seven slow render calls (maximum 837 ms). These setup/cold measurements are
not F1 acceptance failures and do not reproduce or explain every measured stall.
`trace-f1-gpu-summary.json` and the full retained trace distinguish them.

## Legacy performance instrumentation

The remaining 18 performance cases finish 10 passed / 8 failed
(`perf-remaining-coverage.log`). Six failures reveal stale instrumentation:
those tests wrapped synchronous `jobInput`/`installJob`, while the store now
uses the cooperative async APIs. Missing metrics are not accepted as zero and
not counted as product timings. The other two failures are real recorded
budget overruns: partial-upload frame 498 ms / 33 ms, and the full-size effects
window 513 ms / 150 ms.

The six affected tests now wrap the actual async APIs, capture their measured
CPU copy/install costs, and draw the result frame after awaited installation.
The separate animation-frame gap monitors still charge every yield and stall.
Synchronous `layerPixels` instrumentation remains synchronous. An AST comparison
against the published file verifies all 100 assertions, 14 test titles and 16
skip/timeout/browser-option calls are identical (`perf-instrumentation-integrity.json`).
TypeScript checks pass. The six corrected cases finish five passed / one failed
(`perf-cooperative-instrumentation-direct.log`): C1's 100 MP blank Fill installation
records 508 ms / 450 ms, with result frame 18 ms and maximum gap 23 ms. The
previous missing metrics are superseded by these real timings, not omitted.
Across the 29 distinct performance cases, the latest applicable results are
22 passed / seven failed. This is coverage across separate runs, not a passing
single full performance suite or stable acceptance.

## Production package follow-up

Portable `Compositor-portable-0.8.0-20261003-1206.zip` contains the clipboard
origin/DIB fixes and the final buffer retirement code. Its marker is
`COMPOSITOR_BUILD_0.8.0_20261003-1206`; `package-bitmap-final.log` records the
successful optimized native build. Earlier portables and failed logs remain.
ZIP: 4,908,294 bytes, SHA-256
`DE8B480807C8FDD9E67311D70BCF3886A30D178B193FE5539A86C046911D1496`.
EXE: 11,975,168 bytes, SHA-256
`09F544DB8B62077974844AD0E61D27DF7ADAA58DB3254E91D83D012735034FB1`.
ZIP CRCs pass, the executable matches the archive, and both production and
fixed-test assets carry the release WASM hash above. All three native Mac PNGs
match the previously validated Windows exports exactly; all 34 re-saved and
continued package files match the prior checked native outputs. This retains,
not resolves, the previously quantified Mac-export edge differences. Evidence:
`final-delivery-checks.json`.

The real production window passes seven UI groups with zero page errors and
no development test API, including effect preview toggles, exact parameter
retention after cancel, and effect undo/redo. Evidence:
`native-ui-bitmap-final.log`, `native-final/native-overlay-0.8.0-result.json`.

The same package opens the three 1.4.5 return projects, saves copies and reopens
continued edits: four native reads, four atomic package commits, zero page
errors. This run additionally asserts text font-size undo/redo, rounded-shape
width undo/redo, and effect preview/cancel before Apply. The reopened values
remain font size 38, shape width 150, Stroke 7 and Drop Shadow Distance 14.
Only file-picker choices are substituted; reads, edits, rendering and writes
use the production bridge. Evidence: `native-mac-bitmap-final.log` and
`native-final/native-mac-return/`. This proves the Windows half of those gestures.

The first final external clipboard run aborts at its read-only snapshot:
`CLIPBRD_E_CANT_OPEN` (0x800401D0). No clipboard mutation occurs. The production
read-only probe also reports Windows error 5, and an independent Win32 probe
cannot open the clipboard (error 5, no locking window). The process is on
`WinSta0`, and the readable input desktop is `Default`; a locked desktop is not
established as the cause. Earlier intermediate builds completed native clipboard
protocol checks but failed bitmap pixel comparison, exposing the DIB bug fixed
here. At 12:46 the independent Win32 probe confirms access has returned. The exact
1206 portable then passes six native protocol groups with zero page errors and
no development API: external PNG paste, independent PNG consumption of native
Copy, synthesized Windows bitmap paste, independent bitmap consumption of Cut,
Cut/Paste with Undo/Redo, and private-token editable text paste. All four saved
and consumed PNG/bitmap comparisons have maximum channel difference **zero**,
including alpha and row ordering; two editable UTF-16 text records persist.
The original clipboard is restored with its original format set. This closes
the final native clipboard gate; the denied preflight above remains historical.
Evidence: `native-clipboard-restored-access.log`,
`clipboard-20261003-124643/clipboard-image-checks.json` and the native result.

## Rejected performance experiments

Three controlled browser experiments used the original case bodies and assertions,
with diagnostic wrappers outside the official runner. They are diagnostic
evidence only and do not replace the original acceptance runs above.

- Repeating a known uniform fill's first 4 MiB source chunk did not improve
  100 MP installation CPU time: 260 ms baseline versus 272 ms with the experiment.
  The experimental run also failed the unchanged gradient frame-gap assertion.
  No corresponding application change was adopted.
- Requesting a CPU-backed overlay context did not give stable performance:
  F1 failed at 182 ms baseline and 392 ms experimentally against 100 ms;
  24 MP Content-Aware Fill failed at 232.3 and 195.8 ms; the 100 MP case passed
  baseline but failed experimentally at 111.1 ms against 100 ms.
  No overlay context change was adopted.
- Reusing same-size GL texture objects did not close the failing frame budgets.
  The experimental F1 result failed at 283 ms against 100 ms; its effects-image
  case failed at 158 ms against 100 ms (baseline effects: 162 ms). Partial uploads
  passed experimentally but failed at 284.5 ms against 33 ms in the baseline.
  This mixed result is insufficient evidence of stable improvement, so no
  texture-cache change was adopted. The staged WASM destination already reserves
  its complete plane capacity before copying; chunk-by-chunk vector growth is
  not an outstanding allocation issue in this implementation.

Evidence: `uniform-copy-diagnostic-2.json`,
`overlay-context-diagnostic.json` and `texture-reuse-diagnostic.json`, with their
retained logs.

## Pixel-copy source work in progress

An in-memory-only placement diagnostic retains the returned files byte for byte.
For `Mac-edited`, replacing only fractional layers' draw sampling with Nearest
reduces the original 2,118 changed pixels / maximum 121 to **zero differences**.
For `Mac-created`, it reduces 14,037 changed pixels to 13,117, but differences
remain in scaled text and 52 rectangle-edge pixels. Changing the saved origin or
sampling mode is not adopted as a fix. Evidence:
`pixel-placement-diagnostic/comparison.json`, plus its variant PNGs and runner.

The authoritative source is `git show v1.4.5:Compositor/Rendering/LayerRenderer.swift`
in the adjacent `Compositor-1.2.10` repository, not that repository's older
checkout or the unversioned adjacent `Compositor` directory. Lines 44–47 explicitly
choose `.none` for upright final sampling within 0.001 of 1:1. Line 19 preserves
antialiasing according to the layer's saved setting. The initial copy-only
diagnostic's residual rectangle-edge differences therefore need edge coverage,
not a saved-coordinate change.

Current CPU and GL source work chooses that filter at the final prefilter/device
scale, including clipping sources and reduced/full effects representations.
Only outer rectangle edges receive coverage; internal texture-chunk edges retain
their disjoint geometry. Explicit Nearest keeps its hard edge. Stored transforms,
sampling settings and bitmap records remain unchanged. New regression coverage
checks transparent texel preservation, clipping-source alpha, rectangle edges,
rotation/enlargement exclusions and GPU/CPU agreement. Release WASM, original
Mac return exports and the new production package must still verify this source
before the pixel-copy gate can be closed. Scaled-text fidelity remains open.

The first complete release-WASM checkpoint passes 620 native tests (0 failures,
10 existing ignores), 269 UI unit tests and 191 functional browser cases (29
unchanged opt-in performance skips). Its source/package WASM SHA-256 is
`C801FCED79B12E62F39B4E83F531DB962603C19D3C0A34EB9ACC4EACA61EFCF4`.
Against untouched Mac returns, `Mac-no-edit` and `Mac-edited` now export with
**zero changed pixels**. `Mac-created` improves to 13,089 changed pixels / max 51;
24 of these are shape-edge rounding, the remainder is enlarged text.
Evidence: `pixel-copy-mac-checks/comparison.json` and the PNGs.

The upright 1:1 probe initially retains 54 changed boundary pixels / max 8 in
straight RGBA, but only 1 in premultiplied channels. Quantizing rectangle coverage
to `floor(coverage * 255) / 255` before multiplying colour/alpha gives an exact
scratch-model match to that Mac 1.4.5 PNG. The quantized CPU/GL source and a new
four-flip regression are undergoing their own final WASM/build checks; earlier
checkpoint results do not establish those newer checks.

The first probe GPU comparison was captured before the viewport had settled;
its apparent 150-level difference was not a proven application regression.
After fixing the diagnostic to align the viewport on device pixels and draw
synchronously, both the upright and flipped layer agree with CPU within 1/255.
Evidence: `pixel-copy-upright-final-frame.log`; the earlier diagnostic logs remain
retained. Export comparisons above do not depend on viewport alignment.

The final quantized release-WASM checkpoint also passes 620 native tests with
0 failures / 10 existing ignores, both TypeScript checks and all five rendering
cases, including the four-flip edge test. Source and fixed test assets share
SHA-256 `22665BCEDD97916CB4A88433039FA51A0123DD6435CBBEEECEFFAA3464E43607`.
Actual exports now match `Mac-no-edit`, `Mac-edited` **and** the upright 1:1 probe
exactly. The probe's GPU and CPU agree within 1/255. `Mac-created` retains
13,089 changed pixels / max 51, including its 24 overlapping shape-edge pixels.
Evidence: `pixel-copy-coverage-mac-checks/comparison.json`, `checks.json`,
`pixel-copy-coverage-workspace.log`, `pixel-copy-coverage-render-e2e.log` and
`coverage-quantization-model.json`. The quantized full browser suite passes
**192 cases**, with the same 29 opt-in performance skips. The new production
package results are recorded below; performance remains open.

## Production 1455 pixel-copy checkpoint

Commit `051d23bf488bfdf2822e7f414a8d214b0abf4bc1` is pushed and matches
`origin/codex/phase4-and-phase5`. The portable package is
`build-artifacts/windows-x64/Compositor-portable-0.8.0-20261003-1455.zip`,
4,743,423 bytes, SHA-256
`48E83A21F5A0ECDBF5E381D082E80763F952C498459D3A9D9C5CA8E9BCD7FCD6`.
Its EXE is 11,976,704 bytes, SHA-256
`F97BC821E15CB0A09064D14C067EC2E8AD8FFDEC32FD07D3F2E2FB35191F53C4`.
Bundled production WASM matches the quantized test WASM above. The build wrapper
preserves and restores the original build-info bytes; the historical portable
packages and validation outputs remain retained.

Real production Tauri checks pass all seven UI groups with no page errors and no
development API. Real package I/O checks also pass: four reads, four atomic write
commits, preserved editable text and continued Mac-edited text/shape/effects
edits reopened with width 150, stroke 7 and shadow 14. All three native opening
exports are byte-identical to their current WASM exports. Mac-no-edit and
Mac-edited remain exact against the returned Mac PNGs; Mac-created retains the
same 13,089 changed pixels / max 51. Evidence: `native-ui-pixel-copy.log`,
`native-pixel-copy-0.8.0-result.json`, `native-mac-pixel-copy.log`, and
`native-pixel-copy-mac/native-pixel-comparison.json`.

The 1455 clipboard rerun fails **before mutation**: the helper cannot snapshot
the original clipboard. A separate read-only Windows probe verifies no open
clipboard window, WinSta0/Default, and `OpenClipboard` error 5. This is current
OS access rejection, not proof of an application bitmap regression or a
successful current-package protocol check. Earlier 1206 six-group/four-image
proof remains retained. A Windows manual confirmation is requested; current
clipboard contents were not changed. Evidence: `native-clipboard-pixel-copy.log`,
`clipboard-20261003-150934/helper-error.txt` and
`clipboard-pixel-copy-readonly-preflight.json`.

The uniform-mask prototype does not apply to the failing Add Mask fixture,
which is an antialiased ellipse and hence nonuniform. It records **zero**
uniform-upload substitutions. The baseline fails at 519/150 ms, one no-op variant
passes, and the other fails at 371/150 ms. No optimization is adopted and that
isolated pass does not establish stable performance. Evidence:
`mask-uniform-diagnostic.json` and its retained log. The 24 remaining overlapping
shape-edge pixels have an exact independent model after rounding covered source
bytes before blending; this model is not yet a production fix. Evidence:
`copied-overlap-rounding-diagnostic.json`.

## Further GPU diagnostics

The partial-upload/Add Mask trace reproduces the 24 MP Add Mask result-frame
failure (415 ms / 150 ms). Its marked `readPixels` call consumes 412.7 ms;
overlapping trace events show `WaitForGetOffset` at 412.68 ms and the GPU WebGL
command-buffer execution at 396.95 ms. The partial-upload case passes in this
diagnostic. These measurements locate the reproduced wait, not every previous
failure or a proven root cause. Evidence: `trace-partial-mask-summary.json`,
the full trace and log.

Serializing upload/draw commands with diagnostic-only `gl.finish()` still leaves
100 MP Add Mask frames at 474/496 ms against 400 ms; no such synchronization is
added to the application. Reusing one same-size retired mask with a full
`texSubImage2D` also leaves failures (407/506 ms against 400 ms), so no spare-mask
cache is adopted. Evidence: `mask-gpu-phases.json` and
`mask-spare-diagnostic.json` and their logs.

Backend inventory verifies Intel hardware Vulkan and D3D11on12 contexts. The GL
request falls back to SwiftShader, which is rejected as hardware acceptance.
Original case-body diagnostics still fail Add Mask under Vulkan (458 ms / 150 ms)
and D3D11on12 (529 ms / 400 ms); Vulkan also fails partial uploads (401 ms / 33 ms).
Production remains on the original hardware D3D11 path. Alternative-backend
passes do not supersede the official 22/7 result or establish acceptance.
Evidence: `angle-backend-inventory.json` and `angle-budget-diagnostic.json`.

## Covered source-byte rounding checkpoint

The CPU compositor and GL layer shader now round covered premultiplied source
channels to bytes before blending them over the backdrop. A regression derived
from the Mac-created return checks `[28,29,47,255]` for its partially transparent
edge over `[3,4,24,255]`. The source fix removes all 24 remaining rectangle-overlap
differences without changing the saved transforms or sampling settings.

Release WASM SHA-256 is
`A2083E169200D98D85728AC2F4C1977EA8AA870F14D29C4E77C882682727B919`;
the fixed test assets contain the same bytes. Workspace tests pass 621 with zero
failures and the same 10 ignored. All three TypeScript checks pass. The complete
functional browser run passes 193 with the same 29 opt-in performance skips;
the separate rendering run passes all 14 render/adjust/zoom cases. An earlier
render runner encountered an overlapping preview server and connection-refused
errors; its log is preserved and does not represent a pixel assertion failure.

Actual exports of all three Mac returns and 11 sampling probes preserve every
transform. Mac-no-edit, Mac-edited, explicit Nearest enlargement and the upright
fractional/flipped 1:1 probe are exact. Mac-created now has 13,065 changed pixels,
maximum difference 51, confined to the enlarged text at `[137,267,1280,339]`.
Its overlap edges are exact; enlarged-text, scaled and rotated probe differences
remain open. Evidence: `copied-overlap-mac-checks/checks.json`, `comparison.json`,
`copied-overlap-workspace-final.log`, `copied-overlap-render-cmd.log` and
`copied-overlap-functional.log`.

A diagnostic-only viewport-mask prototype reduces a 6000 x 4000 nonuniform mask
upload to 1136 x 798 samples. After correcting its cache transition, the actual
frame differs by at most one channel level, with zero samples above the existing
GPU tolerance of two. The original Add Mask case passes for both the baseline
and prototype in the subsequent isolated diagnostic. This does not establish a
fix for its intermittent performance failures, so the prototype is not adopted.
The original failing cache-transition log remains. Evidence:
`mask-viewport-pixels-cache-fix.log` and `mask-viewport-diagnostic.json`.

Fitting the Mac-created text's alpha by sampling phase still leaves residuals;
77 of 81 phase groups cannot be matched by any four-tap linear coefficients
followed by a single nearest-byte rounding within that grouping. This is a
diagnostic constraint on the model, not a general claim about Core Graphics.
Neither fitted coefficients nor the approximate eighth-phase filter is adopted
as an exact Mac renderer. Evidence: `text-phase-fit-diagnostic.json`.

## Production 1556 covered-source checkpoint

`COMPOSITOR_BUILD_0.8.0_20261003-1556` contains the covered-source fix and the
release WASM above. Its portable ZIP is 4,743,691 bytes, SHA-256
`CAB371029BE5FBE2FE41CA84F8912C7876974437B0FC691198CE217ED312AA3A`.
The 11,976,704-byte executable has SHA-256
`8E6B2456BF31519B498DD95A07A1E503D715ABB52C181CB137A207AB9E739954`.
Archive CRCs pass and the archived executable matches the one actually tested.
The build-info source bytes are restored after packaging.

The real production window passes all seven UI groups with zero page errors and
no development test API. Four native Mac package reads and four atomic save
commits pass, including continued editable text, shape width 150, Stroke 7 and
Drop Shadow Distance 14 after save/close/reopen. All three native open exports
are byte-identical to the fixed release-WASM exports; the remaining Mac-created
text differences above therefore also apply to this production package.
Evidence: `native-ui-copied-overlap.log`, `native-mac-copied-overlap.log`,
`native-copied-overlap-mac/` and `copied-overlap-package-integrity.json`.

Clipboard access has returned: production 1556 passes all six native clipboard
groups through the real bridge and an independent Windows Forms consumer.
All four saved/consumed PNG and bitmap checks have maximum channel difference
zero, including alpha and row ordering; two editable UTF-16 text records persist.
The original format set is restored. The previous access-denied preflights are
retained as historical failures, not a current 1556 clipboard failure. Evidence:
`native-clipboard-copied-overlap.log` and `clipboard-20261003-161117/`.

The subsequent single complete performance run uses the same release WASM and
fixed assets, after compilation and native GUI checks have finished. All 29
original cases run with one worker and no retries: **27 pass / two fail** in
8.3 minutes. The failures are the 24 MP Levels histogram frame gap (151 ms,
budget strictly below 150 ms) and the 100 MP blank-layer gradient frame gap
(102 ms, budget strictly below 100 ms). Neither limit is changed. The earlier
seven failure groups do not all reproduce in this run, but a single passing
observation does not establish a fix or stable acceptance for intermittent
failures. This current full-run result supersedes the earlier 22/7 aggregation
as the latest observation, not as full acceptance. Evidence:
`copied-overlap-performance.log`.

## Production 1654 asynchronous histogram checkpoint

Levels and Curves now prepare large histogram inputs with the existing cooperative
copy path. The panel remains editable while the copy is pending; a request epoch
rejects input/results from an older instance after close/reopen. Allocation failures
close the waiting panel and report the error. Both stored-buffer copies and staged
result installation yield before copying if their initial allocation already consumes
the 8 ms quantum. CPU accounting includes allocation and excludes awaited frames.

Validation: 274 unit tests across 42 files pass, all three TypeScript checks pass,
and the fixed-asset build passes. The complete functional suite passes 193 cases
with the same 29 opt-in performance skips. The engine is unchanged: the 621 native
passes and 10 existing ignores above still apply. New tests cover waiting-panel
editing, stale same-kind reopen results, asynchronous failure, exact staged bytes,
and cancellation during the initial yielded frame. Evidence: `histogram-async-unit.log`,
`histogram-async-test-build.log`, `histogram-async-functional.log` and
`allocation-frame-client-final.log`.

`COMPOSITOR_BUILD_0.8.0_20261003-1654` portable ZIP is 4,743,566 bytes, SHA-256
`A8633356C737FCD34D98C1ABDCB2E7C2D9C547F2835369B07EACFC94F7530E9F`.
Its 11,976,704-byte executable has SHA-256
`5BF540A516F151FB8FBC2BC3CD4F26D1F64AC3F89026A82CBDFE9531937B00E4`.
The archived executable matches the actual tested package and CRC checks pass;
build-info source bytes are restored. Release WASM remains the same as 1556.

The real production window passes eight UI groups with no development test API
and zero page errors. The additional group fills a 24 MP canvas, opens Levels
while the histogram is pending, cancels, reopens and waits for the new histogram.
Four native Mac package reads and four atomic save commits pass; edited font 38,
shape width 150, Stroke 7 and Shadow Distance 14 persist after reopening.
All three native open exports are byte-identical to the fixed release-WASM
exports. Evidence: `native-ui-histogram-async.log`,
`native-async-jobs-0.8.0-result.json`, `native-mac-histogram-async.log`,
`native-async-jobs-mac/` and `histogram-async-package-integrity.json`.

The 1654 read-only clipboard preflight reports Windows access denied (OS error 5).
The mutating protocol suite is not attempted against that preflight. The successful
1556 six-group/four-exact-image evidence remains valid for 1556; it does not establish
the same protocol result for 1654. No user clipboard data is changed by this denied
preflight, and no desktop permissions are changed to bypass it.

The two targeted original job performance cases pass, but the subsequent single
complete 29-case run is **26 passes / three failures** in 8.5 minutes. This is the
latest full-run observation, superseding the historical 27/2 above. The histogram
case passes (24 MP gap 87 ms, 100 MP gap 23 ms). The failures are:

- F1 100 MP Levels: worker-edit frame gap 109 ms, strictly below 100 ms required.
  The actual result-display frame is 12 ms and installation CPU time is 363 ms.
- Growing-mask 24 MP gradient at fit: drag total 64 ms, strictly below 50 ms
  required (engine 12 ms, frame 53 ms).
- C1 100 MP blank gradient: worker-edit frame gap 114 ms, strictly below 100 ms
  required (result frame 17 ms, installation CPU time 355 ms).

Evidence: `histogram-async-performance-targeted.log` and
`histogram-async-performance.log`. Budgets, assertions, warm-ups, skips and retries
are unchanged. Diagnostic traces execute the original case bodies and assertions,
but do not reproduce those three failures; C1 instead has a 24 MP result-frame
failure of 517 ms against its 350 ms budget. The 100 MP blank-gradient reservation
contains a 64 ms V8 sweeping completion. This identifies a remaining allocation
cost, not a complete root cause or stable fix. Evidence:
`async-histogram-job-trace-results.json` and `async-histogram-*-trace.json`.
A diagnostic mask-storage reuse prototype fails the original 24 MP post-apply
frame budget (423 ms / <150 ms), while the isolated baseline passed. It is not
adopted; evidence: `mask-storage-job-trace-results.json`.

## Output capacity and texture-object work in progress

Blank pixel Fill/Gradient jobs can now borrow an existing exact-size canvas RGBA
spare immediately before worker posting. It is a separate output-capacity buffer,
never a WASM input raster. The worker copies its complete output into that buffer;
unused capacity is returned with the normal recycled inputs. Mask edits and
histograms do not borrow it. This avoids simultaneously retaining the old spare
on the main thread while allocating the same-sized blank-layer result.

The real-worker regression verifies complete-byte equality against the direct
engine for both Fill and Gradient, exactly one undo step, blank undo, exact redo
and real transfer detachment. The initial fixture had no layer and failed before
any edit; adding its explicit blank layer corrects the fixture. Original assertions
and timeouts are preserved. Evidence: `blank-output-pool-staging-final.log`.
The output-only source checkpoint passes 277 unit tests and 194 functional cases
with 29 unchanged opt-in skips (`blank-output-pool-unit-full.log`,
`blank-output-pool-functional.log`). Its targeted performance run passes C1 and
growing-mask gradients, but F1 fails with a 24 MP Levels result frame of 511 ms;
100 MP fill also logs a 104 ms worker-edit frame gap. Those failures are retained
in `blank-output-pool-performance-targeted.log`.

A GPU trace of the original F1 body fails at a 101 ms timed 24 MP Fill result frame:
95 ms is `readPixels`, overlapping a 94.5 ms GPU WebGL command flush. A separate
325 ms cold warm-up readback overlaps about 304 ms of Chromium raster-worker
flushing; it is not a timed acceptance failure. This establishes GPU command waits,
not a complete explanation for the unprofiled 511 ms observation. Evidence:
`blank-output-gpu-trace-summary.json` and `blank-output-gpu-job-trace-results.json`.
The host has about 18 GiB free physical RAM at the subsequent snapshot; low free
system RAM is not inferred as the cause.

A diagnostic same-sized texture-object reuse variant passes the original F1
assertions, retaining all required `texImage2D` whole uploads. The source now keeps
existing chunk GL objects for matching dimensions and prefilter levels, replaces
every texel and updates filters; different dimensions or levels still rebuild.
No upload assertion is changed. A chunk-boundary unit regression covers the new
bytes/revision, filter mode, both full chunk uploads, resized replacement and
separate document ownership. Current 278 unit tests and all three TypeScript
checks pass; the rebuilt fixed assets pass. Complete performance and functional
results are pending in this work-in-progress section.

An independent host PInvoke probe confirms current `OpenClipboard` access denied
(error 5) on WinSta0 / Default in active console session 1, with no clipboard
owner process returned. It reads no clipboard contents and performs no mutation
or permission bypass. Evidence: `windows-clipboard-host-access-probe.json`.

## Production 1901 output capacity and texture reuse checkpoint

`COMPOSITOR_BUILD_0.8.0_20261003-1901` contains the reviewed output-capacity
and texture reuse changes (source commit `ca361b7d0de45f2870cee142c1b413aa2193f3d8`,
verified at the authorized GitHub branch). The portable ZIP is 4,743,463 bytes,
SHA-256 `26D10B5F0038C64560278F1D11E1A01EC659DF130EC2B5261285CB8EF8B4D34F`.
Its 11,976,704-byte executable has SHA-256
`C61FE8B5BA54D3C514807CB8D96608E2E3FDCEA5CCC36C964A49F0A7554D63B1`.
ZIP CRC checks pass and its executable exactly matches the tested portable file.
The build-info source bytes are restored; the release and bundled WASM SHA
remains unchanged. Evidence: `output-capacity-package-build-canonical-env.log`
and `output-capacity-package-integrity.json`.

Eight real native UI groups pass, including 24 MP fill and asynchronous histogram
cancel/reopen, with the production native bridge, no development test API and
zero page errors. Four Mac package reads and four atomic save commits also pass.
Continued edits preserve the font size 38, shape width 150, Stroke 7 and Shadow
Distance 14 after reopening. All three native Mac open PNGs are byte-identical
to the prior 1654 exports, so the exact no-edit/edited matches and the remaining
Mac-created enlarged-text discrepancy still apply. Evidence:
`native-ui-output-capacity.log`, `native-output-capacity-0.8.0-result.json`,
`native-mac-output-capacity.log` and `native-output-capacity-mac/`.

A separate normal-user host PInvoke preflight now opens/closes the clipboard
without reading contents. Its captured GetLastError is stale on success and is
not treated as a failed call. This alone does not prove OLE snapshot or native
HWND clipboard access: the 1901 native UI read still reports access denied, and
the independent protocol helper fails at OleGetClipboard with
`CLIPBRD_E_CANT_OPEN (0x800401D0)` before snapshot completion or any mutation.
The protocol suite therefore never starts, and user clipboard contents are
unchanged. No permission/desktop change, protocol retry or unsafe snapshot
workaround is used. Evidence: `output-capacity-clipboard-preflight.json`,
`native-clipboard-output-capacity.log` and
`clipboard-20261003-191221/helper-error.txt`. The prior successful production
1556 clipboard evidence is retained; it is not a 1901 acceptance result.

The subsequent C1 diagnostic runs its unchanged original body and assertions.
It does not reproduce the full-run 100 MP gradient gap failure (96 ms / <100 ms
in the diagnostic), but fails the 24 MP gradient result-frame budget at
366 ms / <350 ms and also records a 100 MP fill worker-edit gap of 463 ms
against <100 ms. The failing 24 MP frame includes a 358.5 ms readPixels call
that overlaps 357.0 ms of GPU command-buffer flushing. This identifies another
synchronous GPU wait without proving the cause of the separate full-run
160 ms gradient gap. These failures remain open; no untested scheduling or
sampling change is adopted. The first diagnostic matched an incorrect case
prefix and failed before executing a case; the corrected entry selects the
original `ruling C1:` test. Evidence: `texture-c1-profile-job-trace-final.log`,
`texture-c1-profile-job-trace-results.json`, `texture-c1-profile-summary.json`
and `texture-c1-profile-blank-gradient-trace.json`.

## Completed-job scratch heap reclamation in progress

A more detailed C1 trace keeps the original body and assertions. Its failures
are 24 MP and 100 MP gradient worker-edit gaps of 333 ms and 385 ms / <100 ms.
The corresponding rAF intervals overlap 327.2 ms and 369.3 ms of GPU
RendererCompositor command flushing, with no overlapping main-thread long task.
The 334 ms GetGLError wait is in the first navigation's cold warm-up, not either
failed timed gradient window. No per-frame application getError call exists,
so removing one would not fix those failures. Evidence:
`texture-c1-detailed-job-trace.log`, `texture-c1-detailed-summary.json` and
`texture-c1-detailed-blank-gradient-trace.json`.

The separate diagnostic-only `--enable-gpu-service-tracing` launch records
per-driver GL calls using [Chromium's documented tracing method](https://chromium.googlesource.com/chromium/src/+/master/docs/gpu/debugging_gpu_related_code.md).
It passes the original C1 assertions once, while its timed 100 MP result frame
still contains 351.6 ms in glReadPixelsRobustANGLE (367 ms total / <600 ms).
It does not establish a source fix or close earlier intermittent failures.
The official regression configuration receives no extra tracing flags. Evidence:
`texture-c1-service-job-trace.log` and `texture-c1-service-blank-gradient-trace.json`.

Texture inspection disproves the proposed level-only reuse opportunity: the
24 MP gradient changes from a 1500 x 1000 preview to a 1264 x 1000 result; the
100 MP gradient changes from 1250 x 1250 to 1123 x 1250. No equal-sized texture
can be reused for these transitions. The first inspection attached too early
before a renderer existed; the corrected attachment records all four navigations.
Evidence: `texture-c1-texture-inspect-job-trace-final.log` and
`texture-c1-texture-transitions.json`. A same-process, same-STA clipboard
comparison fails Win32 open with both a null handle and a valid own hidden
window (error 5), and fails OLE access with CLIPBRD_E_CANT_OPEN. It reads no
contents and changes no data or permissions. Evidence:
`clipboard-owner-compare.json`. The host's power plan is read-only checked
as Balanced; no power setting is changed.

A separate 400 MB synthetic copy experiment compares fresh browser pages.
Original staged WASM copies take 254.2 and 302.0 ms CPU. Zero-prefilling each
chunk before its copy costs 326.9 ms total (276.6 ms just prefilling); plain
ArrayBuffer copying costs 339.6 ms and verifies every byte. Prefill adds CPU
cost and is not adopted. Evidence: `staged-copy-prefill-diagnostic.json`.

The original F1 diagnostic then records a 100 MP Levels worker heap of
960,626,688 bytes after its completed job. That is below the original absolute
1 GiB cap, so it otherwise stays resident while the main engine installs its
400 MB result. An early-reclamation prototype retires that idle heap and
passes the unchanged F1 body, with 308 ms installation CPU, 12 ms result frame
and 20 ms worker-edit gap for 100 MP Levels. It does not prove stable acceptance
by itself. Evidence: `worker-768-prototype-job-trace.log` and
`worker-768-prototype-job-trace-results.json`.

The source now retires a worker after a completed result above 768 MiB, before
resolving that result to its caller. The original 1 GiB absolute cap and fatal
trap handling remain. Transferred pixels belong to the main thread already;
no input retry or result reexecution occurs, and queued jobs use a fresh worker.
A regression covers the measured heap size below the absolute cap, unchanged
result bytes and the queued job on the replacement. All 279 unit tests, all
three TypeScript checks and the fixed-asset build pass. Six real-worker cases
pass, including exact selected Levels/blur pixels and undo, complete Fill/Gradient
bytes, transferred output reuse and rejection of incomplete staged planes.
The complete original performance run finishes with 20 passes and nine failures
in 9.1 minutes. Functional regression passes all 194 cases with the 29 unchanged
opt-in performance skips in 6.1 minutes. Both run sequentially with one worker
and no retries, retaining separate outputs and all prior failed evidence.
100 MP Levels installation is 284 ms / <450 ms (24 MP 64 ms); 100 MP blank Fill
installation is 269 ms. GPU result frames remain over budget: 24 MP Levels
312 ms and 100 MP Levels 230 ms / <100 ms. Other failed gates are Add Mask,
partial uploads, histogram/commit, gradient previews, growing-mask gradients,
small-layer mask Fill, gradient tool on a small mask, and 100 MP brush/retouch
(ContentFill frame gap 101.5 ms / <100 ms). C1 passes this run, but previous
intermittent failures remain open. Evidence: `worker-reclaim-source-check.log`,
`worker-reclaim-jobs-e2e.log` and `worker-reclaim-regression-summary.json`.

## Preview-to-result texture grid reuse in progress

The first F1 texture diagnostic selected the separate Delete-after-job case
because its title shares the prefix; its evidence is preserved and is not used
to justify the result-frame change. The corrected selector runs the unchanged
F1 result body and assertions, recording all four navigations. For Levels, the
settled 24 MP preview and result both upload 1500 x 1000, with prefilter levels
1 and 2; the 100 MP pair both upload 1250 x 1250, with levels 1 and 3.
The diagnostic passes once (100 MP Levels installation 270 ms, result 75 ms,
worker gap 25 ms); this does not close the preceding full-run failures.
Evidence: `texture-f1-result-level-inspect-job-trace.log`,
`texture-f1-result-level-inspect-job-trace-results.json` and
`texture-f1-result-level-transitions.json`.

The source now reuses existing GL chunk objects for the same physical raster
dimensions even when its source prefilter level changes. Byte keys, revisions,
filter mode and level metadata still change; every texel is replaced through
the existing complete texImage2D uploads. Changed dimensions and separate
documents retain their existing ownership behavior. The additional regression
checks a two-chunk boundary, every byte, LINEAR filter updates and the new
level-sensitive upload cache. The initial source replacement did not match
CRLF line endings; the new regression failed against the unchanged guard, and
its failed log is retained. After applying and verifying the actual source diff,
all 280 unit tests, all three TypeScript checks and fixed-asset build pass.
Six real-worker regressions pass. The subsequent complete performance run is
interrupted by a publication tool timeout resetting its execution session, before
any case reaches a terminal result. It is not counted as an application failure
or a passing run; logs and summary remain under `texture-level-interrupted-*`.
The checkpoint is published as `5ea7db4438d9cd98e984319058d367e3766ceff4`,
verified against the authorized remote branch. Evidence:
`texture-level-source-check-final.log`, `texture-level-jobs-e2e.log` and
`texture-level-source-remote-verify.log`.

## First draw fits a newly opened large canvas

The F1 texture inspection records an unnecessary initial full-resolution
upload: a new 6000 x 4000 or 10000 x 10000 canvas draws the default 1:1
viewport before ResizeObserver fits it. The next draw drops those textures
and uploads the reduced fit grid. This is separate from the timed GPU stalls;
no claim that it explains all of them is made.

CanvasView now applies its existing resize/fit synchronously in the size effect
before the draw effect. ResizeObserver retains its later behavior, including
manually zoomed/panned viewports. All 280 unit tests, three TypeScript checks
and the fixed-asset build pass (`first-fit-source-check.log`). Both original
partial-upload cases pass. The new regression verifies actual initial upload
sizes and exact RGBA bytes after opening a 24 MP canvas under the unchanged
default timeout, passing in 8.4 seconds. The initial colour sample landed on
the retained 10 x 10 white centre of CanvasSize; moving it into the newly
filled region fixes the fixture without loosening the exact comparison.
Evidence: `first-fit-targeted-final.log` and
`first-fit-targeted-pixels-simple.log`.

The first launcher stops before tests because the interrupted preview retains
port 1423. Its PID, creation time and command line establish ownership before
terminating only that test preview. A subsequent launcher fails before tests
because the reconstructed COMSPEC path uses forward slashes; restoring normal
Windows separators fixes shell launch. An anchored selector matches no full
Playwright title; the corrected selector runs the new case. These attempts
remain logged separately and are not application acceptance failures.

The complete original performance run finishes 21 passes / eight failures
in 8.7 minutes. All 195 functional cases pass in 6.0 minutes, with the unchanged
29 opt-in performance skips. Both runs use one worker, zero retries, no tracing
flags and unchanged budgets. 24/100 MP Levels result frames pass at 7/60 ms;
100 MP installation is 295 ms / <450 ms. The remaining failed observations are:

| Gate | Measured ms | Original upper bound ms |
| --- | ---: | ---: |
| F1 100 MP blank Fill result frame | 256 | <100 |
| Add Mask 24 MP Reveal Selection frame | 433 | <150 |
| Tab drag initial frame gap | 359 | <100 |
| Effects preemption replacement dispatch | 734 | <400 |
| Growing-mask gradient result frame | 350 | <150 |
| Small-layer mask Fill worker frame gap | 455 | <100 |
| Eyedropper sample and ring | 17.1 | <16 |
| C1 24 MP blank-layer result frame | 501 | <350 |

All previous failures are retained; changing which cases pass in a new run does
not close their intermittent failures. Logs: `first-fit-performance.log`,
`first-fit-functional.log` and `first-fit-regression-summary.json`.
The first-fit checkpoint is published as
`b0f2222d832d7e453b44ac8e10fa3a5b36523f38`, verified against the authorized
GitHub branch. Production 1901 evidence still applies to the preceding revision;
production 2041 builds sequentially after functional success and passes the
package/native checks recorded below.

## Production 2041 checkpoint

`COMPOSITOR_BUILD_0.8.0_20261003-2041` packages source checkpoint
`b0f2222d832d7e453b44ac8e10fa3a5b36523f38`. Its 4,743,715-byte ZIP has SHA-256
`8C9C95C65A937BA00E5124EB0B18FF1784BCEA73DEB42F48FC6B065ABF80CD4B`.
The 11,976,704-byte executable SHA-256 is
`FE59FCA1784D9FA1D6B6F1856002E509538ABE7A0FCD54831AE6889AA99F1688`.
All three ZIP entries pass CRC checks and the archived executable equals the
tested portable. Bundled production/test WASM remains the unchanged release
SHA recorded above. Build-info source bytes are restored; the build completes
after all functional tests, so it does not load the performance run. Evidence:
`first-fit-package-build.log` and `first-fit-package-integrity.json`.

Eight real native UI groups pass with no development API, the actual native
bridge and zero page errors, including painting/retouch, text/effects and 24 MP
fill plus asynchronous histogram cancel/reopen. Four native package reads and
four atomic save commits pass for the returned Mac projects and a continued
Windows edit. The continued manifest records font size 38; reopen confirms
shape width 150, Stroke 7 and Shadow Distance 14. All three open PNGs are
byte-identical to production 1901, preserving the exact no-edit/edited matches
and the open enlarged-text discrepancy. Evidence: `first-fit-native-ui.log`,
`first-fit-native-mac.log`, `native-first-fit-0.8.0-result.json`,
`native-first-fit-mac/` and `first-fit-native-mac-export-comparison.json`.

The native read-only clipboard probe again reports Windows access denied.
No current clipboard protocol write is attempted or counted as passed. The
existing successful production 1556 proof remains historical. A hash-guarded
local handoff, `verify-2041-clipboard-interactive.ps1`, can run the existing
unaltered protocol suite on the user interactive desktop, retaining its full
snapshot-before-mutation and finally-restoration rules. Its syntax passes; it
has not been run by the user and provides no clipboard acceptance evidence yet.
The asynchronous question whether ordinary Windows Notepad copy/paste works
is pending. No permissions, desktop state or clipboard contents are changed
to bypass this gate. All eight current performance failures and pending Mac
oracle/gesture confirmations remain open.

## Interactive priority starts before input preparation

The effects-preemption timing diagnostic shows a 218 ms input preparation before
worker replacement begins, followed by about 143 ms startup; dispatch takes
367 ms in that observed run. The original uninstrumented full run previously
failed at 734 ms / <400 ms. WASM is already compiled once and shared with workers;
no compilation cache or test warm-up is added. Evidence:
`effects-preemption-order-job-trace-results.json`.

JobClient now reserves interactive priority before the store copies histogram
or edit input. A running effects worker is displaced immediately and its
replacement starts during input copying. Queued effects wait until preparations
submit their request or release, including asynchronous cancellation/failure.
Reservations overlap safely and release once; disposing clears outstanding
reservations without a late release restarting the client. Transfer ownership,
worker memory caps, command semantics and all original budgets remain unchanged.
Seven additional regression cases cover dispatch order, pending effects,
overlapping/disposed reservations, creation failure, cancelled histogram copies
that resolve/reject, and failed edit input copying. Existing fixture mocks add
the new method without changing their original assertions.

All 287 unit tests, three TypeScript checks and the fixed-asset build pass. The
unchanged preemption case passes all three internal rounds, dispatching in
300/312/299 ms; frame gaps are 158/118/114 ms under the original <300 ms gate
and effects are requested again in every round. Six real-worker regression
cases pass, including exact pixels, undo/redo and incomplete staged-plane
rejection. Source checkpoint is
`8f5c9d0cc385184f6cf8a18a71603b5c30a46a51`. Evidence:
`interactive-priority-source-check.log`, `interactive-priority-preemption.log`
and `interactive-priority-jobs.log`. These targeted passes do not close prior
intermittent failures. The complete original performance run finishes
25 passes / four failed cases in 11.0 minutes, with one worker, zero retries and
unchanged budgets. Effects-preemption dispatch is 258/232/228 ms in its three
internal rounds, with frame gaps 105/111/116 ms and successful effects re-asks.
24/100 MP Levels result frames are both 8 ms; 100 MP installation is 335 ms.
F1 blank Fill result frames are 10 ms at both sizes. Brush/Blur/Heal/ContentFill
100 MP frame gaps are 31.7/34/26/36.4 ms. Previous intermittent failures remain
open despite passing in this run. Remaining exceeded observations are:

| Gate | Measured ms | Original upper bound ms |
| --- | ---: | ---: |
| 24 MP full uniform mask gradient drag at fit | 50 | <50 |
| 100 MP pixel gradient drag at 1:1 (same failed preview case) | 53 | <50 |
| 100 MP eyedropper sample and ring | 21.9 | <16 |
| 100 MP full-canvas rectangle creation | 2004 | <2000 |
| C1 100 MP blank-layer gradient worker frame gap | 127 | <100 |

The second preview observation is also over its original assertion threshold,
although the case stops at the first failing assertion. The log preserves all
measured values. Evidence: `interactive-priority-performance.log` and
`interactive-priority-regression-summary.json`. All 195 functional cases pass in 7.9 minutes with the 29 unchanged opt-in
performance skips, one worker and zero retries. Evidence:
`interactive-priority-functional.log`. Production package checks complete after
these sequential runs, as recorded below.

## Production 2139 interactive-priority checkpoint

`COMPOSITOR_BUILD_0.8.0_20261003-2139` packages source checkpoint
`8f5c9d0cc385184f6cf8a18a71603b5c30a46a51`. Its 4,743,915-byte ZIP SHA-256 is
`C36D8173FE693A7955E5EEEFAA37056E55F3648E6011B7AF4B291E02FC12EA8E`.
The 11,977,216-byte executable SHA-256 is
`C0C1B99378C48DA999E2F49B7A9FCEDB780D94AD1804D9CB2BF390D052834641`.
All three ZIP entries pass CRC checks, the archived EXE matches the tested
portable, and bundled production/test WASM retain the release SHA above.
Build-info source bytes are restored. The optimized native build reports
8m 18s; it begins after functional regression, keeping all timing runs separate.
Evidence: `interactive-priority-package-build.log` and
`interactive-priority-package-integrity.json`.

All eight real native UI groups pass with the actual native bridge, no
development API and zero page errors, including 24 MP Fill and asynchronous
histogram cancel/reopen. Four native Mac reads and four atomic save commits
pass; continued Windows edits reopen at width 150, Stroke 7 and Shadow
Distance 14. All three Mac open PNGs are byte-identical to production 2041,
preserving the exact no-edit/edited matches and the still-open enlarged-text
discrepancy. Evidence: `interactive-priority-native-ui.log`,
`native-interactive-priority-0.8.0-result.json`,
`native-interactive-priority-mac/native-mac-roundtrip.json` and
`interactive-priority-native-mac-export-comparison.json`.

The native read-only clipboard probe now returns no-image rather than access
denied. Because this is changed access evidence, the original guarded protocol
suite runs once, first taking its complete restorable-format snapshot before
any clipboard writes. All six native groups pass, including external PNG and
Windows Forms bitmap paste, independent consumers of Copy/Cut, Cut/Paste
Undo/Redo and editable private-token text. Four strict RGBA comparisons pass:
two 3 x 2 pasted layer rasters and two complete 640 x 480 copied/cut canvases
with their known 318,239 placement. The initial supplemental comparator
incorrectly expected layer-sized Copy output; its fixture error is retained
separately, and no product assertion or tolerance changes. Two editable
16-unit UTF-16 text records retain identical text/style. The independent
helper confirms the original Text/UnicodeText formats are restored.
Evidence: `interactive-priority-native-clipboard.log`,
`clipboard-20261003-215052/native-clipboard.json` and
`interactive-priority-clipboard-exact-comparison.json`.

Production 2139 clipboard acceptance is established; the Windows Notepad
question and interactive handoff are no longer required. Historical access
failures remain preserved. The four current performance case failures, earlier
intermittent failures, Mac alpha oracle returns and Mac gesture confirmations
remain open; full acceptance is not claimed.

## Solid rectangle coverage bypass checkpoint

An unrounded rectangle covers every pixel of its truncated stored raster,
including fractional bounds. Shape rasterization now writes its opaque RGBA
pixels directly instead of allocating a full grey coverage plane and converting
it pixel by pixel. Rounded rectangles, ellipses and lines retain the general
rasterizer; layer placement and command history are unchanged.

An independent regression first passes against the old implementation, then
against the optimization. It compares the general antialiased selection
rasterizer at four integer/fractional sizes, all 256 channel values, half-rounding
and out-of-range clamp values, and negative/zero corner radii. All dimensions
and RGBA bytes are exact. Source commit is `a72c892`.

The complete native workspace passes 622 tests with zero failures and the same
10 ignored cases. All 287 unit tests, three TypeScript checks and the fixed-asset
build pass. Release and fixed-test WASM share SHA-256
`1223942C74CB77E04FC1606921DA5BC5943E54A9D9E83DDE55F6AD1532ED9B3B`.
All nine shape/text/effect browser cases pass. The unchanged original shape
performance case passes with 100 MP rectangle creation at 1603 ms / <2000 ms;
the complete suite independently measures 1614 ms and passes that case.
Evidence: `solid-rectangle-reference.log`, `solid-rectangle-native.log`,
`solid-rectangle-source-check-final.log`, `solid-rectangle-source-summary.json`,
`solid-rectangle-shape-ui.log` and `solid-rectangle-shape-performance.log`.

The complete original 29-case performance suite finishes 25 passed / four
failed in 27.5 minutes, one worker and zero retries. Three cases fail before
their bodies at the unchanged 30-second page fixture: history, grown mask
gradient and eyedropper sample/ring. They have no new measured values and are
not accepted as zero or passed. The remaining failure is C1's 100 MP blank-layer
gradient frame gap, 134 ms / <100 ms. The gradient-preview case passes all
original pixel-, mask- and selection-preview budgets; older intermittent failures remain
open. Performance evidence: `solid-rectangle-performance.log` and
`solid-rectangle-performance-summary.json`.

All 195 functional browser cases pass in 8.7 minutes, with the same 29 opt-in
performance skips. Those skips do not establish performance acceptance.
Evidence: `solid-rectangle-functional.log` and
`solid-rectangle-regression-summary.json`.

Production 2328 builds after the sequential suites from source `a72c892`.
Its marker is `COMPOSITOR_BUILD_0.8.0_20261003-2328`. The ZIP has 4,743,456
bytes, SHA-256 `5A6B3691031A34547C8D9DB9AAD82AEEFB18351E5B553BF18DCED30E41377EDE`;
the EXE has 11,977,216 bytes, SHA-256
`1BF1CB2D8D123CA99DD1CA8974F9D44BBCF33AE0B7000FB6A1F8C74A039F38F2`.
ZIP CRCs, archived EXE identity and release/test WASM identity pass
(`solid-rectangle-package-integrity.json`).

The first two native attempts fail at page readiness before business assertions.
Read-only CDP evidence shows the target URL can be assigned before the actual
page navigation commits. The final fixture waits for the real Compositor document
title within the original 45-second readiness window. Native UI, Mac and clipboard
business assertions and their budgets are unchanged; CDP cleanup is bounded so
failed fixtures release their owned process. Failed attempts and DOM/context
inventories remain retained separately.

The same verified portable then passes all eight native UI groups, four native
Mac reads and four atomic saves with no development test API and zero page errors.
All three opened Mac PNGs remain byte-identical to production 2139. The continued
text/shape/effect values remain font size 38, width 150, Stroke 7 and Shadow 14.
Evidence: `solid-rectangle-native-ui-title-ready.log`,
`solid-rectangle-native-mac-title-ready.log`,
`solid-rectangle-native-mac-export-comparison.json` and
`solid-rectangle-native-title-ready-summary.json`.

Current production 2328 clipboard acceptance stops at the independent read-only
snapshot with `CLIPBRD_E_CANT_OPEN` (0x800401D0), before any mutation.
The native read-only probe also reports access denied. Production 2139's six
protocol groups, exact pixels and restoration remain historical evidence;
they do not establish the newer package's clipboard gate. Evidence:
`solid-rectangle-native-clipboard.log` and
`clipboard-20261004-001745/helper-error.txt`.

A separate unchanged-budget diagnostic run passes the three page-fixture-failed
original cases: history, grown-mask gradients and eyedropper. Eyedropper worst
sample/ring time is 7.4 ms at 24 MP and 1.7 ms at 100 MP. The full suite still
records its four failures; this targeted run does not replace that result.
`solid-rectangle-failed-page-cases.log` retains the actual measurements.
An independent eyedropper trace passes the original body and identifies a cold
15.4 ms overlay draw; sampleColor itself stays below 1 ms. No speculative
sampling optimization is adopted from this passing diagnostic.

An early staged-allocation prototype is retained only as an investigation.
Both baseline and prototype fail original C1 assertions: the prototype reduces
100 MP gradient gap to 59 ms but records a 24 MP gradient result frame of
450 ms / <350 ms. It also lacks proof of exact-size reservation reuse. No
prototype source is adopted and no budget is relaxed. Evidence:
`solid-rectangle-c1-early-reserve.log` and
`solid-rectangle-c1-early-reserve-job-trace-results.json`.

## Staged plane safety follow-up, 2026-10-04

A new independent browser regression verifies all three staged planes (pixels,
mask and prefiltered display): exact bytes across uneven seven-byte chunks,
one history step, exact undo/redo, plane-specific budget and overflow refusal,
invalid-plane refusal, incomplete transfers and cancellation with document and
history unchanged. It first passes against the unchanged production-2328 WASM,
then against the incremental-reservation candidate. The first test-preparation
attempt omitted a layer and fails before any staging assertion; that failed
fixture is retained separately. The corrected test creates a filled layer and
its mask explicitly. No existing assertion or budget changes.

Incremental reservation is evaluated in a release candidate, not adopted.
`begin_staged_install` records plane lengths; each bounded append reserves its
own additional bytes before the same direct copy. All 287 unit tests, three
TypeScript checks and nine real-worker functional cases pass, including the new
safety regression. The three selected original performance cases all fail:
Levels records 24 MP histogram gap 489 ms / <150 ms, 24 MP installation
162 ms / <150 ms, and 100 MP installation 461 ms / <450 ms; the growing-mask
case records a 24 MP result frame 394 ms / <150 ms. C1 records 100 MP fill gap
107 ms and gradient gap 160 ms, both against <100 ms. These measurements reject
the candidate; passing pixels do not establish performance acceptance.

The candidate source, package bindings and fixed assets are retained locally.
The working implementation and canonical WASM are restored byte-for-byte to
the checked baseline, SHA-256
`1223942C74CB77E04FC1606921DA5BC5943E54A9D9E83DDE55F6AD1532ED9B3B`.
Only the useful new safety regression is retained as source. Its final version
also checks exact unchanged buffer lengths after cancellation, passes against
the immutable baseline (14.6 seconds), and passes all three current TypeScript
checks. Production 2328 remains the verified portable; no new
package is claimed from the rejected implementation. Evidence:
`incremental-reserve-baseline-guards-corrected.log`,
`incremental-reserve-source-check.log`,
`incremental-reserve-worker-functional.log`,
`incremental-reserve-performance.log`,
`incremental-reserve-targeted-summary.json`,
`incremental-reserve-candidate-identity.json` and
`staged-plane-guards-ts.log`, `staged-plane-guards-final-baseline.log` and
`staged-plane-guards-final-ts.log`.

A separate trace of the unchanged baseline C1 body fails the 24 MP gradient
result frame at 415 ms / <350 ms. In that frame, GLES2 ReadPixels waits 410.461 ms,
including 401.84 ms inside the GPU command service; WASM installation measures
74 ms. This establishes a GPU-wait contribution in that run, not an
allocation-only explanation. Evidence: `solid-rectangle-c1-gpu-wait.log` and
`solid-rectangle-c1-gpu-wait-blank-gradient-baseline-trace.json`.

A closer read of the retained trace records thread CPU time as well: the
410.461 ms client ReadPixels has 0.828 ms thread CPU, and the nested
401.840 ms GPU command-service event has 1.228 ms thread CPU. The trace
contains no finer long-running child in that service event. This supports
wait/synchronization time in that observation, not a 400 ms CPU loop; it
does not identify the particular driver/GPU wait or establish a performance
pass. Evidence: `solid-rectangle-gpu-wait-thread-time-summary.json` and
`solid-rectangle-gpu-wait-nested-time-summary.json`.

A diagnostic first-use probe draws all twelve existing GL programs into a
separate 1x1 framebuffer, restores GL state, and checks errors in all four C1
contexts. Per-context total execution is 0.4-1.6 ms with zero GL errors. The
original case still fails the 24 MP gradient gap at 263 ms / <100 ms. The first
probe only captured one context because its readiness condition was too early;
it is retained separately. The corrected all-context probe is diagnostic extra
work, not performance acceptance and not a change to the formal test warm-up.
No startup warm-up is adopted from it. Larger rendering/driver waits remain an
open investigation. Evidence: `solid-rectangle-c1-program-all-contexts.log`
and its retained job-trace-results/trace files.

A new read-only host clipboard probe still reports Windows error 5, with no
clipboard contents read and no mutation attempted
(`solid-rectangle-current-clipboard-readonly.log`). A separate post-run host
snapshot has 15.1 GiB free physical memory and no remaining task-native profile;
it is not evidence of the machine state during the performance run and does not
waive a failed budget. Mac alpha returns and gesture confirmations are still
missing. Full acceptance remains open.

## Framebuffer allocation candidate, 2026-10-04

An unchanged-source texture trace identifies a concrete allocation churn: the
tool options row changes the plain view from 1136 x 819 to 1136 x 795, recreating
all its framebuffer targets because that path uses one-pixel allocation steps.
The gradient source also changes from a 1250 x 1250 preview to a 1123 x 1250
result; that real size change cannot use a same-size texture-storage shortcut.
Evidence: `solid-rectangle-c1-texture-sizes.log` and its job-trace-results file.
The same-size full-upload proxy still fails original C1 gaps (333/115 ms against
<100 ms), so that proxy is not adopted as product source.

The framebuffer candidate uses the existing bounded 256-pixel allocation steps
for plain views as well as spatial-margin passes. Allocations retain at most
255 unused pixels per axis; the logical viewport and draw bounds stay unchanged.
Plain-view whole-buffer composition maps that padding above and right of the
view, preserving its bottom texture row. No shader, pixel format, upload bytes,
engine source, test budget or warm-up policy changes.

A new independent regression changes the actual tool options row and verifies
that the canvas height changes and returns, the allocation and all GPU texture
objects are retained, and clipped/group-opacity output keeps the original CPU
reference tolerance at all three sizes. The immutable old renderer fails its
resource-retention assertion; the candidate passes. The 18 targeted original
render/blend/adjustment/zoom cases and this new regression (19 cases total)
pass in 50.1 seconds.
All 287 units, three TypeScript checks and the fixed-asset build pass; the new
test also passes the current TypeScript checks. The unchanged release WASM SHA
remains `1223942C74CB77E04FC1606921DA5BC5943E54A9D9E83DDE55F6AD1532ED9B3B`.
Evidence: `framebuffer-buckets-original-reference.log`,
`framebuffer-buckets-source-check.log`, `framebuffer-buckets-current-ts.log`
and `framebuffer-buckets-exact-render.log`.

The four selected original performance cases finish one passed / three failed.
Partial uploads record a 24 MP first fit frame of 511.5 ms / <33 ms while
retaining the required upload counters. Levels records a 24 MP histogram gap
of 294 ms / <150 ms. C1 records 24/100 MP gradient gaps of 107/495 ms / <100 ms;
its 100 MP result frame is 536 ms / <600 ms and installation is 387 ms / <450 ms.
Pixel and resource-retention proof does not establish performance acceptance.
Evidence: `framebuffer-buckets-targeted-performance.log`.

The first broad launch mistakenly lists only two performance files, collecting
23 cases. It is intentionally interrupted and retained as incomplete evidence.
The owned root process and every recorded descendant are subsequently absent,
and port 1423 has no listener, before the corrected runner starts. Its five
performance files collect all 29 original cases using one worker and zero
retries. That complete performance run finishes 17 passed / 12 failed in
10.3 minutes; the 256-pixel candidate is not adopted. Its F1 100 MP result frame
is 211 ms / <100 ms, the 24 MP partial-edit fit frame is 457.2 ms / <33 ms,
the grown-mask 24 MP result frame is 157 ms / <150 ms, C1 100 MP fill gap is
435 ms / <100 ms, and 100 MP ContentFill gap is 123.4 ms / <100 ms. All
original failures and measurements are preserved. At the traced 1136 x 819
view, the 1280 x 1024 allocation has 40.9 percent more area than the logical
view. That is a bounded extra cost, not proof that it causes every failure.
The sequential full functional suite finishes 197 passed / 29 unchanged
opt-in skips in 7.3 minutes, one worker and zero retries. This establishes
functional correctness for the 256-pixel candidate, not performance acceptance.
The three-case immutable-baseline comparison starts only after this runner
closes, so performance work does not overlap. Evidence:
`framebuffer-buckets-incomplete-owned-tree.json`,
`framebuffer-buckets-incomplete-cleanup-verification.json` and
`framebuffer-buckets-complete-performance.log`,
`framebuffer-buckets-complete-functional.log` and
`framebuffer-buckets-complete-regression-summary.json`. Production 2328 contains the
preceding renderer; it is not native acceptance evidence for this candidate.


The same-environment immutable-baseline comparison finishes one passed / two
failed in 1.6 minutes. Its partial-upload case passes all original counters and
frame budgets: 24/100 MP fit frames are 6.9/4.8 ms and 1:1 frames are
10.9/10.2 ms. Its F1 24 MP Levels result frame is 465 ms / <100 ms, and C1
24 MP gradient gap is 382 ms / <100 ms. This gives evidence against adopting
the 256-pixel candidate, but does not attribute every failure to allocation
size or waive the original baseline failures. Evidence:
`framebuffer-buckets-baseline-comparison.log` and its identity/summary files.

A smaller candidate is now under evaluation. Plain views use 64-pixel buckets
(the traced view allocates 1152 x 832, 3.0 percent extra area); spatial-margin
passes retain their existing 256-pixel steps. Each canvas dimension is reset
only when that dimension actually changes, avoiding a same-width reset when
only the tool row changes the height. The regression additionally records
actual DOM width/height setter calls. Against the immutable old renderer, it
fails exactly at two width writes versus the required zero, as expected;
that independent reference is not counted as a passing product test. Evidence:
`framebuffer-buckets-small-baseline-reference.log`. The smaller candidate passes all 287 units, three TypeScript checks and the
fixed-asset build, then all 19 exact rendering/resource cases in 50.3 seconds.
The setter regression now passes with zero width writes and exactly two
required height writes, with clipped-group pixels and GPU objects preserved.
Its six selected original performance cases finish three passed / three failed
in 2.8 minutes: F1 100 MP Levels result frame is 235 ms / <100 ms,
100 MP partial-edit fit frame is 121.8 ms / <33 ms and C1 100 MP gradient
gap is 505 ms / <100 ms. This smaller allocation policy is also rejected.
The previous
197 full functional passes belong to the archived 256-pixel candidate.
Evidence: `framebuffer-buckets-small-source-check.log`,
`framebuffer-buckets-small-exact-render.log` and
`framebuffer-buckets-small-identity.json`. Both framebuffer candidates and tests are archived locally; the original
allocation policy, view-corner mapping and framebuffer helper source are
restored exactly. No native package or full acceptance is claimed from
either candidate. Evidence: `framebuffer-buckets-small-targeted-performance.log`.

Only the independent canvas-dimension fix is retained: height-only
changes assign height once and leave width untouched, and conversely for width.
A separate `canvas-resize.spec.ts` preserves the actual tool-options transition,
setter counts and clipped-group CPU pixel references without requiring either
rejected allocation policy. All 287 units, three TypeScript checks, fixed assets
and 19 exact render cases (51.2 seconds) pass. Six original performance cases
finish two passed / four failed in 2.6 minutes: F1 result frame 476 / <100 ms,
partial-edit frame 160.2 / <33 ms, a Levels ready assertion fails its original
5-second timeout before a new body measurement, and C1 first exceeded gap
103 / <100 ms. All later measurements remain retained too. The complete
current-source functional suite finishes 197 passed / 29 original opt-in skips
in 7.0 minutes, one worker and zero retries. Those skips do not establish
performance acceptance.
Evidence: `canvas-resize-source-check.log`, `canvas-resize-exact-render.log`
`canvas-resize-targeted-performance.log`,
`canvas-resize-complete-functional.log` and
`canvas-resize-verification-summary.json`. The engine and WASM remain unchanged.

A read-only clipboard ownership query identifies no window for either the open
holder or the data owner (both handles/PIDs zero). A null handle can also occur
for a caller opening without a window, so this does not establish access or
exclude every possible lock. No clipboard contents are read or changed and
no process is selected for termination. Current native clipboard acceptance
remains open. Evidence: `clipboard-owner-readonly-20261004.json`.

## Production 0244 canvas-dimension checkpoint, 2026-10-04

`COMPOSITOR_BUILD_0.8.0_20261004-0244` packages source
`dbb4d812558ad13db509e83bae89345e216456ad`. The optimized native build
finishes in 8m 23s after all source/functional/timing runs have ended.
The ZIP has 4,743,533 bytes and SHA-256
`E31F6EFC248677BF6ED65F0EE56E134AAD240AD5AEE6DE2E09DA99CB8A8BF55D`;
the 11,977,216-byte EXE SHA-256 is
`F58612EDC2708EC0128C1D2DDE24810272758D67BC847B32592F7319898C37FB`.
All three ZIP entries pass CRC checks, the archived EXE matches the tested
portable, and release/test WASM retain
`1223942C74CB77E04FC1606921DA5BC5943E54A9D9E83DDE55F6AD1532ED9B3B`.
The build-info source bytes are restored. Evidence:
`canvas-resize-package-build.log` and `canvas-resize-package-integrity.json`.

The real production package passes all eight native UI groups, four native
Mac reads and four atomic saves with the native bridge, no development API and
zero page errors. Continued text is saved at font size 38; the reopened shape
retains width 150, Stroke 7 and Shadow Distance 14. All three opened Mac PNGs
are byte-identical to production 2328. Mac-no-edit and Mac-edited remain exact
against the Mac exports; Mac-created remains 13,065 different pixels / 28,222
different channels, maximum difference 51, at 1920 x 1080. This preserves the
known enlarged-text discrepancy and does not close it. Evidence:
`canvas-resize-native-ui.log`, `native-canvas-resize-0.8.0-result.json`,
`canvas-resize-native-mac.log`, `native-canvas-resize-mac/` and
`canvas-resize-native-mac-export-comparison.json`.

The current native clipboard protocol stops at OleGetClipboard's read-only
original snapshot with `CLIPBRD_E_CANT_OPEN` (0x800401D0), before any mutation.
The case has no ready record and zero request files. The UI read-only probe
also reports Windows denied clipboard access and requests an interactive
desktop session. All owned delivery processes have ended; this is not an
ongoing native test. Evidence: `canvas-resize-native-clipboard.log`,
`clipboard-20261004-025545/helper-error.txt` and
`canvas-resize-delivery-summary.json`. Production 2139 remains the completed
historical clipboard checkpoint, not the current package's acceptance.

The local handoff `verify-0244-interactive-acceptance.ps1` binds twenty input
hashes: the actual EXE, fixed assets, both configs, all five original performance
files, existing guarded native clipboard fixtures/helper, and the strict
comparison reader. Its default run executes all 29 original performance cases
with one worker and zero retries, then the guarded clipboard protocol and exact
comparison. It records separate result/log files and restores PERF and the
original clipboard through the existing helper. A clipboard-only run explicitly
records performance as not run; it cannot establish the performance gate.
The final read-only preflight passes; no interactive desktop run is claimed.
Evidence: `interactive-0244-frozen-inputs.json` and
`interactive-0244-readonly-preflight-final.log`.

The reusable local clipboard comparator independently requires all six protocol
groups, four strict full-length RGBA comparisons, two distinct editable UTF-16
text records with identical style, and original format restoration. It passes
against the retained production-2139 evidence. An isolated copy with one red
channel changed from 255 to 254 is refused at the actual native PNG-copy byte
comparison, proving that no image tolerance hides an incorrect result. The
initial standalone attempt could not resolve pngjs; it is retained as a
checker-dependency error, not a product failure or a valid negative control.
The corrected reader uses the already-installed Playwright PNG module. Evidence:
`clipboard-exact-comparator-historical-fixed.log` and
`clipboard-exact-comparator-negative-fixed.log`. Neither is a new 0244
clipboard protocol pass.

The four requested Mac alpha PNGs remain absent and RESULT.txt remains blank;
the older return report still leaves Mac undo/redo and effect preview/cancel/
apply gestures unconfirmed. Windows interactive performance/clipboard results
and the requested Mac oracle/gesture evidence are required before full
acceptance can be completed. Those same external conditions appear in the
last three goal turns; no available automated Mac app or permitted clipboard
snapshot can supply them in this environment. Original budgets, retries,
warm-ups, skips, pixel tolerances and all user return files remain preserved.

## Additional Mac alpha sampling handoff

Four generated projects isolate white-pixel alpha filtering from text rendering:
horizontal/vertical High-quality strips at 1600 percent and a 32 x 32 alpha grid
scaled to 34 x 34 with High quality or Smooth. All four open/export in the Windows
release WASM, preserve their exact transforms, and draw within the unchanged GPU
pixel tolerance (strip maximum zero, grid maximum one channel level).
The first validation compared JSON property order and falsely reported a transform
change; the corrected structural comparison preserves every numeric field.
Evidence: `mac-alpha-probe-validation-final.log` and
`alpha-probe-validation-mac-checks/checks.json`. No Mac oracle is claimed yet.

The local handoff is `mac-alpha-probes-20261003.zip`, 7,241 bytes, SHA-256
`616edb17d97adf8980f5a086c1489e18e6a173afb59fb771d4c409fc97eb0455`;
archive CRC verification passes. README requests unedited transparent PNG exports
in Mac Compositor 1.4.5. RESULT also lists the still-unconfirmed text/shape undo-redo
and effects preview/cancel/apply/reopen gestures. The Mac return report still leaves
those fields blank. The original return files and older user sampling probes are
unchanged.

## Mac confirmation still required

Compositor 1.4.5 return files already prove live opening, saving and text/shape
editing. The return sheet leaves undo/redo and effect preview/cancel/apply blank,
and the manifests retain the original effect parameters. These gestures cannot
be established from the saved files alone.

On a copy of `Mac-edited.comp`, confirm the following:

1. Change the point text, then Cmd+Z and Cmd+Shift+Z. Do the same after resizing
   the rounded rectangle. Confirm the content/size returns and reapplies.
2. Open the rounded rectangle's effects. Change Stroke Size from 5 to 7 and
   Drop Shadow Distance from 12 to 14; toggle preview, then cancel. Reopen and
   confirm the original 5 and 12 remain.
3. Apply those changes, save to a new project, close and reopen. Confirm 7 and
   14 remain and the rectangle and text remain editable.

The existing exact no-edit composite match and quantified edited/created edge
differences are recorded in [the Mac return report](mac-roundtrip-results-2026-10-03.md).
Mac-created enlarged-text export acceptance remains open; no sampling fix
is inferred from metadata preservation or the font-face correction.

## Outstanding gates at the production 0244 handoff

Full acceptance remains incomplete. The retained canvas-dimension fix passes
287 units, three TypeScript checks, 19 exact render cases and all 197
functional cases with 29 original opt-in skips. Its six selected original
performance cases finish two passed / four failed; no complete current-source
performance pass is claimed. Production 0244 passes package integrity, all
eight native UI groups and the native Mac read/save gates, but its clipboard
protocol stops before any mutation at the refused original snapshot. Both framebuffer allocation
candidates are rejected and the original allocation/view mapping are restored.
The solid-rectangle full performance suite
has three original page-fixture failures and C1's 134 ms / <100 ms gradient
frame gap; earlier intermittent failures still need repeatable evidence.
Production 2139 is the latest completed native clipboard checkpoint: six
guarded protocol groups, four exact image comparisons, two editable UTF-16
records and original clipboard restoration. Production 2328 passes package
integrity, eight native UI groups and the native Mac read/save gates. Its
clipboard protocol stops at the read-only snapshot before any mutation.
Earlier production successes and access failures remain preserved. Mac gesture
confirmation and alpha oracle returns remain pending. Current Mac-no-edit and
Mac-edited exports are exact; Mac-created enlarged text remains quantified
and open; covered overlap edges now match. No assertion, budget, warm-up
policy, skip, retry or image tolerance is weakened to close those gates.

The checkpoint stages only reviewed source, regression tests and these reports.
Mac return data, clipboard data, test traces, packages, `.workbuddy` and the user's
untracked sampling probes remain local and untouched. Full acceptance awaits the required Windows interactive and Mac evidence.

## Received Mac and Windows evidence, 2026-10-04

The user returns four independent Mac Compositor 1.4.5 alpha PNGs, RESULT.txt
and Mac-edited-test.comp/.png. A new local snapshot contains all thirty received
files; its manifest records file sizes and SHA-256 values. The original return
folders remain untouched. Evidence: mac-alpha-return-20261004-received/ and
mac-alpha-return-20261004-received-files.json.

RESULT reports text/shape undo-redo, effects preview/cancel and reopening
editability as PASS on macOS 26.5.2. It also reports applied 7/14 persistence as
PASS, but the actual returned rounded rectangle retains Stroke Size 5 and
Shadow Distance 12. That saved-value check remains open; the user has been
asked to return the project actually saved at 7/14. This discrepancy does not
invalidate their reported cancellation and undo/redo gestures.

The user's hash-bound production-0244 interactive run completes all twenty-nine
original performance cases in 8.3 minutes, one worker and zero retries. Every
one of the twenty frozen input hashes still matches. This is a complete timing
pass for source dbb4d81 and the 0244 assets; it is not a timing pass for the new
sampling prototype. The same interactive run restores the clipboard but fails
at native text paste (four layers instead of five). Evidence:
interactive-0244-20261004-092948-416-summary.json and its transcript.

A local diagnostic clipboard run later fails the first external PNG paste. A
second diagnostic run passes, followed by one complete unchanged original
protocol run that passes all six groups with the production bridge, no test API
and no page errors. Its independent comparator passes all four full-length
exact RGBA checks, both editable UTF-16 records and style checks, and original
format restoration. Previous failures remain retained; no clipboard code fix
or repeatable resolution of their intermittent cause is claimed. Evidence:
clipboard-20261004-104553/, clipboard-20261004-104919/,
clipboard-20261004-105142/, native-0244-clipboard-original-recheck.log and
clipboard-0244-original-exact.json/.log. No original assertion, timeout, retry
or clipboard fixture is changed.

The independent alpha exports expose the old bilinear enlargement discrepancy:
maximum alpha error 37 for both grids and 11 for the horizontal/vertical strips.
The two Mac grid PNGs are identical, supporting the previously measured shared
eight-phase enlargement kernel for High quality and Smooth. A local CPU/GPU
prototype uses that kernel only for affine enlargement; automatic upright
pixel copies, explicit Nearest, distortions and reduction rules are preserved.
A new synthetic grid regression uses the independent returned Mac alpha bytes.
The geometry-correct baseline run fails at 37 against its one-level alpha
rounding allowance; the first attempted baseline test misplaced the import and
is explicitly excluded as a valid negative control. Evidence:
mac-alpha-sampling-baseline-red-fixed.log and
mac-alpha-sampling-baseline-red-initial-note.txt.

The targeted native tests pass six cases. Actual prototype native exports
reduce both grid alpha errors to one; horizontal/vertical alpha errors are also
at most one, but raw straight RGBA has transparent-white-versus-zero pixels
and a maximum difference of 255. Mac-created improves to 8,355 different
pixels / 13,763 channels, but raw maximum difference remains 51. Its
premultiplied difference of at most two does not close the required exact
straight-RGBA export gate. Mac-edited-test is exact at its actual saved 5/12.
Evidence: mac-alpha-sampling-targeted-native.log and
mac-alpha-sampling-prototype-comparison.json. Full native, release WASM and GPU
validation are still in progress; this prototype is not published or packaged.

## Exact-byte enlargement candidate, 2026-10-04

The continuous-weight prototype passes 623 native tests, 287 units, all three
TypeScript checks and 198 functional browser cases with the 29 unchanged
opt-in skips. Its first 19-case rendering run has one Image Size mask formula
failure; keeping Image Size on its original resampling path resolves that
failure and the corrected nineteen-case run passes. Those full functional
results belong to the archived v2 candidate, not the new byte model.

The returned strip is explained exactly by anchored, truncated byte products.
Applying that operation vertically before horizontally also matches both
independent grids exactly, while horizontal-first differs at 451 pixels.
The same model matches an independently created colored-text region exactly.
Actual native full exports leave seven differences, all on one mathematical
half column. Keeping the lower texel against one-ulp inverse-affine drift
removes all seven. All six full native exports now match every straight RGBA
byte: the four probes, Mac-created and Mac-edited-test. The independent strict
reader requires zero tolerance and fails the seven-pixel v3 candidate before
passing v4. Evidence: mac-alpha-combined-anchored-models.json,
mac-created-anchored-byte-model.json, mac-alpha-sampling-v3-residual-pixels.json
and mac-alpha-sampling-v4-native-comparison.json.

The CPU and GPU enlargement path now use this byte model. The new synthetic
regressions are strengthened from a one-level alpha allowance to exact Mac
alpha bytes. Existing assertions, image tolerances and budgets stay unchanged.
The obsolete pre-half-fix v3 WASM compilation is stopped after verifying its
owned process identity; its interrupted result is retained and not counted.
The corrected v4 release WASM and complete verification are in progress.
No new production package or full Phase 5 completion is claimed yet.


## Final byte-interpolation source verification, 2026-10-04

Final release WASM SHA-256 is
`933ce4a56b642a390e9ff99a283860279aa7aa717641ca6b699d2bd688aaa752`.
Its independent full-image comparison passes all eight Mac-returned PNGs with
zero changed pixels/channels. The four alpha probes also have exact GPU/CPU
bytes; the three 800 x 600 Mac project GPU checks remain within the original
maximum-two-byte allowance (observed maximum one). The 1920 x 1080 Mac-created
case has complete CPU/WASM/native PNG comparison, not a GPU full-image claim.

Final native workspace: 623 passed, zero failures, ten existing ignored.
The subsequently added mathematical half-column regression and exact alpha-grid
regression pass alongside the five original pixel-copy cases (seven tests).
All 287 unit tests, three TypeScript checks, nineteen targeted browser render
cases and the complete 198-case functional suite pass. The functional suite
retains 29 original opt-in skips; these do not count as performance acceptance.

The separately executed original 29-case timing suite finishes 23 passed / six
failed (10.6 minutes), one worker, zero retries, and no overlapping owned builds
or tests. Failures are typed W redraw (77 / <33 ms), gradient preview step
(567 / <150 ms), mask-gradient step (650 / <400 ms), mask-fill redraw
(464 / <150 ms), C1 blank-gradient frame gap (492 / <100 ms), and the 100 MP
brush/retouch responsiveness case (119.5 / <100 ms). Every original assertion,
budget and warm-up remains intact. The earlier interactive 0244 timing PASS
belongs to its frozen preceding source/assets; it is not transferred to this
new candidate. Prior-version fixed-assets control is being measured separately
to establish whether these failures require a source fix or a different
execution-context investigation. Neither outcome by itself waives a gate.

Evidence remains under `build-artifacts/phase5-acceptance`: final source identity,
V4 full-native/source-check/functional/render logs and terminal exit JSON,
WASM full-return comparison, and the complete final performance failure log.
The seven-pixel V3 negative control and initial invalid origin-placement
regression attempt remain retained and are not counted as passes.


The independent preceding-version fixed-assets control completes six selected
original cases: one pass / five failures (4.0 minutes). Typed W stops at the
original 5-second engine-ready assertion before entering its body; gradient
preview is 553 / <150 ms, mask-fill has 126 / <100 ms, C1 has 127 / <100 ms,
and ContentFill responsiveness is 108.1 / <100 ms. Mask-gradient passes. This
shows that several observed responsiveness failures also exist without the
new interpolation, but does not establish their cause or clear any final-source
performance gate. Logs and terminal exit are retained as
mac-alpha-original-performance-control.log / -exit.json.


## Production 1256 package checkpoint, 2026-10-04

Product source commit: `b4525346c4f2b3ceba5d3dddc935c17b2cc569df`, published
to `codex/phase4-and-phase5`. Marker: `COMPOSITOR_BUILD_0.8.0_20261004-1256`.
The diagnostic portable packages the validated byte-interpolation source,
while preserving the incomplete performance gate. Its optimized native build
finishes successfully in 12m 37s. ZIP size 4,744,791 bytes, SHA-256
`377F39E2B5C2711BAA38BA2173AC7204B19902E5AB922927F1FE9245B67503B7`;
EXE size 11,978,240 bytes, SHA-256
`5F9A41F6E4116711D2AC2174A11E938F71EC3289544A46A85290FF2EBA719563`.
The three ZIP entries pass independent CRC/size checks and its archived EXE
is identical to the tested portable. Production and fixed-test-assets WASM
match the final 933ce4a5 fingerprint. Build-info bytes are restored afterward.

All eight native UI groups pass with the production Tauri bridge, hidden
development API and zero page errors: painting/eraser/undo, blur/aligned clone,
three healing modes, fill preview/cancel/apply, editable text, effects apply,
and the 24 MP fill/Levels histogram/cancel path. The incidental UI clipboard
read reports no image; dedicated clipboard evidence below establishes its
actual interoperability separately.

All eight original Mac projects open/export/save through native file commands.
Nine native reads and nine atomic save commits include the Windows continuation:
font size 38, shape width 150, Stroke 7 and Shadow Distance 14 persist after
reopening the continued copy. Original transforms and manifests remain unchanged.
An independent full-PNG reader requires the complete eight-project corpus and
finds zero changed RGBA pixels/channels in every production-native export.
The 30 newly received original return files and their immutable snapshot retain
their receipt hashes after verification. This Windows continuation does not
substitute for the missing Mac-applied 7/14 saved project.

Original six-group native clipboard protocol passes in fresh case
`clipboard-20261004-131231`. The independent strict reader confirms all four
complete image payloads/placement, two editable 16-unit UTF-16 text records
with identical styling, and restoration of the original restorable formats.
Previous intermittent failures remain recorded; this pass does not claim that
their unidentified cause is fixed. All native jobs run sequentially after
source/browser/performance jobs finish; no timing run overlaps an owned build
or native check.

The new interactive verifier is ready at
`build-artifacts/phase5-acceptance/verify-mac-alpha-final-interactive-acceptance.ps1`.
Its read-only `-CheckOnly` preflight passes all 25 SHA-bound inputs: new EXE,
fixed assets, original five timing files/config, six changed source/regression
inputs, native clipboard protocol/helper and independent exact reader. It runs
all original 29 timing cases with one worker and zero retries, then guarded
clipboard checks, retaining timestamped logs and a terminal summary. New
interactive execution has been requested; the preceding 0244 PASS does not
validate new source/assets. Full Phase 5 acceptance remains incomplete pending
current performance evidence and the actual Mac-applied-and-reopened 7/14
project. No image tolerance, budget, assertion, warm-up or retry policy changes.

Current local artifacts: mac-alpha-final-package-integrity.json,
mac-alpha-final-delivery-summary.json, native-mac-alpha-final-0.8.0-result.json,
native-mac-alpha-final-mac/native-mac-roundtrip.json,
mac-alpha-final-native-return-comparison.json,
mac-alpha-final-original-return-identity.json,
mac-alpha-final-clipboard-exact.json and
interactive-mac-alpha-final-frozen-inputs.json.


## Received applied-effect and interactive evidence, 2026-10-04 afternoon

The human-run production-1256 verifier completes all 29 original timing cases:
26 passed / three failed (10.0 minutes). Typed W redraw is 44 / <33 ms, C1
worker gap is 123 / <100 ms, and the 24 MP brush case stops at the original
30-second page-fixture setup timeout before entering its body. Its clipboard
helper stops at OleGetClipboard with CLIPBRD_E_CANT_OPEN before any mutation.
No strict clipboard comparison is run for that aborted case. All 25 frozen
inputs at execution are verified (the two subsequently edited GL source files
are checked against their byte-preserved received-version archive), and the
verifier itself retains its original fingerprint. The previous production-1256
native clipboard PASS remains a distinct retained result.

New Mac-edited-new.comp and its independent PNG are preserved in a separate
11-file snapshot, mac-applied-7-14-return-20261004. The actual blue rectangle
now stores Stroke 7 / Shadow Distance 14. Production 1256 natively opens,
exports, saves and reopens the returned copy: two native reads, one atomic
save, persistent 7/14, editable text/shape, unchanged transforms and zero page
errors. Full RGBA PNG comparison is exact. This closes the prior saved-effect
artifact mismatch; the original 5/12 project and all earlier evidence remain
unchanged. Evidence: mac-applied-7-14-received-files.json,
native-mac-applied714-1256.log and mac-applied714-1256-comparison.json.

The prior plain shader independently passes the original typed-W body at
13/26 ms. A candidate compiles the measured enlargement kernel into a separate
program, created lazily on its first genuine enlargement and reused/disposed
with the renderer. Ordinary draws use the original shader without a dynamic
enlargement branch. Engine/WASM and all source/test assertions remain unchanged.
Three TypeScript checks, all 287 units and all four selected original timing
cases pass; typed W is 14/18 ms and the 100 MP C1 gradient gap is 60 ms. Complete
functional/timing regression and nine-project Mac GPU/CPU comparison are in
progress. These targeted successes are not yet full Phase 5 acceptance.


## Separated shader verification, 2026-10-04 afternoon

Ordinary composition uses the original single-texture shader again. Affine
enlargement lazily compiles a separate byte-interpolation variant, caches it
and disposes it with the renderer. Engine/WASM, sampling selection, tests,
original assertions and timing budgets stay unchanged.

- Three TypeScript checks and all 287 unit tests pass.
- Complete functional run: 198 passed, 29 original opt-in timing skips,
  7.3 minutes, one worker and zero retries.
- Four targeted original timing cases: all pass. Typed W is 14/18 ms,
  and the 100 MP C1 gradient gap is 60 ms.
- Complete original timing run: 20 passed / nine failed, 10.0 minutes,
  one worker and zero retries. Typed W is 17/27 ms (<33); 24/100 MP
  brush, blur, healing and content-fill responsiveness passes.
- Failed terminal assertions remain: F1 frame 146/<100 ms; Add Mask
  frame 515/<150; partial-upload frame 374.7/<33; histogram gap 283/<150;
  settled gradient 536/<150; mask-gradient result 414/<150; mask-fill frame
  274/<150; small-mask gradient result 557/<150; C1 worker gap 474/<100.
- All nine full straight-RGBA PNG exports exactly match independent Mac
  returns, including the real applied 7/14 project and Mac-created.
  Four alpha GPU/CPU checks are exact; other checked 800x600 GPU/CPU
  results have maximum delta one (original tolerance two). Full Mac-created
  GPU pixels are not claimed; its complete CPU PNG is exact.
- Source identity: shader-isolation-source-identity.json; original five timing
  sources retain their frozen hashes and release WASM retains SHA-256
  933ce4a56b642a390e9ff99a283860279aa7aa717641ca6b699d2bd688aaa752.

Evidence: shader-isolation-first-checks-summary.json,
shader-isolation-complete-verification-summary.json,
shader-isolation-complete-functional.log, shader-isolation-complete-performance.log,
shader-isolation-independent-mac-and-environment-summary.json and
shader-isolation-mac-return-comparison.json. The environment comparison is
diagnostic only. Complete performance and current-source packaged acceptance
remain open.


The same-source visible-window comparison is one pass / one failure (1.3m):
partial uploads pass, while F1's fill frames remain 169/386 ms against <100.
A separate minimal WebGL program loads no application or WASM code, uploads
a 6.25 MB texture and draws a plain quad after synthetic CPU/memory activity.
It reproduces a 446.7 ms draw/read wait in headless Edge with GL error zero;
other warmed frames are 4.8-21.3 ms. Twelve visible-window rounds are 5.0-18.6 ms.
This proves that a long wait can occur outside the application, but does not
identify the specific scheduling, driver or display-service cause and does not
waive the original failed cases. See shader-isolation-minimal-gpu-control.json.
The host GPU is Intel HD Graphics 520, driver 31.0.101.2111. GameViewer and
virtual display adapters are present; their causal role is unproven, so they
are left running. Read-only WPR status says no existing recording. GPU/CPU
trace start fails with 0xc5585011 (performance profiling policy cannot be
enabled); no existing trace or system setting is changed.


## Production 1527, current native checkpoint

Product commit 194296602087364b976b4c9b7a2d62687205da51 is published on
codex/phase4-and-phase5 and verified against the remote ref. The sequential
production build repeats all three type checks, embeds the new marker and
finishes the Rust release build in 6m 10s. It preserves prior packages and
restores the original build-info source bytes.

- Marker: COMPOSITOR_BUILD_0.8.0_20261004-1527.
- ZIP: 4,744,912 bytes; SHA-256
  8DE25664B46EC6C8FFF13C0C1B5896BEBB16557815B84C9C8AADC6E97272F0AF.
- EXE: 11,978,240 bytes; SHA-256
  8BE769F06CDEF9CD752BAA13794CEF9F9177C26AC70A493F20ED12BA9CBF034F.
- All three archive entries pass CRC; archived EXE equals the tested portable.
  Release WASM equals the fixed-assets 933ce4a5... binary.
- All eight native UI groups pass, with native bridge true, Test API undefined
  and zero page errors. The read-only clipboard probe is denied; it is not
  accepted as native clipboard interoperability.
- The initial native Mac run times out after 30s waiting for the second
  project's Save As manifest. Its earlier exports/first save and complete
  failure log remain unchanged. A separate fresh output/profile diagnostic
  retains the same assertions/timeouts and adds failure snapshots; all nine
  opens/exports/saves and both continued/new 7/14 reopen checks pass. This
  passing run does not prove the earlier intermittent timeout fixed.
- This fresh native run records 11 native reads and 10 atomic saves; all nine
  complete straight-RGBA PNGs exactly match independent Mac returns. Text,
  shape and effect metadata remain editable and transforms are unchanged.
- The original six-group clipboard protocol cannot begin: original helper
  OleGetClipboard returns 0x800401D0 before any mutation. Case:
  clipboard-20261004-154410. No exact comparison or second case is run.
- A read-only Win32 context probe independently returns OpenClipboard false /
  error 5, with sequence 2909 before and after, no data read and no writes.
  Current process is in session 1 and a job; its queried UI limits are zero.
  Twenty owner samples show no open window and stable Explorer ownership.
  This does not identify the particular access restriction; nothing is reset
  or overwritten.
- All 30 original receipt files and 11 separately snapshotted new 7/14 files
  retain their full hashes and lengths.

Evidence: shader-isolation-package-integrity.json,
shader-isolation-delivery-summary.json, shader-isolation-native-ui.log,
native-shader-isolation-mac-investigation/native-mac-roundtrip.json,
shader-isolation-native-investigation-return-comparison.json,
shader-isolation-native-followup-summary.json,
shader-isolation-clipboard-context-read-only.json,
shader-isolation-original-mac-receipts-identity.json and
shader-isolation-checkpoint.json. Full acceptance is explicitly false.

The hash-bound capture-phase5-gpu-system-trace.ps1 now waits for the owned
package/native workflow to finish, protects any existing WPR recording, checks
free space and source/assets/package/helper identities, and requires an
administrator terminal. It collects GPU/CPU ETL plus the no-application control
and unchanged original F1 case, then two independent guarded clipboard cases
only if each original snapshot/assertion succeeds. It changes no driver,
display service or security policy and adds no retries. CheckOnly passes.
The need for elevation and error 0xc5585011 are described in Microsoft's
[Windows performance guidance](https://github.com/MicrosoftDocs/windows-dev-docs/blob/docs/hub/apps/develop/performance/power.md).
User execution is pending; traced diagnostics do not replace the 29 original
performance budgets.

## Received system trace and corrected interactive handoff, 2026-10-04

The received `gpu-system-trace-20261004-161655-833` contains a successfully
stopped 2,543,845,376-byte GPU/CPU ETL. Windows `tracerpt` processes 46,211,454
events with zero reported lost events. All 26 frozen source, assets, package
and helper inputs still match. The original F1 case passes in 1.4 minutes,
with result frames of 8/11 ms at 24 MP and 9/12 ms at 100 MP. This is one
original case; it does not replace the previous complete 20/29 result or
prove the intermittent GPU stall fixed. The minimal control has a longest
headless draw/read of 95.8 ms and no GL errors in this recording. The ETL
summary alone does not identify a scheduling or driver cause.

The administrator clipboard run gets past the original safe snapshot. It
then fails before the six-group protocol: no native debug endpoint appears
within the original 45-second readiness deadline. Its sole helper request
is restoration; the successful response preserves the original format set.
A fresh non-elevated launch of the same 1527 package reaches that endpoint
in 4.15 seconds without accessing or writing clipboard contents. The Microsoft
WebView2 [issue record](https://github.com/MicrosoftEdge/WebView2Feedback/issues/5640)
describes the same elevated-host symptom. Elevation is a supported inference
for this readiness failure, not a proven explanation for earlier clipboard
access failures or product responsiveness failures.

Combining interactive clipboard tests with an administrator-only WPR step
was an unsuitable handoff. The recorded script and failure are retained.
Use the new local `build-artifacts/phase5-acceptance/verify-phase5-interactive-1527.ps1`
from an ordinary PowerShell window opened from Windows Start. It rejects an
elevated launch before starting any test, verifies all 26 frozen inputs,
uses a fresh evidence directory, runs the complete original 29-case timing
suite with one worker and zero retries, then runs two independent original
guarded native clipboard protocols and strict RGBA/UTF-16/style/restoration
checks. It stops the clipboard sequence on failure and never adds a retry.
A timing failure is retained while independent clipboard evidence proceeds.
`-CheckOnly` passes. No GPU re-recording, driver change, system policy change
or application assertion/budget change is needed for this handoff. Full
Phase 5 acceptance remains false, including the retained initial Save As
timeout whose cause is still unproven.

## Ordinary interactive receipt and clipboard Cut/Paste failure, 2026-10-04

The received `interactive-1527-20261004-164354-228` passes the complete original
29-case performance suite in 8.8 minutes, one worker, zero retries and no
skips. All 26 frozen inputs match after execution and during independent
review. The earlier failing performance runs are preserved; this pass does
not by itself establish the cause or resolution of their intermittent waits.

Its first clipboard session, `clipboard-20261004-165244`, passes all six
original production-native groups. Independent re-execution of the strict
reader passes four full RGBA comparisons, two editable text records with
UTF-16 length 16, matching styles and original-format restoration.

Its independent session, `clipboard-20261004-165317`, gets through external
PNG paste, native PNG copy, external bitmap paste, native Cut and independent
bitmap consumption. The following Ctrl+V fails the original five-second
assertion: layer count remains two instead of three. The protocol stops; no
retry is added, and its strict completion reader is not run. Its restoration
response passes and preserves the original format set, as in the first
session. No retained failure screenshot or IPC/error-state snapshot exists
for that original harness, so clipboard-access contention, shortcut/focus
and asynchronous-state explanations remain hypotheses. No product patch
is justified yet.

The local `verify-phase5-clipboard-observed-1527.ps1` prepares the next
discriminating check in ordinary PowerShell, without repeating performance.
It uses the same 1527 package and original guarded snapshot/restoration helper.
Removing only the observation additions from its native JavaScript restores
the original harness body byte for byte: assertions, timeouts and commands
are unchanged. It records shortcut target/default-prevention, native IPC
start/response/error metadata, working/layer/error state and a failure
screenshot without adding waits before gestures. Native clipboard payloads
are not logged. Each completed protocol uses the unchanged strict reader;
overall clipboard stability requires both sessions to pass. A failure stops
the sequence rather than retrying.

CheckOnly passes. A separate real-package, read-only observer self-check
confirms that a simulated IPC error and an unmatched key are captured; it
executes no application actions and accesses no system clipboard. An initial
self-check caught a collision between the IPC-error field and DOM-error
field; those fields are now distinct and the corrected self-check passes.
These diagnostic results establish the logger, not clipboard acceptance.
Full Phase 5 acceptance remains false, and the prior native Save As timeout
is still retained with its cause unproven.

## Observer startup defect and isolated native instances, 2026-10-04 evening

The received `clipboard-observed-1527-20261004-173705-570` passes its first
six-group native clipboard protocol and strict RGBA/text/style/restoration
reader. Independent re-execution of the strict reader confirms that result.
Its second case stops before any clipboard test: the engine reports
`TypeError: Failed to construct URL: Invalid URL`. The diagnostic fetch
wrapper incorrectly treats a URL object as a Request and reads `input.url`;
WASM initialization legitimately passes a URL object. This is a defect in
the added observer, not evidence about the earlier Cut/Paste failure. Both
original clipboard format sets restore. The exact failing observer version
is preserved locally before correction. Product source and all 26 original
frozen source/assets/package/helper inputs remain unchanged.

The corrected observer supports strings, relative strings, URL and Request
objects; unrecognized stringable inputs are forwarded to the original fetch.
Key cancellation is recorded after event dispatch, rather than at a capture-
phase microtask checkpoint before the application listener. A real 1527
package self-check installs the observer before initialization, reloads twice,
and confirms engine-ready, five fetch input forms, IPC error metadata, final
key cancellation and zero page errors. It uses real bundled assets and makes
no system clipboard access. A separate production-UI simulation completes
all six original operation groups, then verifies a deliberately refused third
clipboard read produces the original five-second layer-count failure and
records the IPC error, two-layer state and accepted Ctrl+V. These simulations
validate the observer, not system clipboard acceptance.

During the controlled sequential check, the previous fixed port is still
listening just after its host process is stopped; the next-port guard safely
refuses to use it. The original launcher polls a fixed port without verifying
which process owns it. This exposes an isolation risk; it does not prove
that a stale endpoint caused either earlier user failure. Each revised
clipboard session now allocates an independent loopback port. The endpoint
must belong to a WebView process whose parent is the launched native PID
and whose command line identifies that session's fresh profile. The original
45-second readiness deadline is preserved. The two controlled UI sessions
use distinct ports with verified matching owners and profiles. Only the
connection endpoint is parameterized in the native JavaScript; after removing
observations and normalizing that adapter, the original harness body is
byte-identical. Actions, assertions, deadlines and retry count are unchanged.
The updated entry script also verifies hashes of both observed helpers in
addition to all 26 original frozen inputs.

An independent real-native Mac run uses the same isolation guard and the
original action/assertion body: production 1527 completes 11 reads and 10
atomic saves, editable text and persisted 7/14 effects, zero page errors and
all nine full PNG exports exactly matching independent Mac bytes. The
original Save As failure is retained and is not claimed fixed from this pass.
The corrected local `verify-phase5-clipboard-observed-1527.ps1` passes
CheckOnly. Real system clipboard verification still needs the ordinary
interactive desktop because the Codex execution host cannot open that
clipboard. Full Phase 5 acceptance remains false until the outstanding
Cut/Paste evidence is resolved; timing and GPU tracing need not be repeated
for this corrected diagnostic run.


## Corrected observer receipt and failed snapshot restoration, 2026-10-04 18:23

The received `clipboard-observed-1527-20261004-182308-131` verifies all
26 frozen original source/assets/package/helper inputs and both corrected
observed helpers. Independent hash review confirms those same identities.
All 70 non-WebView receipt files are hashed locally; original evidence is
retained unchanged. The first session (`clipboard-observed-20261004-182310`)
passes the original native protocol and strict reader again: four exact RGBA
comparisons, two editable UTF-16 text records with matching style, and original
clipboard format-set restoration.

The second session (`clipboard-observed-20261004-182332`) fails at the final
`external(restore)` after saving `native-text-pasted.comp`. Its four native
clipboard reads and three writes return successful responses. Paste after Cut,
Undo/Redo and editable text Copy/Paste finish their original assertions. The
final application state has five layers, no error banner and working=false.
A separate diagnostic reader copies the original pixel/text assertion block
exactly and confirms all four strict RGBA comparisons and both editable text
records. It explicitly reports nativeProtocolAccepted=false; the unchanged
complete strict reader is not run as a passing second-protocol check.

The original helper response-5, helper-error.txt and restore-error.txt all
record Windows Forms SetDataObject failing with 0x800401D0. Both the main
restoration and its existing finally fallback fail. There is no
restored-after-error.txt, and the helper PID has exited. The original snapshot
was held only in that process; no recoverable payload backup exists in the
receipt. Original clipboard contents must not be reported as restored. No
new retry, relaxed assertion, timeout change or product patch is added.

This failure is in the independent helper restoration step, rather than an
observed application clipboard IPC refusal. Its underlying cause is unknown.
The prior actual Paste-after-Cut failure and initial native Save As timeout
remain retained without a claimed resolution. The complete timing evidence
remains 29/29; full Phase 5 acceptance remains false. No further blind run of
the existing clipboard script is requested.

A new bounded metadata-only ownership probe is compiled and locally checked.
Its native imports are limited to GetOpenClipboardWindow, GetClipboardOwner,
GetClipboardSequenceNumber, GetWindowThreadProcessId and SetLastError. It
never opens the clipboard, retrieves payloads, empties it or writes it. It
records UTC/elapsed time, open-window PID, last-writer PID and sequence changes;
a stop-file ends it without killing other processes. Normal completion,
stop-file handling and refusal to overwrite retained output are verified.
This establishes diagnostic capability only, not the owner of the historical
failure. A zero open-window handle can also mean an anonymous opener; the
last writer is not necessarily a current locker. See Microsoft documentation:
[open clipboard window](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getopenclipboardwindow),
[clipboard owner](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getclipboardowner).
No running user application or system setting is changed.

Local evidence: `clipboard-observed-1527-20261004-182308-131-received-verification.json`,
`clipboard-1823-first-independent-review.json`,
`clipboard-1823-second-data-diagnostic-review.json`,
`clipboard-ownership-readonly-validation.json` and the preserved case files.
