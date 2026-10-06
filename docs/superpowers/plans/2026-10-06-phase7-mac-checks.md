# Phase 7：Mac 详细操作步骤（Compositor 1.4.5）

本说明全部在 **Mac 的 Compositor 1.4.5** 中操作，不需要安装或使用 Photoshop。
说明文字使用中文；菜单、按钮、参数名称保留界面中的英文。请按下面顺序完成。
真实 PSD 的上次 Mac 回传已验证，**不用重做 `Real-PSD` 中的文件**。

## 1．准备文件和结果文件夹

1. 将 Windows 上这个完整文件夹复制到 Mac，例如放在桌面：

   `C:\Users\sr9rfx\.claude-project\compositor-windows\build-artifacts\phase7-acceptance-20261006-order-fixed`

   请使用带 `order-fixed` 的版本；不要使用旧的 `phase7-acceptance-20261006`。
2. 另将下面的相机文件复制到 Mac 的上述文件夹内，与 `bayer-no-preview.dng` 放在同一层：

   `C:\Users\sr9rfx\.claude-project\compositor-windows\build-artifacts\phase7-public-raw-20261006\Canon-EOS40D.CR2`
3. 在 Mac 的测试文件夹内新建一个文件夹，命名为 **`Mac-return`**。所有新保存的项目、导出 PNG 和记录都放在这里。
4. 将测试文件夹内的 `RESULT.txt` 复制一份到 `Mac-return`。每做完一项，就在这份文件里记录实际结果。可用 TextEdit 打开；保持纯文本，菜单为 **Format → Make Plain Text**（已经是纯文本时不需要转换）。
5. 打开 Compositor，在 **Compositor → About Compositor** 中确认版本为 **1.4.5**，在 **Apple 菜单 → About This Mac** 中查看 macOS 版本，将两者写进 `RESULT.txt`。

请保留原来的 `*-source.comp`、PSD、PSB、DNG、CR2，以及 `*-windows-expected.*` 文件。
所有测试都从原始输入重新打开，保存时使用下面指定的新名称。

## 2．先熟悉这几个操作

### 2.1 打开文件：两种菜单用途不同

| 要打开的文件 | 使用的菜单 | 文件选择窗口中怎么选 |
| --- | --- | --- |
| `.comp` 项目 | **File → Open Project…** | 选中整个 `.comp` 项目，点 **Open**。不要进入项目内部选择 `manifest.json`。 |
| `.psd`、`.psb`、`.CR2`、`.dng` | **File → Import Images…** | 选中文件，点 **Open**；如果出现转换或 RAW 窗口，再按后文操作。 |

导入 PSD/PSB/RAW 前，请先关闭其他测试项目，避免把图片插入到上一份文档中。
若关闭时询问保存，已另存的结果不需要再改；没有用的测试副本可选择不保存。

### 2.2 保存项目和导出图片：每次都做这两步

1. 点击 **File → Save As…**，也可按 **Cmd+Shift+S**。
2. 选择 `Mac-return`，输入本节测试要求的完整项目名，例如 `01-light-color-curve-mac-edited.comp`，点 **Save**。
3. 等保存完成，再点击 **File → Export PNG…**，也可按 **Cmd+Shift+E**。
4. 选择同一个 `Mac-return`，输入要求的 PNG 名称，例如 `01-light-color-curve-mac.png`，点 **Save**。
5. 点击 **File → Close Project**（**Cmd+W**），再用 **File → Open Project…** 打开刚保存的 `.comp`，确认画面和图层保留。

Mac 1.4.5 的 **Export PNG…** 直接按文档尺寸导出，并保留透明度；不需要寻找额外的尺寸或透明度设置。
本说明中的滤镜探针和合成 PSD/PSB 文档均为 **512 × 320**。画布显示的缩放百分比不影响导出尺寸。
相机 RAW 则按其实际完整文档尺寸导出，不要缩小到 512 × 320。

### 2.3 输入参数和撤销

