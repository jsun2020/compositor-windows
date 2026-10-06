// Camera Raw v1.4.5 settings; kept in sync with engine/adjust/camera_raw.rs.
export interface CameraRawSettings {
  temperature:number;
  tint:number;
  exposure:number;
  contrast:number;
  highlights:number;
  shadows:number;
  whites:number;
  blacks:number;
  vibrance:number;
  saturation:number;
  texture:number;
  clarity:number;
  dehaze:number;
  glow:number;
  glowRange:number;
  glowSpread:number;
  glowWarmth:number;
  vignetteAmount:number;
  vignetteMidpoint:number;
  vignetteRoundness:number;
  vignetteFeather:number;
  vignetteHighlights:number;
  grainAmount:number;
  grainSize:number;
  grainRoughness:number;
  whiteBalance:"Custom" | "Auto";
  glowStyle:"Diffusion" | "Bloom" | "Halation";
  vignetteStyle:"Highlight Priority" | "Color Priority" | "Paint Overlay";
  curve:CameraRawCurve;
  mixer:CameraRawMixer;
  grading:CameraRawGrading;
  detail:CameraRawDetail;
  optics:CameraRawOptics;
  geometry:CameraRawGeometry;
  calibration:CameraRawCalibration;
  seed:number;
  pixelScale:number;
}
export interface CameraRawCurve {
  shadows:number;
  darks:number;
  lights:number;
  highlights:number;
  shadowSplit:number;
  darkSplit:number;
  lightSplit:number;
  refineSaturation:number;
  rgb:{x:number;y:number}[];
  red:{x:number;y:number}[];
  green:{x:number;y:number}[];
  blue:{x:number;y:number}[];
}
export interface CameraRawDetail {
  sharpenAmount:number;
  sharpenRadius:number;
  sharpenDetail:number;
  sharpenMasking:number;
  noiseLuminance:number;
  noiseLuminanceDetail:number;
  noiseLuminanceContrast:number;
  noiseColor:number;
  noiseColorDetail:number;
  noiseColorSmoothness:number;
}
export interface CameraRawOptics {
  profileDistortion:number;
  profileVignetting:number;
  distortion:number;
  purpleAmount:number;
  purpleHueLow:number;
  purpleHueHigh:number;
  greenAmount:number;
  greenHueLow:number;
  greenHueHigh:number;
  vignetteAmount:number;
  vignetteMidpoint:number;
  removeChromaticAberration:boolean;
  enableLensProfile:boolean;
}
export interface CameraRawCalibration {
  shadowTint:number;
  redHue:number;
  redSaturation:number;
  greenHue:number;
  greenSaturation:number;
  blueHue:number;
  blueSaturation:number;
  process:"Version 1" | "Version 2" | "Version 3" | "Version 4" | "Version 5" | "Version 6";
}
export interface CameraRawWheel {
  hue:number;
  saturation:number;
  luminance:number;
}
export interface CameraRawPointColor {
  hue:number;
  saturation:number;
  luminance:number;
  hueShift:number;
  saturationShift:number;
  luminanceShift:number;
  hueRange:number;
  saturationRange:number;
  luminanceRange:number;
}
export interface CameraRawGeometry {
  vertical:number;
  horizontal:number;
  rotate:number;
  aspect:number;
  scale:number;
  offsetX:number;
  offsetY:number;
  upright:"Off" | "Guided";
  projection:"Perspective" | "Rectilinear";
  constrainCrop:boolean;
  guides:GeometryGuide[];
}
export interface CameraRawMixer {
  hue:number[];
  saturation:number[];
  luminance:number[];
  points:CameraRawPointColor[];
}
export interface CameraRawGrading {
  shadows:CameraRawWheel;
  midtones:CameraRawWheel;
  highlights:CameraRawWheel;
  global:CameraRawWheel;
  blending:number;
  balance:number;
}
export interface GeometryGuide {
  startX:number;
  startY:number;
  endX:number;
  endY:number;
}
export const DEFAULT_CAMERA_RAW:CameraRawSettings = {
  "temperature": 0.0,
  "tint": 0.0,
  "exposure": 0.0,
  "contrast": 0.0,
  "highlights": 0.0,
  "shadows": 0.0,
  "whites": 0.0,
  "blacks": 0.0,
  "vibrance": 0.0,
  "saturation": 0.0,
  "texture": 0.0,
  "clarity": 0.0,
  "dehaze": 0.0,
  "glow": 0.0,
  "glowRange": 0.0,
  "glowSpread": 0.0,
  "glowWarmth": 0.0,
  "vignetteAmount": 0.0,
  "vignetteMidpoint": 50.0,
  "vignetteRoundness": 0.0,
  "vignetteFeather": 50.0,
  "vignetteHighlights": 0.0,
  "grainAmount": 0.0,
  "grainSize": 25.0,
  "grainRoughness": 50.0,
  "whiteBalance": "Custom",
  "glowStyle": "Diffusion",
  "vignetteStyle": "Highlight Priority",
  "seed": 0,
  "pixelScale": 1,
  "curve": {
    "shadows": 0.0,
    "darks": 0.0,
    "lights": 0.0,
    "highlights": 0.0,
    "shadowSplit": 25.0,
    "darkSplit": 50.0,
    "lightSplit": 75.0,
    "refineSaturation": 0.0,
    "rgb": [
      {
        "x": 0,
        "y": 0
      },
      {
        "x": 1,
        "y": 1
      }
    ],
    "red": [
      {
        "x": 0,
        "y": 0
      },
      {
        "x": 1,
        "y": 1
      }
    ],
    "green": [
      {
        "x": 0,
        "y": 0
      },
      {
        "x": 1,
        "y": 1
      }
    ],
    "blue": [
      {
        "x": 0,
        "y": 0
      },
      {
        "x": 1,
        "y": 1
      }
    ]
  },
  "mixer": {
    "hue": [
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0
    ],
    "saturation": [
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0
    ],
    "luminance": [
      0,
      0,
      0,
      0,
      0,
      0,
      0,
      0
    ],
    "points": []
  },
  "grading": {
    "shadows": {
      "hue": 0.0,
      "saturation": 0.0,
      "luminance": 0.0
    },
    "midtones": {
      "hue": 0.0,
      "saturation": 0.0,
      "luminance": 0.0
    },
    "highlights": {
      "hue": 0.0,
      "saturation": 0.0,
      "luminance": 0.0
    },
    "global": {
      "hue": 0.0,
      "saturation": 0.0,
      "luminance": 0.0
    },
    "blending": 50,
    "balance": 0
  },
  "detail": {
    "sharpenAmount": 0.0,
    "sharpenRadius": 10.0,
    "sharpenDetail": 25.0,
    "sharpenMasking": 0.0,
    "noiseLuminance": 0.0,
    "noiseLuminanceDetail": 50.0,
    "noiseLuminanceContrast": 0.0,
    "noiseColor": 0.0,
    "noiseColorDetail": 50.0,
    "noiseColorSmoothness": 50.0
  },
  "optics": {
    "profileDistortion": 100.0,
    "profileVignetting": 100.0,
    "distortion": 0.0,
    "purpleAmount": 0.0,
    "purpleHueLow": 270.0,
    "purpleHueHigh": 310.0,
    "greenAmount": 0.0,
    "greenHueLow": 60.0,
    "greenHueHigh": 120.0,
    "vignetteAmount": 0.0,
    "vignetteMidpoint": 50.0,
    "removeChromaticAberration": false,
    "enableLensProfile": false
  },
  "geometry": {
    "vertical": 0.0,
    "horizontal": 0.0,
    "rotate": 0.0,
    "aspect": 0.0,
    "scale": 0.0,
    "offsetX": 0.0,
    "offsetY": 0.0,
    "upright": "Off",
    "projection": "Perspective",
    "constrainCrop": false,
    "guides": []
  },
  "calibration": {
    "shadowTint": 0.0,
    "redHue": 0.0,
    "redSaturation": 0.0,
    "greenHue": 0.0,
    "greenSaturation": 0.0,
    "blueHue": 0.0,
    "blueSaturation": 0.0,
    "process": "Version 6"
  }
};
export const CAMERA_RAW_FIELDS = [
  {
    "group": "CameraRawSettings",
    "field": "temperature",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawSettings",
    "field": "tint",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawSettings",
    "field": "exposure",
    "default": 0.0,
    "min": -5.0,
    "max": 5.0
  },
  {
    "group": "CameraRawSettings",
    "field": "contrast",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawSettings",
    "field": "highlights",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawSettings",
    "field": "shadows",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawSettings",
    "field": "whites",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawSettings",
    "field": "blacks",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawSettings",
    "field": "vibrance",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawSettings",
    "field": "saturation",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawSettings",
    "field": "texture",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawSettings",
    "field": "clarity",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawSettings",
    "field": "dehaze",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawSettings",
    "field": "glow",
    "default": 0.0,
    "min": 0.0,
    "max": 100.0
  },
  {
    "group": "CameraRawSettings",
    "field": "glowRange",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawSettings",
    "field": "glowSpread",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawSettings",
    "field": "glowWarmth",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawSettings",
    "field": "vignetteAmount",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawSettings",
    "field": "vignetteMidpoint",
    "default": 50.0,
    "min": 0.0,
    "max": 100.0
  },
  {
    "group": "CameraRawSettings",
    "field": "vignetteRoundness",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawSettings",
    "field": "vignetteFeather",
    "default": 50.0,
    "min": 0.0,
    "max": 100.0
  },
  {
    "group": "CameraRawSettings",
    "field": "vignetteHighlights",
    "default": 0.0,
    "min": 0.0,
    "max": 100.0
  },
  {
    "group": "CameraRawSettings",
    "field": "grainAmount",
    "default": 0.0,
    "min": 0.0,
    "max": 100.0
  },
  {
    "group": "CameraRawSettings",
    "field": "grainSize",
    "default": 25.0,
    "min": 0.0,
    "max": 100.0
  },
  {
    "group": "CameraRawSettings",
    "field": "grainRoughness",
    "default": 50.0,
    "min": 0.0,
    "max": 100.0
  },
  {
    "group": "CameraRawCurve",
    "field": "shadows",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawCurve",
    "field": "darks",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawCurve",
    "field": "lights",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawCurve",
    "field": "highlights",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawCurve",
    "field": "shadowSplit",
    "default": 25.0,
    "min": 5.0,
    "max": 90.0
  },
  {
    "group": "CameraRawCurve",
    "field": "darkSplit",
    "default": 50.0,
    "min": 7.0,
    "max": 95.0
  },
  {
    "group": "CameraRawCurve",
    "field": "lightSplit",
    "default": 75.0,
    "min": 9.0,
    "max": 98.0
  },
  {
    "group": "CameraRawCurve",
    "field": "refineSaturation",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawDetail",
    "field": "sharpenAmount",
    "default": 0.0,
    "min": 0.0,
    "max": 150.0
  },
  {
    "group": "CameraRawDetail",
    "field": "sharpenRadius",
    "default": 10.0,
    "min": 0.0,
    "max": 100.0
  },
  {
    "group": "CameraRawDetail",
    "field": "sharpenDetail",
    "default": 25.0,
    "min": 0.0,
    "max": 100.0
  },
  {
    "group": "CameraRawDetail",
    "field": "sharpenMasking",
    "default": 0.0,
    "min": 0.0,
    "max": 100.0
  },
  {
    "group": "CameraRawDetail",
    "field": "noiseLuminance",
    "default": 0.0,
    "min": 0.0,
    "max": 100.0
  },
  {
    "group": "CameraRawDetail",
    "field": "noiseLuminanceDetail",
    "default": 50.0,
    "min": 0.0,
    "max": 100.0
  },
  {
    "group": "CameraRawDetail",
    "field": "noiseLuminanceContrast",
    "default": 0.0,
    "min": 0.0,
    "max": 100.0
  },
  {
    "group": "CameraRawDetail",
    "field": "noiseColor",
    "default": 0.0,
    "min": 0.0,
    "max": 100.0
  },
  {
    "group": "CameraRawDetail",
    "field": "noiseColorDetail",
    "default": 50.0,
    "min": 0.0,
    "max": 100.0
  },
  {
    "group": "CameraRawDetail",
    "field": "noiseColorSmoothness",
    "default": 50.0,
    "min": 0.0,
    "max": 100.0
  },
  {
    "group": "CameraRawOptics",
    "field": "profileDistortion",
    "default": 100.0,
    "min": 0.0,
    "max": 100.0
  },
  {
    "group": "CameraRawOptics",
    "field": "profileVignetting",
    "default": 100.0,
    "min": 0.0,
    "max": 100.0
  },
  {
    "group": "CameraRawOptics",
    "field": "distortion",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawOptics",
    "field": "purpleAmount",
    "default": 0.0,
    "min": 0.0,
    "max": 100.0
  },
  {
    "group": "CameraRawOptics",
    "field": "purpleHueLow",
    "default": 270.0,
    "min": 0.0,
    "max": 360.0
  },
  {
    "group": "CameraRawOptics",
    "field": "purpleHueHigh",
    "default": 310.0,
    "min": 0.0,
    "max": 360.0
  },
  {
    "group": "CameraRawOptics",
    "field": "greenAmount",
    "default": 0.0,
    "min": 0.0,
    "max": 100.0
  },
  {
    "group": "CameraRawOptics",
    "field": "greenHueLow",
    "default": 60.0,
    "min": 0.0,
    "max": 360.0
  },
  {
    "group": "CameraRawOptics",
    "field": "greenHueHigh",
    "default": 120.0,
    "min": 0.0,
    "max": 360.0
  },
  {
    "group": "CameraRawOptics",
    "field": "vignetteAmount",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawOptics",
    "field": "vignetteMidpoint",
    "default": 50.0,
    "min": 0.0,
    "max": 100.0
  },
  {
    "group": "CameraRawCalibration",
    "field": "shadowTint",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawCalibration",
    "field": "redHue",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawCalibration",
    "field": "redSaturation",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawCalibration",
    "field": "greenHue",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawCalibration",
    "field": "greenSaturation",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawCalibration",
    "field": "blueHue",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawCalibration",
    "field": "blueSaturation",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawWheel",
    "field": "hue",
    "default": 0.0,
    "min": 0.0,
    "max": 360.0
  },
  {
    "group": "CameraRawWheel",
    "field": "saturation",
    "default": 0.0,
    "min": 0.0,
    "max": 100.0
  },
  {
    "group": "CameraRawWheel",
    "field": "luminance",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawPointColor",
    "field": "hue",
    "default": 0.0,
    "min": 0.0,
    "max": 360.0
  },
  {
    "group": "CameraRawPointColor",
    "field": "saturation",
    "default": 0.0,
    "min": 0.0,
    "max": 1.0
  },
  {
    "group": "CameraRawPointColor",
    "field": "luminance",
    "default": 0.0,
    "min": 0.0,
    "max": 1.0
  },
  {
    "group": "CameraRawPointColor",
    "field": "hueShift",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawPointColor",
    "field": "saturationShift",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawPointColor",
    "field": "luminanceShift",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawPointColor",
    "field": "hueRange",
    "default": 30.0,
    "min": 5.0,
    "max": 180.0
  },
  {
    "group": "CameraRawPointColor",
    "field": "saturationRange",
    "default": 0.4,
    "min": 0.05,
    "max": 1.0
  },
  {
    "group": "CameraRawPointColor",
    "field": "luminanceRange",
    "default": 0.4,
    "min": 0.05,
    "max": 1.0
  },
  {
    "group": "CameraRawGeometry",
    "field": "vertical",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawGeometry",
    "field": "horizontal",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawGeometry",
    "field": "rotate",
    "default": 0.0,
    "min": -45.0,
    "max": 45.0
  },
  {
    "group": "CameraRawGeometry",
    "field": "aspect",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawGeometry",
    "field": "scale",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawGeometry",
    "field": "offsetX",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  },
  {
    "group": "CameraRawGeometry",
    "field": "offsetY",
    "default": 0.0,
    "min": -100.0,
    "max": 100.0
  }
] as const;
export const freshCameraRaw = ():CameraRawSettings => structuredClone(DEFAULT_CAMERA_RAW);
export type CameraRawGroupName="Light"|"Color"|"Curve"|"Color Mixer"|"Point Color"|"Color Grading"|"Effects"|"Detail"|"Optics"|"Geometry"|"Calibration";
const GROUP_FIELDS:Partial<Record<CameraRawGroupName,(keyof CameraRawSettings)[]>>={
  Light:["exposure","contrast","highlights","shadows","whites","blacks"],Color:["temperature","tint","vibrance","saturation","whiteBalance"],
  Curve:["curve"],"Color Grading":["grading"],Effects:["texture","clarity","dehaze","glow","glowRange","glowSpread","glowWarmth","glowStyle","vignetteAmount","vignetteMidpoint","vignetteRoundness","vignetteFeather","vignetteHighlights","vignetteStyle","grainAmount","grainSize","grainRoughness"],Detail:["detail"],Optics:["optics"],Geometry:["geometry"],Calibration:["calibration"],
};
export function resetCameraRawGroup(settings:CameraRawSettings,group:CameraRawGroupName):CameraRawSettings {
  const next=structuredClone(settings);
  if(group==="Color Mixer"){next.mixer.hue=[...DEFAULT_CAMERA_RAW.mixer.hue];next.mixer.saturation=[...DEFAULT_CAMERA_RAW.mixer.saturation];next.mixer.luminance=[...DEFAULT_CAMERA_RAW.mixer.luminance];}
  else if(group==="Point Color")next.mixer.points=[];
  else for(const key of GROUP_FIELDS[group]??[])Object.assign(next,{[key]:structuredClone(DEFAULT_CAMERA_RAW[key])});
  return next;
}
/** Group bypass is panel state; the same effective settings feed preview and commit. */
export function effectiveCameraRaw(settings:CameraRawSettings,bypass:CameraRawGroupName[]=[]):CameraRawSettings {return bypass.reduce(resetCameraRawGroup,settings);}

