# Mac 1.4.9 → 1.5.7：Windows 更新核对

## 最终验收状态（2026-10-25）

Windows 0.11.0 的七阶段原机验收全部通过：源码17步、功能361项、CPU29项、GPU4项、便携包与真实RAW、25对话框原生回归、16对话框1.5.7原生增量。1801项冻结输入和ZIP25项文件核对通过，总控实际退出0，锁屏策略未改。完整结论见仓库根目录 `pair-test-report.md` 和 `docs/releases/0.11.0.md`。

下文为按时间保留的实施和故障调查记录，其中“待验证”“未完成”等描述属于当时状态，不覆盖本节最终结论。HEIF已改为应用自带解码；高精度/空间滤镜的CPU参考路径、Core Image近似及RAW平台差异仍是已披露的实现限制。正式发布使用干净提交的Windows流水线独立构建验证。


2026-10-24。基线 Windows 0.10.0 / `1af575a32e75e0542855cc16b8ce1616c29fbcb2`。

## 固定来源

- [上游 1.5.7](https://github.com/robbietilton/Compositor/releases/tag/v1.5.7)：`9537736da6c324b948cdcba24adf74760498ea14`。
- [完整比较](https://github.com/robbietilton/Compositor/compare/v1.4.9...v1.5.7)：138 个提交，165 个文件。
- Mac 1.4.9：`0ab3ff58a3dcf3d52a455fe3ab0336ac95923edd`。
- API 响应、源码压缩包及逐文件 SHA256：`build-artifacts/mac-1.5.7-delta-20261024/`。
- 上游 MIT 授权保持。以固定标签源码为准，不跟随持续变动的 main。

## 大变化与验收范围

| 版本 | 上游变化 | Windows 对齐工作 |
|---|---|---|
| 1.4.10 | PatchMatch 重写内容识别填充／污点修复；目录导入；模糊透明度；RAW 降噪、镜头校正、高光恢复；HIF/HEIF | 内核替换与独立参考测试；原生导入及 RAW 适配 |
| 1.5 | 16 位图层、HDR EXR/HDR、8/16 位转换；CMYK 软打样／色域警告和 JPEG/TIFF/PDF；TIFF；新建时粘贴及标签栏拖入 | 高精度像素、保存／撤销／合成／导出链路；色彩管理；导入交互 |
| 1.5.1 | 渐进模糊，拖动控制点；高精度调整层预览 | 新滤镜及交互，高精度合成与显示 |
| 1.5.2 | Camera Raw 自适应 Shadows；RAW 直接进入滤镜；分组重置；快捷键清空、持久化、Delete 重绑定 | 新 Shadows 数据及前后 LUT 顺序；核对已有分组重置；快捷键迁移 |
| 1.5.3 | 全分辨率 Camera Raw GPU 预览及 scope 提速；平滑矩形角；图层中键滚动；缩放光标 | 跨平台 GPU 实现及原预算性能测试；形状与交互 |
| 1.5.4 | 画笔压感；PDF 每页一层；虚线／点线／方圆端点；字体搜索 | Pointer Events 压感；原生 PDF；可编辑形状；字体面板 |
| 1.5.5 | RGB Split；渐进模糊／Bloom／RGB Split 调整层；Channel Pack；PSD 画板独立标签；非拉丁键盘；缩放保持 | 新滤镜／调整层／打包导出；PSD；快捷键及视图 |
| 1.5.6 | GIF 单图层帧序列、播放、逐帧编辑／复制／删除／定时、导出；图层搜索／类型；双列工具条 | GIF 完整保存与编辑链路、时间轴；图层与布局 |
| 1.5.7 | Lens Effects（耀斑、星芒、柔雾、棱镜、多形散景）；GIF 实际速度；RGB Split 双向模糊 | 图像衍生光效、帧调度、对称模糊 |

## 文件格式是关键变更

[固定源码格式说明](https://github.com/robbietilton/Compositor/blob/9537736da6c324b948cdcba24adf74760498ea14/docs/project-format.md)：新保存从 v11 升到 v17。

- v12：16 位 PNG；v13：EXR HDR。
- v14：平滑矩形角；v15：线型与端点。
- v16：三种新调整层及 filterLayerSettings。
- v17：GIF 原始资产、帧编辑、播放顺序、时间与几何变换。

必须实际实现相关像素／资产链路后才提高读写版本。不能只提高版本号而让打开／保存悄悄丢失精度或动画。

## 实施与证据状态

更新分支 `codex/mac-1-5-7`。本文件是实施清单，不是验收通过声明。旧版 0.10.0 和 1.4.9 验收证据保持原样。

- 已核对：Windows 原有 Camera Raw 各分组重置和临时禁用；仍需与 1.5.7 的所有默认值和状态清理逐项比较。
- 已移植并通过参考核对：PatchMatch／Healing（20 组）、Camera Raw Shadows（14 组）。
- 已接入：图层名称／类型筛选、中键滚动、紧凑双列工具栏、小图不自动放大、尺寸调整保持缩放、缩放工具光标即时更新；332 项前端单元测试、生产前端构建及测试代码类型检查通过（后续修改需重新执行）。
- 已接入高精度链路：16 位 PNG、HDR EXR 读写，v12/v13 版本门槛；后台传输、画笔未改区域、撤销、保存重开、合成、缩放、PNG16／TIFF／EXR 导出已接入。新集成测试覆盖细微色阶和 HDR 高光。实际 WASM 的导入／后台任务／分段安装／撤销／重开／剪贴板／高精度导出已通过；大图合成加入带空间滤镜边缘扩展的分段浮点工作区并通过整图对照。GPU 与原机性能仍待验证。
- 已发现并修复：扩展图层后抖动坐标变化误判未绘制像素；深色缓冲区被后台 8 位传输及调色板压缩截断。
- 已接入并验证核心行为：压感尺寸／不透明度、快捷键持久化／清除／Delete 重绑定／非拉丁布局、字体搜索、无文档粘贴建画布、标签栏独立导入；目录导入遵循上游第一层文件规则，天然数字排序（原生模块 2 项测试）。
- 新形状与 v14/v15：平滑圆角、Solid／Dotted／Dashed／Long Dash、Square／Round；保存、重开、缩放重绘及版本门槛已接入。52 组直接编译原始 Swift/Core Graphics 的参考图；独立亚像素面积测试通过。Windows 几何面积覆盖与 Core Graphics 的细线抗锯齿不同，1 像素圆端虚线边缘最大约 108/255；这不是像素完全一致的声明，普通 5 像素线／圆角参考差异不超过 12/255。参考脚本和固定源码哈希可重放。
- RGB Split 的四组原始 Metal 参考通过：固定 GPU 随机采样时每通道差不超过 2/255；CPU/GPU 的 sin 随机纹理在少量像素不同，另以均值／99 分位检查分布。已接入调整层；局部重绘把随机采样固定在文档坐标，避免区域变化导致闪烁。当前 CPU 实现，GPU 内核仍待接入。
- Channel Pack：四通道独立图层／黑／白、隐藏图层及父组隔离、灰度权重、透明像素下 RGB 保留、预览不受打包 Alpha 隐藏，独立 PNG 解码和 300 DPI 测试通过。
- Gaussian Preserve Transparency：原始 Alpha／轮廓保持、颜色按模糊后的 Alpha 归一化，8／16／HDR 三条路径测试通过。画布边缘延拓已实现：仅贴边／越界的未旋转图层延拓对应侧，内部边缘向透明区域扩散；8／16／HDR 及单侧边界测试通过。
- RAW：完整导入改为 RAW8 16 位通信与 PNG16，缩略预览仍为 RAW7 8 位；新增 LibRaw 波形降噪 threshold（0–1000）和高光恢复（highlight=3）。Mac 使用 CIRAWFilter，Windows 使用 LibRaw，不能称为相同显影。已增加显影后进入 Camera Raw 按钮，4 项浏览器流程测试通过。上游普通图层的镜头配置是通用强度，并无独立镜头数据库；CIRAW 的原始文件镜头校正能力是平台差异。C++ helper 必须在 Windows 重新编译和实测。
- 渐进模糊：线性／径向、From／To／Invert、画布端点拖动、重置保留位置、预览／提交／撤销、16 位与 HDR 已接入。Windows 使用相邻 Gaussian 方差层插值及大半径降采样，Core Image 的 CIMaskedVariableBlur 是专有内核；功能行为测试通过，尚不能称为像素等价或性能验收完成。
- Bloom / Glow：独立 Core Image 三组含透明度参考的逐通道最大差 1／1／2，均差 0.0465／0.0476／0.2798；保留原精度。渐进模糊／Bloom／RGB Split 三类调整层的 v16 保存、重开、撤销和局部重绘测试通过。
- 高精度文档及三种新调整层目前在 WebGL 窗口中显示 CPU 合成结果，测试 API 明确报告 `cpu-reference`，不能把此路径算作 GPU 渲染验收。普通 8 位旧调整层保留原 GPU 路径。深色空间调整清除画布外像素，防止边缘被画布外图层污染。
- 最新完整 Rust engine + WASM：795 通过、10 原有忽略，包含 Lens、CMYK、PSD 导入及高精度编辑修复。Tauri shell 在 macOS 已通过 22 项测试（不代表 Windows FFI 已执行）。
- 浏览器交互已实测：形状工具栏、渐进模糊端点、v16 调整层、快捷键保存／清空／Delete、Channel Pack 透明 Alpha 下的 RGB 预览；新增 3 项 Playwright 测试全部通过（Mac Chromium，非 Windows 原机）。新调整层窗口显示与同尺寸 CPU 输出逐字节一致。
- GIF / v17 已接入：不可变原始 GIF、顺序解码缓存、透明度及 disposal 合成、稳定帧 ID、逐帧编辑／复制／删除／排序／时长、撤销、Image Size 几何、整图层剪贴板、项目保存重开、时间轴与基于经过时间的播放、动画导出。导出使用多轨共同的 10 ms 时钟，短轨保持尾帧；4096 帧／总计 100 MP／512 MiB 输出上限。8 项核心测试与独立 image GIF 解码检查通过，新增 3 项浏览器测试通过。Windows shell 的资产白名单已支持 UUID.gif / UUID.frame-N.png，仍需 Windows 原机文件对话框实测。
- GIF 可视化检查发现首次切换帧复用 revision=1 导致 GPU 保留旧纹理，已修复为唯一版本号与整帧失效；新增画布颜色检查在旧实现上失败、修复后通过。手工观察新帧即时显示，导航保持 undoDepth=0 / isModified=false。滤镜临时扩张还原动画矩形与原始蒙版，GIF 帧禁止转换为 16 位。
- Lens Effects 五类光效及全部控件已接入，4 项 Rust、2 项浏览器测试通过。10 组固定上游 Swift / Core Image / Metal 参考及 5 组色调曲线探针可重放。Bokeh 四类光圈逐通道最大差 ≤1/255，Mist ≤5/255；Anamorphic / Starburst / Prism 的旋转与缩放插值不同，最高差分别 16 / 77 / 34，仍为近似实现，未宣称像素完全一致。新增内存工作量预检，拒绝时原图不变。参考文件 `lens-reference.json`。
- 跨项目拖图层与新建画布按钮已接入；选择后代、内部关系、遗漏剪贴源烘焙、蒙版坐标、旋转边界、撤销通过 4 项 Rust 和 2 项浏览器测试。
- PSD 画板按文件顺序拆为命名独立标签，保留尺寸、内部文件夹及原项目；PSD 相关 16 项测试通过（包含 2 项新 CMYK 通道测试），画板浏览器测试通过。核对 Mac 1.5.7 的 PSDReader：仍只接受 8 位，因此 PSD 16 位不属于这次对齐范围。
- PDF 原生分页（新建 300 DPI，现有画布等比适配，每页一层）、HEIF/HIF 的 Windows 系统 16 位解码已接入，NIM1 有界传输；导入/失败原子性/16 位色阶的 3 项 Rust、2 项协议单元测试及 PDF 模拟原生返回的浏览器测试通过。Windows API 交叉编译通过，系统解码器尚未在 Windows 执行。PDF 尺寸按 [Microsoft 的 96 DPI 页面单位](https://learn.microsoft.com/en-us/uwp/api/windows.data.pdf.pdfpage.size?view=winrt-28000) 换算；依赖系统 HEIF 编解码扩展。
- CMYK：Windows ICC 相对色度转换、33³软打样／色域警告、已安装或用户载入的输出配置文件、JPEG/TIFF/PDF 嵌入 ICC；文档仍为 RGB。4 项编码/元数据 Rust 测试、3 项查表单元测试、3 项浏览器交互测试通过。Windows API 交叉编译通过，但 WCS/WIC 的实际色值及文件阅读器仍待 Windows 原机检验。注意 [Windows BMFORMAT 的字节顺序](https://learn.microsoft.com/en-us/windows/win32/api/icm/ne-icm-bmformat)：RGB 字节用 BM_BGRTRIPLETS，CMYK 字节用 BM_KYMCQUADS。
- CMYK PSD 分层转换保留 Alpha／蒙版，PSD 原始墨量取反；JPEG/TIFF 经 WIC 读取和色彩转换，优先内嵌 ICC，无配置文件时要求用户选择印刷配置。NCP1 传输在引擎验证全部资产后一次性导入，支持画板；重新映射 UUID，避免同一资产多次导入冲突。
- 已修复并验证：高精度透视重采样、浮动选区合并、局部清除、修复/液化未编辑区域；新增 4 项保持及撤销测试。缩小的滤镜预览不再把亚像素半径重新钳制到原图范围，新增回归测试通过。仍待验证：新滤镜实际性能及平台近似差异。本地 Rust/WASM 和前端检查通过；Windows CPU/GPU、生产包及原生验收未完成，不能把交叉编译或模拟原生响应算成 Windows 验收。
- Windows 原机仍采用用户选定的有人值守流程，保留公司锁屏策略。

## 0.11.0 候选的 Windows 验收准备

- 入口：`node "C:/Users/sr9rfx/.claude-project/compositor-windows/scripts/resume-mac157-attended.cjs"`；准备完成后才交给操作者运行。
- 逐阶段确认：源码/Windows 系统解码器、浏览器功能、原 29 项 CPU、原 4 项 GPU、便携包与 RAW8、原生回归、新功能原生界面。12 项控制流程检查及 12 项发布/WASM 守卫通过；不会在确认超时后自动执行，不覆盖失败证据。
- Windows WIC/WCS/WinRT 与原生验收测试代码的交叉类型检查通过。此检查在临时目录跳过 C 编译/链接，不能代替实际 Windows 编译或运行。
- 本地打印配置文件只放在忽略的测试工件中，用于独立 LittleCMS 与 Windows 转换比较，不打入程序或发行包。
- 屏保/锁屏策略只读核对；浏览器和性能阶段持续采样单屏/解锁状态，不发送模拟保活输入。
- 一次本地浏览器回归发现新增工具栏样式影响原有 Transform inspector 高度，造成 5 项 GPU/CPU 像素对比失败。收窄 CSS 后相关 7 项全部通过，原容差未改；最终全量复核 361 项功能通过、0 失败、0 重试；33 项性能选择性跳过，将在 Windows 分组执行。

- 新增选区回归定位到关闭数值对话框后的画布坐标未同步：工具栏已恢复高度，ResizeObserver 尚未更新视口。将 sheet 纳入布局同步依赖后，选区/旋转/布局相关 22 项通过；删除前位置及删除后 Alpha 均有断言。旋转测试按 1.5.7 的“保持缩放并居中”更新，未放宽像素或性能容差。
- Windows 候选已冻结 1179 项输入，原性能测试与 0.10.0 提交逐字核对（仅归一化换行）。当前本地日志与失败历史位于 `build-artifacts/mac157-acceptance-20261024/local-preflight/`；Windows 阶段尚未启动。
- CMYK TIFF 导入目前转为 8 位 RGB；16 位 CMYK 原精度导入尚未实现。HEIF 依赖 Windows 系统扩展，实际可用性以原机测试为准。

- 最终本地预检：795 项 Rust engine/WASM、22 项 macOS shell、332 项前端单元、361 项浏览器功能、24 项控制/发布守卫通过；三套 TypeScript 类型检查与生产前端构建通过。WASM SHA256 `d3778ab78950bdb033963107d942a3affcd42059742e2b5a9ec15ab1d22f7400`。汇总为 `build-artifacts/mac157-acceptance-20261024/local-preflight/summary.json`。已请求操作者启动 Windows 有人值守入口，尚未收到原机结果。

## 首轮 Windows 原机结果（2026-10-24）

- 真实 Windows WASM 发布构建及 117 个接口验证通过，高精度传输/编辑/撤销/保存重开/剪贴板/PNG16/TIFF/EXR 测试通过。WASM SHA256 `89f74d1bbe90d9b34a08b8e3f33182a7eb221591d8a966d7803cc09b5c4c19ee`。
- 原生 engine/WASM 795 通过、0 失败、10 原有忽略；Windows shell 24 通过、0 失败，另 1 项真实系统验收在本阶段显式另行运行。三套 TypeScript 检查、前端生产构建及 332 项单元测试通过。
- 真实系统验收在首个 PDF 页面尺寸断言失败：72×48 pt 页面应为 300×200 像素，实际为 420×280。尚未运行到 HEIF/ICC，不将它们计为通过；浏览器/CPU/GPU/包/原生界面阶段尚未启动。旧断言保持不变。需诊断 Windows 页面 Size 与渲染目标的缩放关系。
- 本轮锁屏策略核对未修改。失败日志和最初冻结输入保留于 `build-artifacts/mac157-acceptance-20261024/`。已准备只读 `scripts/diagnose-mac157-pdf.ps1`，等待操作者运行。

## PDF 缩放诊断与修复候选（2026-10-24）

- 原机只读诊断 `mac157-pdf-diagnostic-20261024T073715216Z` 确认页面 Size 正确为 96×64 / 48×96 DIP；`RenderToStreamAsync` 的 300×200 请求在 PowerShell 中输出 450×300，在测试宿主中输出 420×280。偏差来自宿主缩放，而非 PDF 页面单位。HEIF 解码器可读取 48×32 元数据，完整 16 位解码仍待复测。
- 改为 Windows 原生 PDF interop + 显式 96 DPI / 像素单位 Direct2D 位图，WARP 软件设备，2048 像素分块，保持原 100 MP / 30,000 像素限制。新增横纵分块边界回归，原 300×200 与 64×43 断言不变；不调整桌面 DPI 或锁屏策略。此路径是 CPU 软件渲染，不能计为 GPU 加速。
- 修复及测试的 Windows API 交叉类型检查通过；尚待 Windows 实际编译和渲染。控制检查现为 28 项通过。
- 续跑只允许经逐文件哈希审阅的 PDF/native 及控制脚本变更，验证旧日志、单元报告与全部 WASM/前端产物哈希后，复用先前成功的 795 核心测试、WASM、三套类型检查和 332 前端测试。Windows shell、真实系统解码器及独立色彩阅读器重新执行；原失败记录完整保留，不转换为通过记录。

## PDF 修复的 Windows 实测结果（2026-10-24）

- 值守尝试 `attended-2026-10-24T07-46-37-505Z`：28 项控制检查、重新编译后的 24 项 Windows shell 测试通过；已核验的核心/WASM/前端成功结果按冻结记录复用。
- PDF 真实渲染全部尺寸/色值/预算断言通过：300×200、150×300；适配 64×43、32×64；跨 2048 像素分块的 2300×1533、1150×2300。独立 Pillow 读取 6 张 PNG 核对尺寸、中心色及 300 DPI，通过回执 `pdf-render-independent-verification.json`。生产 UI 导入还未执行，不能将渲染测试等同于完整原生验收。
- 测试随后在 HEIF 的实际像素请求中失败，错误 `0xC00D5212`（找不到合适的编解码转换器）。之前可读取尺寸/像素格式仅证明容器和元数据可读。样本为有效 48×32、8 位 HEVC Main Still Picture；待只读诊断区分默认/8 位/16 位像素请求及 HEIF/HEVC 扩展状态。ICC 仍未执行，后续阶段未启动。
- 本轮完整保留失败结果，锁屏策略前后相同，未修改系统设置。新增 `scripts/diagnose-mac157-heif.ps1` 只读取目标编解码扩展信息并解码固定的 PNG/HEIC 测试样本，不安装组件。

## HEIF 系统依赖诊断（2026-10-24）

- 只读报告 `build-artifacts/mac157-heif-diagnostic-20261024T080530156Z/result.json`：当前账号安装 `Microsoft.HEIFImageExtension 1.0.63001.0`，未列出 HEVC 扩展。PNG 的默认/BGRA8/RGBA8/RGBA16（含两种色彩管理模式）7 项全部成功；同样的 7 项 HEIC 像素请求均报 `0xC00D5212`。这排除了仅由 16 位输出或色彩管理选项引起的问题，与缺少可用 HEVC 解码器一致。
- 需要先安装/修复系统 HEVC 解码能力，再按原 16 位输出断言复测。官方入口：https://apps.microsoft.com/detail/9nmzlz57r3t7 。价格和可用性以操作者商店页面为准；没有自动购买或安装组件。微软 WIC 文档：https://learn.microsoft.com/en-us/windows/win32/wic/heif-codec 。
- 产品现在在像素阶段收到 codec-not-found 时也给出检查 HEIF/HEVC 扩展的明确提示，保留系统错误详情，不降级为 8 位来绕过验收。
- 续跑守卫增加原始复用日志与原始输入清单的链式核对，29 项控制/发布/WASM 检查通过。HEIF、ICC、浏览器、CPU/GPU、生产包及原生界面仍待完成。

## 改用应用自带 HEIF 解码（2026-10-24，Windows 待验收）

- 操作者明确选择无法安装系统 HEVC 扩展，改用应用自带方案。此前“需要安装扩展”的建议及系统解码路径已被本节替代。应用随包附带 CompositorHeif、libheif 1.23.6 与 libde265 1.1.3，使用可替换 DLL；提供完整固定源码归档、许可证及同一构建脚本。官方归档 SHA256 固定，不依赖商店或加载系统 HEVC 插件。
- 隔离进程输出 RGBA16，保留 10 位源细节；支持主图旋转、裁切及透明度。100 MP/30,000 像素/512 MiB 输入限制，180 秒超时及输入不覆盖检查。ICC 由 Windows 的 16 位色彩转换应用一次；此路径仍须原机验证。HDR gain map 重建未实现。
- 本地用同一 CMake 工程编译并独立读取结果：8 位色条、10 位渐变、90 度旋转、透明度、确定性、坏输入、像素预算及拒绝覆盖均通过。10 位渐变全部像素相对 Apple 参考图最大差 1、均差 0.078（8 位尺度）；4:2:0 色条与 Apple 色度重建不同，完整图最大差 85/旋转后 65，条带中心最大差 2/3。测试明确记录边缘差异，不能据此宣称逐像素对齐。
- 本地 25 项 shell、31 项发布/WASM/值守/续跑控制检查通过；Windows API 交叉类型检查通过，但没有在 Mac 执行 Windows 二进制。PowerShell 语法和 CMake 参数传递通过实际 PowerShell 解析检查。源码、许可证和重建回执校验通过合成组件检查，不等同于 Windows 包通过。证据位于 local-bundled-heif-preflight。
- 两次失败历史分别完整保留于 source-failed-pdf-dpi-20261024 与 source-failed-hevc-codec-20261024；PDF 独立验证回执原 source/system-codecs 路径对应后一个归档目录的 system-codecs。仅复用未改动的核心/WASM/前端成功记录；Windows shell、内置解码器、真实 PDF/HEIF/ICC 重新执行，随后继续功能、原预算性能、生产包和两组原生界面。锁屏策略仍保持不变。

## 内置解码首次 Windows 构建与参数修复（2026-10-24）

- 尝试 attended-2026-10-24T08-35-29-064Z 中，31 项控制检查、重新编译的 27 项 Windows shell 全部通过。CMake 配置阶段失败：Windows PowerShell 5 的 ValueFromRemainingArguments 把传入数组串为单个参数，源码路径包含后续所有参数。没有执行解码器或其像素检查，不能计为 HEIF 通过。锁屏策略前后完全一致。
- 将 9 个 configure/build/install 调用改为显式绑定 string[] 参数；新增真实原生 argv 读取检查，逐项比较带空格的 Windows 路径，并检查非零退出传播。该检查加入 Windows 源码阶段和 CI；本地 PowerShell 的 Legacy/Standard 模式通过，但不冒充 Windows PowerShell 5 实测。
- 完整失败、输入清单、控制回执保留于 source-failed-heif-build-arguments-20261024。后续仍复用原始未变核心/前端证据，重新运行 Windows shell、参数检查和解码构建。

## 参数检查程序的跨版本调整（2026-10-24）

- attended-2026-10-24T08-49-07-531Z 再次通过 31 项控制与 27 项 Windows shell，但新增参数检查在 Legacy 分支报告数量不一致，因此未进入 CMake。此次日志未打印原生参数，不能仅据此确定构建调用仍有问题。失败完整保留于 source-failed-heif-argv-check-20261024，锁屏策略前后不变。
- 为排除 PowerShell 5/7 的 ConvertFrom-Json 管道数组展开差异，将比较移入接收参数的 Node 进程，打印 expected/actual 并在任何差异时返回 18；新增故意合并参数的反向检查，要求确实返回 18。只更改此检查程序，构建调用保持显式 string[] 参数绑定。本地 Legacy/Standard 检查、反向检查及 31 项控制通过；仍须 Windows PowerShell 5 实测，不能计为 HEIF 解码通过。

## Windows 参数验证成功与 C++ 头文件修复（2026-10-24）

- attended-2026-10-24T08-55-26-894Z：27 项 Windows shell、31 项控制通过；Windows PowerShell 5 实测确认原生参数逐项相同，错误参数与非零退出也按预期拒绝。libde265 41 步、libheif 98 步编译成功并生成 DLL，配置仅启用 libde265 HEVC 后端。
- 最后 CompositorHeif 编译停在 std::stoull：MSVC 要求显式包含 <string>，本地工具链的间接包含曾掩盖此问题。补齐 <string>/<new>；同时在消费者包含 libheif API 前取消上游 CMake PUBLIC 传播的 LIBHEIF_EXPORTS，明确按 DLL import 使用 API。
- 本地同一 CMake 工程重编译及全部独立像素检查通过，31 项控制检查通过；Windows 辅助程序编译、HEIF/ICC 像素测试、后续功能/性能/生产包/原生界面仍待验收。当前构建脚本只缓存完整成功运行包，本轮成功 DLL 已保留为证据，但尚不复用半成品缓存。失败完整保留于 source-failed-heif-msvc-header-20261024，锁屏策略未修改。

## 内置解码原机通过，ICC 转换待诊断（2026-10-24）

- attended-2026-10-24T09-19-53-374Z：修复后的 CompositorHeif.exe 成功通过 MSVC 编译和链接，完整运行缓存已生成。独立 Windows 像素检查通过 8 位、10 位、旋转、透明度、确定性、预算、坏输入及拒绝覆盖；四个 HIF1 输出 SHA256 与本地相同。辅助程序 SHA256 为 ebfbe350a02303a6d2aac2d8c3fbbf6eeda61f3ce1ff8202be776c5b42560a08。
- 原生测试通过 PDF 和 HEIF 普通/高位深检查后，停在 Adobe RGB ICC 转换：WIC 输出与 Apple 参考最大差 22、均差 2.472，超过原最大阈值 12，未放宽阈值。之前测试先断言后保存，所以本轮未保留失败的 ICC PNG；后续需补强失败诊断留存。
- 同一解码数据在本地通过独立 LittleCMS 转换，最大差 7、均差 1.111，符合原阈值。准备只读 Windows 诊断，比较 16 位 normal、best、WCS best 和 sequential 路径并保存像素；不改变产品实现或判定哪条路径正确，等待原机证据。
- Windows API 参考：https://learn.microsoft.com/en-us/windows/win32/api/icm/nf-icm-translatecolors 与 https://learn.microsoft.com/en-us/windows/win32/api/icm/nf-icm-createmultiprofiletransform 。诊断输出仅写新工件目录，不改系统或锁屏设置。

## ICC 诊断与内置转换修复（2026-10-24，Windows 复验待运行）

- 原机诊断 mac157-heif-icc-diagnostic-20261024T093311648990Z：normal/best/WCS best/WCS sequential 的最大色差为 20/21/19/21，均未达到原 12 阈值；独立 LittleCMS 为 7、均差 1.111。切换 Windows 转换模式不足以解决该样本问题。
- HEIF 的 RGB ICC 转换改用已在 image 依赖图中锁定的 moxcms 0.8.1，保持 RGBA16 输入输出、64 行有界缓冲及精确 Alpha。不更改 CMYK 打印色彩路径。包内增加 moxcms BSD 许可；HEIF 重建说明同步更新，因此下一次运行会重新核验和构建完整辅助包。
- 正式 Rust decode 路径在本地使用同源 helper 和私有 Adobe RGB HEIF/Apple 参考：最大差 7、均差 1.034722，原 12/3 阈值保持不变。28 项本地 shell 测试通过，包含深色阶、跨缓冲边界、透明度与坏 ICC 检查；Windows MSVC 目标 API 类型检查通过。仅类型检查，不冒充 Windows 执行。
- ICC 查表有少量 16 位舍入：合成同色域测试允许每通道 4/65535 的误差，以 8 个码值间隔的 130 色验证未降为 8 位，Alpha 逐值完全相同。原始深像素传输仍单独验证相邻码值。真实 Windows ICC 断言前保存 PNG 和误差 JSON，便于保留失败证据。
- 续跑仅允许 Cargo.lock 中 compositor-shell 新增一个对原已锁定 moxcms 的直接依赖边，所有其余字节须一致；原锁文件保存并校验哈希，新增版本、校验和、其他依赖边或任意额外修改均拒绝复用。32 项控制检查通过。核心/WASM/前端证据继续按原始哈希核验，Windows shell/解码/ICC 全部重新执行。
- 前轮失败已保留于 source-failed-heif-icc-20261024，本地修复证据位于 local-icc-fix。Windows 功能、CPU/GPU、生产包、两组原生验收尚未完成，不能提交为已验收或发布。

## HEIF ICC 原机通过与 CMYK ICC 长度查询修复（2026-10-24）

- attended-2026-10-24T10-48-45-620Z：32 项控制、29 项 Windows shell 通过；libde265/libheif/CompositorHeif 重建成功，独立 8/10 位、旋转、透明度、预算与坏输入检查再次通过。新 helper SHA256 d11b0f015c2aaf5e804ece7cf4537ec63d68a813c28ce40c384645fb3532e8e8。
- 真实 PDF、HEIF 及 moxcms ICC 转换通过。Windows Adobe RGB HEIF 的最大差 7、均差 1.034722，与本地正式路径一致，原 12/3 阈值不变。ICC proof 构造和 JPEG/TIFF/PDF 导出已执行，但独立阅读器尚未运行，不能算作最终 CMYK 输出通过。
- 首个 CMYK 导入在 WIC 报 0x88982F8C（缓冲不足）。源码的 GetColorContexts/GetProfileBytes 长度探测传空切片，其指针不是 NULL；微软文档要求 NULL 才表示长度查询。改为经原接口 vtable 显式传 NULL，随后按有界长度读取；异常增加具体查询步骤。新增 Windows 原生 ICC 字节查询/读取一致性测试。Windows MSVC 目标类型检查与 32 项控制检查通过，实际修复效果待原机复测。
- 同时修正验收代码中 CMYK 循环误写 HEIF 诊断路径的问题；CMYK 现在按各文件名保存 PNG 和比较 JSON，仍在断言前落盘。像素样本、阈值和判定保持不变。此轮首次 CMYK 导入便已停止，HEIF 比较 JSON 未被覆盖。
- 完整本轮证据保留于 source-failed-cmyk-profile-buffer-20261024；锁屏策略前后相同。辅助包缓存绑定的构建输入未再修改，下轮复用已验证完整包；Windows shell/实际解码/色彩全部重新运行。浏览器、CPU/GPU、生产包和原生界面仍未完成。
- 接口依据：https://learn.microsoft.com/en-us/windows/win32/api/wincodec/nf-wincodec-iwiccolorcontext-getprofilebytes 与 https://learn.microsoft.com/en-us/windows/win32/api/wincodec/nf-wincodec-iwicbitmapframedecode-getcolorcontexts 。

## 源码与 Windows 真实色彩验收全部通过（2026-10-24）

- attended-2026-10-24T11-04-27-470Z，源码阶段于 11:10 UTC（台北 19:10）完成：32 项控制、30 项 Windows shell（含实际 WIC ICC 长度/字节回归）通过；795 核心/WASM、三套类型检查与 332 前端单元的未改动成功证据按原始哈希复用。
- 完整辅助包缓存通过核对并复用；8/10 位 HEIF、旋转/透明度、PDF 六项渲染与边界、真实 HEIF ICC 转换均通过。HEIF ICC 最大差 7、均差 1.034722。
- CMYK TIFF/JPEG/PSD/无内嵌 profile TIFF（显式 fallback）均通过，最大差 5、均差 1.055556；RGB 文件不受失效旧 profile 偏好影响，缺失必要 profile 被拒绝。WIC NULL 长度探测修复已实际验证。
- 独立 Pillow/LittleCMS 读取 CMYK TIFF/JPEG，墨量最大差 3、均差 0.791667；PDF ICC 嵌入、页面打印尺寸和 48×32 渲染通过；33³ 软打样 LUT 最大差 15、均差 0.887748，符合原 16/3 阈值。诊断 PNG/JSON 已分别保存，未相互覆盖。
- `source/result.json` 为 passed:true，当前 manifest 绑定、各步骤退出码及运行产物哈希已核对。浏览器功能/CPU/GPU/生产包/两组原生界面尚未完成；当前等待操作者启动功能阶段。

## 首轮完整 Windows 功能结果与服务器配置修复（2026-10-24）

- 功能阶段 11:12–11:25 UTC（台北 19:12–19:25）：333 通过、28 失败、33 项性能用例按原设计跳过；1 worker、0 retries。显示器/会话监测始终采样为单屏解锁，healthy:true；锁屏策略未改。
- 28 个失败报告逐项核对均为 Failed to fetch dynamically imported module，涉及 gpu-warp.ts、brush.ts、warp-client.ts 的源码 URL。功能配置误用了只提供构建产物的 Vite preview；该结论来自实际错误和配置对照，不能计作功能通过。
- 已核对 1.4.9 最终通过的 mac149-attended-20261024 配置：functional/GPU 使用开发服务器与 mock bridge/test API，CPU 使用生产构建 preview。恢复同样分工；除端口和独立构建目录外，三份配置与该基准一致。CPU 配置、原性能测试正文/预算、功能用例、断言、worker/retry 均未修改。
- 使用当前冻结的同一份 Windows WASM/glue（只复制到 Mac 本地镜像，没有反向覆盖共享目录）复测 5 个受影响文件：40 项全部通过、0 重试，19.8 秒。本地证据 local-browser-server-fix/module-tests.log；32 项控制通过。不能替代 Windows 完整功能复验。
- 本轮功能失败保留于 functional-failed-preview-server-20261024；此前完整通过的源码阶段保留于 source-passed-before-browser-server-fix-20261024。下一轮仍按新冻结清单重新执行 Windows shell/真实 codecs，复用未改动核心/WASM/前端证据及完整 HEIF 包缓存，再完整运行功能与剩余验收。

## Windows 完整功能验收通过（2026-10-24）

- attended-2026-10-24T11-35-39-659Z：新冻结配置下源码阶段再次通过，30 项 shell、实际 PDF/HEIF/ICC/CMYK 与独立阅读器通过，32 个运行产物核验一致。
- 功能阶段于 11:52–12:06 UTC（台北 19:52–20:06）运行：361 通过、0 失败、0 flaky、0 retries；33 项性能用例按原设计留待独立阶段。上轮 28 个源码模块加载错误全部消失。
- 环境健康判定通过：显示观察器 1625 次采样为单屏解锁，会话观察器 3776 次采样为解锁；不声称采样间状态。结果与当前 inputs.json 哈希绑定一致，原用例/阈值未修改。
- 当前等待 CPU 29 项原预算验收；GPU 4 项、生产包、真实 RAW、原 25 对话框及 1.5.7 新功能原生验收尚待完成。

## Windows CPU 原预算性能验收通过（2026-10-24）

- 同一 attended-2026-10-24T11-35-39-659Z，CPU 测试于 12:07–12:13 UTC（台北 20:07–20:13）完成：29 通过、0 失败、0 跳过、0 flaky；1 worker、0 retries，原断言和预算未修改。
- 覆盖 24/100 MP 编辑后的局部上传、任务完成帧、图层拖动/尺寸、效果任务抢占、吸管/渐变、画笔/修复/填充、复杂选区及空间效果导出。生产构建 preview 配置与 1.4.9 基准一致。
- 当前输入哈希绑定、环境健康、监测收尾均通过；等待 GPU 4 项，其后仍需生产包/RAW/原生界面，尚不宣称全量验收通过。

## Windows GPU 原预算性能验收通过（2026-10-24）

- GPU 阶段于 12:14 UTC（台北 20:14）完成：4 通过、0 失败/跳过/flaky，1 worker、0 retries；环境健康且输入哈希绑定一致。
- 24/100 MP 直接 GPU stroke 与生产实时变形/完整提交/撤销历史预算均通过，使用修正后与 1.4.9 相同的开发服务器配置。适配器为 Intel HD Graphics 520 的 ANGLE D3D11。
- 源码、361 功能、29 CPU、4 GPU 至此全部通过。生产便携包、真实 RAW、原 25 对话框及新功能原生验收尚未完成，仍不作发布或全量验收声明。


## 生产包已构建，Tauri 格式化触发冻结保护（2026-10-24）

- 同一轮 package 于 12:21–12:39 UTC（台北 20:21–20:39）执行：Tauri release 编译成功（15m02s），RAW helper 编译成功，HEIF 完整缓存核验复用，生产便携 ZIP 已生成。随后冻结校验发现 src-tauri/Cargo.toml 从 998 增至 1007 字节，立即停止，尚未执行该阶段的 RAW 或原生界面验收。
- 全量扫描 1208 项冻结输入，仅该 Cargo.toml 改变。已按原 SHA256 精确恢复 998 字节版本；保存两个原文并用 Python tomllib 比较，解析结构完全相同，唯一字节差为九个 Windows features 列表逗号后的空格。复核入口还固定两个原文 SHA256，并只接受这九处空格插入。依赖、features、版本、锁文件和产品源码均未改变。
- 失败完整保留于 package-failed-cargo-format-20261024；原 attended 结果保持 failed，policyUnchanged:true。source/functional/cpu/gpu 原始通过结果没有改写；inputs.json 仍为 5b55485a44c1b8c54b2835d4973bce07a8c5eb2e6531d61d3f5b24ade5471b99。
- 保留生产包 Compositor-portable-0.11.0-20261024-2021.zip，15721139 字节，SHA256 0ed14ba81349b9260e611cff8ce1efb78e7a000dec2dba5e651d2c577ff6b461。本地执行原 verify-portable-release.mjs，通过 25 项 ZIP CRC/长度/内容、展开目录一致性、RAW/HEIF 来源、完整源码/许可、生产 WASM 一致性检查；证据 local-package-format-recovery/integrity。该包仍是 sourceDirty:true 的本地候选，尚未发布。
- 专门续跑入口 scripts/resume-mac157-package-recovery.cjs 以 package-format-recovery.json 另外固定 53 项证据，包含入口/控制测试、原冻结清单、原始四阶段结果、失败日志/策略回执、两个 Cargo 原文、生产 dist、主 EXE、ZIP 及完整展开目录。5 项控制检查通过，拒绝依赖变化、其他格式变动、证据散列变化和路径越界。入口不重编译或更改旧结果，须重新执行原 ZIP 验证与两组 RAW 检查，成功后衔接未修改的原 attended 入口完成两组原生验收；全程保留锁屏策略和人工确认。
- 原 build-windows-x64.ps1 的 Tauri 格式化行为仍被原冻结保护识别；本次仅恢复精确原文并审阅复用已生成包，不把此次构建标记为完整通过。RAW、两组原生界面和最后策略回执尚待 Windows 实机运行。

- 续跑入口本地 `--check` 退出 0：1208 项原冻结输入、53 项补充证据及原 source 运行产物全部一致；记录 local-package-format-recovery/preflight.json。已请求操作者从包独立检查处续跑，尚不作 RAW 或原生通过声明。


## 生产便携包及真实 RAW 16 位显影通过（2026-10-24）

- 续验于 12:56–12:57 UTC（台北 20:56–20:57）完成。保留包在 Windows 重新执行原完整 ZIP 验证，25 个文件一致；新 package/result.json 为 passed:true，原失败报告仍保留。当前原冻结清单与补充审阅清单哈希均核对一致。
- Canon EOS 40D 原始 CR2（3908×2602）与无预览 Bayer DNG（128×96）分别通过 8 项显影检查：as-shot/repeat、曝光、色温、色调、tone-zero、8 位预览、降噪恢复；全分辨率输出为实际 RAW8 16 位码值，Alpha 完整，重复哈希一致，原输入未改动。两个独立二进制阅读器 receipt 均 PASS。
- package 阶段 policyUnchanged:true；仍为 sourceDirty:true 本地候选、published:false。随后原 attended 控制器自动核验并复用 source/functional/cpu/gpu/package，等待启动原 25 对话框验收；两组原生验收与总控最终回执尚待完成。


## 原生回归导航器测试起点修正（2026-10-24）

- attended-2026-10-24T12-57-19-247Z，原生阶段于 13:00–13:01 UTC（台北 21:00–21:01）运行，已完成 9 个真实文件对话框及物理尺寸、旋转 Undo/Redo、Scanlines/Last Filter、PNG/JPEG/PDF 导出/取消检查；随后在 Navigator 可见性断言停止。invalid:null，146 个桌面采样均为单屏解锁，observerExit:0，最终 policyUnchanged:true，因此没有把错误归因于锁屏。
- failure-window.png 中 128×96 图像仍完整位于画布内。1.5.7 自动 fit 默认不再放大小图（Viewport.fit 的 enlarging 默认 false，zoom 上限 1），原 1.4.9 回归脚本在导入后固定执行 8 次 Ctrl+=，得到 1.25^8≈5.9605、762.94×572.20 图像；该窗口下尚未溢出画布，不满足原 Navigator 的溢出条件。原脚本假定小图导入时自动放大到窗口的前提已不成立。
- 修正只在原 Navigator 序列中加入一次真实 View > Fit，再执行原 8 次 Ctrl+=、原可见性断言、点击和后续 fit。没有删除断言、增加超时、减少 25 个真实对话框或修改产品源码。保留整个原脚本与原冻结清单，通过补充固定输入脚本派生仅增加这一操作的回归脚本、指向它的 PS/controller/attended 包装；每次启动逐字节核对允许的派生差异。
- 本地镜像与冻结 app/src 全部一致（0 项需要同步）。同一 Windows WASM 在 1920×1080、1536×864、1280×720 三种视口运行：3/3 通过，单 worker、0 retries，3.8 秒；确认默认 zoom1、旧八步 zoom5.9605，以及显式 fit 后原八步到 zoom32 时导航器可见，点击不修改文档，fit 后隐藏。它只验证浏览器测试起点，不替代 Windows WebView2/真实对话框执行。
- 5 项控制测试通过：精确派生、仅一次 Fit、拒绝改超时/移除断言、拒绝减少对话框/停用解锁检查、拒绝额外代码。证据 local-native-navigator-fix，失败原文完整保留 native-failed-navigator-precondition-20261024。source/functional/cpu/gpu/package 五份通过结果与原生产 ZIP 均保留未改写；待补充输入检查后，仅重跑两组原生验收。

- 新原生入口 `--check` 退出 0：31 项补充证据、精确脚本派生、现有生产包和五阶段结果核对一致，1208 项原冻结输入与原 CPU/GPU 预算通过；记录 local-native-navigator-fix/preflight.json。已请求在 Windows 启动原生复验。


## 导航器原机通过，真实 RAW 保存响应待诊断（2026-10-24）

- attended-2026-10-24T13-10-38-756Z 于 13:10–13:14 UTC（台北 21:10–21:14）执行：Navigator、全屏/窗口几何恢复、原有滤镜与 PNG/JPEG/PDF 导出、WebP 保存重开均通过，完成 23 个真实对话框。真实 Canon RAW 已预览、取消、重置并导入；随后 save('native-canon-as-shot') 的原 30 秒轮询未完成，原生阶段失败。376 个桌面采样均单屏解锁、invalid:null，锁屏策略前后相同。
- 保存对话框回执时间 13:13:28.869Z；落盘 manifest 为 13:13:29.720Z、16 位 PNG 最后写入 13:13:35.821Z、最终 .comp 目录 ctime 为 13:13:41.359Z。13:13:59.462Z 记录轮询超时。失败 DOM 读取只得到 Target page, context or browser has been closed，截图未成功生成；单凭这些证据不能确认崩溃原因，也不能把已写出文件等同于保存界面验收通过。
- 实际 PNG 为 3908×2602 RGBA16、58344314 字节，所有四个 PNG 块 CRC 与 IEND 校验通过。本地同一冻结 WASM/前端、mock bridge 的诊断运行成功：读取约 410.6ms，UI 保存约 1426ms，仍保存为格式 v12 的 RGBA16。此本地结果不替代 Windows Tauri IPC/磁盘/界面结果。首个本地尝试误放 Vite public 根目录而读取 HTML，失败日志已保留；修正后以 PNG 签名检查保证输入正确。
- 新只读诊断 diagnose-mac157-raw-save.ps1（PowerShell 语法检查通过）限定本次故障 13:09–13:16 UTC，读取 Compositor/WebView2 的应用事件与相关资源不足事件及当前内存状态，不终止进程或改设置。已请求操作者执行，同时保留原 native 失败目录、原 30 秒预算及所有产品/测试冻结输入。尚未确定根因、未修改产品实现、未作完整原生通过声明。


## RAW 保存定向记录准备（2026-10-24）

- 操作者已运行只读系统诊断，结果 mac157-raw-save-diagnostic-20261024T1319580016341Z/result.json：指定时段 Application 1000/1001/1002、System 2004 均没有匹配事件；13:19:59 UTC 当前可用物理内存约17GB。该采样发生在测试退出后，不能代表测试峰值，也不能排除 WebView2 内部故障。
- 准备 run-mac157-raw-trace.cjs：核对原输入、补充审阅和生产包后等待人工回车，使用同一生产 EXE、独立 WebView profile，原封不动加载已审阅25对话框协议。trace-mac157-native.cjs 仅增加 CDP console/crash/disconnect、IPC请求起止、每秒单个待处理界面探针及独立磁盘轮询；不改变原断言/超时，不安装产品测试 API。PowerShell 增加仅限本次启动进程树的内存采样。
- 独立 raw-trace-* 目录保留全部记录；锁屏/多屏/观察器失联停止本次拥有的子进程，8分钟诊断上限，前后只读核对原锁屏策略。不会覆盖已有 native 失败或写入任何验收通过结果。
- 本地真实 Chromium CDP 记录检查通过，捕获 busy/idle、console、独立manifest出现、page close和browser disconnect；JS语法和PowerShell语法通过。Windows内存采样及故障根因仍待本次实机记录，当前产品源码没有改动。

- 新入口 --check 退出0，1208项冻结输入、原CPU/GPU预算、五阶段通过证据和生产包核对一致；预检及本地记录检查证据存于 local-raw-trace-preflight。待操作者启动定向复现。


## RAW 定向记录：文件已提交，CDP通知停止（2026-10-24）

- raw-trace-2026-10-24T13-29-11-494Z 定向运行退出1，invalid:null、observerExit0、policyUnchanged:true。仍在原23号保存对话框后的30秒断言失败；没有覆盖 native 原始失败或已通过的五阶段记录。
- 13:32:17.777Z write_package_manifest请求完成；之后图片请求的CDP通知不再到达。独立文件轮询于13:32:28.334Z确认最终.comp目录、manifest及58344314字节PNG存在，约10.6秒。此前发出的UI探针一直等待，直到13:33:04Z连接关闭；没有crash事件，相关进程在关闭前后仍存在。连接关闭本身不能作为进程崩溃证据。
- 114次只读内存采样中，可用物理内存最少14.679GiB，本次进程树私有内存峰值2598.24MiB；桌面及策略检查正常。因此当前证据不支持系统内存耗尽或锁屏。
- 本地使用相同58344314字节PNG作实际HTTP POST，接收端完整校验SHA256 0fe2b3b1d0ba25293e63c2ec11b165c9ddc5d67e8053ef4447212abf731ce0a2，但默认Playwright CDP连接随后关闭，复现了“文件已收到而控制连接失效”。正在验证Network.enable的maxPostDataSize对调试通知副本的限制；该选项来自Chromium CDP官方协议 https://chromedevtools.github.io/devtools-protocol/tot/Network/#method-enable ，不改变应用实际请求体。

- 本地对照完成：默认CDP在15500ms后报告连接关闭，限制调试POST副本为65536字节后162ms正常返回OK；两次接收端均核对58344314字节和相同SHA256。代理仅在Network.enable命令增加maxPostDataSize参数，实际应用请求/响应及其他CDP消息原样转发；大POST在调试通知中不附带数据，真实上传保持完整。该对照证实本地故障与调试网络数据采集有关，Windows仍需实机验证。
- 新入口 run-mac157-raw-bounded.cjs 保留原步骤、25对话框与全部断言/时限，只把CDP连接经过回环代理；复用原只读内存/桌面记录和8分钟上限，所有输出进入独立raw-bounded-*目录，不更改产品、原失败或通过结果。1208冻结输入与原性能预算预检通过；本地AB、脚本散列及语法检查记录于local-raw-bounded-preflight。


## Windows 调试连接复验与后续 Camera Raw 耗时（2026-10-24）

- raw-bounded-2026-10-24T13-47-47-370Z 的64KiB CDP POST通知上限实际生效。真实RAW的write_package_image与commit均正常返回，随后原保存断言、关闭、真实Open对话框及画面哈希一致性检查通过，已完成24个真实对话框。保存的3908×2602 RGBA16 PNG为58344314字节，四个PNG块CRC有效，SHA256仍为0fe2b3b1d0ba25293e63c2ec11b165c9ddc5d67e8053ef4447212abf731ce0a2。
- Camera Raw Exposure/Contrast/Texture预览与Cancel随后继续，最后Exposure0.5 Apply在原30秒working消失断言超时。该阶段的每秒CDP界面探针持续响应（数毫秒），working:true；不是先前连接失效。最终code1、invalid:null、observerCode0、policyUnchanged:true，保持诊断失败，未作原生通过声明。
- 进一步定位到16位Camera Raw使用原十次8位重建；相同颜色重复查表。仅在/tmp本地镜像试验曝光单独调整时复用精确颜色结果，保留原重建和C内核舍入；尚未修改冻结生产源码。本地原Windows WASM对同一实际RAW进行worker run_edit_job：4781.6ms、81348928字节16位结果SHA256 7d9c993cb20589912d7ffa2edf53c358819500090ec3c697cf5ad937007834fe。Windows已观察到原30秒失败，Mac计时不可替代其预算。
- 本地镜像最初的Rust源包含部分较早代码；已逐项比对并同步其余当前生产engine源码及workspace锁文件，保留两处实验修改，确保接下来候选WASM比较基于当前实现。原Windows生成的WASM已保存在/tmp/mac157-raw-apply-baseline-pkg，未写回共享工作区。


## Camera Raw 16位曝光精确缓存修复与新验收（2026-10-24）

- 为大幅（至少1MP）16位图像的单独曝光调整增加一次操作内的精确RGB缓存：仍调用原C内核进行第一次计算，随后相同RGB直接复用其8位结果；保持原十次deep_split/deep_merge重建、双精度计算和舍入。透明像素仍走原内核，其他参数组合及HDR不进入该路径。固定最多64MiB缓存随操作释放，不作8位降精度或图像缩小。
- 5种正负曝光、随机颜色及透明/半透明alpha的原重建逐样本一致性单元检查通过。同一真实Canon16位图在原Windows WASM与候选WASM中的81348928字节结果完全一致，SHA256 7d9c993cb20589912d7ffa2edf53c358819500090ec3c697cf5ad937007834fe；本地worker计时4781.6ms→2418.5ms，实际UI应用3777ms且Undo/Redo逐字节恢复。本地计时不能替代Windows30秒验收。
- 当前源码本地核心796通过、0失败、原10项ignored；Camera Raw原10项浏览器测试及新增实际10MP诊断通过。首次本地全套编译因镜像缺失include文档、随后因本任务debug缓存占满磁盘失败，已保留日志，补齐精确文档并仅清理本任务24GiB临时debug构建缓存、关闭调试符号后完整重跑通过；未清理用户文档。证据 mac157-raw-performance-20261024/local-validation。
- 已将两个已验证的引擎源码变更写入共享工作区；原两份源码、Cargo原文及完整Windows WASM包先保存在旧验收raw-performance-before。src-tauri/Cargo.toml预先采用已验证的Tauri九个空格格式，tomllib解析完全一致，以免重建再次触发纯格式冻结失败。原input/result/ZIP及失败报告全部保留不覆盖。
- 新验收独立目录mac157-raw-performance-20261024，全量重建并重新执行七阶段，禁用旧源码通过记录携带；新增真实10MP WASM字节及Undo/Redo检查。原361功能、29CPU、4GPU、25旧原生对话框、新功能原生要求与时限均保留；两组原生入口仅加回环CDP POST副本上限65536字节，产品请求体不受影响。14项流程/精确派生检查通过，两个PowerShell启动器语法通过。仍需Windows实机验收，未发布。

- 新冻结共1248项，输入校验及原性能测试体校验通过；清单SHA256 d6773f0115de11afc4578539229d2a483bcecfa3d0b7c53ec11624e8f1c1883b。相对上一清单仅两个引擎文件及Cargo格式修改，新增项为测试入口/辅助源码/真实RAW样本等；旧source-resume凭证不纳入新验收。Windows尚未启动。

## 精确缓存候选 Windows 全量复验（2026-10-24）

- 值守入口已启动，记录于 mac157-raw-performance-20261024/attended-2026-10-24T14-10-25-625Z。46项发布/流程守卫通过，WASM重建完成（wasm-pack总计7m46s），117项接口检查通过；产物18879500字节，SHA256 cb3c098db3a9d95a03b7c7d22292b1d71d31ce0c8fd95e35aeb8d3ed43e0a98a。
- Windows实际10MP16位RAW曝光worker检查通过，12370.6406ms，81348928字节输出SHA256 7d9c993cb20589912d7ffa2edf53c358819500090ec3c697cf5ad937007834fe，与原算法完全一致；分段安装、Undo/Redo通过。原deep-WASM完整导入/编辑/保存/导出检查也通过。此时仍需核心/前端、功能、CPU/GPU及生产原生验收，不视为最终通过。

- Windows原生核心/engine-wasm已完成：796通过、0失败、10原有忽略，文档测试通过。当前正在重新构建Windows外壳，源码整阶段尚未完成。

## 完整功能测试首项导航超时与就绪检查（2026-10-24）

- 本轮源码17步全部通过（14:54:54Z），随后完整功能测试的首项adjust-layers.spec.ts:19在page.goto等待load时超过原30秒，尚未进入功能断言。运行继续，当前其余已执行测试未见新增失败；最终数量仍待完整报告。失败当时空闲物理内存约19GiB、无活动编译器，监测保持单屏解锁。
- Mac隔离开发缓存冷启动约744ms，未复现Windows30秒超时，不能据此断言根因已证实。首个本地诊断因/tmp与/private/tmp路径映射导致Vite路径错误，改用真实绝对路径后完成；失败和成功日志均保留。
- 现有webServer仅等待首页HTML响应。新测试服务器入口使用锁定Vite的transformRequest及waitForRequestsIdle，静态模块完成后才在专用路径返回200；之前返回503，转换错误则关闭，不运行应用/WASM或任何测试，不改变测试本身30秒时限。参考官方说明 https://vite.dev/config/server-options#server-warmup 。只用于功能测试开发服务器，CPU/GPU和生产启动要求不变。
- 本地2项就绪正反向检查、3项严格续跑来源检查、原5项调整图层UI测试通过。拟仅保留本轮源码与运行产物成功凭证，完整重跑功能及后续阶段；仅当本轮最终恰好为首项导航1失败且环境有效时允许生成该续跑清单。旧输入、失败结果和策略回执保持原样。候选目录mac157-ready-20261024，尚未冻结或在Windows运行。

- 功能完整报告已结束：360通过、1首项导航超时、33原性能分组跳过、0flaky；healthy:true、error:null、invalid:null。已按实际details.json验证唯一失败为adjust-layers.spec.ts:19的page.goto原30秒超时，所有其他功能通过。总控尚未写出mac157-raw-performance-20261024/result.json或policy-after回执；已请操作者取消控制台文字选择并报告末行。prepare-mac157-ready在完成旧输入核对后因缺少总控回执主动停止，尚未创建新inputs.json或来源携带结果；错误日志保留为prepare-awaiting-controller-receipt.log。新入口仍不得启动，待回执后完成冻结与预检。


- 操作者解除控制台暂停后，原总控于2026-10-24T15:16:57.557Z完整退出：passed:false、error:null、policyUnchanged:true、settingsChanged:false。原360通过/1导航失败及所有证据保持原样。
- 就绪复验入口已完成冻结和预检：1354项，清单SHA256 2978551bfa09c73d2e76869a292e7031f6242fd4d15297dd34faa11863ebcac3；原1248项无删除、无改动，CPU/GPU配置逐字节一致。只携带本轮已完成的17步源码通过及相同Windows构建产物，功能测试仍要求完整重跑，随后CPU/GPU、生产包和两组原生验收。预检退出0，原性能预算核对通过；日志及回执存于mac157-ready-20261024/local-validation。新Windows复验尚未启动，未作验收或发布通过声明。


## 就绪入口控制自检的启动窗口修正（2026-10-24）

- Windows就绪入口停在验收前控制自检：readiness-controls-1792855366225.log记录5项中4通过、1失败，尚未生成status或执行功能测试。失败是新增自检等待static import的10秒计时；该测试连清理共50852ms，不能由此确认具体初始化阶段或把原页面30秒失败认定为已修复。负向无效模块检查正常拒绝并通过。原失败日志SHA256 51ed0e6660f0cb5fddd7396bc36df04365812f3e151c96d7882859336d1f05bc完整保留。
- 原控制检查把Vite配置/服务初始化也计入任意10秒限时，混入了本不属于该正确性检查的启动性能要求。独立的新自检改用锁定Playwright已有的60秒webServer启动窗口，并记录config-resolved/server-configured/server-listening/static-import-reached各阶段时间；保留准备前503、完成后200、静态模块数、不执行应用及错误模块关闭的全部检查。应用page/test30秒、CPU/GPU预算和0重试均不改；此处只调整新增控制自检的初始化等待，不能宣称所有测试时限完全没变。
- 本地2项就绪检查及3项来源检查通过。额外在隔离本地配置初始化注入11000ms等待，两个就绪检查仍通过（总11398ms），证据保留local-validation/simulated-cold-test.mjs.txt与日志。Vite启动器、全部产品源码/运行产物、已冻结的1354项均未修改。
- 新独立目录mac157-ready-startup-20261024冻结1368项，SHA256 2b8e74cdfb547b25711e82650072d070490627b59baceef46fbe3c91407bd3f1；只新增14项流程/来源证据，旧输入无删除/变更。CPU/GPU配置逐字节相同，功能配置只更换回执目录；17步源码成功及运行产物精确携带，仍需完整361项功能和所有剩余阶段的Windows通过记录。

- 新入口--check退出0，1368项输入与原CPU/GPU预算全部核对；冻结日志/预检回执已存local-validation。等待操作者启动Windows复验，未运行或声明通过任何新的Windows功能/性能/原生验收。


## Windows Codex 接管与第二轮页面创建超时（2026-10-24）

- 用户恢复Windows Codex额度并授权协作；当前Mac应用未连接Windows主机，因此通过共享目录交接，文档mac157-windows-codex-handoff.md，运行记录build-artifacts/mac157-windows-codex-handoff。Windows Codex确认原总控PID18036及PowerShell10684，未重复启动、不改冻结输入/锁屏策略；Mac只读核验。
- ready-startup Windows控制自检5/5通过（3125ms），Vite静态模块就绪4809ms/110模块，应用/WASM/测试未预执行。原adjust-layers首项导航失败未再次出现。功能完整执行19.2分钟：360通过、1失败、33原性能分组跳过、0flaky；1worker、0retry。
- 唯一失败full-coverage-oracle.spec.ts:8，Multiply opacity255/255，在15:38:35.879Z开始，30134ms后报告“Test timeout of 30000ms exceeded while setting up page”。测试体尚未执行，无像素差异断言，不能归因于混合算法。前序Normal254/Normal255/Multiply254分别6012/13966/25251ms；失败后新worker的Screen254/255为10767/6620ms。Windows额外只读进程查询同时曾出现延迟，尚无因果证据。
- 功能healthy:true、error/invalid:null；显示器2163次采样全部单屏解锁，会话4875次采样全部解锁。总控15:50:39.163Z以passed:false、error:null结束，policyUnchanged:true、settingsChanged:false；CPU/GPU/包/原生未运行。原失败保留，不按完成声明。
- Windows只读诊断限定15:37:30Z–15:40:00Z，Application1000/1001/1002与System2004均无事件；事后可用物理内存17772300KiB、无编译器，这不代表失败时资源情况。安装的Chromium1243文件版本153.0.8010.12，旧日志未记录实际headless启动参数，不能补造历史实际版本/参数。诊断脚本首次错误假定Playwright内部目录，已改公共API且保留说明；它不是产品失败。
- 已让Windows Codex准备独立原六项oracle定向诊断，保留30秒/单worker/零retry，增加API/browser日志及trace、只读桌面监测；准备稿正在审阅，尚未运行。不改产品算法，不放宽产品时限，不用诊断结果替代完整功能验收。


## 六项混合定向诊断通过，准备完整可观测复验（2026-10-25 台北）

- 用户授权Windows Codex执行后，Mac通过共享协作文件审阅13项准备输入（SHA256 da2597153bfd7a840d781036f76d1e23364ac17e728c0b0f8307412bb5e033ea），Windows直接执行repro-v2一次。该诊断明确CODEX_CONTROLLED_DIAGNOSTIC、humanAttendanceClaimed:false，不发送键鼠、不改公司策略；旧attended协议保留。
- 原full-coverage-oracle六项、30秒/1worker/0retry全部通过：6/0/0/0（通过/失败/跳过/flaky）。Playwright55501ms，总92557ms；error/invalid:null，healthy:true，双桌面观察器、资源采样均exit0，policyUnchanged:true/settingsChanged:false。原Multiply255用时6845ms，6次newPage为741/203/214/224/200/228ms。没有复现原故障，不能把诊断替代完整功能验收。
- DEBUG记录实际Chromium headless shell1243启动及正常退出，6个trace ZIP完整。13次资源采样间隔5344–6136ms，可用物理内存最低15.912GiB，整个诊断拥有的进程私有内存总峰值1242.65MiB。浏览器有ReadPixels GPU stall提示，相关像素断言均通过；这不能证明此前page fixture超时的根因。
- 正准备独立mac157-observed-20261025完整复验：相同产品/运行产物，保留全部历史失败，完整361功能与剩余CPU/GPU/包/原生，只有功能阶段增加浏览器日志和轻量资源记录。计划采用明确Codex监测的阶段控制，用户已授权执行，不假称人工、不生成定时输入，仍严格保留实际单屏/解锁检查及失效停止。入口须审阅来源、预算、控制检查并冻结后才能启动。尚未完成全部验收，未发布。


- mac157-observed-20261025已完成准备与双端审阅：1419项，清单SHA256 6adf014e48d69ea1c3cb008eb811c4463ed6cdcf4c0bc3e13049602a1204a115；原1368项完整保留，Mac独立核对新增51项、17步source精确携带、CPU/GPU配置字节一致与HEAD。Windows预检exit0，Windows及Mac各11项控制检查通过。凭证local-validation/mac-review.json、mac-controls.log、preflight-result.json。
- Windows按针对该清单的执行通知于16:10:16Z启动supervised总控（台北2026-10-25 00:10），PID19624；原sequence体保持，明确CODEX_SUPERVISED/humanAttendanceClaimed:false，无自动输入/策略改动。只在功能阶段增加API/browser日志和5秒owned-process资源采样，性能阶段不加该负载。功能正在执行，未作完整通过声明；仍保留全部旧失败。

### 2026-10-25 完整功能复验通过（observed 轮）

本轮 `build-artifacts/mac157-observed-20261025/functional/result.json`：361通过、0失败、33项原性能分组跳过、0 flaky，单worker、零重试，原预算/断言保持；功能运行15.4分钟。双桌面观察器全部样本单屏解锁，资源观察器167样本零错误；healthy=true，error/invalid=null，绑定1419项清单SHA256 `6adf014e48d69ea1c3cb008eb811c4463ed6cdcf4c0bc3e13049602a1204a115`。此前page fixture超时未复现，根因未确认，旧失败记录保留。CPU/GPU、当前候选生产包和两组原生验收仍待完成，不能宣称全部通过。

### 2026-10-25 observed 轮性能结果及剩余阻碍

CPU 28通过、1失败、0跳过/重试/flaky。唯一失败为 `perf-4b1.spec.ts:1395` T9-6：24MP渐变拖动最慢53ms，原断言必须低于50ms；100MP33ms。该项仍标失败；未修改预算或以诊断代替验收。CPU环境healthy=true、error/invalid=null，双观察器全部样本单屏解锁。独立GPU4/4通过、0失败/跳过/flaky，healthy=true。总控16:35:20Z状态incomplete，未执行包/两组原生。此前候选该项31/42ms，仅作历史对照，不证明本轮根因。正在与Windows Codex准备逐tick分项诊断。

### 2026-10-25 渐变逐帧诊断（不替代CPU验收）

`diagnostics-gradient-tick/repro-v2`在原Edge、当前dist/WASM、单worker零retry下顺序运行原样T9-6及仅增加逐tick分项的副本，2/2通过，原样24/100MP最慢29/36ms，副本29/21ms。原8tick、排首项、严格低于50ms均保持。双桌面观察器/前后输入核对和策略回执通过，总44.17秒；无profile/trace/DEBUG/额外资源采样。准备清单19项SHA256 `a24708f9aa36a6ebf9fd76a6c13d5f55470cc91784a23b21266a4628f6d7e682`。53ms没有复现，根因仍未确认，旧CPU失败保留。未改产品；准备在新独立目录完整复核29CPU，再决定包和原生验收。

### 2026-10-25 完整CPU复核仍未通过

`mac157-cpu-reviewed-20261025`清单1471项SHA256 `005af6ca6490367ec88a5eff2f1b0e1bedb6a90569373ef13cd7b08676a47c2b`，source17/functional361/gpu4精确携带同一候选通过记录；新运行完整CPU结果26通过/3失败、单worker零retry、0skip/flaky、环境healthy=true。T9-6本轮27/18ms通过，但旧53ms失败保持。新失败为两项100MP Levels（最长帧间隔296/300ms，原<100；第一项install482ms亦超过450）及100MP brush-retouch（Blur280.5、Heal262.3、ContentFill304.6ms，原<100）。实际结果绘制8ms，根因未定，正在定向分析结果传回/安装/释放与长任务。总控00:19:17Z结束、error=null、policyUnchanged=true/settingsChanged=false，包和原生未执行。产品未改，不安排无诊断的自动完整重试。


### 2026-10-25 Levels 定向诊断复现，并排除本次安装收尾长停顿

`diagnostics-levels-install/repro`以原样F1四场景及独立记录副本各运行一次，2项均未通过，单worker零重试；100MP Levels原样最长帧间隔275ms、install CPU429ms，副本295ms、363ms。总118.921秒，healthy=true、error/invalid=null、policyUnchanged=true/settingsChanged=false。21项准备清单SHA256 `a19a986270d58005ef5b411f3dee9f1c7f7d59ce1354813310d011ed5c58717c`；记录包装全部恢复、没有丢事件。

副本100MP Levels的287ms主线程长任务从performance.now()11477.3ms开始，而编辑结果的jobs.finish在22260.5ms，随后installJobAsync为22260.7–22814.6ms。对应Worker.terminate仅0.1ms、400MB ArrayBuffer.transfer约0ms；WASM begin最大16.4ms、repeat4.9ms、append3.5ms、finish0.2ms。这次卡顿早于结果返回约10.8秒，不能归因于安装或结果收尾；输出witness字段为null。下一步针对输入准备、任务派发和渲染调用记录主线程轨迹。当前3项CPU失败未解决，产品未改，完整验收及发布仍未完成。


### 2026-10-25 Grok 派发诊断：同步 postMessage 长耗时已捕获

用户授权 Grok Bot 接手 Windows 执行。独立 `diagnostics-levels-dispatch/grok-repro` 19项清单 SHA256 `5a9bb8cd42498ae628eeb6163b6f3782ba0e881d26dfdf89b68f8ff6eb38ae5a`，绑定旧人工18项及原1471项输入。Mac/Windows各11项控制检查通过后执行一次；总126.561秒，2失败，healthy=true、error/invalid=null、policyUnchanged=true/settingsChanged=false，GROK_SUPERVISED/humanAttendanceClaimed=false。未更改产品、旧测试预算或锁屏策略。

原样基线100MP Levels仍有285ms帧间隔（原<100），install435ms。Timeline记录副本100MP Levels未复现（gap35ms），24MP Levels则捕获96MB输入Worker.postMessage同步70.6ms，输入对象确实在transfer列表，返回后byteLength=0；对应75.87ms主线程RunTask、线程CPU71.014ms，无该区间GC记录。26MB预热编辑派发多次17–27ms，而同大小新输入histogram派发约0.1ms。记录副本100MP fill另有114.951ms/CPU106.441ms任务及130ms gap，JS包装未捕获同等同步耗时，仍未归因。诊断副本有trace开销，不作为完整验收通过。

Chromium当前公开ArrayBufferContents::TransferOrCopy在backing store仍有额外引用时会复制，说明transfer+detach不保证零复制；这仅为候选机制，尚未证明Windows Edge156具体构建使用该分支。Mac Chrome154独立26.2/96/400MB Worker往返对照（返回后直接post与先.transfer()再post各固定3次）均0–0.1ms，未复现。正在请求Grok准备同类Windows微对照，并核对原trace；不能凭Mac结果认定Windows已修复。CPU完整26/29失败和全部历史证据保持，包及当前候选原生验收仍未完成。


### 2026-10-25 Windows独立缓冲对照稳定复现，并准备所有权保持候选

`diagnostics-buffer-ownership/run-20261025`，准备清单SHA256 `e1896be759c608357fb4c9716a32e7f17b2468387a1244fbd44c76200d9c8cc8`，Edge156.0.4314.42/V8 15.6.12.6。6条路线、24次真实Worker交换固定一次完成，20.357秒总/6.242秒浏览器负载；数据按4096字节抽检及末字节校验、真实detach全部通过，healthy=true、error/invalid=null、单屏解锁/1471输入前后检查/锁屏策略相等通过。

直接将worker返回的buffer再次post，26.2/96/400MB均0–0.2ms。先ArrayBuffer.transfer()再post：26.2MB为22.1/33.8/24.2ms；96MB82.7/75.7/73.8ms；400MB303/316.6/319.6ms。transfer自身约0ms，后续post出现同步长耗时。这一独立对照在同一Windows运行时复现了原问题的量级，支持回收池`spare=movable.transfer()`为重要触发路径；尚不能确认具体浏览器C++分支，也不能把微对照成功当完整CPU通过。准备期间错误计时（把回复等待计入sync）被Mac实际harness检查拦截并修正；最终11控制+4实际正反向检查通过才运行。

候选`candidate-structured-transfer`仅将退休缓冲的保存改为`spare=structuredClone(buffer,{transfer:[buffer]})`，保持调用者真实detach及池容量/尺寸/空闲回收策略，覆盖私有recycled和公开JobResult两条路径。共享生产源码仍未修改。Mac全新隔离目录332单元/52文件、类型检查、前端构建通过；131项构建输入只有该源码不同，WASM仍为Windows验证过的cb3c098d…；在同一干净目录重建旧源码，7文件全部与Windows原dist逐字节相同。候选清单SHA256 `63dfa89ba3fdf9cdb44a8a8809b85b58faa67d054d928b8ee724a9984b77a588`。此前Mac旧临时目录有额外发射JS，已拒用并记录，交付来自全新目录。下一步由Grok准备原F1四场景baseline/candidate固定对照；候选尚未Windows验证，CPU及剩余包/原生验收仍未完成。
