# Compositor for Windows 产品需求

更新时间：2026-10-24。

## 产品与当前基线

面向 Windows 的免费图像编辑器，使用 Tauri 2、Rust/WASM、WebGL2 和 React。当前稳定版为 0.9.0；原定 Phase 1～7 及 Phase 3.5d 的 v1 移植范围已完成，验收基线是 Mac Compositor 1.4.5。历史验收记录继续保留。

现有项目管理、图层/蒙版、24 种混合模式、选择、绘画/修复、文字/形状、六种效果渲染、Liquify/Smudge、Camera Raw、受限分层 PSD/PSB 与真实相机 RAW 导入属于产品基线。字体、Apple CIRAWFilter 与 LibRaw、超大图预览等已声明限制仍适用。详见 README 和原阶段验收文档。

## 本次增量目标

跟进 Mac **v1.4.9** 的用户可见变化，固定提交 `0ab3ff58a3dcf3d52a455fe3ab0336ac95923edd`。对照起点为 v1.4.5 `086f1631573ccb2b57644e53b52bf1488fc976aa`。研究依据见 [版本差异](docs/superpowers/research/mac-1.4.9-delta.md)。

本次增量定版为 Windows 0.10.0。本轮必需验收已通过，用户已授权 commit、推送 GitHub 和发布；沿用现有 Windows CI 发布门槛，不改写已发布的 0.9.0。`.comp` 保持读版本 1～11、写版本 11，无格式迁移。

## 2026-10-24 当前验收与发布授权

本轮必需验收已完成：原生引擎及 Windows 外壳 751 项、前端单元 310 项、完整浏览器功能 344 项、原 CPU 29 项、GPU 4 项全部通过；生产便携包、两类 RAW 各六组显影、25 次原生对话框及独立图片/PDF 读取通过。36 组当前生产 WASM/Mac 最终像素对比精确。968 项冻结输入一致，锁屏策略前后相同。10 项原有忽略诊断不计作通过；33 项功能层性能 opt-in 由独立硬件协议覆盖。详见 [本轮验收报告](docs/superpowers/research/mac-1.4.9-acceptance-20261023.md)。

用户已在本轮验收后明确授权提交、推送和发布。Windows 版本元数据统一为 0.10.0；新发布包由 clean-source Windows CI 验证并构建，旧 0.9.0 发布保持不变。下文旧“未完成／不发布”条目保留为对应日期及源码的历史状态，不代表当前授权或验收结论。

## 范围与验收要求

| 编号 | 需求 | 必须满足的行为 |
| --- | --- | --- |
| A01 | Camera Raw LUT 管线 | 同步 1.4.9 实测色表、插值与阶段顺序；包括 Light/Color、白平衡吸管/Auto、Mixer、Calibration、曲线、Grading。Point Color、移动曲线分界、Refine Saturation 等走适用的独立路径。 |
| A02 | Camera Raw 局部算法 | 同步 Texture/Clarity 引导滤波、画面相关 Dehaze、Sharpening、彩色降噪与暗角；透明像素结果稳定，预览/Apply 参数一致。 |
| A03 | Camera Raw 预览性能 | 缓存会话统计，组合色表，减少重复计算与 scope 开销。保持有界内存、取消与旧任务丢弃；在 Windows 实测，不承诺直接达到 Mac 宣称的 20 倍提升。 |
| A04 | 半透明图层效果 | Color Overlay、Inner Glow、Inner Shadow 修改源 RGB 并保留源 alpha，再与底下效果合成；阴影不被错误染色。普通不透明输入保持合理一致。 |
| A05 | 图层属性保留 | Canvas Size、Crop、仅修改 DPI 时保留 effects/live text/live shape；实际重采样和破坏性像素编辑可栅格化文字/形状，保留图层效果。核对非等比、旋转/翻转图层的 Image Size 效果参数。 |
| A06 | RAW 导入渐变与状态 | 在真实传感器解码基础上评估 16→8 位抖动量化；导入直到图层就绪始终有状态。保留 LibRaw/Apple 解码差异说明、预算、取消及辅助程序完整性。 |
| B01 | New Canvas | 支持 px/in/cm/mm，72/300 DPI，透明/白/黑背景；默认透明、px、72 DPI；换算与预算检查正确，DPI 存入项目/导出。 |
| B02 | Rotate Canvas | Image 菜单提供顺/逆时针 90°；交换宽高，转动所有图层/文件夹/独立蒙版/选区/参考线，保留源像素，一个 Undo。 |
| B03 | Search Commands | 搜索共享菜单命令、工具及模式；显示禁用/开关状态，支持方向键、Enter、Esc；Windows Ctrl 快捷键，文本输入优先。 |
| B04 | Last Filter | Filter 菜单重复最近成功 Apply 的滤镜和完整参数，直接执行、一个 Undo；取消不覆盖记录；Image 调整和 Content-Aware Fill 不计入。 |
| B05 | Fullscreen | F 切换仅画布的全屏显示；Esc 优先取消编辑，再退出全屏；退出恢复窗口/布局，输入框不误触。 |
| B06 | Navigator | View 开关；300% 起且画面超出视口时显示右上角小地图及视口框；点击/拖动定位，编辑后有界刷新。 |
| B07 | Export As / PDF | 统一 PNG/JPEG/单页 PDF、输出尺寸/比例与预览；JPEG 质量/底色、PDF 底色；PDF 页面点数=输出像素÷DPI×72。导出副本不改变项目尺寸或保存状态。 |
| B08 | Scanlines | 独立滤镜：Spacing、Thickness、Glow、Dots、Wobble、Displace、Smoothness、Threshold、Color Split、Density、Contrast、Black Level、颜色；支持 Preview/Cancel/Apply/Undo 和 Last Filter。 |
| B09 | 中键缩放 | Ctrl+中键拖动缩放并固定锚点；普通中键拖动仍平移。 |
| B10 | 图层/UI 改进 | 画布点选后展开并滚动定位图层；各工具条窄窗口横向滚动；重命名无跳动，按钮/禁用态清晰；平移期间闭合手形。 |
| B11 | 标签名称 | 未保存文档使用最低可用 Untitled / Untitled N；关闭最后文档清空工作区，原明确保存/放弃/取消流程保持正确。 |
| C01 | 已有 WebP 回归 | 验证有损/无损/透明 WebP 导入、拖入、保存重开和预算限制，无重复解码器开发。 |
| C02 | 分层验证 | 通过 Rust、WASM、前端、浏览器、Windows 原生包、文件选择器及性能检查；原性能断言/预算不得为通过而放宽；上游明确改变的像素语义使用固定新版独立基准，并记录变化、保留旧输入。 |
| C03 | Mac 1.4.9 对比 | 新建独立 oracle 与回传目录，冻结源/参数/版本；保留旧 1.4.5 记录。尽量一次回传，中文 MAC-STEPS.md 保留英文菜单并写明参数。 |
| C04 | 发布准备 | 更新 README、兼容性边界、版本与 CI；验证 LUT/RAW helper/portable ZIP 的来源、许可、哈希与完整性；未经授权不发布或推送。 |

## 范围之外

本次不自动扩展到 Vision 智能选择、HEIC、自动更新器/Sparkle、安装器/签名、PSD/PSB 全格式、单文件项目容器，或早于 1.4.5 已有但未纳入 v1 的额外滤镜。其他四种效果的完整编辑 UI 可另行规划；本次必须修复它们已有的渲染/保留行为。

## 实施顺序与完成定义

1. 先更新此 PRD、冻结上游源码/色表与探针；在独立开发分支实施。
2. 先 A04/A05 和 A01～A03，再 A06；每项使用针对真实边界的回归验证。
3. 实施 B01～B11，复用已有 engine、Worker、ShellBridge 和菜单/快捷键系统。
4. 完成 C01～C04，记录源测试、浏览器、原生包、性能和 Mac 对比各自的证据；缺少任一必要层就保留“未完成”状态。

允许逐批提交可审核实现，但不能把部分通过记作全部验收。用户操作仅用于必须在 Mac 或物理桌面进行的验证；Windows 可自动完成的验证由开发方执行。

## 2026-10-16 性能收尾的证据与实施约束