- **Cmd** 是键盘上的 **⌘**；**Option** 是 **⌥**；**Tab** 是制表键。
- 滤镜面板中带数字输入框的参数：单击右侧数字，按 **Cmd+A** 全选，输入新数值，再按 **Tab**。负数使用普通的 `-`，小数使用 `.`，例如 `-20`、`0.5`。
- 调参时优先用 **Tab** 离开输入框；不要用 Return 提交每一个数值，以免提前触发 **OK** 或 **Import**。
- **Cmd+Z** 撤销一次，**Cmd+Shift+Z** 重做一次。要求保存修改后的结果时，最后应停在 **重做后的画面**。
- 部分控件只有滑块或色轮，旁边数字只是读数，不能输入。下面会逐项说明；不要一直寻找输入框。
- 找不到控件、出现错误或读数达不到目标时，记录实际情况并保留截图。不要把未完成的项填成“通过”。

## 3．四组 Camera Raw Filter 测试

这里测试的是普通图层上的 **Filter → Camera Raw Filter…**，与第 6 节的相机 RAW 导入窗口不同。

### 3.1 每一组都按这个流程完成

1. 用 **File → Open Project…** 打开该组指定的 `CameraRaw/...-source.comp`。
2. 在右侧 **Layers** 中单击 **Camera Raw probe** 图层的名字或图像缩略图，选中图层。
3. 点击 **Filter → Camera Raw Filter…**。在参数区域内向下滚动；点击分组名称旁的小箭头可展开，例如 **Curve**、**Detail**。
4. 按该组参数表修改。没有列出的参数保留初始值，不要随意选择预设。
5. 找到 **Preview**：取消勾选时应显示未修改画面；重新勾选时应显示修改效果。然后点击底部 **Cancel**，确认回到原始画面。
6. 再次点击 **Filter → Camera Raw Filter…**，重新按同一张表设置，并核对读数。点击底部 **OK**，等处理结束。
7. 按一次 **Cmd+Z**，确认回到原始画面；再按一次 **Cmd+Shift+Z**，确认修改重新出现。**到这里停止撤销，保留修改后的画面。**
8. 按第 2.2 节保存 `.comp`、导出 PNG、关闭再打开。文件名使用该组给出的名称。
9. 在 `RESULT.txt` 分别记录 **Preview**、**Cancel**、**Cmd+Z**、**Cmd+Shift+Z** 和重开结果。保存文件本身不能证明这些中间操作，所以也需要文字记录。

每组都打开对应的原始项目，不要对上一组已应用滤镜的结果继续叠加。
如新打开的面板意外带入别组参数，先记录，再关闭该测试副本并重新打开原始项目；仍有问题时退出 Compositor 后重开。

### 3.2 第 01 组：Light、Color、Curve

打开：**`CameraRaw/01-light-color-curve-source.comp`**。
按第 3.1 节打开滤镜后，逐行设置：

| 展开的分组 | 控件或选项 | 要设置的值 |
| --- | --- | --- |
| **Light** | **Exposure** | `0.5` |
| **Light** | **Contrast** | `12` |
| **Light** | **Highlights** | `-20` |
| **Light** | **Shadows** | `15` |
| **Color** | **White Balance** 下拉框 | **Custom** |
| **Color** | **Temperature** | `10` |
| **Color** | **Tint** | `-6` |
| **Color** | **Vibrance** | `18` |
| **Curve** | 上方模式选项 | **Parametric** |
| **Curve** | **Darks** | `-10` |

这里的 **Temperature = 10** 是滤镜的相对数值，不是相机导入窗口的 `4000 K`。
完成第 3.1 节的取消、重新设置、应用、撤销和重做，再保存：

- 项目：`01-light-color-curve-mac-edited.comp`
- 图片：`01-light-color-curve-mac.png`

### 3.3 第 02 组：Color Mixer、Color Grading

打开：**`CameraRaw/02-mixer-grading-source.comp`**。

