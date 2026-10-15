# Mac 1.4.5 → 1.4.9：Windows 更新范围核查

核查日期：2026-10-11。本文是源码与发布记录调查，尚未实施 1.4.9 更新，也不构成新版本验收通过的声明。

## 基线与结论

- Mac 原验收基线：`v1.4.5`，提交 `086f1631573ccb2b57644e53b52bf1488fc976aa`。
- 本次目标：固定标签 `v1.4.9`，提交 `0ab3ff58a3dcf3d52a455fe3ab0336ac95923edd`；不混入标签之后的 main 更改。
- Windows 核查基线：`cbbc1eb7838fe7db13140f4e7a518ed5cbb0118a`，当前稳定版 0.9.0。
- GitHub 标签比较返回 76 个提交、78 个变更文件，包含合并、发布和测试文件，并不意味着 76 项独立功能。
- 原定 v1 Windows 移植目标已经完成；跟进 Mac 1.4.9 是新的增量更新工作。
- `.comp` 仍写版本 11，项目结构没有升级。需要验证新行为和像素结果，无需先设计格式迁移。
- 最大工作量在 Camera Raw 算法与预览路径，其次是新增功能。WebP 和部分图层属性保留已经存在于 Windows。

依据：[标签比较](https://github.com/robbietilton/Compositor/compare/v1.4.5...v1.4.9)、[1.4.9 项目格式](https://github.com/robbietilton/Compositor/blob/0ab3ff58a3dcf3d52a455fe3ab0336ac95923edd/docs/project-format.md)。

## 各版本主要变化

时间采用北京时间。

| 版本 / 发布时间 | 主要变化 |
| --- | --- |
| [1.4.6](https://github.com/robbietilton/Compositor/releases/tag/v1.4.6)，10 月 8 日 12:16 | 命令搜索；Canvas Only；New Canvas 增加 in/cm/mm、72/300 DPI、透明/白/黑背景；画布顺/逆时针旋转 90°；RAW 导入改善速度与渐变；修正半透明像素的 Color Overlay、Inner Glow、Inner Shadow；修正 Camera Raw 彩色降噪。 |
| [1.4.7](https://github.com/robbietilton/Compositor/releases/tag/v1.4.7)，10 月 8 日 18:23 | Last Filter 用原参数重复上一次滤镜；F 切换全屏、Esc 退出；Search Commands 使用 ⌘F；图层按钮、重命名、窄窗口工具栏、Don't Save、平移手形的交互改进。 |
| [1.4.8](https://github.com/robbietilton/Compositor/releases/tag/v1.4.8)，10 月 9 日 13:44 | 统一 Export As：PNG/JPEG/单页 PDF，输出尺寸及预览；Scanlines 成为独立滤镜并扩展参数；300% 起显示 Navigator 小地图；画布选择图层后面板定位；Camera Raw 阴影/高光保持亮度顺序。 |
| [1.4.9](https://github.com/robbietilton/Compositor/releases/tag/v1.4.9)，10 月 10 日 14:11 | Camera Raw 多项计算改用 Photoshop 实测色表，并改进局部算法和预览速度；尺寸/裁剪/调整时保留图层效果与适用的可编辑信息；WebP 导入；修饰键+鼠标中键拖动缩放；Untitled 编号修正；更新提示显示更新内容。 |

应以 1.4.9 最终代码作为目标。中途提交曾更换命令搜索快捷键、Navigator 位置和 Don't Save 样式，不应分别照搬。

## Windows 对应工作

“已存在”来自当前源码核查；它不等于已经完成 Mac 1.4.9 的运行时对比。

| 项目 | Windows 0.9.0 现状 | 更新工作 / 验收重点 |
| --- | --- | --- |
| Camera Raw Light/Color | 使用 1.4.5 参数与算法 | 引入新版实测 LUT；同步 Temperature/Tint、Exposure、Contrast、Highlights/Shadows、Whites/Blacks、Saturation/Vibrance；白平衡吸管与 Auto 也必须同步。验证透明像素、极值和不同亮度图片。 |
| Camera Raw 曲线、Mixer、Grading、Calibration | 已有 UI，算法基线较旧 | 同步 LUT 阶段顺序、参数插值与组合；覆盖默认/移动曲线分界、Refine Saturation、Point Color、四个 Grading 色轮及 Blending/Balance。Calibration 主色改走色表，Shadow Tint 保留单独路径。 |
| Camera Raw Texture/Clarity/Dehaze/Detail/Optics | 已有功能，仍是旧算法 | 同步引导滤波、随图片大小变化的半径、画面相关 Dehaze、Sharpening Masking=0 的行为、保留亮度的彩色降噪和新版暗角。比较预览与 Apply。 |
| Camera Raw 预览性能 | 已有 Worker 与有界预览 | 移植合成 LUT、会话亮度统计缓存和 scope 优化；在 Windows WASM 实测。上游 Mac 的最高 20 倍速度提升不能直接作为 Windows 成果。 |
| 三种半透明内效果 | 六种效果可渲染、保留；只有 Stroke/Drop Shadow 提供完整编辑入口 | Color Overlay、Inner Glow、Inner Shadow 要先改变源像素 RGB 并保留 alpha，再与底下效果合成，避免变灰、变不透明和污染阴影。新建编辑 UI 属于另一个范围，修复已有渲染无需先补 UI。 |
| Canvas Size / Crop 保留图层属性 | 已通过克隆文档保留 effects/text/shape | 增加 1.4.9 往返用例，核查实际 Crop 路径；无需照搬 Mac 修复已存在的 Windows 逻辑。 |
| Image Size 保留/缩放效果 | 已保留 effects，按像素到画布变换缩放并转换阴影方向 | 核对与新版 Mac `sqrt(sx*sy)` 规则的差别，尤其已旋转/缩放/翻转图层及非等比尺寸。不能仅凭“都保留效果”认定像素一致。 |
| 活文字/形状保留 | 像素未被改写时保留；`set_pixels` 清除 live metadata | 与新版边界一致地检查：仅改 DPI、Canvas Size/Crop/Trim 应保留；实际重采样与破坏性像素编辑会栅格化。**1.4.9 并不保证所有 Image Size/调整操作后文字仍可编辑。** |
| WebP 导入 | 解码器、Cargo feature、原生文件选择器已支持 | 无需重复开发；补有损/无损/透明 WebP 的文件选择、拖入、保存重开与安全限制回归。 |
| RAW 导入平滑渐变 | LibRaw 辅助程序当前直接输出 8 位 | 评估 LibRaw 16 位输出到统一 8 位抖动量化，避免只在已经量化的 8 位 PNG 上加噪声。保留真实传感器解码、预览/取消、预算限制和辅助程序校验。Apple CIRAWFilter 与 LibRaw 的解码差异仍须明确。 |
| RAW 导入状态 | 已有 Developing… 与文件操作 busy 状态 | 核查从传感器处理到图层真正加入的全过程状态，避免对话框结束后短暂无提示。 |
| Search Commands | 无命令搜索入口 | 基于共享命令注册表搜索菜单、工具、模式和蒙版命令；支持禁用状态、开关状态、方向键、Enter、Esc。Windows 快捷键用 Ctrl 映射，并避免文本输入冲突。 |
| Toggle Fullscreen | 无对应编辑器功能入口 | 增加 F 全屏/仅画布显示，Esc 先取消正在编辑的操作，再退出全屏；覆盖窗口恢复、输入框与模态面板。 |
| Last Filter | 无入口，也无重复滤镜状态 | 保存已成功 Apply 的滤镜及参数，用相同设置一次执行、一个 Undo；取消预览不覆盖记录。Image 调整及 Content-Aware Fill 不计入。Windows 可采用 Ctrl+Alt+F，但须先核查冲突。 |
| New Canvas 单位/背景 | 当前只输入像素宽高；文档已有 DPI 字段 | 增加 px/in/cm/mm、72/300 DPI、透明/白/黑背景；区分单位换算和改变 DPI，检查四舍五入与预算。默认仍是透明、像素、72 DPI。 |
| Rotate Canvas 90° | 只有 Flip Canvas，未发现画布 90° 命令 | 同步宽高互换，旋转图层/文件夹/独立蒙版/选区/参考线，保留源像素，一个 Undo；不是仅旋转当前图层。 |
| Export As / PDF | 分开的 PNG 导出与 JPEG 预览；没有 PDF | 统一格式选择、尺寸/百分比、保持比例、透明棋盘预览、JPEG 质量/底色和 PDF 底色。只改变输出副本，不修改项目尺寸/dirty。PDF 单页大小为输出像素÷DPI×72 points，嵌入栅格画面，不导出可编辑 PSD 图层或矢量文字。 |
| Scanlines | 当前没有独立滤镜入口 | 移植新版 `scanlines_apply` 及 Glow；UI 覆盖 Spacing、Thickness、Displace、Smoothness、Dots、Threshold、Color Split、Black Level 等；Preview/Cancel/Apply/Undo 与 Last Filter 联动。 |
| Navigator | 无小地图入口 | 按最终版在画布右上角显示，300% 起、视图只覆盖部分画面时出现；View 开关、红色视口框、点击/拖动定位、编辑后延迟缩略图刷新。 |
| 图层/UI 细节 | 已有部分展开父组；Move 工具条已横向滚动 | 补画布点选后自动展开并滚动到图层；扩展滚动到其他工具条；检查重命名布局、按钮可读性/禁用态、拖动时闭合手形。 |
| 中键缩放 | 已有中键平移和 Ctrl+滚轮缩放 | 加 Ctrl+中键拖动缩放，保持缩放锚点，避免同时平移；无修饰键仍平移。 |
| 最后标签/Untitled | 关闭最后文档已经进入空工作区；未保存标签统一显示 Untitled | 无无限递增问题。若跟进 Mac 的最低可用 Untitled N 命名，需新增会话名称；不必改变已验收的清空画布/关闭确认行为。 |
| 更新窗口 release notes | Windows 通过 GitHub portable release 分发；原定 v1 不含自动更新器 | 保持发布 CI 并补新功能、资源和发布说明校验。Mac Sparkle 更新窗口不直接移植；Windows 自动更新器可独立立项。 |

## 核心技术注意点

1. 新增 `CameraRawTables.bin` 为 1,247,790 字节，SHA256：`e67ff4dbcce35cb7765752acff4405be5c934d913ef1396b2ca3fc6dc8a85190`。格式 `CRT3`，763 张 17³ RGB 表，LZMA 压缩；展开表数据约 10.7 MiB，组合网格为 33³。需要冻结资源哈希、移植解压/重建/插值/阶段组合并计入 WASM/Worker 内存。
2. 新版 `AdjustPixels.c` 使用 Apple `dispatch/dispatch.h` 与 C Blocks。不能直接替换当前 `engine/native/AdjustPixels.c` 后假定 MSVC/WASM 能编译；应先保持数值算法，适配跨平台循环，再测 Worker 或其他并行方案。
3. Geometry 的既有坐标证明可以保留，但 Camera Raw 混合配方中的 Calibration、Light/Color 已变化，需要重新比较，不能继续使用旧 PNG 作为新版期望。
4. `Image Size` 的实际重采样仍将 live text/shape 转成普通像素。需以 [格式说明](https://github.com/robbietilton/Compositor/blob/0ab3ff58a3dcf3d52a455fe3ab0336ac95923edd/docs/project-format.md) 和 [ImageResizer](https://github.com/robbietilton/Compositor/blob/0ab3ff58a3dcf3d52a455fe3ab0336ac95923edd/Compositor/IO/ImageResizer.swift) 的边界解释发布说明。

核心依据：[CameraRawTables](https://github.com/robbietilton/Compositor/blob/0ab3ff58a3dcf3d52a455fe3ab0336ac95923edd/Compositor/Document/CameraRawTables.swift)、[CameraRaw 管线](https://github.com/robbietilton/Compositor/blob/0ab3ff58a3dcf3d52a455fe3ab0336ac95923edd/Compositor/Document/CameraRaw.swift)、[AdjustPixels](https://github.com/robbietilton/Compositor/blob/0ab3ff58a3dcf3d52a455fe3ab0336ac95923edd/Compositor/Rendering/AdjustPixels.c)、[新版效果合成](https://github.com/robbietilton/Compositor/blob/0ab3ff58a3dcf3d52a455fe3ab0336ac95923edd/Compositor/Rendering/MetalLayerEffects.swift)、[RAW 导入](https://github.com/robbietilton/Compositor/blob/0ab3ff58a3dcf3d52a455fe3ab0336ac95923edd/Compositor/IO/RawImporter.swift)。

## 建议推进顺序

### A：先处理会改变画面的更新

冻结 1.4.9 源码与新资源；升级 Camera Raw 算法、彩色降噪、三种半透明效果；核对图层属性保留和 Image Size 效果规则；评估 RAW 16→8 位量化。建立透明渐变、颜色网格、暗/中/亮照片和组合参数回归。先确认像素与取消/Undo，再做速度优化。

### B：补用户可见的新功能

New Canvas 单位/背景、画布旋转、Search Commands/Last Filter/Fullscreen、Navigator、Export As/PDF、Scanlines、中键缩放与图层定位。WebP 和已经具备的保留行为做回归，避免重复实现。以上功能分别验证未保存状态、取消、Undo 和多标签切换。

### C：新基线验收与发布

- 保留历史 1.4.5 验收证据，新建 1.4.9 专用结果目录及预期输出；改变了算法的案例使用新版 oracle。
- Windows 自动完成 Rust/WASM/前端、浏览器、原生 WebView2、文件选择器、打包和性能验证。性能限制不因上游宣传而放宽，也不直接照搬 Mac 的速度倍数。
- Mac 回传集中为一批：Camera Raw 单项/组合 PNG，半透明效果 PNG，尺寸/裁剪/旋转前后可编辑属性的 `.comp`，Scanlines PNG，以及 300 DPI 下的 PDF/PNG/JPEG 导出。由脚本生成可自动完成的探针；需要 UI 操作的部分提供中文 `MAC-STEPS.md`，保留英文菜单和准确参数。
- RAW 的 Apple/LibRaw 差异采用各自真实传感器解码和渐变质量验收，不把两个解码器的所有输出强行定义为逐像素相同。
- 资源、辅助程序与 portable ZIP 完整性通过后再更新 README、兼容性说明、版本、CI 发布说明和 source receipt。可规划 Windows 0.10.0 作为此增量的候选版本；版本号尚未更改。
- 原 v1 不包含的 Vision 选择、HEIC、自动更新器、签名/安装器，以及早于 1.4.5 已有但未纳入原范围的额外滤镜，不自动计入这次版本差异移植。

## 本机证据与复查入口

缓存位于 `build-artifacts/mac-1.4.9-delta-20261011/`：

- `compare-node.json`：完整标签比较响应。
- `releases.json`：1.4.5～1.4.9 发布日期与说明。
- `source-1.4.9/`：固定标签的独立源码 checkout。
- `raw-source-1.4.9/`、`source-receipt.json`：26 个关键源码/许可/资源文件及 SHA256。

Windows 对照入口：`engine/src/adjust/camera_raw.rs`、`engine/native/AdjustPixels.c`、`engine/src/effects/render.rs`、`engine/src/effects/settings.rs`、`engine/src/ops/image_size.rs`、`engine/src/ops/canvas_size.rs`、`engine/src/codec.rs`、`app/src/panels/MenuBar.tsx`、`app/src/sheets/NewCanvasSheet.tsx`、`app/src/sheets/JpegExportSheet.tsx`、`app/src/canvas/CanvasView.tsx`、`src-tauri/native/raw-develop.cpp`。

上述差异表是实施前的源码研究快照。当前实施与分层验收进度见 `prd.md`。2026-10-12 的 Mac 1.4.9 回传已核对：固定 28 组 RGBA 中 27 组逐字节一致，Scanlines Glow 尚有最大 49 级的 RGB 差异；两组安装版效果导出最大差异 1。实际 Cancel/Undo/Redo/Last Filter Undo/Navigator 像素关系、600×300/300 DPI 保存项目、300×150 PNG/JPEG 和 72×36pt PDF 均已独立读取，PDF RGB 与 PNG 一致。旧结果不覆盖，Glow 和原 CPU 性能失败继续保留。

Glow 的上游先 `clampedToExtent`，再缩小、Gaussian Blur、放大。Windows 当前先截成有限缩小网格再模糊；诊断中保留无限边缘可明显减小差异，但不能代替逐字节验收。Core Image 的默认 [highQualityDownsample](https://developer.apple.com/documentation/coreimage/cicontextoption/highqualitydownsample) 在 macOS 启用多遍缩小，与单遍采样不同。已准备独立中间平面诊断，须由真实 Mac 执行；未据近似结果改写门槛。

2026-10-13 的 `0b34aca` 验收汇总保存在 `acceptance-status-20261013.json`：308 项单位测试、343 项浏览器功能测试、原 4 项 GPU 性能、`20261012-2233` 生产包的原生对话框/独立导出读取器/六组真实 RAW 解码和 11 项 ZIP 完整性检查均通过。原生 298 条、GPU 141 条独立记录全部单屏解锁。原完整 CPU 仍为 28/29，156ms Fill 帧间隔发生在锁屏前，完整运行后段环境也失效。单屏解锁的私有原序诊断复现 121ms，随后原生轨迹显示计时外的 cppgc 清理任务约 153ms；资源生命周期关联尚待证明，诊断不代替完整验收。Mac Glow 中间平面仍待回传，整体状态保持未完成。

## 验收基准的版本变化

- Camera Raw 的五项旧公式用例及一个预览提交中的曝光数值改为固定 1.4.9 CRT3 数据的独立三线性插值检查；默认曲线分界走实测 LUT，移动分界仍走独立路径。旧版本历史结果保留。
- Image Size 的三个效果方向用例跟进固定 `ImageResizer.swift:26–31,101` 和 `LayerEffects.scaled`：重采样后距离/模糊/描边使用 `sqrt(sx*sy)`，阴影角度保留。原旋转/翻转投影规则不再作为新版期望；新增 DPI-only 与非等比 Undo/Redo 检查。
- 旧 `mac-1.2.10-probes` 中 `effects-inner-shadow`、`effects-inner-glow`、`effects-color-overlay`、`effects-all-six` 的 PNG 和项目保持不变。1.4.9 `MetalLayerEffects.swift:369–389` 先改变源 RGB、保留源 alpha，再置于阴影之上；这些四项当前输出改与独立全平面 Metal 公式对照，并断言确实不同于旧 PNG。其余旧像素界限保持不变；这不能替代安装版 Mac 1.4.9 导出验收。
- 原 29 项 CPU/UI 和 4 项 GPU 性能断言、时间预算、单 worker、零重试保持不变。所有失败日志保留，新运行写入独立目录。

2026-10-13 后续当前源码 `3926872` 的完整结果为原生引擎 701、WASM 边界 11、功能浏览器 343 均通过；原完整 CPU 28/29、GPU 4/4，唯一 CPU 实际断言失败为 100MP Fill 最长间隔 107ms 对 <100ms。CPU 815、GPU 115 条独立采样全部单屏解锁，755 项输入前后不变。`20261013-2100` 生产包另行通过 ZIP 11 项、真实 RAW 六组、原生交互 25 次和独立图片/PDF 读取，264 条采样全单屏解锁；汇总见 `acceptance-status-carried-mask-final-20261013.json`。旧失败、旧包和全部原预算保留，尚未发布增量。

Mac 直接核回传 `Mac149-glow-direct-20261013-202111` 因 CRLF 启动失败而未执行。修正过程又发现 v2 校验清单含旧哈希；最终 v3 包同时修正 Unix 换行、执行权限和全部启动哈希，真实 Bash 语法和 16 项哈希检查通过，等待 Mac 执行。既有 104 个中间平面已确认叠加公式精确，Gaussian/缩放仍未跨样本精确。私有旧运行时 Worker 边界轨迹没有复现间隔，不能把它的 24ms 数值作为当前完整性能通过证明。

Mac 后续回传 `Mac149-glow-direct-20261013-212859` 的 16 项启动哈希通过，随后 Swift 在 flat-113 的泛型数字数组表达式处类型检查超时，尚未执行基线及 Gaussian。v4 将四个归一化通道写为显式 Float，并增加九组独立 Float32 输入的整平面核对；本机 Bash 语法、25 项启动哈希、ZIP 与数值检查通过，但 macOS 编译/执行仍待验证。此处修复诊断程序的编译表达式，上游算法、sigma、尺寸、原基线及 Windows 产品实现均未改动。

2026-10-14 复查成功回传 `Mac149-glow-direct-20261013-222947`：九组 Float32 输入、原基线和全部 213 输出平面正确；v4 真实编译执行问题已解决。83/85 模糊上下文对满足 default=half RTZ(Float32-working)，其中全部 45 个 Float32 源样本精确满足。横向/恒色 35 平面仅为最近 half 的 0 或 ±1 Float32 ULP；纵向/二维 10 平面存在更大偏差，因此不能把所有结果都还原为同一 half 临时格式。固定线性核的叠加/平移反例还可能来自内部非线性量化，不能单独归因于采样位置。

不同核形状、成对采样、坐标及数值积累的有界模型未跨脉冲/台阶/恒色精确，未改产品算法或门槛。下一判别是复用已在 Mac 运行的 Phase 7 公开 CIKernel 观察器，加上 [Apple 官方 CI_PRINT_TREE 调试输出](https://developer.apple.com/videos/play/wwdc2020/10089/) 的原始/优化/GPU 执行图。`Mac149-glow-arguments-v1.zip` 的 245 项启动哈希、ZIP、Unix 换行/权限及 Bash 语法已通过，218 项既有真实输出控制和自测保护已审核；实际 Mac 编译/执行尚未运行，未捕获内置入口明确记为不可取得。汇总见 `acceptance-status-glow-arguments-prepared-20261014.json`。Windows 原 107ms Fill 性能失败保持独立，整体仍未完成。

参数回传 `Mac149-glow-arguments-20261014-110302` 的 245 项启动哈希通过，但新增 Swift 局部变量保留 Objective-C 返回的 Data?，write/JSONSerialization 两处编译失败，尚未记录参数或执行 218 项对照。原静态审核漏识别此桥接类型问题，不能作为实际编译证明。v2 仅追加 nullable 声明和 guard-let 非空检查，参考平面、观察器实现及启动脚本均保持；本机哈希、脚本、ZIP、换行/权限通过，完整 Mac 编译仍待验证。新汇总 `acceptance-status-glow-arguments-v2-prepared-20261014.json` 继续保留此前成功的 213 平面证据和全部失败。

真实回传 `Mac149-glow-arguments-20261014-144235` 已编译执行成功。独立核对全部 245 启动哈希、218 参考输出、九组输入、平面尺寸/有限值及自测像素/参数位模式均通过。806 次公开调用包括 Reduce4 350 次、Blur3 210 次、Blur5 70 次、cubic 175 次和自测一次。原始 run.log 的第 347 行直接 sigma=4 初始图含候选 switch；第 361 行优化图、第 368 行执行图实际选择 `CIBlurProcessor-4`（默认 RGBAh，另一工作上下文 RGBAf）。全日志中 Reduce4/cubic 出现在 25 个初始图，在优化图与执行图中均为零；公开参数记录仅证明候选图构建，不能证明 processor 内部算法。独立回执为 `glow-arguments-144235-return-review-20261014.json`，不可据此宣布 Glow 已修正。下一步核对实际 processor 的公开边界，保留原参考输出及性能失败。

下一独立诊断 `Mac149-glow-mps-v2.zip` 保留 218 项原控制，记录 [MPS 公开 encode](https://developer.apple.com/documentation/metalperformanceshaders/mpsunaryimagekernel/encode(commandbuffer:sourcetexture:destinationtexture:)) 和 Gaussian/convolution 构造入口；恢复后再用公开 Gaussian 重放 135 项纹理精度组合。实际原渲染是否经过 MPS、独立 MPS 数值是否等于 CI 输出均待真实 Mac 判别，未作归属假设；MPS Gaussian 的公开说明明确允许近似，也不提供精确内部系数。上传 Float32/显式 Float16 RTZ 字节均保存，RTZ 在此是候选输入实验。失败格式/encode 异常/GPU 错误分别保留并继续其余独立项，不计作通过。248 项实际启动哈希、Bash、ZIP/权限/LF 校验通过；静态审阅不替代 macOS 编译。最新汇总 `acceptance-status-glow-mps-v2-prepared-20261014.json` 继续标记增量未完成，Windows 原 107ms 失败不变。

Windows 后续在当前 `3926872` 冻结资产完成一次低干扰 Long Animation Frame/LongTask/rAF 定向归因，两个原 F1 用例的断言和预算不变；无 CPU profiler/CDP tracing。100MP Fill 的计时间隔 21ms、安装 CPU 246ms、首帧 10ms；174 条采样全单屏解锁，755 项输入前后不变。页面始终 visible，观察器支持已在真实 Edge 验证，但原 107ms 间隔未复现；冷启动期间的长帧不在原计时窗内，不能作为根因。`cpu-current-frame-attribution-summary-20261014.json` 保留完整归因和原失败，未作产品补丁或原完整协议重试。MPS 回传仍待真实 Mac，后续新实现须重新绑定生产包与原完整验收。

真实 `Mac149-glow-mps-20261014-175354` 回传已独立核对：248 启动哈希、218 原输出、九组 Float32 输入、18 上传缓冲及 135 个 MPS 平面通过，失败项为零。实际原路径记录 176 次 MPS Gaussian 构造及 176 次 encode，默认 source/destination 为 RGBA16Float，Float32 工作上下文实际为 RGBA32Float。独立 half→half 与默认 CI 相同 27/45，奇偶相位的测试图对称关系解释 41/45；后者依赖 fixture 对称性，不能作为任意图片实现。45/45 独立 f32→f16 等于 f32→f32 的最终 half RTZ，但这不足以替代 CI 默认路径，源格式/内部精度仍有差异。公开 [MPS options](https://developer.apple.com/documentation/metalperformanceshaders/mpskerneloptions) 允许选择 reduced precision；原 kernel 的具体选项值尚未取得，不能从格式或相似像素倒推。

下一包 `Mac149-glow-metal-v2.zip` 保留全部 353 旧输出，补公开 options/offset/clip/edgeMode 与同步 Gaussian constructor/encode 范围的 Metal 参数观察；仅限实际 method owner、类型编码完全一致的入口并保存/恢复原 IMP。记录最多 4096 字节的公开 setBytes/shared CPU 绑定窗口、函数常量、纹理元数据、pipeline label 和直接 dispatch；其它线程、private 缓冲及未覆盖入口记为未知，pipeline label 不认定为函数名，常量不直接认定为 taps。三分量步长未验证时不抓取，非完整 clip 不比较未写输出。独立 90 项重放复制捕获的配置，上传内容仍是已知 fixture 的 padding 假设。386 项实际哈希、Bash 语法、ZIP/权限/LF 通过；静态检查不替代 Mac 编译。最新 `acceptance-status-mps-success-metal-prepared-20261014.json` 保持未完成，原 Windows 107ms 失败保留。

实际 `Mac149-glow-metal-20261014-195526` 已在 Apple M5 Pro/macOS 26.5.2 编译执行：独立复核 386 个启动哈希、385 收据文件、全部 218 CI/135 MPS 旧输出、九组输入及 90 个新上传缓冲。配置重放复制原 options=2、edgeMode=1、offset、完整 clip 和纹理格式/尺寸，默认及 Float32 工作上下文原方向分别 45/45 精确，翻转方向各 35/45；这是已知测试输入 padding 假设的验证，未读取真实 CI 输入纹理。176 次原 Gaussian 构造/176 次 encode 与旧记录一致。5,284 条原 Metal 事件含 1,060 条有界参数事件，零丢失；16 字节自测、八项 method owner/类型编码检查通过。公开 pipeline label 为 D2F6V/H、F11V/H 或 F13V/H、U2P3F3H/V；这些标签不作为已证明的函数名。

参数布局目前支持：首字为输出宽高的两个 UInt16，六阶段均有原点/affine 字段、128 字节配置及输出/输入纹理元数据。六抽头缩小系数、对称卷积系数和放大参数的数学角色、运算顺序/half 精度、sigma→系数函数仍需独立软件重建与外部 Glow 验证；末段缓冲含上一段残留字段，不把全部字节解释成有效 taps。生产实现不得固定测量系数表或 fixture 对称规则。`glow-metal-195526-return-review-20261014.json` 与 `acceptance-status-metal-success-20261014.json` 保留证据，配置精确不替代产品门槛或 Windows 原 107ms 失败。

`glow-metal-stage-layout-20261014.json` 进一步验证全部 176 条六级纹理/格式链、packed 输出尺寸和 32×32 dispatch；工作 Float32 context 的中间五级仍写 RGBA16Float，最后一级才写 RGBA32Float。独立候选支持 quarter-linear upsample＋fine-grid `[b,A,b]`（`b=(1−A)/2`），融合后 coarse 中央权重 `.5+.25A`、两侧 `.375−.125A`/`.125−.125A` 按相位互换；这是候选数学，尚未通过全部像素。简单 Gaussian/方差修正的系数误差 0.00220～0.00726，conditional factorization least squares 仍相差 0.000108～0.000856。六级独立常量状态判别留下 74 个白场代表路径，RGB 未穷举、最佳仍 20/120 常量通道不同；没有据此选择猜测性产品补丁。

下一 `Mac149-glow-stages-v1.zip` 保护全部 443 旧输出和 90 上传缓冲，原控制结束并恢复旧观察器后，才在独立重放的每级 dispatch 后插入 snapshot shader/公开 texture barrier，恢复原 pipeline、纹理和参数。即刻复制避免 MPS 临时存储后续复用；只等待自有 command，不在原控制 command 中加读回/复制。所有非负有限 half 位模式复制自测（31,744 项）、90 旧最终输出 exact、73 新有/无 snapshot 基线 exact 是分别的门；阶段不可读/类型不匹配/失败保留为未知，复制导致调度改变不被隐藏。公开接口依据：[compute memory barrier](https://developer.apple.com/documentation/metal/mtlcomputecommandencoder/memorybarrier(scope:)) 和 [Apple feature tables](https://developer.apple.com/metal/Metal-Feature-Set-Tables.pdf)，不以文档替代真实设备执行。570 启动哈希、Bash 语法、ZIP/权限/LF 校验和静态检查通过；Windows Git Bash 无 shasum，保留该本地失败并改用 native sha256sum 核对相同 570 输入，Mac 已验证的 shasum 启动方式保持。完整 Mac 编译/复制自测/163 项诊断仍 `NOT_RUN`。`acceptance-status-stages-prepared-20261014.json` 继续保留原 Windows 107ms 失败与未完成状态。


## 2026-10-15：六级数学精确复现，组合路径仍待判别

真实回传 `Mac149-glow-stages-20261014-205225` 已独立核对 570 个启动哈希、569 收据文件、443 个旧输出、90 个旧上传缓冲和 73 个重建上传。163 个有/无 snapshot 最终结果一致，978 个中间平面有限、非负，最终 snapshot 与最终纹理一致。完整 half 位模式复制自测实际通过，但原始自测缓冲没有另存。原 Metal 观察为 5,282 事件、1,058 参数事件；specialization constants 两条而前轮四条，是编译期观察差异，未改像素守卫。证据 `glow-stages-205225-return-review-20261015.json`。

独立软件模型的 D2、FIR、U2 各 326/326 平面逐字节相同。D2/FIR 使用 1/256 双线性相位、half 中点向上采样、F32 FMA 和 half RTZ 输出；FIR 成对采样相邻抽头。U2 是三支持点、分段二/三次且满足 partition of unity 的 cubic；系数计算的独立 F32 舍入顺序影响结果，不能用融合 Horner 表达式替换。从一次上传串联计算的 163/163 最终结果、978/978 中间结果精确。另以最终 extent、offset 和抽头支持反推工作矩形，不再读取捕获的阶段 ROI，163/163 仍精确。数值参数仍取自实际捕获，这不是一般 sigma 参数生成的证明。证据 `glow-whole-pipeline-proof-20261015.json`、`glow-generic-geometry-proof-20261015.json`。

全部 163 项参数按约 0.05 的 sigma 区间分为 21 组；`ceil(Float32(sigma)*20−0.5)` 是覆盖当前记录的分组假设。合法 Line Spacing 2～32 所需十组参数都出现，但仅 16 项具体 sigma 直接测过；原连续 Gaussian 假设不成立，尚未把观测系数表或拟合多项式写入产品。外部缩小从 RGBA8 源开始，包括无限边缘、mip 精度/舍入和残余采样，九个 F32 及九个 RGBA8 平面精确；实际缩小输入的 Gaussian 九平面也精确。完整源图的软件候选五组 Glow 中三组精确，另外两组共三个颜色字节各差一级。独立二维采样候选仍不跨九图精确，不能继续凭近似值宣布通过。相关证据 `20261015-reduce-proof.json`、`glow-gaussian-external-check-20261015.json`、`glow-from-drawn-candidate-20261015.json`。

下一包 `Mac149-glow-composite-v1.zip` 在全部 443 原控制完成后，继续守卫上次 978 阶段及 163 最终输出，并验证新增实际输入 snapshot 与已知上传相同。再对所有 31 个合法整数间距重放完整上游绘制/缩小/Gaussian/放大/原 C 合成，原入口和独立组合结果必须一致；捕获同线程 Gaussian encode 范围内的真实输入及六阶段输出，避免把独立 halo 误当组合路径输入。另以四 texel/相位和四个独立中心自测做 99,306 项 half/float Metal sampler 判别。旧输出对比是诊断，不自动把差异算通过。当前 compute sampler 的默认 LOD=0 符合 Apple Metal Shading Language Specification（2026-06-04，PDF 第 239 页），本机无 Mac 编译器，不能据静态审阅声称编译通过。

该新包 1,716 项实际 Bash SHA256 校验、脚本语法、ZIP CRC/内容/Unix 权限及本地控制字节检查通过；ZIP 4,917,650 bytes，SHA256 `d710a7727ddafcded35055f3fdfe87b24828139e757f33fe55ce75a8937e7304`。Mac 编译、组合捕获及 sampler 输出均 `NOT_RUN`，中文 `MAC-STEPS.md` 随包提供。最新汇总 `acceptance-status-stages-success-composite-prepared-20261015.json`。Windows 产品源码仍为 `3926872`；原 107ms Fill 失败、所有原预算和历史失败保留，整体状态 `INCOMPLETE`。

## 2026-10-15：完整间距覆盖与合成 FMA 修正

`Mac149-glow-composite-20261015-075932` 已真实编译运行；独立复核 1,716 启动哈希、1,715 收据文件、443 原输出、978 旧阶段和 163 旧最终输出。新增 163 个输入 snapshot 与实际上传相同；31 种间距的组合重放与 untouched `Scanlines.apply` 全字节一致，186 个新中间平面的尺寸/格式/参数/有限值均核对。两种独立 sampler 各 99,306 样本（397,224 分量）彼此和旧目标全字节相同，直接排除这些旧样本的组合路径输入不一致假说。收据的 `NOT_RUN` 是制作时快照，本次结果由 run.log 和独立文件复核证明。证据 `glow-composite-075932-return-review-20261015.json`。

实际 Gaussian 输入串联既有软件数学，31×6=186 个平面精确；独立推导 ROI 的 31 个最终纹理也精确。全部构造器/encode 的 sigma Float32 位模式和阶段参数与前轮记录一致，补齐此前未直接测过的 15 个 sigma 设置。可采用有限、完整的间距 2～32 → 十组内核参数目录（103 个唯一 Float32 数值）；不需要拟合或推断 sigma 分组，目录保存算法系数而非像素答案。该方案的适用域必须限定为标准化 Scanlines，不能外推其它效果、任意 sigma、尺寸规划或其它 GPU/MPS 版本。私有目录 `20261015-direct-spacing-kernel-catalog-candidate.json` 尚未用于产品。

外部 reduction 旧模型直接预测实际输入仅 23/31 精确。相邻 mip 层比较发现 shrink=9、17、18 的五项应少一层，其余大差异消失；具体通用 LOD 决策尚未证明，不能写成三个 shrink 特例。纠正已观察层级后还有四个独立 half 采样差一阶的反例；它们和 standalone sampler 都证明普通四项加权、half 中点向上模型不充分。新数据表明扩大到 31 项后不能继续沿用旧五项“仅差三个字节”的整体描述。固定共享指数截断虽能减少差异，仍使九图的 RGBA8 输出有 22 个错误字节，不采用近似候选。`sampler-byte-boundaries-native-fma-20261015.json` 单独保存了使用真实 C FMA 合成后的误差，较早 NumPy exp 模型的数值报告保留为历史诊断。

新增独立合成判别明确发现第二个问题：传入真实 Mac bloom 时，Windows 原 C 非融合末步在 spacing 10/15 共四字节不同，显式 FMA 则 31/31 精确。两个独立 bright-edge 回归值可在不经过 Gaussian 的情况下复现：[250,151,196,255] 与 bloom [57,58,58,234] 应为 [253,185,224,255]，旧代码红色为254。现在仅将 `dither_glow` 末步写为 `phase7_mul_addf`，由 Rust `f32::mul_add` 在 native/WASM 共用边界实现，保留 expf、其它运算顺序和其它滤镜。回归先在原代码实际失败（exit101），修正后通过；引擎库22项、Mac149集成5项及 WASM边界11项均通过。实际 release WASM 构建及导出验证另行记录；这些局部检查不替代生产包、完整功能和原性能验收。

下一独立判别 `Mac149-glow-sampler-basis-v1.zip` 保持原 99,306 项 half/float sampler 输出为严格前缀守卫，使用同一已在 Mac 运行的 shader/atlas 实现；新增19组已知基向量、指数差、同指数 mantissa 和四个独立缩小反例，覆盖全部257×257相位，共1,254,931项。另以三种原始尺寸、shrink1～24保存72个独立缩小平面及 unfiltered 行方向对照，不再重跑整套 Glow。新数值未知，不以占位零差异作为成功断言；全部原参考保持。12项实际 Bash哈希、语法、ZIP内容/CRC/权限及全部输入四texel/Q8相位检查通过，Mac编译/新结果仍 `NOT_RUN`。包3,766,455 bytes，SHA256 `aec6afd76b7d9b604f083646f2fb46dab1b63d94d0381a749460c0ab3d798f56`。中文 `MAC-STEPS.md` 随包，无需手动调参数。

FMA 修正的实际生产 release WASM 已构建并通过二进制/108 个 JS glue 调用导出核对：16,987,539 bytes，SHA256 `86f21ed2ae923061e0a926c77807f695ef5759e681ce9dca37e7efec6214b68d`。Node 实际装入该生产二进制，直接调用 `phase7_mul_addf` 与 `compositor_exp`，用逐步独立 Float32 舍入重放周围 C 标量计算，真实 Mac bloom 的 31 项最终合成全部精确，结果为 `glow-fma-wasm-runtime-check-20261015.json`。该检查验证真实 WASM 数学边界，不是完整 C 调用图或当前 Gaussian 路径，也不替代新的 portable 和完整验收。原 107ms Fill 失败继续保留。

三项 TypeScript 检查和前端生产构建也通过，日志 `glow-fma-web-build-20261015.log`；仅保留原有未使用参数/包体大小警告，没有放宽配置。最新独立汇总 `acceptance-status-composite-fma-basis-prepared-20261015.json` 将本轮局部验证与历史 `3926872` 完整包验收分开；当前仍未生成新 portable，1.4.9 增量 `INCOMPLETE`。

## 2026-10-15：缩小有限域验证、绘线修正与格式判别

`Mac149-glow-sampler-basis-20261015-153432` 已完成编译执行，独立核对 12 启动哈希、11 收据文件、完整输入拼接、19 个 quad 的全部257×257相位。half/float 两个输出共1,354,237样本逐字节相同且均可无损表示为half；原99,306项严格守卫不变。72个缩小平面均有限、非负且尺寸正确，三项shrink1的无限边缘及行方向与源RGBA8精确一致。证据 `glow-sampler-basis-153432-return-review-20261015.json`；收据中的旧 `NOT_RUN` 不覆盖实际执行结果。

缩小模型修正了两个结构问题：shrink9/17/18的实际路径比旧floor(log2)少一级；shrink2奇数高度须按原始行相位直接采四个RGBA8像素，不能先做固定mip再插值。当前全部23种合法整数shrink在三种尺寸上的69/69平面精确；采用覆盖完整有限域的LOD状态目录，不声称已确定Apple连续缩放阈值。正好2的幂的相邻LOD在现有输入上不可区分，选用较低层亦全部精确。`20261015-reduction-domain-catalog.py/.json` 提供纯函数与源哈希。由 supplied drawn RGBA8 计算Gaussian最终纹理31/31精确；实际MPS输入还有四个half元素不同，186个阶段仅一个stage0元素不同，下一阶段消失。这不等于输入/全阶段已精确，也不代表模型已集成产品。

19个采样网格中14个精确，尾数敏感的五个网格共7,800个分量不符。独立控制否定了max-input/product共享指数截断、共享RGBA指数、普通IEEE F32 RNE/FMA、串行/成对求和、带符号lerp及仅排除零权重输入等候选；停止扩大这类拟合。即使到RGBA8，九图naive仍差62字节、前轮e−19近似仍差22字节，不能用于严格通过。记录 `20261015-sampler-basis-verdict.md` 及关联JSON，保留所有反例。

Windows额外端到端绘线检查另发现两个真实问题：native和生产WASM的Glow0均为29/31，spacing24的[90,101,R]和spacing29的[51,94,B]各差一字节。四个独立C对照证明只需把beam因子`0.29+0.71*sqrt(level)`写成显式FMA；仅修改灰度加权没有修复，所以后者保持。两个可生成的1列24/29像素回归先在旧代码实际失败（exit101），之后只在beam表达式复用已有Rust `phase7_mul_addf`。原生和真实生产WASM的31组绘线全部精确，原四组非Glow配方也各4/4精确。新结果不再把Mac预先绘制数据当成Windows绘制证明。源码回归/集成/边界23+5+11通过；完整Glow仍待独立采样规则与最终实现。

新release WASM已构建并核对108个glue调用：16,987,544 bytes，SHA256 `e632bfc78b25ac6a8fa4e164aa19c9c0c5c0f525e65d62ba34c5911d5e280fba`。`drawn31-wasm-comparison-beam-fma-20261015.json` 使用真实WasmEngine的paste/ApplyFilter/层像素API；`glow-fma-wasm-runtime-check-beam-fma-20261015.json` 另行确认既有31组合成数学边界精确，不能代替完整C调用图或Gaussian。native对应结果及四个旧配方对照均保留。未生成新portable，旧原100MP Fill107ms失败与完整包/source绑定保持。

下一判别包 `Mac149-glow-format-matrix-v1.zip` 固定相同输入/相位，仅切换RGBA16F、R16F、RGBA32F、R32F；新增格式不假定与旧格式等价。沿用字节不变的原Metal shader，RGBA16F全部1,354,237项必须保持；31,744个非负有限half→Float转换保存实际缓冲并逐字节自测，四个独立texel中心必须精确。32-bit过滤按[公开设备能力](https://developer.apple.com/documentation/metal/mtldevice/supports32bitfloatfiltering)自动判断；不支持记录UNAVAILABLE，不计通过。记录设备/能力和Swift版本，以区分格式转换与滤波运算，避免继续在同一近似族中穷举。本机9项实际Bash哈希、语法、ZIP内容/CRC/权限及输入/旧shader字节检查通过；包10,343,545 bytes，SHA256 `90563af4a9e125c32a021c04bc14894ae95dfd6e8b6d64c3cfb249a4737f8580`。中文步骤已提供，Mac新编译/执行仍 `NOT_RUN`，用户回传待核对。

当前beam源码的三项TypeScript/前端构建已通过，`app/dist/assets` 内的WASM资产与上述e632二进制哈希一致。新汇总 `acceptance-status-basis-beam-format-matrix-prepared-20261015.json` 保留两次回归失败、完整范围对照及源码哈希；它只描述当前局部验证，不沿用旧 `3926872` 的完整包通过结论。完整Glow、最终portable与原性能重新绑定仍未完成。

## 2026-10-15：Gaussian Rust 组件验证，产品集成继续保留门槛

先更新 PRD 后，将六阶段运算和反推工作矩形移植到 `engine/src/adjust/scanlines_glow.rs`。模块仅由 `cfg(test)` 及独立诊断编译，产品 `ScanlinesSettings.apply` 尚未切换；没有引入近似二维 sampler，也未改其它效果 blur。显式实现正有限 half 解码、RTZ store、采样中点向上舍入、Q8 相位、F32 FMA，以及 U2 系数的分别舍入。half 存储使用 packed u16，尺寸乘法/单平面预算先检查，分配使用 try_reserve；链只保留输入与当前/下一阶段，诊断流式写出，避免同时保留六个完整中间平面。

参数目录 `scanlines_glow/catalog.rs` 覆盖全部31个标准化整数间距，直接记录十组已观察 Float32 内核、external shrink、真实 halo 和 constructor sigma 位模式。转录时重新核对全部31个实际 report/encode/参数缓冲，回执 `gaussian-rust-catalog-transcription-20261015.json`。halo 保留实际值，不从 Float32 sigma 再取 ceil 推断；目录不接受其它间距，不作为任意 sigma 或其它效果接口。

真实 native release 组件对照 `gaussian-rust-native-v3-comparison-20261015.json` 和独立优化 WASM 对照 `gaussian-rust-wasm-v4-comparison-20261015.json` 各1,358/1,358项逐字节一致：原163组加组合路径31组，捕获矩形六阶段连续计算的1,164个实际平面精确；另从最终尺寸、offset 和抽头支持推导最小矩形的194个最终平面精确。输入仅在链开头读取一次，没有每一级喂回 Mac 的正确上一层；对照直接读取真实回传文件，未把 Python 数值候选当 oracle。独立 WASM 1,521,612 bytes，SHA256 `c01950c5f782fb2e8b0bdbaf35404c6dc382eee1b38911e4db1fdc55626aaed8`，它是独立 Rust 组件诊断，不能冒充生产 WasmEngine 接口或新 portable。

引擎库29项通过，包括六项新回归：全部31,744个正有限 half 位模式/相邻中点及其下一个 binary64 邻居、真实常量场的一 half ULP store 损失、实际 D2 half 采样中点反例、无效数据/尺寸/参数/中止传播和完整有限间距域。原生与 WASM 的源组件及目录哈希一致，运行前冻结的源/输入哈希在对照时再次检查；独立 runner 和编译产物另行绑定。早期独立 WASM wrapper 相对路径编译失败保留为 `gaussian-port-wasm-v3-build-20261015.log`，仅修正诊断路径后 v4 实际编译和全部对照通过，未改变数值组件。

原 Mac149 集成五项亦通过，包含 Scanlines alpha/一个 Undo、物理单位/导出、旋转和效果保留。剩余外部缩小/最终二维采样仍需精确规则；当前没有新的 `Mac149-glow-format-matrix-日期-时间` 回传目录。原100MP Fill107ms失败、完整产品 Glow、新 portable 和当前源码的原性能协议均保持独立待验收。本轮没有运行新的硬件性能重试、改动门槛或发布。
