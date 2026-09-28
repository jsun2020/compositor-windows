export type Sampling = "Nearest" | "Smooth" | "High quality";
export type BlendMode = "Normal" | "Multiply" | "Screen" | "Overlay" | "Darken" | "Lighten" | "Difference"
  | "Color Dodge" | "Color Burn" | "Hue" | "Saturation" | "Color" | "Luminosity"
  // Mac 1.2.6 additions (R 2.4), drawn since Phase 3.5b.
  | "Linear Burn" | "Linear Dodge (Add)" | "Soft Light" | "Hard Light" | "Vivid Light" | "Linear Light"
  | "Pin Light" | "Hard Mix" | "Exclusion" | "Subtract" | "Divide";

export interface LayerTransform { origin: [number, number]; size: [number, number]; rotation: number; flipX: boolean; flipY: boolean; sampling: Sampling; }

export type PointTuple = [number, number];
export type Corners = [PointTuple, PointTuple, PointTuple, PointTuple];

export type AdjustmentKind = "Hue/Saturation" | "Levels" | "Curves" | "Exposure" | "Gradient Map" | "Grain"
  // Mac 1.2.6 additions (R 3).
  | "Add Noise" | "Gaussian Blur" | "Motion Blur" | "Invert" | "Black & White" | "Color Balance";
/** Every adjustment kind, in the order the Mac's New Adjustment Layer menu lists them
 * (`AdjustmentKind.allCases`, LayerAdjustment.swift:4-9). */
export const ADJUSTMENT_KINDS: AdjustmentKind[] = ["Hue/Saturation", "Levels", "Curves", "Exposure", "Gradient Map", "Grain",
  "Add Noise", "Gaussian Blur", "Motion Blur", "Invert", "Black & White", "Color Balance"];
/** Whether the kind opens a panel: all but Invert, which has nothing to set (engine `AdjustmentKind::is_editable`). */
export const isEditableKind = (kind: AdjustmentKind): boolean => kind !== "Invert";
/** The kinds that blur what lies beneath them (engine `AdjustmentKind::is_spatial`). */
export const isSpatialKind = (kind: AdjustmentKind): boolean => kind === "Gaussian Blur" || kind === "Motion Blur";
export type LevelsChannel = "RGB" | "Red" | "Green" | "Blue";
export type ColorRange = "Master" | "Reds" | "Yellows" | "Greens" | "Cyans" | "Blues" | "Magentas";
export type LevelsAuto = "Contrast" | "Color" | "Neutral";
export type LevelsSample = "Black" | "Gray" | "White";

export interface LevelRange { black: number; gamma: number; white: number; outputBlack: number; outputWhite: number; }
export interface LevelsSettings { channel: LevelsChannel; ranges: LevelRange[]; }
export interface CurvePoint { x: number; y: number; }
export interface CurvesSettings { channel: LevelsChannel; channels: CurvePoint[][]; }
export interface HueBand { falloffStart: number; rangeStart: number; rangeEnd: number; falloffEnd: number; }
export interface RangeAdjustment { hue: number; saturation: number; lightness: number; }
export interface HueSaturationSettings {
  range: ColorRange; colorize: boolean; invertRange: boolean;
  adjustments: Partial<Record<ColorRange, RangeAdjustment>>;
  bands: Partial<Record<ColorRange, HueBand>>;
}
export interface AdjustmentColor { red: number; green: number; blue: number; }
export interface ExposureSettings { exposure: number; offset: number; gamma: number; }
export interface GradientMapSettings { shadows: AdjustmentColor; highlights: AdjustmentColor; reversed: boolean; }
export interface GrainSettings { amount: number; size: number; roughness: number; seed: number; }
export interface BlackWhiteSettings {
  blues: number; cyans: number; greens: number; magentas: number; reds: number;
  tint: boolean; tintHue: number; tintSaturation: number; yellows: number;
}
export interface ColorBalanceSettings {
  highlightCyanRed: number; highlightMagentaGreen: number; highlightYellowBlue: number;
  midCyanRed: number; midMagentaGreen: number; midYellowBlue: number; preserveLuminosity: boolean;
  shadowCyanRed: number; shadowMagentaGreen: number; shadowYellowBlue: number;
}
/** What an absent `blackWhiteSettings` means: Photoshop's mix, not identity (ImageAdjustments.swift:117-126). */
export const DEFAULT_BLACK_WHITE: BlackWhiteSettings = { reds: 40, yellows: 60, greens: 40, cyans: 60, blues: 20, magentas: 80, tint: false, tintHue: 40, tintSaturation: 20 };
/** What an absent `colorBalanceSettings` means: all zero, with Preserve Luminosity on (ImageAdjustments.swift:148-157). */
export const DEFAULT_COLOR_BALANCE: ColorBalanceSettings = { highlightCyanRed: 0, highlightMagentaGreen: 0, highlightYellowBlue: 0,
  midCyanRed: 0, midMagentaGreen: 0, midYellowBlue: 0, preserveLuminosity: true, shadowCyanRed: 0, shadowMagentaGreen: 0, shadowYellowBlue: 0 };