- 已绑定 `0330d22` 的源码／数值、343 项浏览器功能、原 4 项 GPU 性能、新生产包 25 次 Windows 原生对话框与独立读取器验证通过。原 CPU 完整验收仍为 27/29：100MP Fill 安装 458ms（<450ms），覆盖蒙版下渐变拖动 53ms（<50ms）；环境观察全部单屏解锁。当前增量仍未完成，不发布或推送。
- 用户回传的 Windows 系统轨迹 `cpu-system-stack-20261016-184138-816` 已保存，438 条环境观察均满足条件。逐项诊断复现 113ms 长帧及约55ms渐变；控制器退出码误记为 null，独立逐项失败日志保持失败，不把空值视为成功。ETL 解码未报告丢失事件；系统与 Edge 的匹配符号将本次长帧追到前一页面 WASM 地址区的 `DOMArrayBuffer` 主线程清扫／虚拟内存释放，旧地址区曾提交约1GB。这里只确认该样本的机制，不能将所有历史超时归为同一原因。
- 保留原断言和预算的旧版／当前版无采样钩子隔离对照分别为3/3与2/3。旧版 Fill 安装404ms、帧间隔25ms、渐变35/30ms；当前版 Fill 安装275ms、帧间隔113ms、渐变39/48ms。隔离样本不替代原29项完整验收，也不足以把浏览器回收问题直接认作某一源码改动导致。
- 渐变的后续实施范围先限于轴对齐 Linear：复用每列、每行中完全相同的 Float64 算术项，保留原减法／乘法、最终加法、clamp 与通道舍入顺序；不使用递推、近似 LUT、额外 FMA 或降低预览分辨率。非轴对齐、Radial 和无效变换保留原路径。先做独立内核的完整字节／有界计时实验，有收益后再接入真实引擎，并核对既有冻结像素、实际生产 WASM 与原完整性能协议。
- 长帧继续单独处理。没有资源所有权与异步任务安全证据前，不直接在 pagehide 释放仍可能使用的 WASM 指针，不借延后到计时外、强制 GC、改变浏览器配置、提高预算或增加重试来计通过。
- 轴项缓存额外工作内存上限为1MiB；零尺寸、非有限／非轴对齐变换、超过缓存上限或可选缓存分配失败均退回原逐像素计算。独立实验36组完整像素相同；其计时期间发现另一个磁盘清理扫描进程，因此只保留为未充分控制环境的候选数据，不能据此声称原性能门槛通过。
- `0e9206b` 的新生产包、原25次原生交互及原4项GPU性能通过；单屏解锁的原完整CPU对照为26/29，881条环境采样全部满足条件。实际失败为100MP任务帧间隔122ms（<100ms）、Levels结果安装464ms（<450ms）、覆盖蒙版渐变52ms（<50ms）。不能再将这些失败归因于锁屏；此前无测试接口的错误测试构建、锁屏运行与全部原断言／预算均保留。
- 完整浏览器342/343的首项5秒就绪超时仍保留。独立单次启动诊断的页面加载后等待约870ms，流式编译约223ms、实例化约12ms，没有复现；该诊断不替代原功能失败。只读检查记录当时连接交流电、节电关闭、Balanced方案，不据测试结束后的进程快照推断测试期间的负载或修改系统策略。
- 下一步渐变候选只复用不透明灰度RGB中完全相同的通道算术：须验证RGB的起值与差值Float64位模式相同，且本像素覆盖完整、源alpha为1；沿用原表达式与舍入，将同一字节写入三个通道。彩色、部分覆盖、非不透明与单通道蒙版保留原计算。先做独立真实内核的完整像素／有界AB/BA实验，有收益再接入；不能据此宣称解决安装耗时或主线程回收长帧。
- 上述隔离实验的81组输出逐字节相同；不透明灰度的两种网格与Linear／Radial／非轴对齐样本内核中位耗时改善约1.72～2.02倍，彩色与部分覆盖不据微基准宣称改善。首次私有脚手架编译因换行／Rust edition／类型标注失败，未运行实验，失败原件保留；修正后结果不代替原性能协议。现仅合入相同位模式下的通道表达式复用，并新增独立原标量混合算术的舍入／fallback回归；当前原完整验收仍未完成。

## 2026-10-18 当前验收与黑色渐变候选

- 不透明灰度候选的完整 Rust 工作区为747项通过、10项原有忽略；前端45个文件308项、三项TypeScript检查与构建通过。实际生产WASM的96组渐变完整缓冲／两种预览／提交／Undo／Redo、36组Mac Glow输出逐字节一致。本轮原功能为342/343，首项引擎就绪超过原5秒；原CPU为27/29，失败为100MP Fill帧间隔104ms（<100ms）及100MP覆盖蒙版渐变50ms（<50ms）。原GPU4/4通过。794条CPU和128条GPU独立环境采样全部单屏解锁；761/760项冻结输入前后一致，单worker、零重试。记录见`acceptance-status-gray-20261018.json`，整体仍未完成。
- 独立启动诊断未复现原5秒失败：流式编译约261ms、实例化约13ms；其单次通过不替代原功能失败。渐变分段诊断确认默认配方是RGB为零、alpha从1到0，原不透明灰度分支不覆盖该路径。独立诊断的24/100MP最坏tick为32/24ms，仅作分段观察，不能改写原27/29结果；此前未进入计时的诊断和脚手架失败记录保留。
- 共享半透明灰度RGB源项的候选215组像素精确，但实际网格内核变慢约3%～12%，未接入。后续零RGB候选230组完整像素精确；将判断移到网格入口后，实际两种网格的有界AB/BA内核中位耗时改善约2.09～2.51倍。非目标样本仍有计时波动，不能据此宣称原完整性能已通过或所有fallback均有收益。
- 本轮接入仅限：RGB两端位模式均为正零、像素完整覆盖、采样位置和源权重有限、目标三个RGB字节已为零时，保留这些零字节并沿用原alpha表达式。网格入口选择常量专用循环；其它颜色、不透明／蒙版路径不增加该逐像素判断，部分覆盖和非有限输入保留原计算。不增工作内存、不改预览尺寸、不增加FMA或改变舍入顺序；新增独立原标量算术回归覆盖透明度、目标alpha、符号零、近零值、覆盖率及非有限采样。
- 新候选源码、实际生产WASM、完整浏览器、原29+4硬件协议与生产包仍需重新验证；旧通过与失败均绑定旧源码，不沿用为本轮验收。版本保持0.9.0，不推送或发布。

## 进度

