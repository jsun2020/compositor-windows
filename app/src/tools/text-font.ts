/** Mac records font face names; CSS selects a family plus weight/style. Preserve
 * the saved name and translate only when rasterizing on Windows. */
const faces: Record<string, [string, string]> = {
    ArialMT: ["Arial", ""], "Arial-BoldMT": ["Arial", "bold "],
    "Arial-ItalicMT": ["Arial", "italic "], "Arial-BoldItalicMT": ["Arial", "italic bold "],
    TimesNewRomanPSMT: ["Times New Roman", ""], "TimesNewRomanPS-BoldMT": ["Times New Roman", "bold "],
    "TimesNewRomanPS-ItalicMT": ["Times New Roman", "italic "], "TimesNewRomanPS-BoldItalicMT": ["Times New Roman", "italic bold "],
    CourierNewPSMT: ["Courier New", ""], "CourierNewPS-BoldMT": ["Courier New", "bold "],
    "CourierNewPS-ItalicMT": ["Courier New", "italic "], "CourierNewPS-BoldItalicMT": ["Courier New", "italic bold "],
    Helvetica: ["Arial", ""], "Helvetica-Bold": ["Arial", "bold "],
    "Helvetica-Oblique": ["Arial", "italic "], "Helvetica-BoldOblique": ["Arial", "italic bold "],
};
export function textFontCss(name: string, size: number): string {
  const face = Object.prototype.hasOwnProperty.call(faces, name) ? faces[name] : undefined;
  const standard = /^(Verdana|Georgia|Tahoma|TrebuchetMS)-(BoldItalic|Bold|Italic)$/.exec(name);
  const full = /^(Arial|Verdana|Georgia|Tahoma|Times New Roman|Courier New|Trebuchet MS) (Bold Italic|Bold|Italic)$/.exec(name);
  const match = standard ?? full;
  const family = face?.[0] ?? (match ? match[1] === "TrebuchetMS" ? "Trebuchet MS" : match[1] : null);
  const variant = match?.[2].replace(/ /g, "");
  const prefix = face?.[1] ?? (variant === "BoldItalic" ? "italic bold " : variant === "Bold" ? "bold " : variant === "Italic" ? "italic " : "");
  const quoted = (value: string) => JSON.stringify(value.replace(/[\r\n]/g, ""));
  return `${prefix}${size}px ${quoted(name)}${family && family !== name ? `, ${quoted(family)}` : ""}, sans-serif`;
}
