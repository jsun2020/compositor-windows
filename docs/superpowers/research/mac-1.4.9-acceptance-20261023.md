# Windows 跟进 Mac 1.4.9：本轮验收

本轮开始日期：2026-10-23。**本轮必需验收全部通过，2026-10-24 11:12:57（北京时间）完成收尾。** 未提交、推送或发布新版本。

最终回执：`build-artifacts/mac149-attended-20261024/result.json`，`passed=true`、`error=null`、`policyUnchanged=true`，总控状态 `complete`。汇总与证据哈希见同目录 `acceptance-summary.json`。下文按时间保留原失败及中间进度记录；当前结论以本段和完成条件表为准。

## 固定范围

- 工作区基线提交：`0e9206b56ad25b3c9dadee4aa2af0d04038cd182`，含现有未提交修改。
- 当前实现包含流式缩图/预览和选区删除修复，保留原来的 4 MiB 原始像素复制块。
- 当前生产 WASM：17,039,651 字节，SHA256 `27bef0df674e630ab11d65f3cf36f9c05f2891373330e365aa693c9bc8d230cf`。
- 冻结清单：`build-artifacts/mac149-full-20261023/inputs.json`，877 项输入。原 29 项 CPU 和 4 项 GPU 测试正文与历史 Git 基准核对，预算、单 worker、零重试均保持。
- Windows 本地执行脚本不调用 Codex 或模型 API。Mac 通过共享目录读取证据；Windows 原生、界面和硬件结果必须来自 Windows 实际执行。

## 已完成的判别

缩小原始复制块至 1 MiB 的独立候选不合入。固定 AB/BA 的 100MP 渐变安装结果：原实现 338/388ms，候选 556/421ms，原预算 <450ms。共 3 通过、1 失败，失败来自候选；173 条显示观察和 398 条会话观察均合格，原输入未变。38 项配对功能测试通过不能替代性能结果。详见本轮目录的 `raw-copy-decision.json` 和保留的原始输出。

当前生产 WASM 在 Mac 上执行原 WasmEngine/ApplyFilter 数值检查：31 个 Scanlines 合法间距与 5 个原始配方，共 **36/36 与真实 Mac 1.4.9 回传逐字节相同**。原脚本的 spacing7 Undo/Redo 也精确。此项不代替 Windows 浏览器、原生包或硬件性能验收；四个历史 MPS 内部 half 差异继续保留，不能宣称全部中间平面相同。证据：`build-artifacts/mac149-full-20261023/numerical-result.json`。

## 本轮完成条件

| 验收层 | 当前状态 | 本轮证据目录 |
| --- | --- | --- |
| 打包保护、固定上游资源、WASM 接口 | 通过；12 项保护测试、108 项 glue 接口 | `source/` |
| 完整原生引擎与 WASM 桥接 | 732 通过、0 失败，10 项原有忽略保留 | `native-engine-summary.json`、`source/` |
| Windows 外壳 | 19 通过、0 失败；原生包合计 751 通过 | `native-all-summary.json`、`source/` |
| 三项 TypeScript、前端构建、完整单元测试 | 通过；310/310 单元测试，构建 WASM 哈希一致 | `source/` |
| 完整浏览器功能 | 当前完整复核 344 通过、0 失败、33 原性能开关跳过；旧失败保留 | `mac149-attended-20261024/source/` |
| 原 29 项 CPU 硬件协议 | 29/29 通过，零重试，环境合格 | `mac149-attended-20261024/cpu/` |
| 原 4 项 GPU 硬件协议 | 4/4 通过，零重试，硬件及环境合格 | `mac149-attended-20261024/gpu/` |
| 当前生产 WASM 的 Mac 数值对比 | 36/36 精确 | `numerical-result.json` |
| 新生产便携 ZIP、源码绑定、许可和哈希 | 通过；11 个 ZIP 文件独立 CRC/哈希核对 | `package/` |
| 真实 Canon RAW 与无预览合成 DNG 显影 | 通过；各 6 组显影及原输入保护 | `package/` |
| 原生 WebView2、25 次 Windows 对话框、独立导出读取 | 通过；351 条环境观察合格，独立读取器通过 | `native/` |

表中以 `mac149-attended-20261024/` 开头的路径位于 `build-artifacts/`；其余相对证据路径位于 `build-artifacts/mac149-full-20261023/`。缺少结果文件、非零退出码、环境不合格或输入变更均不能记作通过。先前源码的 CPU 27/29、GPU 4/4 等历史结果继续保留，不沿用为当前源码的成绩。