- PRD 与差异研究：已完成。
- A01～A06：实现已落地；无嵌入预览的合成 Bayer DNG 和真实 Canon EOS 40D CR2 各六组显影检查已通过，源文件保持不变，最终 release WASM 已构建并核对 108 项导出；性能及 Mac 1.4.9 像素对比待验收。
- B01～B11：实现已落地；完整浏览器功能回归 343 项通过（含新增 8 项）；旋转 Undo 自动适配已修复并通过 8 项新版功能及 19 项尺寸/保存/历史补充回归。修复后的 Windows 原生包已通过 25 次实际对话框操作、窗口全屏恢复、WebP、真实 Canon RAW 保存重开和 100MP PDF 导出；独立 Pillow/PyMuPDF 读取验证通过。
- C01～C04：验收进行中；完整 Rust 工作区 729 项已通过（原 728 项及新增覆盖蒙版回归），固定 1.4.9 LUT/内核来源已验证，新的独立 Mac 对比包和中文操作文档已生成。产品版本暂保留 0.9.0，不更新已发布包。
- 44 个文件的 306 项单位测试已在单 worker 下全部通过，原 5 秒预算不变。早期失败证据继续保留：并发执行时一个单位测试超时、浏览器启动未就绪，以及完整 Rust 构建因磁盘耗尽失败。清理可再生成的编译缓存后继续验证，原时间预算不变；Camera Raw、Image Size 及四个旧效果探针按固定 1.4.9 更新独立语义基准，具体变化见差异研究，原图片和失败记录保留。
- 原生首次保存出现 Windows `Access is denied`，失败窗口与日志保留。目录原子替换增加 250 ms 等待预算处理 Windows 暂时占用，永久占用仍报错且保留旧项目；两项真实不共享删除的文件句柄回归已通过。验收控制器另行等待落盘或错误提示，并保留每次画布 PNG 和精确 RGBA 哈希，验收运行仍不增加重试。
- 原 4 项 GPU 性能用例已通过，单 worker、零重试。原 29 项 CPU/UI 用例的完整单屏解锁运行仍为 27 通过、2 失败：覆盖蒙版下渐变拖动 52ms（原预算 <50ms），100MP 空白图层 Fill 的安装 622ms（原预算 <450ms）；755 项输入前后核对一致。200ms 会话观察 1887 条、500ms 单屏/会话观察 822 条全部满足条件，因此本轮不能归因于锁屏。原 v3 锁屏失败与 v4 解锁失败均保留，性能验收尚未完成。分段诊断表明渐变重复上传相同覆盖蒙版，纯色填充安装主要消耗在全尺寸展开；先验证像素不变的优化，再以原完整协议验收。
- 首版效果探针误将 PNG 写到 `.comp` 根目录；固定上游要求 `images` 子目录。旧探针和失败记录保留，新 `Mac149-probes-v2.zip` 仅修正目录、保持原参数与像素，通过 32 项输入哈希核对及 Windows 实际打开/导出验证。生成器及打包前结构检查已修正；Mac 1.4.9 回传仍待完成。
- 覆盖蒙版性能修正：新预览的蒙版尺寸和所有字节相等时，复用原缓冲与 GPU 上传版本；不同网格或像素仍更新。10 项渐变预览回归和完整 729 项 Rust 回归通过。此修正后的 WASM、浏览器、原生包与完整性能协议待重新验证，旧包的通过记录继续绑定原提交。
- 覆盖蒙版上传修正后的原完整验收：GPU 4/4 和原生 25 次对话框及独立读取器均通过；CPU 26/29，保留引擎加载超过原 5 秒、普通渐变 52ms 对 <50ms、100MP Fill 安装 485ms 对 <450ms 三个失败。200ms 会话与 500ms 单屏/会话的所有记录均满足条件；原 755/754 项输入前后核对一致。覆盖蒙版拖动 30/44ms 已达标。继续减少普通渐变的冗余清零/复制、纯色安装展开和加载时的整块缓冲，原断言及预算保持不变；整体验收仍未完成。
- 后续优化已完成源码验证：完整 Rust 729 项、单位测试 306 项（44 文件，单 worker、原 5 秒预算）通过；新 release WASM 的 108 项接口校验通过，真实浏览器验证 4 MiB 跨块的四组 RGBA 全字节一致、不完整数据不改项目/历史、流式和兼容 MIME 加载及损坏输入拒绝共 5 项通过，另 16 项功能回归通过。新输入将冻结到下一轮原性能协议；当前性能与 Mac 对比仍保持未完成。
- 2026-10-12 最新独立验收（`d06b238`）：GPU 原 4 项、生产包 25 次实际 Windows 对话框与独立导出读取器均通过；CPU 原 29 项为 23 通过、6 失败。失败为三次引擎就绪超过原 5 秒、任务期间最长帧间隔 151ms 对 <100ms、变换后的帧 40ms 对 <33ms、效果任务抢占派发 569ms 对 <400ms。755 项 CPU 输入和 755 项 GPU 输入前后一致；CPU 的 2242 条 200ms 会话记录、1027 条 500ms 显示/会话记录全部单屏解锁。普通渐变 24/100MP 拖动已降至 36/39ms，但不能代替整套验收通过。保留全部失败，继续定位启动和响应开销。
- Mac 1.4.9 回传 `Mac149-return-20261012-202235` 已收到并独立读取：固定提交、布局和 32 项输入记录相符，28 组 premultiplied RGBA 中 27 组逐字节一致，Scanlines Glow 最大 RGB 差异 49、alpha 一致，须修正。实际应用的两组图层效果导出最大通道差异 1；Cancel/Apply/Undo/Redo、Last Filter Undo、Navigator 的已导出像素关系成立。PNG/JPEG 为 300×150，PDF 为 72×36pt，PDF 嵌入 RGB 与 PNG 完全一致；保存项目为 600×300、300 DPI、两层。操作记录和截图保留；有差异的 Glow 与 CPU 性能未解决前仍不宣称整体完成。
- 启动分段诊断（独立样本，不能替代原失败）确认首次画布创建存在 4.5～5.3 秒主线程长任务；细分为多个串行 GPU 程序链接等待，WASM 流式编译约 0.25～0.54 秒、实例化约 13～15ms。下一步先批量提交同一套 GPU 程序、复用顶点着色器，再检查链接状态与填充 uniform；不把编译延迟转移到首次文档绘制，不更改 GLSL 数值算法，并清理成功/失败路径中的着色器资源。Glow 另行核对无限边缘与缩小中间平面，不能用较小的近似差异替代原逐字节验收。
- 2026-10-13 已完成 `0b34aca` 的生产包验收：308 项单位测试、三项 TypeScript 检查、343 项完整浏览器功能测试通过，原 33 项硬件 opt-in 独立运行。`20261012-2233` 包的 11 项 ZIP CRC/文件哈希、源码与 WASM 来源核对通过；生产 WebView2 的 25 次 Windows 原生对话框、独立 Pillow/PyMuPDF 导出读取及真实 Canon RAW 六组传感器解码通过。原生过程的 298 条环境记录全部单屏解锁。原 4 项 GPU 性能全部通过、755 项输入前后一致，141 条独立环境记录全部单屏解锁。源码、包和结果继续绑定各自标记，不覆盖旧包。
- 同一代码的原完整 CPU 协议为 28/29，唯一断言失败是 100MP Fill 最长帧间隔 156ms（原预算 <100ms）。失败发生在锁屏之前；完整运行后段锁屏，环境门槛也未通过。按原顺序的私有诊断在单屏解锁下复现 121ms 间隔，已记录的主线程 JavaScript 约 10ms，另有约 106ms 原生线程 CPU 耗时尚未精确归属；后续带采样器和原生任务轨迹的样本没有复现该间隔，但记录到计时区间外的浏览器 cppgc 清理任务约 153ms。这只是资源生命周期调查线索，不能替代原完整协议或认定已修复；没有据此修改数组分配、放宽预算或增加自动重试。
- 本轮汇总为 `acceptance-status-20261013.json`，状态仍为 `INCOMPLETE`。剩余为 CPU 帧间隔和 Scanlines Glow。Mac 新增步骤仅运行 `Mac149-glow-diagnostic-v2.zip` 中的自动命令，回传缩小/模糊/放大的中间平面；先前手工操作不需要重做。尚未定稿 0.10.0，也未发布本次增量。
- 后续先修正已确认的关闭行为：没有活动文档时，清画面同时释放文档的 GPU 纹理、蒙版、调整表和离屏缓冲，丢弃已关闭文档的效果缓存及放置记录；保留固定纹理和编译程序以便再次打开。增加真实 GL 句柄计数与清空后重新绘制回归。此处只是缩短已关闭项目的资源寿命；不能据计时外的 cppgc 样本认定它就是帧间隔原因，原硬件门槛继续独立验收。
- 上述空画布资源释放修改已通过三项 TypeScript 检查、前端构建及 21 项真实浏览器回归。关闭后仅保留初始固定纹理、文档 FBO 数归零，重新创建画布的程序创建计数不增加、无 GL 错误；画面缓存、调整层与 CPU 对照、关闭取消/保存/重开及多标签均通过。本次修改后的原完整性能和生产包验证另行绑定新提交，不能沿用 `0b34aca` 的包记录。
- `fbf5be9` 原完整性能已保留：CPU 26/29、GPU 4/4，755 项输入分别前后一致；CPU 826 条、GPU 115 条独立记录全部单屏解锁。CPU 实际断言失败为 100MP Levels 安装 550ms 对 <450ms、渐变输入复制 477ms 对 <450ms、覆盖蒙版渐变拖动 50ms 对 <50ms；同一 F1 日志还有 Fill 帧间隔 108ms 对 <100ms。不能把此前 257ms Fill 安装数值扩展为整个 F1 通过。
- 本次完整单位测试 307/308，未变的调色板用例超过原 5 秒。私有对照中编码/解码各约 6～17ms，而通用 `toEqual` 对大型 Uint8Array 逐字节迭代后又分配每字节的属性条目，约占 97.5% 测量成本。只替换该测试的字节断言：仍检查 Uint8Array 类型、完整长度、每个 RGBA 字节、源输入不变及两个调色板深度，输入和原 5000ms 预算不变；不修改生产编码算法或原性能用例。
- Mac 新回传 `Mac149-glow-kernel-20261013-074734` 的 104 个中间平面已核对，原基线和 drawn-spacing7 全字节相符。使用实际光晕 RGBA8 平面可精确重建 5 个 spacing 的 Glow 输出，叠加公式无需改动。缩小/放大的相位、夹取及 half 量化得到进一步约束，Gaussian 的跨样本精确核仍未确定；小于一字节的拟合差异也不替代原严格比较。
- 等价完整字节断言修改后，45 个文件的 308 项单位测试全部通过，原 5000ms 预算保留；单位测试 TypeScript 检查也通过。提交 `56985d5` 只调整测试与需求记录，相对 `fbf5be9` 的运行时代码不变，不通过重跑原性能用例替换此前失败。
- `56985d5` 的 `20261013-0815` 生产包已完成 11 项 ZIP CRC/文件哈希、源码与 WASM 来源核对，真实 Canon RAW 六组传感器解码也通过。该包的原生 25 次交互与独立导出读取器已准备，但尚未运行：准备结束时系统报告单屏锁定，等待物理桌面解锁；不能沿用旧包的原生通过记录。
- 对中间平面的有限分析仍未得到跨样本精确 Gaussian；half 截断与像素面积核是诊断线索，不能作为已验证的生产修复。已准备 `Mac149-glow-direct-kernel-v1.zip`，由 Mac 自动执行 `MAC-GLOW-DIRECT.command`，直接比较没有缩放的 Gaussian 与两种工作精度，回传 `Mac149-glow-direct-日期-时间`。该包当前只完成本地内容与 ZIP 校验，尚未在 Mac 执行；此前手工项目操作无需重做。
- `56985d5` 的 `20261013-0815` 包后续完成原 25 次生产原生交互及独立 Pillow/PyMuPDF 导出读取，425 条独立环境采样全部单屏解锁；生产测试 API 关闭。当前包的原生门槛已通过，后续源码修改仍须重新绑定验证，不能沿用该包的结果。
- 下一步只消除已确认的覆盖蒙版渐变冗余：当源蒙版缓冲、修订号、图层/画布几何、选区及预览网格均相同，仅改变渐变线时，复用上次完整携带蒙版，不再先重采样再比较全字节。缓存持有源 Arc，避免地址复用误命中；改变蒙版、尺寸、变换、选区或拖动/定稿网格时保持原计算。补充与独立无缓存计算的完整像素对照，原 50ms 预算及完整性能协议不变。安装与输入复制尚无已证实的可删除整层冗余，继续用分段诊断定位。
- 上述携带蒙版复用已实施并通过 701 项完整原生引擎测试、11 项 WASM 边界测试、三项 TypeScript 检查和前端构建；10 项原生忽略项保持原状态。新增检查逐字节对照独立无缓存计算，覆盖蒙版内容/修订号、图层尺寸/修订号/变换、画布、蒙版放置/启用/链接、选区修订及拖动/定稿网格变化；现有取消恢复与实际提交采样检查也通过。重新构建的 WASM 校验 108 个导出调用，SHA256 为 `910b268301cbd83a397113987df19d76b5277f64e5d923c364cc2c949c99724e`。完整浏览器与原性能协议仍须绑定新源码/资产，不能据源测试宣布性能失败已解决。

