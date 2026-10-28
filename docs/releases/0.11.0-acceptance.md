# Windows Compositor 0.11.0 / Mac 1.5.7 acceptance

Date: 2026-10-25. Mac Codex reviewed source and evidence; Windows Grok ran the Windows tests. Coordination used the pair-test skill and Reason Network. The Windows desktop was single-display and human-attended; native UI phases ran without operator keyboard/mouse input. Lock policy was unchanged. Tests retained their original assertions, performance limits, one worker and zero automatic retries.

## Results

| Gate | Result |
| --- | --- |
| Source / WASM / real Windows codecs | 17 steps passed; 796 engine tests, 332 frontend unit tests, Windows shell and three type checks |
| Browser functionality | 361 passed; 33 performance cases run separately |
| CPU / GPU | 29 / 4 passed |
| Portable and RAW | ZIP and all 25 entries verified; real Canon CR2 and preview-free DNG passed |
| Production native regression | Passed; 25 real dialogs, independent image/PDF readers |
| Mac 1.5.7 native additions | Passed; 16 real dialogs, independent image/PDF readers |
| Final controller / input audit | Exit 0; all 1801 inputs unchanged; policy unchanged |

Evidence root: `build-artifacts/mac157-package-observer-recovery-20261025` (local, not included in the source repository). The complete final receipt ended at 2026-10-25T12:23:34.828Z. Earlier failed/interrupted attempts are retained. Source/function/CPU/GPU passes were carried only after exact source, runtime and receipt hash checks.

Frozen input manifest SHA256: `da10b0e753fafecac4f5a49b275de266a383201708169c3ac9404aa4272d5664`.
Accepted local ZIP: `Compositor-portable-0.11.0-20261025-1954.zip`, 15721905 bytes, SHA256 `0fb47499a59a96f9102dadc0b8b16a3922a9c08444a5ff0fa34a166a27973891`.

## Findings and fixes

- PDF rasterization now preserves requested page size and 300 DPI across Windows scaling.
- Bundled HEIF decoding removes the system HEVC extension dependency; ICC and CMYK import/export were checked with independent readers.
- RAW save/reopen testing bounds debug-protocol notification copies without changing payload bytes. Exact Camera Raw caching reduced the measured 10 MP apply to 19.259 s under the original 30 s limit.
- Buffer ownership and aligned uniform fill avoid unnecessary large-array copies. The 100 MP Levels result installation measured 390 ms under the original 450 ms limit.
- Native navigation assertions follow the upstream small-image zoom behavior. Controller recovery preserves failed evidence and captures real exit codes and both observer cleanup receipts.

## Limits and release provenance

No remaining local acceptance failures. Mac/Windows rendering differences and unsupported cases are documented in `docs/superpowers/research/mac-1.5.7-delta.md` and `docs/releases/0.11.0.md`.

The accepted local package was built from the reviewed working tree. Publication uses the repository Windows release workflow to rebuild a clean tagged commit and verify its own ZIP, marker, source commit and checksums. The CI package is not claimed to be byte-identical to the local acceptance ZIP.

Published [v0.11.0](https://github.com/jsun2020/compositor-windows/releases/tag/v0.11.0) from clean commit `824488defc40bd55dc314efb2e6dcd8c6820f8e1`. Both [Windows Release](https://github.com/jsun2020/compositor-windows/actions/runs/40195836163) and [Windows CI](https://github.com/jsun2020/compositor-windows/actions/runs/40195836198) succeeded. The downloaded public ZIP, all 25 entries, release manifest, checksum file and tag/source commit were independently verified on 2026-10-25.

Published ZIP: `Compositor-portable-0.11.0-20261025-1322.zip`, 15981646 bytes, SHA256 `9797cfba903e58531e703cb7681f8577a84525d9df78a77f0e4da918bf19f212`.
