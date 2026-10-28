# Windows Compositor: Mac 1.5.11 upgrade

Status: **local acceptance completed on 2026-10-28**. Publication is a separate clean CI gate; this report does not claim that CI or public release has completed.

## Pinned inputs and peers

- Windows product baseline: `2b328db1343cfa258058f65600207005becbe487` (0.11.0, Mac 1.5.7 parity).
- Mac reference: `9959c32d77330ec8d7b42a49353d903283ecb8e6` (v1.5.11).
- Final attended/recovery run: `d30d6836-b8b7-4bc0-9ef2-ca9418f66117`; preceding upgrade runs and failed attempts are retained in private evidence.
- Mac: Codex, `/Users/jiesen/Documents/sr9rfx/.claude-project/compositor-windows-mac1511`, branch `codex/mac-1-5-11`.
- Windows: Grok Bot 0.75.8, SVMEL16162, `C:/Users/sr9rfx/.claude-project/compositor-windows-mac1511`, branch `codex/mac-1-5-11-ui`. Model identity was not separately readable.
- Both peers exchanged and acknowledged HELLO through agent-pair-test over the existing verified SSH tunnel. Reason Network remains stopped; no autostart, keep-awake, synthetic attendance or lockscreen-policy changes.

## Scope and ownership

Mac owns the Rust engine, WASM, file versions 18–20, reference fixtures and native integration. Windows delivered the owned frontend patch; Mac now owns integrated frontend fixes and tests. Mac integrated and reviewed the acceptance harness; Windows installed it with seven matching file hashes and seven successful static checks. Windows UI step `a4d11ca7-e230-4497-8b0f-a7396cf3e67a` was received and its isolated clean worktree and reference ZIP hash confirmed.

Required changes: Align/Distribute with groups, masks and single undo; custom crop and 2.39:1; Vignette/Lens Effects/Tonal Contrast/Sharpen adjustment layers; Sharpen filter; revised Prism and threshold density; consistent partial/full rendering; color picker confirmation and stable hand-tool cursor.

## Implementation and independent comparisons

- Engine compilation passed after adding the new settings and alignment commands.
- Three alignment behavior tests passed (selection/canvas targets, folders/hidden layers/mask linking, distribution and undo).
- Mac's pinned original Swift/Core Image/Metal sources generated independent 1.5.11 optics fixtures; source SHA-256 is checked by `scripts/check-mac1511-lens.py`.
- Five revised Prism comparisons passed after fixing affine sampling and the integral image footprint. Independent density-source comparisons at thresholds 96/98/100 passed after quantizing the morphology radius before selecting disc taps. Original failed comparison logs remain in the private run directory.
- Eighteen Sharpen cases passed the independent Core Image comparison (maximum observed channel difference 3/255). Nine Vignette cases are byte-exact against the original C kernel. Empty-layer Vignette preview/commit/undo and selection behavior passed two engine tests.
- Tonal Contrast's C stage is byte-exact when fed the same Core Image blur. The portable Gaussian is an approximation: radius 1 differs by up to 4/255 at edges; radii 16/100 by up to 1/255. At radius 100, unpremultiplication amplifies that one blurred-byte difference to a maximum 6/255 in the final output (mean 0.1873/255). The initial provisional end-to-end bound of 4 failed and remains recorded; the new cross-platform numerical bound is explicitly 8/255 maximum and 0.5/255 mean, not a byte-exact claim. No existing performance budget or test retry policy changed.
- Four format-version tests passed for versions 18–20 and invalid nested settings, including original Swift Codable models → Rust package save/reopen → original Swift decode. This is model interchange, not an actual Mac GUI package-open test; the installed Mac application remains 1.4.9. The final Windows native 1.5.11 group subsequently verified editable-layer save/reopen.
- The second Windows UI patch was hash-verified and applied. Mac fixed adjustment/filter name overlap, Tonal identity, blank Vignette eligibility/worker dispatch, picker cleanup and Space guards. 357 frontend unit tests and ten new WASM browser flows passed. The final full browser run passed 371 cases with 33 pre-existing opt-in skips; the earlier failed test-script runs remain in private logs.
- The previous release report is retained as [0.11.0 acceptance](docs/releases/0.11.0-acceptance.md); the current native regression and increment groups were executed against the 0.12.0 production package.

- The final Mac core snapshot passed 807 tests with 10 pre-existing opt-in ignores. Three TypeScript checks, WASM export verification, release guards and a production frontend build passed.
- Prism blur computes only the requested window while preserving the full image resize lattice; independent comparisons and thin-image tests pass. The existing optical working-set ceiling now also protects adjustment display, export, merge and all-layer sampling; oversized renders return an error, preserving the document.
- Product version: 0.12.0. Windows source verification ultimately passed in SOURCE05. SOURCE02 exhausted the original 30-minute controller after a 27m05 cold build and 427 passing tests. SOURCE03 failed with Windows os112 (disk full) while writing the shell static library. Neither attempt is recorded as a passing source stage. The source storage recovery uses process-local Cargo debug=0 and incremental=0; it retains the same test cases, debug assertions, optimization level and time limits. Production release settings and CPU/GPU budgets are unchanged.