- `3926872` 后续通过完整浏览器功能回归 343 项，原 33 项硬件开关用例在功能层保持跳过并独立执行。原完整 CPU 协议为 28/29，唯一实际失败是 100MP Fill 最长帧间隔 107ms 对 <100ms；原 4 项 GPU 均通过。755 项冻结输入分别前后一致，CPU 815 条、GPU 115 条独立记录全部单屏解锁。原断言、预算、单 worker 和零重试均保留，不重试替换失败。
- 同一提交的 `20261013-2100` 生产包通过 11 项 ZIP 完整性/源码与 WASM 来源检查、六组真实 Canon RAW 传感器显影、原 25 次实际原生交互及独立 Pillow/PyMuPDF 导出读取；264 条独立采样全部单屏解锁，生产测试 API 关闭。当前汇总为 `acceptance-status-carried-mask-final-20261013.json`，状态仍为 `INCOMPLETE`。
- Mac 回传 `Mac149-glow-direct-20261013-202111` 只包含启动失败记录：Windows CRLF 导致 `pipefail` 报错，未执行内核诊断。v1/v2 包与失败记录保留；v2 的校验清单仍引用旧启动文件哈希，本地检查发现后由最终 `Mac149-glow-direct-kernel-v3.zip` 替代。v3 使用 LF、Unix 可执行权限并同步校验清单，实际 Bash 语法和全部 16 项启动哈希已通过，上游算法/参数/基线未改；Mac 执行仍待回传。
- 剩余 Fill 的私有分段诊断曾记录 109ms 间隔，最大单次 staged 调用 8.9ms；实际 Levels 返回 palette runs，且保留预览仅为缩小网格，不能按全分辨率预览直接接管。后续 Worker 收发/销毁、缓冲转移、WASM、原生任务及 CPU 采样对齐诊断使用较旧的冻结 `fbf5be9` 资产，两项通过、199 条采样全单屏解锁，但 Fill 间隔 24ms，未复现。其采样开销和旧运行时使它不能代替当前原完整 107ms 失败；不据此推断根因或作猜测性修复。整体剩余仍是严格 Glow 像素差异与原 Fill 帧间隔。
- 新回传 `Mac149-glow-direct-20261013-212859` 的 16 项启动哈希全部通过，但 `main.swift:61` 在生成 flat-113 输入时发生 Swift 泛型/数字表达式类型检查超时，基线与 Gaussian 诊断均未执行，不能记为 Mac 像素验收通过。下一步在独立修正版中将四个归一化通道写成显式 Float 赋值，保留 43/255、79/255、97/255、113/255 的参数及全部上游/基线；重新核对启动哈希和脚本，再由真实 Mac 编译执行。原返回及旧包保留。
- 上述诊断修正版 `Mac149-glow-direct-kernel-v4.zip` 已生成：四个 flat-113 通道采用显式 Float 赋值，九组 33×33 Float32 输入新增独立整平面字节检查。Bash 语法、全部 25 项启动文件哈希、ZIP CRC/文件内容/可执行权限及归一化通道的独立字节检查均通过；完整 macOS Swift 编译和运行仍标记 `NOT_RUN`，等待真实 Mac。汇总更新为 `acceptance-status-glow-direct-212859-20261013.json`，整体仍未完成，原 Windows 性能失败及所有旧包/回传保留。
- `Mac149-glow-direct-20261013-222947` 的 v4 诊断已真实编译并完整运行：启动输入哈希、九组独立 Float32 输入、原 Glow32 基线及 213 个输出平面均通过本机复查。此前编译失败问题已在真实 Mac 验证修正，目前不要求重复用户操作。直接数据排除普通对称整数核与可直接移植的固定经验脉冲核；RGBAf 工作模式仍保留部分半精度行为，但并非所有输出均与默认模式相同。新的数学模型继续在既有不同 sigma、输入、缩放网格上交叉核对，尚未据拟合改写产品算法或严格像素门槛。汇总为 `acceptance-status-glow-direct-success-20261014.json`，整体仍为 `INCOMPLETE`；原 107ms Fill 间隔失败保持独立。
- 既有数据的有界交叉检查仍未得到精确 Gaussian：默认结果等于 Float32 工作结果的 half RTZ 对 83/85 组成立（全部 45 组 Float32 输入成立），但固定线性卷积及不同权重/坐标/半精度积累模型不能同时匹配脉冲、台阶与恒色。下一步停止扩大拟合，复用 Phase 7 已在 Mac 成功的公开 CIKernel 参数观察器，追加官方 `CI_PRINT_TREE=7` 原始/优化/执行图记录。以本次 213 个平面和原五组 Glow 输出逐字节核对观察前后；若内置 Gaussian 不经过公开入口，明确记录不可取得，不当作参数通过。新包必须先完成本地完整哈希、换行、ZIP 和脚本语法校验，实际 macOS 编译/运行单独验证。
- 上述 `Mac149-glow-arguments-v1.zip` 已准备并通过 245 项实际 Bash 启动哈希、脚本语法、ZIP CRC/内容与 Unix 执行权限检查；只读审核独立核算 213+5=218 项对照，未发现明确的类型/编译/计数缺陷。完整 macOS 编译和内置 Gaussian 参数可见性仍标记 `NOT_RUN`，待真实 Mac。该包复用 Phase 7 已运行的公开 CIKernel 观察器，新增有界 NSData 参数序列化；未改变上游算法、九组输入、sigma、原输出或 Windows 产品实现。汇总为 `acceptance-status-glow-arguments-prepared-20261014.json`，状态保持未完成。
- 回传 `Mac149-glow-arguments-20261014-110302` 的 245 项启动哈希通过，但 Swift 在新增局部变量 `argumentBytes` 处将 Objective-C 返回值推断为 `Data?`，后续 write/JSONSerialization 两处未解包导致编译失败，218 项对照和参数记录均未执行。此前静态审核没有识别该桥接问题，不能替代实际编译。下一步仅为桥接声明补明确 nullable 注解，并在 Swift 用 guard-let 取得非空 Data；不使用强制解包或空数据兜底，保持原观察器、上游、参数、参考输出及所有门槛。失败回传和 v1 保留，独立 v2 包重新校验。
- 修正版 `Mac149-glow-arguments-v2.zip` 已生成，变更仅为 Swift 非空 guard-let、桥接 nullable 注解、中文文档及重新计算的校验记录。启动脚本、观察器实现、上游及 218 项参考输出全字节不变；245 项实际启动哈希、Bash 语法、ZIP CRC/内容、LF 和执行权限均通过。macOS 编译/执行仍是 `NOT_RUN`，不能据本机检查宣称编译已过。汇总为 `acceptance-status-glow-arguments-v2-prepared-20261014.json`，原失败和所有既有门槛保留。
- 后续真实回传 `Mac149-glow-arguments-20261014-144235` 已完成编译和执行。本机独立核对 245 项启动哈希、218 个原参考输出、九组 Float32 输入及 218 平面的尺寸/有限值全部通过；观察器自测前后像素一致，四个 Double/Float 参数位模式也一致。记录 806 次公开入口、630 次 Gaussian 名称入口，证据为 `glow-arguments-144235-return-review-20261014.json`。回传内的 receipt 是准备时的输入清单，其旧 `NOT_RUN` 标签不代表本次执行结果；实际通过由 run.log、输出和独立复核证明。
- 本次初始图含 `_gaussianReduce4`、小核和 `_cubicUpsample10` 的候选分支，但优化图和执行图均选择 `CIBlurProcessor`，未执行图上那组公开候选核。不能仅凭捕获名称和参数把 Reduce4/cubic 模型认定为真实算法。下一步定位实际 processor 的公开边界和可观测数值；只有跨原始直接核及完整 Glow 样本通过严格对照后才修改生产实现。原 100MP Fill 的 107ms 帧间隔失败仍独立保留，1.4.9 增量保持未完成。
- 已准备独立 `Mac149-glow-mps-v2.zip`：在原路径启动前记录公开 MPS constructor/encode 的可观察调用，原 218 项像素守卫保持；原输出完成后自测参数位模式并恢复观察器，再重放九组输入×五个 sigma×三组纹理精度，共 135 项独立诊断。保留实际上传字节，逐项记录格式不支持、Objective-C encode 异常或 GPU 失败，分别报告 renderedPlanes/failedCases；构造调用不认定为执行，零事件不认定为未使用。v1 的格式失败中断风险经静态审核后在 v2 补齐，旧包保留。v2 的 248 项实际启动哈希、Bash 语法、ZIP 内容/CRC、LF 和执行权限通过；完整 macOS 编译和重放仍为 `NOT_RUN`，未更改 Windows 产品实现或门槛。最新汇总为 `acceptance-status-glow-mps-v2-prepared-20261014.json`。
- Windows 当前 `3926872` 冻结运行时的低干扰定向观察已完成：两个 F1 用例及原断言通过，没有使用 CPU 采样器或 CDP tracing；浏览器实际支持 Long Animation Frame/LongTask，页面可见性记录全为 visible。100MP Fill 帧间隔 21ms、安装 CPU 246ms、首帧 10ms；174 条物理环境采样全部单屏解锁，755 项冻结输入前后一致。这是新的归因诊断，不是原完整协议重试，也不替代原 107ms 失败；冷启动长帧在计时区间之外，不能认定为其原因。证据为 `cpu-current-frame-attribution-summary-20261014.json`。当前未发现 MPS 新回传，不继续猜测性性能改动；取得 Glow 实际数据并实施新代码后，再绑定原完整包/性能门槛。最新状态为 `acceptance-status-current-frame-attribution-20261014.json`，保持未完成。
- 新回传 `Mac149-glow-mps-20261014-175354` 已真实编译执行并通过独立复核：248 项启动哈希、218 原输出、九组输入、18 个纹理上传缓冲和 135 项重放全部核对；没有格式失败。原路径实际记录 176 次 MPSImageGaussianBlur 构造及 176 次 encode，确认实际渲染经过该公开 MPS 编码入口。默认 half 重放仅 27/45 原平面相同；测试图解析对称关系可解释 41/45，但不是任意图片的生产算法。MPS Float32 最后做 half RTZ 不足以重建默认路径，真实内部 half 行为仍需证明。下一判别补齐原 kernel 的 options、offset、clipRect、edgeMode，并在同线程 Gaussian encode 范围观察公开 Metal 常量/纹理元数据/dispatch；不读 private GPU 缓冲、不等待或改变原 command buffer。以捕获配置做独立重放，继续保留 218 原输出及现有 135 重放的字节守卫。可见常量未证明数学角色前不作为 taps；完整 Mac 编译、常量可见性和产品一致性分开验收。复核为 `glow-mps-175354-return-review-20261014.json`，生产实现与原 107ms 失败未改。
- 上述独立 `Mac149-glow-metal-v2.zip` 已准备：353 项真实旧输出全字节保持，386 项实际 Bash 启动哈希、脚本语法、ZIP 内容/CRC、LF 及 Unix 执行权限通过。静态复查补齐三分量常量步长未验证时不抓取、非零/部分 clip 不读取未写输出，并增加退出时的观察器恢复；v1 保留。90 项独立配置重放明确使用测试输入边缘复制假设，另列 outputFlipY，未宣称读到真实 CI 输入纹理。完整 macOS 编译/执行和 Metal 常量可见性仍是 `NOT_RUN`。最新汇总为 `acceptance-status-mps-success-metal-prepared-20261014.json`，1.4.9 增量未完成。
- 新回传 `Mac149-glow-metal-20261014-195526` 已真实编译执行并通过独立复核：386 项启动哈希、385 个收据文件、218 原 CI 输出、135 原 MPS 输出、九组输入和 90 个配置重放上传缓冲均核对。复制捕获的 options=2、edgeMode=1、offset/完整 clip、纹理格式和尺寸后，默认与 Float32 工作上下文各 45/45 重放原输出逐字节相同；原方向正确，翻转方向各仅 35/45 相同。原路径仍为 176 次 Gaussian 构造及 176 次 encode，新增 5,284 条公开 Metal 事件、1,060 条有界参数事件，没有事件丢失，16 字节独立自测精确；八项公开入口的类型编码/安装记录完整。
- 该回传实际观察到 D2F6、F11/F13、U2P3F3 的纵横六阶段、纹理尺寸和 128 字节参数；pipeline label 不等同已证明的 shader 函数名，未使用 private GPU 缓冲。后续先在独立软件模型验证六抽头缩小、对称卷积、放大相位及 half 运算顺序，再从数学规则推导 sigma 参数并验证完整五组 Glow；禁止把观测的五组系数或 fixture 对称变换写成产品查表。配置重放精确只证明这些输入的状态/布局假设，不认定便携数学或产品 Glow 已通过。复核为 `glow-metal-195526-return-review-20261014.json`；最新状态为 `acceptance-status-metal-success-20261014.json`。Windows 原 100MP Fill 107ms 失败保留，增量继续未完成。
- 六阶段元数据的 176 条完整链均通过纹理格式/尺寸、packed 输出大小和 dispatch 网格检查。独立软件候选支持 quarter-phase 放大再 fine-grid 三抽头滤波，但普通点/CDF Gaussian 和固定方差修正不能生成全部捕获系数；更强的因式分解拟合仍不精确。六阶段算术的有界白场判别留下 74 个代表路径，RGB 仅保留代表路径、最佳仍有 20/120 常量通道不同，不能当作完整 RGB 穷举或产品通过。候选失败和参数角色未证明的字段继续保留。
- 新 `Mac149-glow-stages-v1.zip` 先保护 218 CI、135 MPS、90 已精确配置重放输出及上传字节，再只在独立、自有 MPS command 中插入每级 snapshot 复制和纹理内存屏障，恢复原 pipeline/两张纹理/128B 参数。原控制的 command 不插入复制或读中间 GPU 输出。Snapshot 的 Float16 复制须先通过全部 31,744 个非负有限位模式自测，独立重放的最终输出须仍精确；未支持/失败不得计通过。新增 73 个 sigma（4～5，步长 1/64，另八个判别点）分别比较有/无 snapshot 的自有输出，共 163 项新诊断。570 项启动哈希、Bash 语法、ZIP 内容/CRC、LF/执行权限和静态边界检查通过；完整 Mac 编译、复制自测和中间输出可用性仍 `NOT_RUN`。最新状态为 `acceptance-status-stages-prepared-20261014.json`；产品实现及原性能门槛不变。
- 真实回传 `Mac149-glow-stages-20261014-205225` 已独立复核：570 启动哈希、569 收据文件、443 个旧输出、90 上传缓冲、73 新上传重建、163 个有/无 snapshot 最终结果及 978 中间平面全部通过；复制自测实际运行通过（原始自测缓冲未另存）。本轮 specialization constant 事件为两条，前轮为四条，编译时观察数量不属于像素守卫，旧输出保持精确。D2/FIR 的双线性采样有 1/256 相位、half 中点向上舍入、F32 FMA 与 half RTZ store；U2 为 partition-unity 三支持点 cubic，系数分别乘加舍入、中心乘法再左右 FMA。各组件 326/326 精确，从一次上传串联预测六级的 163/163 最终结果和 978/978 中间平面也精确；独立反推工作矩形同样 163/163，不依赖捕获的阶段 ROI。
- 新 73 点表明实际参数按约 0.05 的 σ 区间保持不变；163 项记录共 21 个数值参数 family，连续 Gaussian 系数生成的原假设不成立。归纳的 `ceil(Float32(sigma)*20−0.5)` 分组规则覆盖当前全部记录，31 种合法 Line Spacing 需要其中十组，但仅 16 种间距的具体 σ 直接出现于现有采样；尚不据此写入产品查表或宣称所有边界已证明。外部缩小 9 个 F32/9 个 RGBA8 平面、由实际缩小输入计算的外部 Gaussian 九个平面已精确。完整源图软件候选的五组 Glow 中三组精确、另外两组总共三个颜色字节各差一级；原严格像素门槛不放宽。
- 下一判别只补完整组合路径的真实 MPS 输入与六级输出，并覆盖全部 2～32 整数 Line Spacing 的原代码结果；同时对已知四 texel/相位进行独立 half/float Metal 双线性采样。保持已有 443 输出及 978 阶段/163 最终输出守卫；新观察仅在原控制结束后的自有重放及同线程 Gaussian encode 范围，不能把独立渲染 halo 当作组合路径实际输入。先判明输入差异或采样精度，再实施 Scanlines 专用更改。其它效果的 blur 与原 100MP Fill 107ms 失败继续保留。
- 上述 `Mac149-glow-composite-v1.zip` 已生成：1,716 项实际 Bash 启动哈希、脚本语法、ZIP CRC/内容/Unix 权限及控制字节检查通过，31 种间距及 99,306 项独立 sampler 输入已准备。完整 Mac 编译、组合路径输入捕获及 sampler 输出仍为 `NOT_RUN`；中文步骤随包交付。当前不改生产源码，最新汇总 `acceptance-status-stages-success-composite-prepared-20261015.json` 保持 `INCOMPLETE`。
- 真实回传 `Mac149-glow-composite-20261015-075932` 独立核对 1,716 启动哈希、1,715 收据文件、443 原输出、978 阶段及 163 最终输出通过；新增 163 个输入 snapshot 与上传相同，31 种合法整数间距的组合结果与原入口精确。99,306 项 half/float 独立采样输出彼此及旧组合目标逐字节相同，排除旧独立 halo 输入不同作为这些采样差异的原因。实际输入运行六级软件模型的新增 186 平面精确；剩余在外部 reduction 的 LOD/二维 sampler 精度，不能以普通 bilinear 近似代替严格结果。
- 31 个有限合法间距的实际构造/encode sigma 位模式及全部阶段参数已直接核对，可采用完整、直接按标准化间距索引的十组算法参数目录作为受限实现方案；这是输入无关的 Float32 内核参数，不是像素答案、五点拟合或推断 sigma 分组。只适用于 Scanlines 间距 2～32，不能称任意 sigma/其它效果/其它设备通用；完整图像与尺寸验证仍必需。
- 新增 31 项揭示合成末步的独立数值差异：使用真实 Mac bloom 时，Windows 原 C 独立乘加在间距 10/15 产生四个错误字节，显式 `fmaf(room,1-expf(...),v)` 则全部 31 项精确。实施范围先补该处明确 FMA，并给 WASM 提供 Rust `f32::mul_add` 边界及真实 Mac 单像素回归，避免编译器平台默认收缩不同。该修正不代表 Gaussian/缩放或完整产品 Glow 已通过；原性能失败及所有严格像素门槛保持。
- FMA 修正已实施，回归先失败后通过；引擎库 22、Mac149 集成 5、WASM 原生边界 11 项通过。实际生产 release WASM 构建/导出检查通过；调用该二进制的 `phase7_mul_addf`/`compositor_exp` 并独立重放周围 Float32 标量运算，31 组使用真实 Mac bloom 的合成全部逐字节相同。这是实际 WASM 数学边界验证，不能冒充完整 C 调用图或源图到 Glow 验收。下一包 `Mac149-glow-sampler-basis-v1.zip` 已通过 12 项实际 Bash 哈希、语法、ZIP 及输入字节检查：保留原 99,306 项 sampler 严格守卫，新增 19 组已知输入的全部 Q8 相位和三种尺寸/72 项缩小平面，Mac 编译运行仍 `NOT_RUN`。不重跑完整 Glow，不放宽原断言。
- `Mac149-glow-sampler-basis-20261015-153432` 已实际编译运行并独立复核：12 启动哈希、11 收据文件、输入拼接、19 组全部 Q8 相位和两种 sampler 的 1,354,237 项完整结果一致；其中原 99,306 项严格对照不变。72 个缩小平面有限有效，三个未缩小控制与原 RGBA8/无限边缘完全一致。所有 23 种合法整数 shrink 在三种尺寸上 69/69 缩小平面逐字节相同；shrink2 奇数高度需直接在原 RGBA8 上按相位采样，不能先固定2×2 mip。采用完整有限 LOD 状态目录，不能把实测区间任取偏置当 Apple 通用阈值。源像素推导 Gaussian 最终纹理 31/31 精确，但实际输入仍有四个 half 元素差异，186 阶段中一元素差异下一层消失，不冒充全链全阶段精确。
- 新基向量/指数差数据否定了此前共享指数截断等近似；普通双线性在19组中14组精确，五个尾数组共差7,800分量。IEEE F32/FMA/不同累积顺序等有界判别仍未给出精确方案，停止扩大该类拟合。下一判别固定全部已知 texel/相位和已回传的完整 RGBA16F 输出，仅切换 RGBA16F/R16F/RGBA32F/R32F 纹理格式；32-bit 过滤以公开设备能力为准，未支持不计通过。补全部正有限 half→Float CPU 转换自测，记录设备/能力，区分格式路径与通用滤波计算。新 sampler 数值未知，不据近似改生产实现或门槛。
- 额外 Windows 端到端检查发现绘线本身仍有两处单字节差异：31 种间距的真实 native/WASM Glow0 路径均为29/31，失败为 spacing24/29。四个独立 C 对照定位为 beam 的 `0.29+0.71*sqrt(level)` 需显式 FMA；仅改灰度加权无效，单独 beam FMA 即31/31精确。先新增可生成的1列24/29像素回归复现旧失败，再只修改该表达式复用既有 Rust FMA 边界，避免把 Mac 已绘制像素当 Windows 原始绘制证明。其它灰度/绘线表达式不据猜测改动；完整 Glow 和原性能门槛继续独立。
- beam 修正已实施：回归先失败后通过，引擎库23项、Mac149集成5项、WASM原生边界11项通过。真实 native 及生产 WASM 的31种间距 Glow0 绘制均与Mac逐字节相同，原四组非Glow Scanlines配方也各4/4相同；实际新WASM数学边界的31组合成继续精确。release WASM构建及108项glue调用导出检查通过，二进制16,987,544 bytes、SHA256 `e632bfc78b25ac6a8fa4e164aa19c9c0c5c0f525e65d62ba34c5911d5e280fba`。最终Gaussian/缩放模型尚未集成，新portable/完整验收仍待完成。`Mac149-glow-format-matrix-v1.zip` 已通过9项实际Bash哈希、语法、ZIP和输入守卫校验，完整1,354,237项输出及31,744转换控制已冻结；Mac新格式结果仍 `NOT_RUN`。
- 后续先把已验证的 Gaussian 六阶段、反推工作矩形和受限间距参数移植为 Rust 组件，使用独立 native/WASM 诊断逐级核对真实回传。half 采样中点、RTZ store 和 F32 FMA 须显式实现，尺寸/分配须有界并拒绝无效参数。最终二维 sampler 尚未证明，组件暂由测试与诊断编译，取得精确规则前不接入产品 Glow，也不把组件通过记为完整产品通过；缩小部分仅在保留其四个已知 half 差异的前提下另行移植。原性能协议、旧失败和其它效果 blur 均保留。
- 上述 Rust Gaussian 组件及完整31间距/十组参数目录已落地，先保持 test/diagnostic-only：真实 native release 和独立优化 WASM 各1,358/1,358项逐字节一致，其中六级连续预测1,164个实际平面全部精确，自行反推最小工作矩形的194个最终平面全部精确。两者各覆盖原163组及新增31组；源码/输入哈希和独立二进制均记录。新增六项回归包含实际常量场 store 损失、half 中点反例、全31,744位模式及输入/参数/尺寸/中止边界，引擎库总29项及原Mac149集成5项通过。该结果不含原始RGBA8到最终Glow的产品调用，不沿用旧完整包结果；等待格式对照回传后继续缩小/二维采样集成。
- 新回传 `Mac149-glow-format-matrix-20261015-234827` 已实际编译执行，9项启动哈希、8个收据文件、全部1,354,237个旧采样与31,744个CPU转换控制通过；四种格式均支持。R/RGBA同精度红通道相同，32位纹理符合精确加权后正中点向上Float32舍入，16位纹理另走有限精度路径。新“输入指数＋合并二维权重指数、归一化前对齐、17位乘积网格截断、求和后half中点向上”的统一规则全部1,354,237项精确，独立整数实现亦须复核后集成。后续只替换Scanlines的缩小/Gaussian/放大链，保留其它效果blur；先验证原始像素到实际MPS输入、六级和bloom，再验证完整native/真实生产WASM及原Mac输出，不放宽像素/性能门槛。预滤波需保留无限原始边缘语义并限制工作内存，避免按大halo复制全尺寸源图。
- 新采样规则的浮点与整数实现、实际native Rust及独立WASM均为1,354,237/1,354,237精确。已接入Scanlines正式路径：native完整原图→Glow的31个合法间距均与Mac原始入口输出相同；真实生产WasmEngine的ApplyFilter为31间距＋原五配方36/36精确，Undo/Redo像素亦精确。生产WASM17,001,858 bytes、SHA256 `c309d1a5f6aa008a902b7d8f3edf9d0e103a41475eee20d56bc7b3b5d4dd7856`，108项glue校验通过。缩小继续采用独立69/69平面已验证的运算模型；直接把最终纹理sampler套入缩小会使控制降为64/69，失败保留，不据此推断未知CI内部格式。组合MPS输入原四个half差异仍存在，31个Gaussian最终纹理/bloom/完整Glow均精确，不冒充内部全阶段精确。原性能失败保留，完整回归、原29＋4硬件协议和新生产包仍需绑定当前源码验收。