/** Mirrors only the settings' active/idle switches; pixel arithmetic stays in WASM. */
export function cameraRawIsIdentity(s:CameraRawSettings):boolean {
  const zero=(values:number[])=>values.every(v=>v===0);
  const curve=(points:{x:number;y:number}[])=>points.length===2&&points[0].x===0&&points[0].y===0&&points[1].x===1&&points[1].y===1;
  return zero([s.temperature,s.tint,s.exposure,s.contrast,s.highlights,s.shadows,s.whites,s.blacks,s.vibrance,s.saturation,s.texture,s.clarity,s.dehaze,s.glow,s.vignetteAmount,s.grainAmount])
    &&zero([s.curve.shadows,s.curve.darks,s.curve.lights,s.curve.highlights,s.curve.refineSaturation])&&[s.curve.rgb,s.curve.red,s.curve.green,s.curve.blue].every(curve)
    &&zero([...s.mixer.hue,...s.mixer.saturation,...s.mixer.luminance])&&s.mixer.points.every(p=>zero([p.hueShift,p.saturationShift,p.luminanceShift]))
    &&[s.grading.shadows,s.grading.midtones,s.grading.highlights,s.grading.global].every(w=>zero([w.saturation,w.luminance]))
    &&zero([s.detail.sharpenAmount,s.detail.noiseLuminance,s.detail.noiseColor])
    &&!s.optics.removeChromaticAberration&&!s.optics.enableLensProfile&&zero([s.optics.distortion,s.optics.purpleAmount,s.optics.greenAmount,s.optics.vignetteAmount])
    &&zero([s.calibration.shadowTint,s.calibration.redHue,s.calibration.redSaturation,s.calibration.greenHue,s.calibration.greenSaturation,s.calibration.blueHue,s.calibration.blueSaturation])
    &&zero([s.geometry.vertical,s.geometry.horizontal,s.geometry.rotate,s.geometry.aspect,s.geometry.scale,s.geometry.offsetX,s.geometry.offsetY])&&(s.geometry.upright!=="Guided"||s.geometry.guides.length===0);
}