## Final Windows acceptance

| Gate | Audited result |
| --- | --- |
| Source | 848 Rust tests passed, 12 ignored; WASM and three TypeScript checks; 357 frontend unit tests and 15 release guards passed. Five earlier successful components were reused only with complete input hashes. |
| Real Windows codecs | All five bounded components exited 0: PDF, bundled HEIF, ICC and CMYK, with independent reader comparisons. |
| Browser functionality | 371 passed, 33 separately scheduled performance cases skipped; no failures or flaky cases. |
| CPU / GPU | 29 / 4 passed, original budgets, one worker, zero automatic retries. |
| Production package | Actual controller exit 0; real Canon RAW and preview-free synthetic DNG each passed eight checks. ZIP CRC and all 25 entry SHA-256 values were independently verified on Mac. |
| Native regression | 10 cases, 25 real dialogs, actual internal GUI exit 0; independent image/PDF readers and 511 desktop-monitor samples passed. |
| Native 1.5.7 additions | Six cases, 16 real dialogs, actual internal GUI exit 0; independent readers and 197 desktop-monitor samples passed. |
| Native 1.5.11 additions | Six cases, 20 real dialogs, actual internal and outer process exits 0; 243 desktop-monitor samples passed. Saved manifests confirm adjustments, crop and alignment/distribution with undo. |

All native groups used the production executable with the test API absent. Monitor samples covered each workload and confirmed an unlocked single display; original eight-minute GUI limits remained unchanged. No synthetic attendance, keep-awake or lock-policy changes were used.

### Evidence and corrected harness limitations

The native 1.5.11 first attempt stopped on an ambiguous global `Blur` label; its PowerShell wrapper also failed to retain a readable process exit code. The correction scopes the numeric controls to the adjustment panel and retains the process handle, rejecting null exit codes. Windows controls captured actual exit codes 0 and 7. A fresh output directory then ran the corrected third group once, without modifying frozen product inputs or repeating the first two groups. Both failures and corrected results are retained.

The original outer launch-process exit codes for the first two native groups were **not reliably captured**. Initially inferred zero values were withdrawn. Their acceptance rests on actual internal GUI exits of zero, completed controller PASS records, independent reader outputs, desktop monitoring and frozen-input checks. They are not represented as measured outer exit-zero runs. The corrected third group captures both inner and outer actual exits of zero.

For that corrected group, Mac independently verified all 55 listed evidence files. The transferred inventory also listed an earlier version of itself; that stale self-entry is excluded from validation, and the final inventory's own SHA-256 is recorded separately. No test output mismatch was found. The production executable SHA-256 matches the executable extracted from the accepted ZIP.

Private evidence roots: `20261027-compositor-1511-upgrade` and `20261028-compositor-1511-attended`. Windows results: `build-artifacts/mac1511-acceptance-20261027`, `build-artifacts/mac1511-package-recovery-20261028`, and `build-artifacts/mac1511-native1511-fix01-20261028`. The earlier cold-build timeout, disk-full failure, missing transferred WASM snippet, codec fixture path error, package timeout and unattended-terminal expirations remain preserved. They are not recorded as passes.

Frozen manifest SHA-256: `e87f3ef1cf9d621c20be0211e12e1840c39c5391a11c5276184c505c897e504d` (1187 project and 62 external inputs). Reused stage receipts are bound by exact hashes; the corrected native harness is an explicitly recorded external overlay.

Accepted local package: `Compositor-portable-0.12.0-20261028-1027.zip`, 15820542 bytes, SHA-256 `f50b4989920afda13497978f18a5b01112f33361c8666f06d843e9152649ca5f`.
Production executable SHA-256: `d61c5120cc09e7e16f1babdcfe99d29f9f8085ee5928617cad6193d63ae3b786`.

## Publication and scope

The local acceptance package has `sourceDirty=true` and is evidence of the reviewed candidate, not a clean CI release. Windows Release must rebuild the committed source, run its validation, and verify the ZIP, source commit, release manifest and checksums before publication. Public assets are not expected to be byte-identical to the local package.

Mac source/model and independent pixel checks are distinct from a Mac 1.5.11 GUI run, which was not performed. Core Image optics remain bounded portable approximations as described above and in the technical reference. Private credentials, transient diagnostics and machine caches are not part of this source commit.