- 2026-10-19：black-zero 生产 WASM 的108项接口、130组渐变和36组Mac Glow对照通过，同源码完整Rust为748通过、10项原有忽略。该前端候选的生产原生25次真实对话框、独立导出读取器及GPU4项通过，全程330/140个单屏解锁采样均通过；此前锁屏结果保留，不覆盖旧记录。
- 已确认并修正开发模式StrictMode重复创建/销毁渲染器：保留同步effect重放中的同一资源，引擎替换先释放旧资源，真实卸载在下一帧前释放一次。真实组件回归先红后绿，完整46文件/310项单测、三组TypeScript检查、完整浏览器344通过/33原有跳过/0失败通过，816项输入前后相同，原5000ms断言和零重试不变。新前端须另行绑定新生产包/原生/GPU验收，不直接沿用上一前端结果。
- CPU完整协议仍保留27/29：100MP Levels安装470ms对<450ms；同一日志的100MP Fill帧间隔130ms对<100ms（先在前一断言失败，未执行到此断言）；24MP输入W后的帧34ms对<33ms。独立诊断不替代验收，尚未据波动放宽预算或反复重跑。整体验收及0.10.0发布继续保持未完成。

- 2026-10-19 17:45起的新前端完整硬件验收已完成：生产包原生25次真实对话框、独立读取器及292个单屏解锁采样通过；原GPU4项及136个环境采样通过。原CPU29项为26通过、3失败、0跳过/重试/flaky，816项输入前后相同，876个独立采样全程单屏解锁。失败为100MP Levels安装503ms（<450ms）、24MP选区渐变补丁57ms（<50ms）、100MP渐变安装539ms（<450ms）；F1同时记录Fill帧间隔113ms（<100ms，前一断言已失败），输入W为13/25ms、T9-6带遮罩拖动28/18ms均通过。旧失败全部保留，不把本轮失败归因于锁屏。隔离word写入的24组实际WASM安装/Undo/Redo/越界守卫已精确，端到端AB/BA已准备，首次在锁屏前置检查退出、未执行用例；未据此集成或声称性能通过。最新验收为acceptance-status-renderer-lifetime-hardware-20261019-1745.json，整体未完成、未发布。