先展开 **Color Mixer**，选择 **HSL**。下面的 **Hue / Saturation / Luminance** 是三个切换选项，按顺序设置：

| 当前选项 | 找到的颜色行 | 右侧输入值 |
| --- | --- | --- |
| **Hue** | **Reds** | `12` |
| **Saturation** | **Blues** | `-20` |
| **Luminance** | **Greens** | `10` |

不要在三个选项中都改同一颜色；每次切换后确认行名再输入。

然后展开 **Color Grading**：

1. 点击显示 **Three-Way** 的下拉菜单，选择 **Shadows**，只显示这一只色轮。
2. 目标为 **Hue 210°、Saturation 15**。从色轮中心往**左下方的蓝青色方向**小幅拖动控制点；角度决定 Hue，离中心越远 Saturation 越高。这里饱和度较低，控制点应靠近中心。
3. 看色轮下方读数，例如 **`210° 15`**。它是只读文字，不能点击后输入数字。
4. 再用同一个下拉菜单选择 **Highlights**。
5. 目标为 **Hue 40°、Saturation 12**。向**右上方的黄橙色方向**小幅拖动，仍然靠近中心，参考下方 **`40° 12`** 的读数。
6. 保留 **Midtones**、**Global** 的默认值；**Blending** 保留 `50`，**Balance** 保留 `0`；不要移动色轮下方的亮度滑块。

**这一项的精度限制：**Mac 1.4.5 没有供色轮 Hue/Saturation 输入的数字框，下方读数也经过取整。
尝试几次即可；无法达到目标时，请记录实际显示的两组数值并截图，然后继续保存实际结果。
即使显示目标整数，也不能据此认定内部数值完全相等；这一组的精确对比由回传文件继续核实。
在第 3.1 节重新打开滤镜的第二轮操作中，也要记录最终应用时的实际色轮读数。

完成取消、重新设置、**OK**、一次撤销和重做，再保存：

- 项目：`02-mixer-grading-mac-edited.comp`
- 图片：`02-mixer-grading-mac.png`
- 建议截图：`02-mixer-grading-readouts.png`（应用前能看到实际色轮读数的截图）

### 3.4 第 03 组：Effects、Detail、Optics

打开：**`CameraRaw/03-effects-detail-optics-source.comp`**。

| 展开的分组 | 在分组里继续找 | 参数 | 要设置的值 |
| --- | --- | --- | --- |
| **Effects** | 分组上部 | **Texture** | `12` |
| **Effects** | 分组上部 | **Clarity** | `10` |
| **Effects** | 分组上部 | **Dehaze** | `8` |
| **Effects** | **Vignette** 小标题下 | **Amount** | `-15` |
| **Detail** | **Sharpening** 小标题下 | **Amount** | `35` |
| **Detail** | **Noise Reduction** 小标题下 | **Luminance** | `15` |
| **Optics** | **Manual** 小标题下 | **Distortion** | `8` |

**Amount** 在多个位置出现，务必先看上方小标题。
**Optics** 中保留 **Enable Lens Profile Corrections** 未勾选；使用 **Manual** 下的 **Distortion**。
不要误改镜头配置区域里的另一个同名参数。

完成第 3.1 节全部流程，再保存：

- 项目：`03-effects-detail-optics-mac-edited.comp`
- 图片：`03-effects-detail-optics-mac.png`

### 3.5 第 04 组：Geometry、Calibration

打开：**`CameraRaw/04-geometry-calibration-source.comp`**。

| 展开的分组 | 控件位置 | 要设置的值 |
| --- | --- | --- |
| **Geometry** | **Upright** | 保留 **Off** |
| **Geometry** | **Projection** | 保留 **Perspective** |
| **Geometry** | **Rotate** | `3` |
| **Geometry** | **Vertical** | `12` |
| **Geometry** | **Horizontal** | `-6` |
| **Geometry** | **Aspect** | `8` |
| **Geometry** | **Constrain Crop** | 保留未勾选 |
| **Calibration** | **Process** | 保留 **Version 6** |
| **Calibration** | **Shadows** 小标题下的 **Tint** | `10` |
| **Calibration** | **Red Primary** 小标题下的 **Hue** | `15` |
| **Calibration** | **Blue Primary** 小标题下的 **Saturation** | `-10` |