/** The Mac's LayerAdjustment: the optional settings are written only when present, so a project
 * saved by either app re-encodes byte for byte. */
export interface LayerAdjustment {
  kind: AdjustmentKind; hue: number; saturation: number; lightness: number; colorize: boolean;
  hsvSettings?: HueSaturationSettings; levels: LevelsSettings; curves: CurvesSettings;
  exposureSettings?: ExposureSettings; gradientMapSettings?: GradientMapSettings; grainSettings?: GrainSettings;
  blackWhiteSettings?: BlackWhiteSettings; colorBalanceSettings?: ColorBalanceSettings;
  blurRadius?: number; motionAngle?: number; motionDistance?: number;
  noiseAmount?: number; noiseGaussian?: boolean; noiseMonochromatic?: boolean; noiseSeed?: number;
}

export type FilterParams =
  | { filter: "GaussianBlur"; radius: number }
  | { filter: "MotionBlur"; angle: number; distance: number }
  | { filter: "AddNoise"; amount: number; gaussian: boolean; monochromatic: boolean; seed: number }
  | { filter: "LensCorrection"; distortion: number };
export type FilterKind = FilterParams["filter"];

export type PreviewRequest =
  | { preview: "Adjustment"; layer: string; adjustment: LayerAdjustment }
  /** The same while a slider moves: previewed from a smaller copy until input settles. */
  | { preview: "DragAdjustment"; layer: string; adjustment: LayerAdjustment }
  | { preview: "Filter"; layer: string; params: FilterParams };

export interface LayerState {
  id: string; name: string; visible: boolean; isGroup: boolean; parentId: string | null; opacity: number;
  blendMode: BlendMode; transform: LayerTransform; pixelsWidth: number; pixelsHeight: number; pixelsRevision: number; hasMask: boolean;
  hasPixels: boolean; maskWidth: number; maskHeight: number; maskRevision: number; maskEnabled: boolean; maskLinked: boolean;
  maskSourceId: string | null; maskPlacement: LayerTransform | null; maskBackground: number; adjustment?: LayerAdjustment;
}

export interface Guide { id: string; axis: "horizontal" | "vertical"; position: number; }

export interface Coverage { layerId: string; maskRevision: number; placement: LayerTransform; corners: Corners | null; width: number; height: number; background: number; nearest: boolean; }
export interface LayerDraw {
  id: string; transform: LayerTransform; corners: Corners | null; pixelsWidth: number; pixelsHeight: number; pixelsRevision: number; opacity: number;
  blend: BlendMode; coverages: Coverage[]; clip: string | null; adjustment?: LayerAdjustment;
  /** An adjustment layer whose own mode is not Normal: full coverage, original alpha kept (engine LayerDraw::keeps_alpha). */
  keepsAlpha: boolean;
  /** Drawn with its layer effects (engine LayerDraw::effects): `transform`, `corners` and the pixel
   * size are then the padded effects image's, and its texture comes from `EngineClient.drawPixels`. */
  effects: EffectsDraw | null;
}
/** A layer drawn with its effects (engine `EffectsDraw`): `inset` pixels added on every side; a new
 * `key` means a new image. */