- 2026-10-19 解锁后完成安装word填充私有AB/BA：原实现100MP Levels307/347ms、Fill247/249ms，候选Levels552/421ms、Fill314/434ms，两轮候选均退化，决定不合入；4轮完整执行、3通过1超限、0重试，406个采样全部单屏解锁，输入前后相同。随后原选区渐变补丁体的12次函数采样完成，64个环境采样通过；与生产WASM执行相关段逐字节相同的带名副本将热点准确对应paint_grid_channels（24MP WASM自身57样本中56，100MP43样本全部），未据此增加覆盖缓存。采样器启动停止计入原窗口，诊断75/71ms失败不作为验收；原CPU26/29及四个超限503/113/57/539ms继续保留。生产源码/WASM未因这两个诊断改变，未发布。诊断汇总：UNLOCKED-DIAGNOSTICS-20261019.md。

- 2026-10-19后续渐变循环诊断：保留函数边界、全RGBA Float64 SIMD、限定半透明RGBA SIMD三个隔离候选各通过278组原实现/独立标量全字节与命中范围比较，但均有退化或关键24MP选区补丁收益不足（限定版15.842→15.771ms，约0.07ms），全部不合入，产品源码/WASM未变。完整原29项执行顺序的连续采样诊断为27通过2超限，817个单屏解锁采样通过；补丁37/36ms未重现旧57ms，实际超限为100MP Fill帧间隔112ms与100MP普通渐变1:1拖动54ms。采样不替代正式26/29验收，不将旧超限归因于本轮未重现的GC或编译事件。下一步原前两项的F1专门回收/帧轨迹已准备并验证收集数2，桌面再次锁定前置退出，尚未执行。记录GRADIENT-CANDIDATES-20261019.md，未提交/推送/发布。