原生默认忽略的 10 项保留：9 项手动原生性能诊断、1 项 Mac 探针生成器。它们不计作通过；正式浏览器功能中的 33 项性能开关由独立 29 CPU + 4 GPU 协议覆盖。稳定版本发布不是本轮自动执行步骤。

## 浏览器启动异常与后续复核

2026-10-23 22:09–22:17（北京时间），首轮开头 8 项分别出现页面创建、页面加载或引擎就绪超时。用户明确确认该时段“期间有重任务，后来停止”；停止后剩余用例恢复连续通过，完整结果为 336 通过、8 失败、33 跳过、0 flaky，耗时约 26.3 分钟。22:18 的只读系统快照显示约 14.5 GiB 可用物理内存、8.7 GiB 磁盘余量及单屏解锁；此为恢复后的采样，不能倒推失败发生时的资源状态，也不能精确证明根因。

准备 `scripts/continue-mac149-observed.cjs`：先等待首轮结束，要求非浏览器源码检查已全部通过，且首轮仅为已调查的 8 项失败；有其他失败则拒绝继续。复用相同冻结输入的原生、类型、构建和 310 项单元测试证据，完整复核 344 项功能并保留 33 项性能 opt-in。浏览器阶段增加与硬件阶段相同的单屏/解锁观察，仍使用原测试正文、时间限制、单 worker 和零重试。复核通过后依次运行原 29 CPU、4 GPU，随后自动执行生产包、RAW 与原生界面验收。

后续功能/硬件证据位于 `build-artifacts/mac149-observed-20261023/`，固定输入 885 项；生产包和原生 GUI 仍拟写入原本轮目录的 `package/`、`native/`。原始失败保留在 `mac149-full-20261023/source/`，不覆盖、不改成通过。用户已启动后续流程，22:36 开始带环境观察的完整浏览器复核，先前失败的 8 项通过。

22:44:16.605／22:44:16.617，两套独立观察器分别首次记录 `unlocked:false`。控制器按设计停止自有测试进程树；日志到第 157 项，停止前没有功能断言失败，但未生成完整 Playwright 汇总。总计 870 条显示记录、1910 条会话记录均保留，`healthy:false`、`passed:false`。这不是完整功能通过，后续 CPU、GPU 和生产包未启动。必须先解决桌面锁屏条件，再继续验收；不得把中断前部分结果拼成完整通过。

## 2026-10-24 自动锁屏处理

用户确认是自动锁屏、已经解锁，并明确要求处理后继续。新的执行目录为 `build-artifacts/mac149-unlocked-20261024/`，固定输入 904 项，旧输入、原测试正文和两次失败证据均保留。`session-plan.md` 记录精确范围：当前进程的系统/屏幕保持唤醒请求，以及必要时当前用户普通屏保 active 状态的临时调整（不写用户配置文件、不改密码要求）。独立辅助进程记录旧值、调整后回读、监控控制器存活、结束后比较并恢复；受管理锁屏或动态锁配置会在修改前停止。

启动入口为 `scripts/resume-mac149-unlocked.cjs`。语法及 904 项输入核对已通过；5 项隔离模拟控制测试通过，涵盖前置阻断、验收失败后的恢复、辅助进程丢失、恢复失败及完整成功。模拟检查不代表 Windows API 已实际执行，具体调整及恢复以 `lease/` 回执为准。当前已请求用户在 Windows 启动此入口，尚未取得本轮验收结果。

08:55，首个入口的辅助 PowerShell 在产生 host/prepared/ready 回执前退出（退出码 0、空日志）；启动器判为未完成，未启动任何功能或性能测试。不能据退出码宣称桌面状态调整成功。失败保存在该目录的 `launch-result.json`。

改用 `scripts/resume-mac149-session.cjs`，由 Windows 自带 .NET Framework 编译器构建独立辅助程序，避免依赖长期运行的 PowerShell 进程。状态检查和临时恢复范围不变。新证据目录 `build-artifacts/mac149-unlocked-native-20261024/`，固定输入 917 项；6 项模拟控制测试和语法检查通过，额外覆盖编译失败不启动验收。请求用户启动新入口，Windows 实际调整结果仍待回执。