本组改的是 **Calibration → Shadows → Tint**，不是 **Color** 分组中的 **Tint**。
完成第 3.1 节全部流程，再保存：

- 项目：`04-geometry-calibration-mac-edited.comp`
- 图片：`04-geometry-calibration-mac.png`

## 4．Camera Raw 的补充交互检查

这些检查使用 **`CameraRaw/00-source.comp`** 的测试副本，不要修改上面四组的输出。
参考图从上到下有黑白阶梯、较宽的灰色渐变带、彩色色带和棋盘纹理。
每项开始前重新打开原始 `00-source.comp`，选 **Camera Raw probe**，打开 **Filter → Camera Raw Filter…**。
前一项以 **Cancel** 结束时，也可直接在同一份未修改文档重新打开滤镜。

### 4.1 Auto 白平衡和吸管

1. 展开 **Color**，将 **White Balance** 设为 **Auto**，记录 **Temperature**、**Tint** 的实际值和画面变化。
2. 点击 **White Balance** 附近的吸管图标，在图像中间较宽的**灰色渐变带中央**单击一次。不要点黑白端点或底部棋盘格。
3. 记录新的 **Temperature**、**Tint** 读数，再点击吸管图标结束取色。
4. 点击 **Cancel**，确认画面恢复。把 Auto 和吸管两个结果分别记录。

### 4.2 Point Color 取色与 Visualize Range

1. 展开 **Color Mixer**，切换到 **Point Color**，点击吸管图标。
2. 在彩色色带偏右的蓝色区域单击；检查面板里是否出现所取颜色的色块。
3. 找到 **Saturation Shift**，把滑块从中间向左拖一小段，观察所选蓝色区域是否变得不那么鲜艳。这里没有数字输入框，不要求输入某个精确数值。
4. 勾选 **Visualize Range**，观察范围预览；取消勾选，观察正常画面；再勾选，并截一张图存为 `extras-point-color-preview.png`。
5. 在 **Visualize Range** 仍勾选时点击 **OK**。最终画面应保留颜色调整，但不能保留范围预览的遮罩/变暗显示。
6. 保存 `extras-point-color-mac.comp`，导出 `extras-point-color-mac.png`，重开检查。记录吸管、范围显示和应用后的结果。

### 4.3 分组临时关闭与单项重置

1. 重新打开原始 `00-source.comp` 的滤镜，展开 **Light**，将 **Exposure** 输入为 `1`，按 **Tab**。
2. 此时 **Light** 标题附近应出现眼睛图标。点击它，观察此分组的效果暂时关闭，但 Exposure 数字仍为 `1`；再点击一次，应恢复效果。
3. 在 **Exposure** 的文字标签或滑块控制点上**双击**，确认数值恢复为 `0`。
4. 点击 **Cancel**，记录眼睛开关和单项重置是否有效。

这里检查的是分组的眼睛开关和单项双击重置；不需要寻找名为“分组 Reset”的按钮。

### 4.4 Point 曲线

1. 打开未修改副本的滤镜，展开 **Curve**，选择 **Point**，将 **Channel** 设为 **RGB**。
2. 在曲线图的对角线中间单击增加一个点，再将这个点略向上拖动，观察曲线和画面变化。
3. 记录图下方 **In / Out** 的读数，点击 **Cancel**，确认原画面恢复。

### 4.5 Histogram / Vectorscope 与曝光裁切提示

