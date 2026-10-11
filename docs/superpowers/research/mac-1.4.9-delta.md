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

上述差异表是实施前的源码研究快照。当前实施与分层验收进度见 `prd.md`；Mac 1.4.9 回传尚待核对。

## 验收基准的版本变化

- Camera Raw 的五项旧公式用例及一个预览提交中的曝光数值改为固定 1.4.9 CRT3 数据的独立三线性插值检查；默认曲线分界走实测 LUT，移动分界仍走独立路径。旧版本历史结果保留。
- Image Size 的三个效果方向用例跟进固定 `ImageResizer.swift:26–31,101` 和 `LayerEffects.scaled`：重采样后距离/模糊/描边使用 `sqrt(sx*sy)`，阴影角度保留。原旋转/翻转投影规则不再作为新版期望；新增 DPI-only 与非等比 Undo/Redo 检查。
- 旧 `mac-1.2.10-probes` 中 `effects-inner-shadow`、`effects-inner-glow`、`effects-color-overlay`、`effects-all-six` 的 PNG 和项目保持不变。1.4.9 `MetalLayerEffects.swift:369–389` 先改变源 RGB、保留源 alpha，再置于阴影之上；这些四项当前输出改与独立全平面 Metal 公式对照，并断言确实不同于旧 PNG。其余旧像素界限保持不变；这不能替代安装版 Mac 1.4.9 导出验收。
- 原 29 项 CPU/UI 和 4 项 GPU 性能断言、时间预算、单 worker、零重试保持不变。所有失败日志保留，新运行写入独立目录。