- 2026-10-20 08:27开始，解锁单屏后完成已准备的原前两项F1专门轨迹：2通过、0失败/跳过/重试，200个独立环境采样全部单屏解锁，816项输入前后相同。24MP Levels/Fill与100MP Levels/Fill帧间隔18/21/27/19ms，安装CPU101/31/399/331ms；本次未重现旧112/113ms长帧。当前轨迹捕获主线程CppGC.IncrementalSweep114.395ms（自然回收forced=false），其中SweepFinalizeSweptPages113.930ms；时钟区间证明其发生在100MP Fill页面的预热中、甚至早于预热Levels结果安装，不能归为正式Fill计时失败，也不能据此宣称已证实旧失败根因。无产品源码/WASM修改、无新包/提交/发布；正式CPU26/29仍保留。记录F1-FRAMEGAP-20261020.md与acceptance-status-f1-framegap-20261020.json。


### 2026-10-20 private uniform spare diagnosis (not acceptance)

Real Worker comparison verified seven cases including removal of a synthetic 400,000,000-byte spare for 100 MP uniform output; original uniform/palette pixel and Undo/Redo checks passed for both variants (23/23 overall). Candidate remains private. All four AB/BA timing cases are INVALID: the desktop locked at 08:03:51 UTC, before the first case at 08:03:59 UTC; 388 read-only samples preserved. No performance benefit, regression, or root cause is established. An ESM fixture error and its corrected successful run are separately retained. Formal CPU acceptance remains 26/29. A fresh human unlock response is pending; a guarded one-time diagnostic is prepared and will stop its owned test tree if the desktop locks. See `build-artifacts/mac-1.4.9-delta-20261011/UNIFORM-SPARE-DIAGNOSTIC-20261020.md`.


### 2026-10-21 guarded CPU diagnosis (not acceptance)

Valid private uniform-spare AB/BA completed with 432 single/unlocked samples and 816 original hashes exact. All four cases failed original assertions: 100 MP Fill gaps baseline125/108ms, candidate112/125ms; Levels install baseline477/335ms, candidate545/410ms. Candidate rejected as a performance fix and not integrated. Exact-window baseline trace then passed1/1 (123 valid samples): current24MP52ms gap overlapped37.474ms CppGC;100MP136.103ms sweep was in warm-up, not the timed gap. No current100+ms root cause established. Formal CPU remains26/29. Current frozen WPR CPU/VirtualAllocation capture prepared and CHECK_ONLY_PASS; current AdministratorToken=False, native recording not run. Manual administrator step pending. See `build-artifacts/mac-1.4.9-delta-20261011/GUARDED-CPU-DIAGNOSIS-20261021.md`.


### 2026-10-21 native capture controller repair (not acceptance)

User capture completed F1 with106ms 100MPFill gap;167saved monitor samples single/unlocked. Controller StrictMode success-property error caused WPR to stop before the child finished. Offline ETL has zero lost events but ended06:44:03.306, whereas106ms gap began06:44:11.777; no native attribution of that gap is valid. Separate fixed controller handles blank/null/partial/missing-field monitor records and stops owned child on all controller errors;12WindowsPowerShellStrictMode regressions and read-only preflight passed. Original script/identity/logs/ETL preserved. Replacement administrator capture is required because the old ETL misses the failure, not to seek a passing performance result. FormalCPU26/29 unchanged. See `build-artifacts/mac-1.4.9-delta-20261011/CPU-STACK-CONTROLLER-FIX-20261021.md`.


### 2026-10-21 current native gap attributed (not full acceptance)

Corrected user capture covers current105.4msFill failure, zero ETL events lost,262single/unlocked samples;816runtime+7helpers exact. Main thread103samples includes94DOMArrayBufferFinalizer/VirtualFree. Allocation history identifies preceding main-thread WASM store:8GiB virtual reservation,969.0625MiBmaximumcommit, freed during newer-page job. This confirms one gap mechanism, not all prior failures. Private post-render zero-page-grow view-retirement candidate passed30/30 actual-Edge functional checks, no product change; overhead stays inside originalframe timing. AB/BA not yet run because finalgatefoundlockeddesktop; freshunlockpending. FormalCPU26/29unchanged. See `build-artifacts/mac-1.4.9-delta-20261011/NATIVE-STACK-ATTRIBUTION-20261021.md`.


### 2026-10-21 after-render zero-page-grow rejected (not acceptance)

Private30/30functional pass did not imply a performance fix. ValidABBA401single/unlocked samples,816originalhashes exact:Fill gaps baseline118/113ms,candidate118/130ms;4originalF1casesfailed. Candidate160actualdetachcalls preservedpages/bytes withmax0.2ms/call butlonggapsremain. Rejectanddonotintegrate. Native105.4ms attribution remains separate. FormalCPU26/29unchanged. NextprivatehypothesisisindependentactualpixelcopyatWebGLboundary, withallcopycostinsideoriginalframewindow. See `build-artifacts/mac-1.4.9-delta-20261011/FRAME-VIEW-LEASE-20261021.md`.


### 2026-10-21 bridge ownership follow-up (not acceptance)

After-render grow0 candidate rejected after valid401-sampleABBA:Fill baseline118/113ms,candidate118/130ms,all4F1failed. Texture-only private candidate30functionalpass retained buttimingnotrun:actualAPIcensus1passfound9CSPRNG+3GL+21decoder calls borrowingmainWASM. Newprivateowned entropy/UTF8bridge+pixeluploads passedrealWASMencoding/error/UUID/pixel/historychecks and30/30actualEdgefunctionalchecks;candidateobservednativeWASMborrowcounts0,GLcopyshapeverified. No productsourceintegration,forcedGC,movedtimedworkorbudgetchange. CombinedABBApreparednotrunbecause07:58:15localpreflightlocked;freshunlockpending,noadminneeded. FormalCPU26/29unchanged. See `build-artifacts/mac-1.4.9-delta-20261011/OWNED-BROWSER-BRIDGE-READY-20261021.md`.


### 2026-10-21 browser ownership candidate rejected after timing (not acceptance)

Original F1 AB/BA completed once with 413 single-display/unlocked samples and 816 frozen original inputs exact; all four executions failed. 100 MP Fill gaps baseline 115/124 ms, candidate 118/107 ms (budget <100). Candidate verified zero observed native CSPRNG/decoder main-WASM borrowing and 30/30 independent texture copies per execution, but added 119.9/121.7 ms aggregate copy cost and did not fix the gap. Reject; no product source integration or retry. Formal CPU remains 26/29. Native baseline finalizer attribution remains valid for its captured gap; no candidate native trace was taken. Next diagnostic: locate main-WASM high-water growth by engine operation. See `build-artifacts/mac-1.4.9-delta-20261011/OWNED-BROWSER-BRIDGE-REJECTED-20261021.md`.


### 2026-10-21 main-WASM high-water attribution (not acceptance)

One unchanged-body F1 growth diagnostic passed with 114 single-display/unlocked samples and 816 frozen original inputs exact. No product change: this instrumented pass does not replace the four valid ownership failures or formal CPU 26/29. 100 MP Levels grew from warm 116064256 to 516096000 bytes at CanvasSize, to 616103936 at first layer_pixels_len, to 1016135680 at staged 400 MB result reservation; peak exactly matches earlier native region history. These observations locate image/prefilter/staging allocations, not a live-allocation leak or physical RSS. Rejected bridge copies remain unintegrated. Next discriminating check is candidate native failing-interval/region attribution, not another ownership retry. See `build-artifacts/mac-1.4.9-delta-20261011/WASM-GROWTH-CENSUS-20261021.md`.