09:07，独立程序入口在编译时因相对源码参数被 Windows csc 错误解析而报 CS1504；辅助程序和测试均未启动，未修改系统状态。该失败保留在 `mac149-unlocked-native-20261024/launch-result.json` 与 `lease/compile.log`。修正版入口 `scripts/resume-mac149-session-paths.cjs` 使用 Windows 原生绝对源码路径，并将后续打包、原生 GUI 的 PowerShell 文件参数统一为原生绝对路径。新证据目录 `mac149-session-paths-20261024/`；6 项控制模拟已改为 Windows 路径语义，断言编译参数为反斜线绝对路径，全部通过。尚须 Windows 实际启动验证，不计作验收通过。

修正版语法检查与 930 项冻结输入核对通过，所有原 CPU/GPU 测试正文与历史 Git 基准一致。Windows 启动请求已发出；当前仍无辅助程序成功启动或新增验收通过的证据。

09:11，修正版在真实 Windows 成功编译并运行独立辅助程序，确认当前桌面单屏、解锁。只读准备回执显示屏保 active=1、timeout=600 秒、secure=1；HKCU\Software\Policies\Microsoft\Windows\Control Panel\Desktop 中 ScreenSaveActive="1"、ScreenSaveTimeOut="600"、ScreenSaverIsSecure="1"。检测到策略配置后，在任何临时设置变更和测试启动前停止，恢复状态 UNCHANGED。不能仅凭该策略路径认定由组织管理，已询问用户策略来源；不会据普通屏保调整授权接管未知来源的策略。证据 `mac149-session-paths-20261024/policy-diagnosis.json` 及 `lease/prepared.json`。全部验收仍未完成，CPU/GPU/生产包/原生 GUI 尚待执行。

用户在策略来源澄清后明确表示“公司电脑，但我有管理员权限，可以设置”。据此准备 `scripts/resume-mac149-authorized-session.cjs`，范围限定为已观察 SID 的 HKCU ScreenSaveActive 单值临时从字符串 1 改为 0；保留 600 秒及密码要求，不修改机器／域／MDM 策略。当前会话调整回读、策略漂移停止、字段级比较恢复、独立手动恢复入口均已实现。10 项 Windows 路径控制模拟通过；原生辅助程序还将于真实 Windows 启动时先执行 8 项恢复判断自检，自检不操作 Windows API。新证据目录 `mac149-authorized-session-20261024/`。当前仍为已准备，尚未新增验收通过证据。

授权版语法检查及 949 项冻结输入核对通过，所有原 CPU/GPU 正文与 Git 基准一致。已请求用户在原账号的管理员 PowerShell 启动授权版入口；本机 10 项控制模拟通过，真实 Windows 编译及 8 项原生纯逻辑自检仍待启动回执。

09:22，授权版在原 SID 的管理员会话执行，真实 .NET 编译及 8 项恢复判断自检通过。HKCU ScreenSaveActive 临时写入 0 的回读通过，但 SPI_SETSCREENSAVEACTIVE 返回 1260（ERROR_ACCESS_DISABLED_BY_POLICY）。辅助程序随后将本次单值恢复为 1，并验证 policyRestored、screenSaverRestored、registrySnapshotMatches 均为 true，restoreStatus=RESTORED、restoreError=null。整轮 passed=false，功能及硬件测试尚未启动。完整证据见 `mac149-authorized-session-20261024/policy-block-result.json`。当前等待选择策略允许的测试会话或有人值守运行，不继续通过改其他策略字段尝试放行。

用户选择“采用有人值守方式，保留锁屏策略”。准备 `scripts/resume-mac149-attended.cjs`：只读核对三个屏保策略值及 SID，不调用调整辅助程序。五阶段分别等待本人在 Windows 终端按回车；浏览器／硬件阶段提醒本人值守，原生 GUI 阶段提示勿操作键鼠。可复用已完整通过且输入一致的阶段，不重跑已有失败或中断阶段；CPU 有效断言失败仅允许独立 GPU 检查，仍阻断生产包。10 项控制测试通过。新目录 `mac149-attended-20261024/`，真实 Windows 执行尚待启动。

有人值守版最终 968 项输入及原 CPU/GPU 测试正文核对通过，10 项控制测试通过。已请求用户在普通 PowerShell 启动入口并按回车开始第一阶段；无需管理员，不修改屏保策略。全部真实验收结果仍待本轮 Windows 执行。

