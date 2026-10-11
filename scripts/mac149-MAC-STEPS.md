# Mac Compositor 1.4.9 对比操作

这份步骤分为自动像素对比和实际应用操作。自动部分不需要你改 Camera Raw 的参数，也不需要 Photoshop。实际应用部分只操作随包附带的副本。

## 1. 准备并运行自动部分

1. 将整个 `Mac149-probes` 文件夹复制到 Mac 的桌面，保留文件夹里的子目录。
2. 确认 `/Applications/Compositor.app` 已更新到 **1.4.9**。在菜单 **Compositor > About Compositor** 检查版本。
3. 在 Finder 打开文件夹，双击 `MAC-149.command`。如果提示没有执行权限：打开 **Terminal**，输入 `chmod +x `（最后有一个空格），把 `MAC-149.command` 拖入 Terminal，按 Enter；随后输入 `bash `，再次拖入同一文件，按 Enter。
4. 脚本先检查哈希、记录 Mac 和应用版本，再编译固定 1.4.9 源码并自动计算所有参数。需要已安装 Apple Command Line Tools；如果提示没有 `xcrun`，在 Terminal 执行 `xcode-select --install`，安装完成后重新运行脚本。
5. 成功时最后显示 `PASS: all source pixel cases completed`，并生成 `Mac149-return-日期-时间` 文件夹。后续文件全部放进这个文件夹。
6. 如果脚本报错，保留已生成的文件夹及 `run.log`，直接回传；不用自行修改源码或参数。

自动部分包括曝光、画面相关明暗、白平衡、Calibration、Mixer、Grading、参数/点曲线、Texture/Clarity/Dehaze、Glow、暗角、Detail、Optics、Grain，以及 Scanlines 的默认、位移、原色、双色和 Glow 参数。完整参数已冻结在 `cases.json`。

## 2. 在实际 Compositor 应用中导出两个效果项目

这一步检查真正安装的应用，与自动源码结果分别记录。

1. 在 Compositor 选择 **File > Open Project…**，打开包内的 `effects-opaque.comp` 文件夹。
2. 不修改图层、效果、缩放或画布尺寸。选择 **File > Export PNG…**。
3. 保存到刚才的 `Mac149-return-日期-时间`，文件名写 **`effects-opaque.png`**。输出尺寸保持 **128 × 96 px**，保留透明度。
4. 再选择 **File > Open Project…**，打开 `effects-alpha.comp` 文件夹。
5. 同样选择 **File > Export PNG…**，保存为 **`effects-alpha.png`**，保持 **128 × 96 px**，保留透明度。
6. 如提示是否保存修改，选择 **Don't Save**；这两个输入项目不需要更改。

## 3. New Canvas、Rotate、Scanlines 和 Last Filter

1. 选择 **File > New Canvas…**。单位选 **in / Inches**，分辨率选 **300 DPI**，Width 输入 **1**，Height 输入 **2**，背景选 **White**，点击 **Create canvas**。
2. 应得到 **300 × 600 px** 的白色画布。选择 **Image > Rotate Canvas 90° Clockwise**，应变为 **600 × 300 px**。
3. 按 **Cmd+Z** 撤销旋转，应恢复 **300 × 600 px**；按 **Cmd+Shift+Z** 重做，应再变为 **600 × 300 px**。
4. 拖入包内 `input.png`，或选择 **File > Import Images…** 导入它，点选导入的图层。
5. 选择 **Filter > Scanlines…**，将 **Line Spacing 改为 7、Thickness 改为 61、Glow 改为 32**，其余参数保持默认。
6. 确认预览发生变化，点击 **Cancel**；画面应恢复原样。
7. 再打开 **Filter > Scanlines…**，重复上面三个参数，点击 **OK** 应用。按一次 **Cmd+Z** 应恢复应用前，再按 **Cmd+Shift+Z** 恢复 Scanlines。
8. 选择 **Filter > Last Filter: Scanlines**，应直接再应用一次，不打开参数面板。再按一次 **Cmd+Z**，应撤销这次重复应用。
9. 选择 **File > Save As…**，将目前项目副本保存到回传目录，命名 **`ui-edited.comp`**。这里要保留实际更改，不要再撤销回到最初状态后才保存。

## 4. Export As 的三个格式

1. 保持上一步项目打开，选择 **File > Export As…**。
2. **Format 选 PNG**，输出尺寸输入 **300 × 150 px**，预览应对应这个比例。点击 **Export…**，保存为 **`export-300x150.png`**。
3. 再打开 **File > Export As…**，**Format 选 JPEG**，输出 **300 × 150 px**，**Quality 选 85%**，背景保持白色，点击 **Export…**，保存为 **`export-300x150.jpg`**。
4. 再打开 **File > Export As…**，**Format 选 PDF**，输出 **300 × 150 px**，背景保持默认，点击 **Export…**，保存为 **`export-300x150.pdf`**。
5. 上述项目 DPI 为 300，因此 PDF 的 Page 应为 **72 × 36 pt**（物理尺寸 **1 × 0.5 in**）。导出后项目画布仍应为 **600 × 300 px**。

## 5. Search Commands、Fullscreen 和 Navigator

1. 按 **Cmd+F** 打开 **Search Commands…**，输入 **Scanlines**。用方向键选中命令，按 **Enter**，应打开 Scanlines 面板；点击 **Cancel** 退出。
2. 点击画布空白处，使输入框失去焦点。按 **F** 应进入仅画布全屏，按 **Esc** 应退出，并恢复原窗口。
3. 再进入 Scanlines 面板，点一个参数输入框，输入字母 **f**，应只影响输入内容，不能切换全屏；点击 **Cancel**。
4. 在 **View** 菜单勾选 **Navigator (300% and above)**。用 **View > Zoom In** 多次放大到至少 **300%**，继续放大，直到画布部分超出窗口。
5. 右上角应出现小地图，点击或拖动小地图应平移可见区域，图层和历史不应被改变。

## 6. 回传

1. 把包根目录的 `RESULT.txt` 复制到 `Mac149-return-日期-时间`。
2. 用 TextEdit 以纯文本修改各项：通过写 **PASS**；有异常写 **FAIL**，并在 Notes 说明发生在哪一步。不必调整其他参数。
3. 把整个 `Mac149-return-日期-时间` 文件夹复制回 Windows：
   `C:\Users\sr9rfx\.claude-project\compositor-windows\build-artifacts\mac-1.4.9-delta-20261011\`
4. 回复回传的文件夹名称即可。自动程序、日志、RGBA 和 PNG 都保留，便于我核对实际像素；不需要逐项手工判断颜色差异。