export interface EffectsDraw { inset: number; key: string; }
export type PlanNode = { kind: "layer"; draw: LayerDraw } | { kind: "stack"; base: LayerDraw; children: LayerDraw[]; folderCoverages: Coverage[] };
export interface RenderPlan { nodes: PlanNode[]; sources: LayerDraw[]; /** Document pixels the plan's blurs reach (engine RenderPlan::spatial_margin). */ spatialMargin: number; }
/** A blur adjustment's sizes at one output scale (engine `SpatialBlur`): sigma, or distance and angle, in output px. */
export interface SpatialBlur { level: number; sigma: number; distance: number; angle: number; }
/** The halving lattice's cell and the render's pad, in output px (engine `SpatialGrid`). */
export interface SpatialGrid { cell: number; pad: number; }
export type PreviewEdit =
  | { kind: "layer"; id: string; draft: LayerTransform; corners?: Corners | null }
  | { kind: "group"; ids: string[]; box: LayerTransform; draft: LayerTransform; corners?: Corners | null }
  | { kind: "mask"; id: string; draft: LayerTransform }
  | { kind: "adjustment"; id: string; adjustment: LayerAdjustment };

export interface DocumentState {
  id: string; documentId: string; width: number; height: number; resolution: number; activeLayerId: string | null;
  canUndo: boolean; canRedo: boolean; isModified: boolean; path: string | null; layers: LayerState[];
  /** Entries on the undo stack: at most 100, fewer when their pixels pass 256 MiB (engine `History`). */
  undoDepth: number;
  /** The id of the entry an undo would take back, null with nothing to undo. A gesture that
   * recorded one command reads it straight after and compares it later to tell whether its own
   * entry is still on top; the depth cannot tell, once the cap trims the oldest entry. */
  undoEntryId: number | null;
  guides: Guide[];
  /** What this project contains that this build does not draw yet (`Document::undrawn`). */
  undrawn: string[];
  /** The selection's summary; null when nothing is selected. Its outline is fetched with
   * `EngineClient.selectionOutline` whenever `revision` changes. */
  selection: SelectionState | null;
}

/** How a new outline meets the selection (engine `SelectionMode`). */
export type SelectionMode = "Replace" | "Add" | "Subtract";
/** The drawn outline's kind (engine `SelectionShape`): the Lasso's two and the Marquee's two. */
export type SelectionShape = "Freehand" | "Polygonal" | "Rectangle" | "Ellipse";
/** The Magic Wand's options (engine `WandSettings`): tolerance 0-255, sample radius 0 (point),
 * 1 (3x3) or 2 (5x5). */
export interface WandSettings { tolerance: number; sampleRadius: number; contiguous: boolean; allLayers: boolean; }
export const DEFAULT_WAND: WandSettings = { tolerance: 32, sampleRadius: 0, contiguous: true, allLayers: false };
/** Engine `SelectionState`. `empty`: an explicit empty selection, which every edit refuses. */
export interface SelectionState {
  revision: number; empty: boolean; bounds: { x: number; y: number; width: number; height: number } | null;
  antialiased: boolean; feather: number; points: number;
}