09:31:57，有人值守版收到第一阶段回车确认并启动完整浏览器功能复核。初始三个策略值仍为 ScreenSaveActive=1、ScreenSaveTimeOut=600、ScreenSaverIsSecure=1；未修改设置。09:34 日志推进到第 45/377 项，尚无失败记录，两个独立观察器持续报告单屏解锁。该进度不是完整阶段通过，最终以 source/result.json 为准。

09:45:56，有人值守版完整浏览器功能通过：344 通过、33 原性能 opt-in 跳过、0 失败、0 flaky、零重试，约 13.2 分钟。独立核对 details.json 每项均仅一次结果，数量与 result.json 一致，当前 968 项清单哈希一致；1520 条显示观察全部单屏解锁，3533 条会话观察全部解锁，均覆盖工作负载结束。证据 `mac149-attended-20261024/source/result.json`、`details.json`、`review.json`。旧 8 项启动失败和锁屏中断仍保留，不覆盖。接下来等待在场确认后执行原 29 CPU。

当前源码完整原 CPU 协议 29/29 通过：约 6 分钟，0 失败／跳过／flaky／重试。726 条显示、1715 条会话观察全部合格；原始 details 每项一次执行，与结果及冻结清单哈希独立核对一致。24MP 删除 worst98ms（原<150ms），100MP 渐变安装320ms（原<450ms）；保留此前较早源码27/29失败，不改历史结果。本轮没有新的产品源码修改，不能将所有性能差异归因于单一原因。09:54:05 用户确认开始原4项GPU。

当前源码原 GPU 协议4/4通过：41.2秒，0失败／跳过／flaky／重试；105条显示、239条会话观察全部合格。实际渲染为 Intel HD Graphics 520／D3D11。原始details每项一次，结果及清单哈希独立核对一致。功能阶段33项性能opt-in已由完整29CPU+4GPU覆盖。09:55:53 用户确认启动生产便携包与RAW阶段；全部验收仍待生产包和原生GUI结果。

10:20，生产便携包与 RAW 阶段通过。新包 `Compositor-portable-0.9.0-20261024-0956.zip`，10,582,758 字节，SHA256 `29d04543e1704cb3a5ca0653ef9387b10f4708e3f391ed56c08a802213379a83`；11 个 ZIP 文件的 CRC、字节数及 SHA256 独立复核相符。真实 Canon CR2 与不含预览的合成 DNG 各六组显影通过，可重复 As Shot、控制项生效、无效输入拒绝及原输入保持均通过。构建保留已有警告但正常退出；包内版本仍为 0.9.0，属于本地 Mac 1.4.9 对齐候选，未发布。

10:24:11，原生生产 WebView2 验收通过，25 次真实 Windows 对话框全部关闭；32 项 Mac 探针输入前后相同，无页面错误。覆盖全屏／Escape／窗口恢复、输入优先级、Navigator、Scanlines Cancel/Apply/Undo/Redo/Last Filter、PNG/JPEG/PDF、WebP、真实 Canon RAW 与 100MP PDF。351 条独立环境采样全部单屏解锁并覆盖工作负载结束；Pillow/PyMuPDF 独立读取器通过，100MP PDF 为 10000×10000 像素、2400×2400pt，并包含透明度软蒙版。`native/review.json` 独立核对原始对话框日志、环境记录与读取回执。

五阶段测试均通过，Mac 端再次独立核对全部 968 项冻结输入一致；当前等待总控最终锁屏策略前后核对回执，尚未将整体验收标记完成。

## 最终收尾

2026-10-24 11:12:57（北京时间），总控最终回执落盘：五阶段全部通过、无阻断、无错误，状态 complete。已独立核对总控嵌入的五阶段结果与原文件一致、全部结果哈希与此前复核一致、便携 ZIP 哈希一致。968 项冻结输入已在原生验收结束后独立完整核对相符。

当前用户策略前后 JSON 完全相同：ScreenSaveActive="1"、ScreenSaveTimeOut="600"、ScreenSaverIsSecure="1"，类型均为 String，SID 一致。有人值守流程未修改锁屏策略。自动锁屏导致的中断通过有人值守、阶段确认和环境观察完成恢复；未关闭公司的锁屏要求。此前临时调整尝试的恢复回执和所有失败证据保留。

本轮必需验收已完成。原有 10 项默认忽略的原生诊断仍不计作通过；浏览器的 33 项性能 opt-in 已由独立 29 CPU + 4 GPU 全部覆盖。包内版本保持 0.9.0，验证范围为本工作区跟进 Mac 1.4.9 的实现。新便携包仅保存在本地，未提交、推送或发布。
