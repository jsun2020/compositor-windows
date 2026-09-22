export type Sampling = "Nearest" | "Smooth" | "High quality";
export type BlendMode = "Normal" | "Multiply" | "Screen" | "Overlay" | "Darken" | "Lighten" | "Difference"
  | "Color Dodge" | "Color Burn" | "Hue" | "Saturation" | "Color" | "Luminosity";

export interface LayerTransform { origin: [number, number]; size: [number, number]; rotation: number; flipX: boolean; flipY: boolean; sampling: Sampling; }

export type PointTuple = [number, number];
export type Corners = [PointTuple, PointTuple, PointTuple, PointTuple];

export type AdjustmentKind = "Hue/Saturation" | "Levels" | "Curves" | "Exposure" | "Gradient Map" | "Grain";
export const ADJUSTMENT_KINDS: AdjustmentKind[] = ["Hue/Saturation", "Levels", "Curves", "Exposure", "Gradient Map", "Grain"];
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
/** The Mac's LayerAdjustment: the optional settings are written only when present, so a project
 * saved by either app re-encodes byte for byte. */
export interface LayerAdjustment {
  kind: AdjustmentKind; hue: number; saturation: number; lightness: number; colorize: boolean;
  hsvSettings?: HueSaturationSettings; levels: LevelsSettings; curves: CurvesSettings;
  exposureSettings?: ExposureSettings; gradientMapSettings?: GradientMapSettings; grainSettings?: GrainSettings;
}

export type FilterParams =
  | { filter: "GaussianBlur"; radius: number }
  | { filter: "MotionBlur"; angle: number; distance: number }
  | { filter: "AddNoise"; amount: number; gaussian: boolean; monochromatic: boolean; seed: number }
  | { filter: "LensCorrection"; distortion: number };
export type FilterKind = FilterParams["filter"];

export type PreviewRequest =
  | { preview: "Adjustment"; layer: string; adjustment: LayerAdjustment }
  | { preview: "Filter"; layer: string; params: FilterParams };

export interface LayerState {
  id: string; name: string; visible: boolean; isGroup: boolean; parentId: string | null; opacity: number;
  blendMode: BlendMode; transform: LayerTransform; pixelsWidth: number; pixelsHeight: number; pixelsRevision: number; hasMask: boolean;
  hasPixels: boolean; maskWidth: number; maskHeight: number; maskRevision: number; maskEnabled: boolean; maskLinked: boolean;
  maskSourceId: string | null; maskPlacement: LayerTransform | null; maskBackground: number; adjustment?: LayerAdjustment;
}

export interface Coverage { layerId: string; maskRevision: number; placement: LayerTransform; corners: Corners | null; width: number; height: number; background: number; nearest: boolean; }
export interface LayerDraw {
  id: string; transform: LayerTransform; corners: Corners | null; pixelsWidth: number; pixelsHeight: number; pixelsRevision: number; opacity: number;
  blend: BlendMode; coverages: Coverage[]; clip: string | null; adjustment?: LayerAdjustment;
}
export type PlanNode = { kind: "layer"; draw: LayerDraw } | { kind: "stack"; base: LayerDraw; children: LayerDraw[]; folderCoverages: Coverage[] };
export interface RenderPlan { nodes: PlanNode[]; sources: LayerDraw[]; }
export type PreviewEdit =
  | { kind: "layer"; id: string; draft: LayerTransform; corners?: Corners | null }
  | { kind: "group"; ids: string[]; box: LayerTransform; draft: LayerTransform; corners?: Corners | null }
  | { kind: "mask"; id: string; draft: LayerTransform }
  | { kind: "adjustment"; id: string; adjustment: LayerAdjustment };

export interface DocumentState {
  id: string; documentId: string; width: number; height: number; resolution: number; activeLayerId: string | null;
  canUndo: boolean; canRedo: boolean; isModified: boolean; path: string | null; layers: LayerState[];
  /** Entries on the undo stack. A gesture that recorded one command compares this against the
   * depth it saw beforehand to tell whether its own entry is still the one on top. */
  undoDepth: number;
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
  | { type: "SetAdjustment"; id: string; adjustment: LayerAdjustment };

export interface Dirty { structure: boolean; canvas: boolean; layers: string[]; }

export interface PackageFiles { manifest: string; images: { name: string; bytes: Uint8Array }[]; }