1. 打开未修改副本的滤镜，在参数区域最上方的图表上**右键单击**，或按住 **Control** 再单击。
2. 在弹出菜单中选择 **Vectorscope**，确认图表切换；再次右键，选择 **Histogram**，确认切回来。
3. 将 **Light → Exposure** 设为 `2`，**Blacks** 设为 `-40`。
4. 点击图表**左上角小三角形**（**Shadow Clipping Indicator**），应以蓝色标出暗部裁切；点击**右上角小三角形**（**Highlight Clipping Indicator**），应以红色标出亮部裁切。
5. 再点一次对应三角形可关闭该提示。确认开关有效后，将提示打开，截图为 `extras-clipping-preview.png`。
6. 在提示打开时点击 **OK**。最终图像应保留曝光变化，但不能把蓝色/红色诊断覆盖层写进去。
7. 保存 `extras-clipping-mac.comp`，导出 `extras-clipping-mac.png`，重开确认。记录切换图表、两个三角形和应用结果。

### 4.6 Sharpening Masking 预览

1. 打开未修改副本的滤镜，展开 **Detail**，在 **Sharpening** 下将 **Amount** 设为 `35`。
2. **按住 Option 不松开，同时拖动 Masking 滑块**，例如拖到约 `50`，观察是否出现黑白的锐化蒙版预览。
3. 松开 **Option**，应恢复正常彩色画面。点击 **Cancel**，记录结果；这一项无需另存项目。

## 5．合成 PSD / PSB 的导入与编辑

### 5.1 组、蒙版、裁切：先 PSD，再 PSB

先关闭其他测试项目，再点击 **File → Import Images…**，打开：
**`Photoshop/01-groups-masks-clipping.psd`**。
如果出现导入转换说明，等待读取完成，记录全部提示，再点 **Import** 继续。

按以下顺序查看右侧 **Layers**：

1. 找到 **Folder**，点击它左侧的展开箭头。它应该包含 **Masked blue 基底** 和 **Clipped orange** 两个子图层；这里是一层文件夹，不要求多层嵌套。
2. 找到 **Hidden green**，初始应该隐藏。点它的可见性图标使其显示，应在画面左上方出现绿色块；再点一次隐藏，恢复初始状态。
3. 单击 **Clipped orange**，查看 **Blend** 应为 **Multiply**，**Opacity** 约为 `78%`（源文件值为 200/255，显示可能取整）。确认有裁切图层标记，例如名称前的 **↳**；不要修改混合方式或透明度。
4. 查看 **Masked blue 基底** 的中文名字是否保留，以及图层旁是否有独立的蒙版缩略图。画面中蓝色区域受蒙版影响；不需要在蒙版上绘画。
5. 保持 **Hidden green** 隐藏，按第 2.2 节保存、导出并重开：
   - `01-groups-masks-clipping-psd-mac.comp`
   - `01-groups-masks-clipping-psd-mac.png`
6. 关闭项目，再用 **File → Import Images…** 打开 **`Photoshop/01-groups-masks-clipping.psb`**，重复步骤 1–4。两种格式的画面和图层结构应一致。
7. 将 PSB 结果另存为：
   - `01-groups-masks-clipping-psb-mac.comp`
   - `01-groups-masks-clipping-psb-mac.png`
8. 在 `RESULT.txt` 中分别写 PSD 和 PSB 的结果，包含文件夹、隐藏状态、中文名称、混合模式、透明度、蒙版、裁切和重开检查。

### 5.2 可编辑文字和矩形