### 2026-10-21 candidate native discriminator prepared, not run

Rejected bridge comparison and growth attribution are retained. A separate candidate CPU/VirtualAllocation controller freezes 816 original runtime files plus 10 candidate/helper files, retains original test assertions/budgets, UTC largest-gap metadata, exact rejected candidate ownership code and all failures. CheckOnly passed; Playwright list validated exactly one F1 test. Current AdministratorToken=False; no recording/workload/elevation started. Next action requires human Administrator PowerShell execution of capture-owned-browser-bridge-system-trace-20261021.ps1 with one unlocked display. This is a mechanism discriminator, not an ownership retry or acceptance replacement; formal CPU remains 26/29. See build-artifacts/mac-1.4.9-delta-20261011/OWNED-BRIDGE-NATIVE-CAPTURE-20261021.md.



### 2026-10-21 candidate native disposal attribution (not acceptance)

Returned candidate trace fully covers the 183.3 ms failing 100 MP Fill gap: 3,622,992 events, zero lost, 319 single-display/unlocked samples, 826 frozen runtime/helper hashes exact, ownership replacements verified. PID 7776/TID 6388: 173 main CPU samples, 169 mutator sweeping, 167 VirtualFree; 43 DOMArrayBuffer-finalizer samples followed by 120 disjoint V8 CppGCManagedWrapper/WASM reservation-free samples. Old main-WASM region 0x28c00000000, 8 GiB virtual reserve/1016135680 maximum committed bytes, was created before the previous Levels page and released at gap+182.246 ms after the newer Fill memory existed. Matching Edge .62 PDB/native bounds verified; null controller testExit remains unknown, independent original F1 failure preserved. This confirms remaining V8-managed disposal despite native bridge copies. Close ownership-copy tuning; no source integration, identical recapture or acceptance retry. Formal CPU remains 26/29. Next engineering boundary is deterministic main-WASM peak/lifetime reduction preserving exact semantics and windows. See `build-artifacts/mac-1.4.9-delta-20261011/OWNED-BRIDGE-NATIVE-ATTRIBUTION-20261021.md`.


### 2026-10-21 late halving-cache compaction rejected (not acceptance)

Scratch source/WASM only: native 21/21 pass and 32 distinct browser case executions validated, with four fixture errors transparently preserved across two repairs; candidate unchanged. Original F1 footprint comparison: 235 single/unlocked samples and original 816 hashes exact; both cases failed. All four case high-water marks identical to baseline, including 100 MP Levels 1016135680 bytes. No meaningful peak saving: reject and do not run prepared performance ABBA or integrate. Formal CPU26/29 unchanged. Next allocation design must avoid initial intermediate prefilter allocation and first prove exact rounding/edge/cache behavior plus bounded native live allocation. See `build-artifacts/mac-1.4.9-delta-20261011/COMPACT-HALVINGS-REJECTED-20261021.md`.


### 2026-10-22 streamed display reduction: native saving, unchanged browser peak (not acceptance)

Private WASM passed18 native and32/32 fresh browser functional checks; JS glue unchanged,108 calls verified. Guarded original F1 footprint comparison had261 single/unlocked samples and143 private+816 original hashes exact; both original cases failed, all four high-water marks unchanged (100MP Levels1016135680 bytes). Baseline100007936-byte growth at layer_pixels_len disappears in candidate but reappears at set_preview, whose separate helper still calls full-image halved repeatedly. No integration or prepared performance ABBA. Formal CPU26/29 unchanged. Next separate candidate must cover preview without changing its source-pixel/adopted-cache semantics. See build-artifacts/mac-1.4.9-delta-20261011/STREAMED-PREFILTER-NO-PEAK-SAVING-20261022.md.


### 2026-10-22 streamed preview candidate ready, not integrated or accepted

Separate private raster/preview candidate preserves preview source-pixel semantics and authoritative display-cache precedence. Six native targets passed51 checks; even/odd scalar full-byte references, final-cache reuse/clear and deliberately different adopted displays passed. Display/preview peak extra requested native allocation1089536/4214784 bytes, not WASM high-water or RSS. Release WASM17037177 bytes SHA256bc07c60000c6940b4df14ae0811337452c46eb2b1b8c0a247b5c276fbc9da996; generatedJS exact,108 calls verified. Fresh browser32/32passed;147 private+816 original inputs exact before/after. Footprint/performance not run: latest launch gate recorded separately; require single unlocked display, then meaningful100MPLevels saving before ABBA. Prior no-saving display-only candidate retained. FormalCPU26/29 unchanged; no product integration/commit/push/release. See build-artifacts/mac-1.4.9-delta-20261011/STREAMED-PREVIEW-READY-20261022.md.


### 2026-10-22 streamed display and preview reduction integrated, broader validation pending

Private native51/51 and browser32/32 passed. Original F1 footprint232 single/unlocked samples:100MPLevels1016135680 to916127744 bytes, saving100007936 bytes; other three peaks unchanged. Normal fixed ABBA431 valid samples,147 private+816 original hashes exact: baseline1/2 failed (Fill gaps117/134ms; Levels install405/471ms), candidate1/2 passed original assertions (Fill gaps24/24ms, Levels418/346ms, Fill414/251ms). No retries, forcedGC, growth wrappers or changed budgets/windows. Exact two source changes plus meaningful allocation/byte regression test now in working tree; dirty CanvasView/raster_edit and Cargo.lock hashes preserved, originals backed up. Full native, production WASM/frontend, full browser and original29CPU remain pending; formalCPU26/29 unchanged, no commit/push/release. See build-artifacts/mac-1.4.9-delta-20261011/STREAMED-PREVIEW-ABBA-20261022.md.


### 2026-10-23 integrated streamed reduction: native and full functional pass, new hardware pending

Native packages run serially: engine/wasm730 passed plus Windows shell19 passed, total749, original10ignored retained, zero failures. All581 production build inputs exact after native/build; workspace releaseWASM17,037,177 bytes SHA256bc07c60000c6940b4df14ae0811337452c46eb2b1b8c0a247b5c276fbc9da996 exactly matches the two successful private ABBA candidates. GeneratedJS unchanged,108 interface checks, three TypeScript scopes and new frontend build passed. Complete original browser functional344 passed,33 original performance opt-ins skipped, zero failures/retries/flaky;858 frozen inputs exact before/after,847 GPU inputs frozen and current. New complete29CPU/4GPU not run; old26/29 remains historical earlier-source result. Next read-only single/unlocked hardware gate is recorded. No new packaged-runtime, GUI or actualRAW acceptance yet, no commit/push/release. See build-artifacts/mac-1.4.9-delta-20261011/STREAMED-PREVIEW-INTEGRATED-20261023.md.


### 2026-10-23 integrated streamed reduction CPU interrupted, not accepted

Pre-timing host module inheritance failure retained with 11 files; only child-process PSModulePath corrected, no frozen helper/source/runtime/test/budget changes. Original CPU29 then started with one worker and zero retries, but display observer detected lock at 2026-10-23T07:13:07.0427552Z, stopped owned tree at case15/29; no final Playwright JSON/result and no complete score. Before interruption original F1 Levels100MP install500ms and jobs Levels100MP install564ms failed <450ms; F1 Fill gap109ms also logged, its assertion unreached. All558 samples single display, not all unlocked.858CPU and847GPU frozen inputs verified exact after interruption. GPU and rebuilt package/nativeGUI/actualRAW not run. Prior native749+10ignored and functional344+33opt-in skips remain valid, while two private F1 passes and95.4MiB peak saving do not establish full acceptance. No automatic fullCPU retry. See STREAMED-PREVIEW-INTEGRATED-20261023.md and acceptance-status-streamed-preview-cpu-interrupted-20261023.json.


### 2026-10-23 15:17 frontend unit verification

46 frontend test files/310 assertions passed, zero failures or skips. All858CPU/847GPU frozen inputs exact afterward. No full CPU score due to retained lock interruption, GPU/package/nativeGUI/RAW not rerun. Latest read-only gate15:17:56 still single display but locked. No commit/push/release.


### 2026-10-23 complete integrated CPU27/29 and GPU4/4, partial acceptance

After explicit sustained-unlock response, original CPU29 completed once in new retained browser-streamed-preview-sustained-20261023:27 passed/2 failed,0skip/flaky/retry,1worker;858 source/runtime bytes exact plus10controller inputs. Valid500ms display924 samples and200ms session2064 samples all single/unlocked as applicable. Original failures:24MP Delete worst156ms (<150),100MP gradient install454ms (<450). Earlier interrupted500/564ms Levels installs and109ms Fill gap remain retained, not superseded. Original GPU4/4 passed,847inputs exact,192display/336session samples valid. New private original-body phase diagnostic:58records,1pass/1fail,0retry,127display/294session valid,858inputs exact;100MP gradient raw400MB,406.25MB total append442.4ms of465.4msCPU,begin21.4ms,finish0.2ms. Instrumented times never replace acceptance. Delete primarily WASM, further internal attribution needed; no speculative source edits. Native749+10ignored,frontend46files310,browser344+33opt-in skips retained. New production package/nativeGUI/realRAW not rebuilt or accepted; fourMPS half differences retained. No commit/push/release. See STREAMED-PREVIEW-HARDWARE-20261023.md and acceptance-status-streamed-preview-hardware-20261023.json.