export type Command =
  | { type: "AddBlankLayer" }
  | { type: "RenameLayer"; id: string; name: string }
  | { type: "SetLayerVisible"; id: string; visible: boolean }
  | { type: "DeleteLayer"; id: string }
  | { type: "SetActiveLayer"; id: string | null }
  | { type: "CanvasSize"; width: number; height: number; anchor: number; fill: [number, number, number] | null }
  | { type: "Crop"; x: number; y: number; width: number; height: number }
  | { type: "ImageSize"; width: number; height: number; resolution: number; sampling: Sampling }
  | { type: "FlipCanvas"; horizontal: boolean }
  | { type: "SetLayerOpacity"; id: string; opacity: number }
  | { type: "SetLayersOpacity"; ids: string[]; opacity: number }
  | { type: "SetLayerBlendMode"; id: string; mode: BlendMode }
  | { type: "AddGroup" }
  | { type: "GroupLayers"; ids: string[] }
  | { type: "PlaceLayer"; id: string; parent: string | null; above: string | null; atBottom: boolean }
  | { type: "MoveLayerBy"; id: string; offset: number }
  | { type: "DuplicateLayer"; id: string }
  | { type: "DuplicateLayerTo"; id: string; parent: string | null; above: string | null; atBottom: boolean }
  | { type: "DuplicateLayerTransformed"; id: string; transform: LayerTransform }
  | { type: "DeleteLayers"; ids: string[]; bake: boolean }
  | { type: "SetLayerTransform"; id: string; transform: LayerTransform }
  | { type: "TransformLayers"; ids: string[]; box: LayerTransform; draft: LayerTransform }
  | { type: "FlipLayers"; ids: string[]; horizontal: boolean }
  | { type: "NudgeLayers"; ids: string[]; dx: number; dy: number }
  | { type: "DistortLayer"; id: string; transform: LayerTransform; corners: Corners }
  | { type: "DistortLayers"; ids: string[]; box: LayerTransform; draft: LayerTransform; corners: Corners }
  | { type: "SetMaskPlacement"; id: string; placement: LayerTransform }
  | { type: "AddMask"; id: string; revealing: boolean }
  | { type: "DeleteMask"; id: string }
  | { type: "SetMaskEnabled"; id: string; enabled: boolean }
  | { type: "SetMaskLinked"; id: string; linked: boolean }
  | { type: "InvertMask"; id: string }
  | { type: "FillMask"; id: string; white: boolean }
  | { type: "BlurMask"; id: string; radius: number }
  | { type: "CopyMask"; from: string; to: string }
  | { type: "ToggleClipping"; id: string }
  | { type: "ReleaseClipping"; id: string }
  | { type: "LinkMask"; source: string; target: string }
  | { type: "MergeLayers"; ids: string[] }
  | { type: "ApplyAdjustment"; id: string; adjustment: LayerAdjustment }
  | { type: "InvertPixels"; id: string; mask: boolean }
  | { type: "ApplyFilter"; id: string; params: FilterParams }
  | { type: "AddAdjustmentLayer"; kind: AdjustmentKind; seed: number; shadows: [number, number, number] | null; highlights: [number, number, number] | null }
  | { type: "SetAdjustment"; id: string; adjustment: LayerAdjustment }
  | { type: "SelectShape"; kind: SelectionShape; points: PointTuple[]; mode: SelectionMode; antialiased: boolean }
  | { type: "SelectAll" }
  | { type: "Deselect" }
  | { type: "InvertSelection" }
  | { type: "MoveSelection"; dx: number; dy: number }
  | { type: "ExpandSelection"; amount: number }
  | { type: "ContractSelection"; amount: number }
  | { type: "FeatherSelection"; amount: number }
  | { type: "MagicWand"; at: PointTuple; mode: SelectionMode; settings: WandSettings; antialiased: boolean }
  | { type: "LoadLayerSelection"; id: string; mode: SelectionMode; antialiased: boolean }
  | { type: "LoadMaskSelection"; id: string; mode: SelectionMode; antialiased: boolean }
  | { type: "ClearSelectedPixels"; id: string; mask: boolean }
  | { type: "AddMaskFromSelection"; id: string; revealing: boolean };

export interface Dirty { structure: boolean; canvas: boolean; layers: string[]; }
/** A rectangle of a layer's pixel grid (or its mask's), in whole pixels (engine `PixelRect`). */
export interface PixelRect { x: number; y: number; width: number; height: number; }

export interface PackageFiles { manifest: string; images: { name: string; bytes: Uint8Array }[]; }
