# Phase 7 Windows 人工验收

使用本次 Phase 7 开发便携包，完整解压，运行其中的 `Compositor.exe`。
文件夹中必须保留 `CompositorRaw.exe`、三个 DLL 和 `LibRaw-notices`。
旧的 0.8.0 发布包不包含这些新功能。

测试文件位于 `build-artifacts/phase7-acceptance-20261006`。所有操作使用副本，
输出保存到新建的 `Windows-return` 文件夹，不覆盖原 PSD、DNG 或 source.comp。
将下面每项实际操作的成功、失败或缺少按钮记录在 `WINDOWS-RESULT.txt` 中。

## Camera Raw

打开 `CameraRaw/01-light-color-curve-source.comp`：File → Open Project，
选择整个 `.comp` 文件夹。选中 `Camera Raw probe` 图层，点击 Filter → Camera Raw。

1. Light 中将 Exposure 改为 **1**，按 Enter 只提交输入框数值；图像应变亮，
   对话框仍然打开。取消勾选/重新勾选底部 Preview，对比原图与预览。
2. 点击 Cancel，确认恢复原图。重新打开，Exposure 设为 **1**，点击 OK。
   **Ctrl+Z 一次**恢复原图，**Ctrl+Shift+Z 一次**恢复更改。
3. 保留更改，File → Save As 保存 `camera-raw-windows.comp`，关闭/重新打开，
   确认仍是变亮后的图像；Export PNG 保存对应 PNG。
4. 重新打开 Camera Raw，选择 Clipping view → Highlights，观察诊断图。
   不改其他参数，点击 OK；诊断图应消失，保存图像不得变成黑白诊断图。
5. 检查 Light 的 Enabled 开关与 Reset Light；检查 Curve 的点曲线拖动、
   右键删除中间点；检查 Auto White Balance 和两个 Eyedropper。
   选择 Histogram/Vectorscope 应显示对应图表。完成后 Cancel。

其余三份 Camera Raw 项目可按 `MAC-STEPS.md` 的数值表重复 Preview、Cancel、
Apply、Undo/Redo、保存重开。Windows 按 Ctrl，Mac 按 Cmd。

## PSD/PSB

用 **File → Import Images** 分别导入 `Photoshop` 下的四份文件。
每次使用独立窗口或关闭上一份已保存的项目，避免导入为同一项目的新图层。

- `01-groups-masks-clipping.psd` / `.psb`：图层应保留 Folder、隐藏的绿层、
  中文名称、蒙版、剪贴和 Multiply；PSD 与 PSB 应显示相同结果。
- `02-editable-text-shape.psd`：选择文本层，经 Layer → Edit Text 修改文字，
  保存副本；选择矩形层调整尺寸，保存副本。重新打开后仍应可编辑。
  导入时的缓存文本/字体提示应记录。Windows 的图层行双击用于重命名，
  请使用 Layer → Edit Text。
- `03-adjustment-conversions.psd`：保留 Levels，明确提示不支持的 Posterize
  被跳过，不能静默消失。记录完整提示。
- `Real-PSD/source.psd`：确认画布 **2364 × 1330**、`图层 1` 和 `背景` 两层；
  前者 Screen，后者 Normal。保存项目副本并导出 PNG。

## 相机 RAW

先用 File → Import Images 选择 `bayer-no-preview.dng`，之后还需真实相机 RAW。
合成 DNG 只有传感器数据，没有内嵌 JPEG/缩略图。

1. 应打开 Develop Camera RAW，显示 **128 × 96** 和预览。
2. Exposure **+1**、Temperature **4000**、Tint **+20**，等待预览变化。
   在数值框按 Enter 不应直接导入；Cancel 后不新增文档或图层。
3. 再导入同一文件，修改数值，点击 Reset to As Shot，确认 Exposure 回到 **0**，
   Temperature/Tint 恢复初始值。点击 Import，保存 `raw-as-shot-windows.comp` 和 PNG。
4. 再单独导入，只有 Exposure **+1**，保存 `raw-exposure-plus1-windows.comp` 和 PNG。
   关闭/重开两份项目，确认图像保留、尺寸完整。
5. 使用真实相机 RAW 重复上述操作，记录文件名、相机型号、输出尺寸和错误。
   原始 RAW 文件不得覆盖。Windows 与 Mac 使用不同 RAW 解码器，不能要求 RGB 完全一致。

## 记录与回传

`WINDOWS-RESULT.txt` 写明包的构建标记、Windows 版本、各手势结果、所有错误或
缺少的控件。回传完整 `.comp` 文件夹及 PNG。保存前要保留修改结果；若测试过
Undo，请 Redo 后再保存。关闭最后一个标签应显示空的深色工作区。
