export type Sampling = "Nearest" | "Smooth" | "High quality";
export type BlendMode = "Normal" | "Multiply" | "Screen" | "Overlay" | "Darken" | "Lighten" | "Difference"
  | "Color Dodge" | "Color Burn" | "Hue" | "Saturation" | "Color" | "Luminosity";

export interface LayerTransform { origin: [number, number]; size: [number, number]; rotation: number; flipX: boolean; flipY: boolean; sampling: Sampling; }

export interface LayerState {
  id: string; name: string; visible: boolean; isGroup: boolean; parentId: string | null; opacity: number;
  blendMode: BlendMode; transform: LayerTransform; pixelsWidth: number; pixelsHeight: number; pixelsRevision: number; hasMask: boolean;
}

export interface DocumentState {
  id: string; documentId: string; width: number; height: number; resolution: number; activeLayerId: string | null;
  canUndo: boolean; canRedo: boolean; isModified: boolean; path: string | null; layers: LayerState[];
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
  | { type: "FlipCanvas"; horizontal: boolean };

export interface Dirty { structure: boolean; canvas: boolean; layers: string[]; }

export interface PackageFiles { manifest: string; images: { name: string; bytes: Uint8Array }[]; }