1. 关闭其他项目，用 **File → Import Images…** 打开 **`Photoshop/02-editable-text-shape.psd`**。若提示字体替换或转换，请记录提示原文后继续。
2. **先不修改**，保存 `02-editable-text-shape-original-mac.comp`，导出 `02-editable-text-shape-original-mac.png`，保留未编辑的比较图。
3. 在 **Layers** 中找到 **Editable text**。**双击它的文字缩略图**进入文字编辑；不要双击图层名字，那会进入重命名。
4. 若双击缩略图没进入编辑，选择左侧 **Aa** 文字工具，再点画布上的原文字。
5. 在画布上的文字编辑框中按 **Cmd+A**，输入 **`Editable ABC X`**，然后按 **Cmd+Return** 完成文字编辑。画面上应比原来多出末尾的 ` X`。这里不要按 Escape，Escape 会取消文字编辑。
6. 单击 **Editable rectangle** 图层的名字，选中矩形。点击左侧最上方的双向斜箭头 **Transform** 工具，勾选上方的 **Show Controls**。
7. 在上方 **W / H** 附近找到链条图标。如果它处于选中/高亮的锁定状态，点击一次解除宽高比例锁定。
8. 点击 **W** 数字，按 **Cmd+A**，输入 **`240`**，按 **Tab**；再将 **H** 改为 **`150`**，按 **Tab**。画面上的矩形应变宽、变高。普通数值修改离开字段后会应用；如果你之前进入了等待确认的 Transform 操作，且仍显示 **Apply**，再点击 **Apply**。
9. 保留文字和矩形的修改，保存 `02-editable-text-shape-edited-mac.comp`，导出 `02-editable-text-shape-edited-mac.png`。
10. 关闭并重开这份 edited 项目。再次双击文字缩略图，确认仍能编辑且文字为 **Editable ABC X**，按 **Cmd+Return** 完成；选中矩形查看 **W=240、H=150**，并记录是否仍标识为形状图层。有任何栅格化/转换提示也请记录。

如果文字编辑失败，请保存实际结果，并记录“没有进入文字编辑”或出现的错误；不要通过新增一层文字来代替原图层。

### 5.3 Levels 可编辑与不支持的 Posterize 提示

1. 关闭其他项目，用 **File → Import Images…** 打开 **`Photoshop/03-adjustment-conversions.psd`**。
2. 检查导入转换说明：针对 **Unsupported posterize**，预期提示为 **“This adjustment type isn’t supported and was skipped.”**。记录实际看到的完整文字，可截图为 `03-adjustment-conversions-notice.png`，再点 **Import**。
3. 单击 **Levels** 图层，选择 **Layer → Edit Adjustment…**；也可双击该调整图层的缩略图。
4. 应出现包含 **Input black**、**Gamma**、**Input white**、**Output black**、**Output white** 的编辑窗口。将 **Output white** 从 `255` 改为 **`200`**，按 **Tab**，观察预览变暗；点击 **Cancel**，应恢复原图。
5. 保存未修改结果为 `03-adjustment-conversions-original-mac.comp`，导出 `03-adjustment-conversions-original-mac.png`。
6. 再选中 **Levels**，打开 **Layer → Edit Adjustment…**，将 **Output white** 设为 **`200`**，按 **Tab**，点 **OK**。
7. 保存 `03-adjustment-conversions-edited-mac.comp`，导出 `03-adjustment-conversions-edited-mac.png`，关闭并重开。
8. 再次编辑 **Levels**，确认 **Output white** 保留为 **`200`**，然后点 **Cancel** 关闭检查窗口。

不要用 **Image** 菜单中的图像调整代替 **Layer → Edit Adjustment…**；这里要检查导入的调整图层。
不支持的 Posterize 被明确提示并跳过属于预期行为；若没有提示、整个导入失败或 Levels 无法编辑，请如实记录。

## 6．相机 RAW：CR2 与无预览 DNG

### 6.1 预览改参数，然后 Cancel

1. 先关闭其他项目，再点 **File → Import Images…**，选 **Canon-EOS40D.CR2**，点 **Open**。
2. 应出现标题为 **Develop “Canon-EOS40D.CR2”** 的窗口。等待预览加载完成；记录初始 **Exposure**、**Temperature**、**Tint**、**Boost**。
3. 此窗口四项参数都是**滑块加只读数字**，不能像滤镜面板一样直接输入。依次拖动滑块，目标如下：

   | 参数 | 目标读数 | 拖动参考 |
   | --- | --- | --- |
   | **Exposure** | `1.00 EV` | 从中间向右，约到滑轨的三分之二处 |
   | **Temperature** | `4000 K` | 约到滑轨从左起五分之一处 |
   | **Tint** | `20` | 中间略偏右 |
   | **Boost** | `0.80` | 从右端向左一点，约到滑轨五分之四处 |

4. 每次拖动后等待预览刷新，观察亮度、冷暖、色偏或明暗变化。尝试几次即可，记录实际读数；达不到整数目标时不要花很久寻找输入框。
5. 点击底部 **Cancel**。不应该新增导入文档或图层。记录是否符合预期。

### 6.2 Reset 后导入原始白平衡结果

1. 再用 **File → Import Images…** 打开同一份 CR2。
2. 随意将 **Exposure** 拖到约 `1.00 EV`，**Temperature** 拖到约 `4000 K`，确认预览变化。
3. 点击底部 **Reset**。应恢复 **Exposure=0.00 EV、Boost=1.00**，Temperature/Tint 应回到初始相机白平衡读数。
4. **Reset** 是此处实际按钮名；没有名为 **Reset to As Shot** 的按钮，也不需要寻找 **As Shot** 下拉选项。
5. 点击 **Import**，等待完整图像导入。记录实际文档宽高；Windows 的参考尺寸为 **3908 × 2602**，Mac 解码可能采用不同有效区域，请记录实际值，不要人为改尺寸。
6. 保存 `canon-as-shot-mac.comp`，导出 `canon-as-shot-mac.png`，关闭并重开，确认画面和尺寸保留。

### 6.3 只改 Exposure，再导入另一份

1. 关闭上一份项目，再从原始 CR2 使用 **File → Import Images…** 新导入一次。
2. 确认初始状态；如有旧参数，先点 **Reset**。本轮只将 **Exposure** 拖到目标 **`1.00 EV`**。
3. **Temperature / Tint 保留初始值，Boost 保留 1.00**。不要沿用第 6.1 节的 4000 / 20 / 0.80。
4. 记录实际 Exposure 读数，最好截图为 `canon-plus1-develop.png`，再点 **Import**。
5. 画面应比 as-shot 结果明亮。保存 `canon-plus1-mac.comp`，导出 `canon-plus1-mac.png`，关闭并重开，确认亮度和尺寸保留。

RAW 窗口里的 Return 会触发 **Import**；请用鼠标调滑块，全部确认后再点击 **Import**。
两种平台的 RAW 解码色彩和明暗算法不同，这里记录真实导入、控件、取消、重置和保存行为，不要求 PNG 的 RGB 完全一致。

### 6.4 无内嵌预览的 DNG

1. 关闭其他项目，点击 **File → Import Images…**，选测试文件夹根目录的 **bayer-no-preview.dng**。
2. 如果可以打开并显示预览，记录“接受”及实际读数/尺寸；点 **Cancel** 检查没有新增文档。
3. 如接受，可再次导入初始状态，保存 `synthetic-dng-as-shot-mac.comp`，导出 `synthetic-dng-as-shot-mac.png`。
4. 如果被拒绝，记录“拒绝”及完整错误提示，截图为 `synthetic-dng-error.png`。此时没有 `.comp` / PNG 是正常的，不要转换成 JPEG 来代替这个测试。

此 DNG 没有内嵌 JPEG；Mac 拒绝这个合成样本时，保留结果即可。真实 CR2 的测试仍按上面继续。

## 7．记录与回传检查清单

### 7.1 RESULT.txt 怎么写

请在 `Mac-return/RESULT.txt` 中按下面格式填写。可以直接复制模板；冒号后是需要补写的内容。
每项填“通过 / 失败 / 未完成”，并写出实际现象。只看过保存图片、没有操作 Cancel 等按钮时，不要填写对应操作通过。

```text
macOS 版本：
Compositor 版本：

01：Preview 关/开；Cancel；OK；一次 Cmd+Z；一次 Cmd+Shift+Z；保存重开：
02：以上操作结果：
02 最终 Shadows 读数（Hue / Saturation）：
02 最终 Highlights 读数（Hue / Saturation）：
02 色轮是否达到目标显示值，有无截图：
03：Preview 关/开；Cancel；OK；一次 Cmd+Z；一次 Cmd+Shift+Z；保存重开：
04：Preview 关/开；Cancel；OK；一次 Cmd+Z；一次 Cmd+Shift+Z；保存重开：

Auto 白平衡的 Temperature / Tint；吸管后的读数；Cancel：
Point Color 取色；Visualize Range 开关；OK 后覆盖层是否消失；重开：
Light 眼睛关闭/开启；Exposure 双击重置到 0：
Point 曲线拖动与 In / Out；Cancel：
Histogram / Vectorscope 切换：
暗部蓝色与亮部红色裁切提示；OK 后是否消失；重开：
Option + Masking 拖动时黑白预览；松开后的正常画面：

PSD：Folder 子层；Hidden green；中文名；Multiply；Opacity；蒙版；裁切；重开：
PSB：上述检查结果；与 PSD 是否一致：
文字是否变成 Editable ABC X；重开后能否编辑：
矩形 W / H 实际值；重开后形状图层标识：
字体替换或其他转换提示全文：
Posterize 提示全文：
Levels Output white 255→200 的预览/Cancel；OK 后重开是否保留 200：
真实 PSD：此前已回传，不重复测试。

CR2 初始 Exposure / Temperature / Tint / Boost：
CR2 改为目标 1.00 EV / 4000 K / 20 / 0.80 后的实际读数与预览：
CR2 Cancel 是否没有新增文档：
CR2 Reset 后的读数：
as-shot Import 完整宽高；保存重开：
plus1 Import 时 Exposure 实际读数，其余是否保持初始值：
plus1 完整宽高；是否更明亮；保存重开：
DNG 接受/拒绝；完整错误或导入结果：

找不到的按钮、失败步骤、其他问题：
```

### 7.2 回传前核对文件

| 测试 | `Mac-return` 内应保留的结果 |
| --- | --- |
| 四组滤镜 | 四个 `*-mac-edited.comp` 和四个 `*-mac.png`，名称见第 3 节 |
| Point Color | `extras-point-color-mac.comp` / `.png` 与范围预览截图 |
| 裁切提示 | `extras-clipping-mac.comp` / `.png` 与提示开启时截图 |
| PSD / PSB | 各一套 `01-groups-masks-clipping-psd-mac` 和 `...-psb-mac` 的 `.comp` / `.png` |
| 文字/形状 | `02-editable-text-shape-original-mac`、`...-edited-mac` 各一套 `.comp` / `.png` |
| 调整图层 | `03-adjustment-conversions-original-mac`、`...-edited-mac` 各一套 `.comp` / `.png`，以及实际出现的转换提示 |
| 相机 CR2 | `canon-as-shot-mac`、`canon-plus1-mac` 各一套 `.comp` / `.png`，plus1 的实际参数记录 |
| 合成 DNG | 接受时的结果，或拒绝时的提示/截图 |
| 操作记录 | 填好的 `RESULT.txt`；色轮等只能拖动的控件需记录实际值 |

未完成或失败项没有结果文件时，在 `RESULT.txt` 说明原因即可，不需要伪造输出。
`.comp` 在 Mac 上可能显示为一个项目图标，实际内部包含文件。回传时**复制整个 `.comp` 项目及内部 images**，不要只复制 `manifest.json` 或预览图。

将整个 `Mac-return` 文件夹复制回 Windows，建议放在：

`C:\Users\sr9rfx\.claude-project\compositor-windows\build-artifacts\phase7-acceptance-20261006-order-fixed\Mac-return`

回传完成后提供实际路径，即可继续核对。请保留源文件及失败记录；不需要在最终保存前再次撤销已保留的修改。
