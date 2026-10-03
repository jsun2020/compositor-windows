import {expect,it} from "vitest";
import {textFontCss} from "../../src/tools/text-font";

it("selects the bold/italic family for Mac PostScript face names without changing their spelling",()=>{
  expect(textFontCss("Verdana-Bold",36)).toBe('bold 36px "Verdana-Bold", "Verdana", sans-serif');
  expect(textFontCss("TimesNewRomanPSMT",36)).toBe('36px "TimesNewRomanPSMT", "Times New Roman", sans-serif');
  expect(textFontCss("Arial-BoldItalicMT",24)).toBe('italic bold 24px "Arial-BoldItalicMT", "Arial", sans-serif');
  expect(textFontCss("Helvetica-Oblique",90)).toBe('italic 90px "Helvetica-Oblique", "Arial", sans-serif');
});

it("keeps arbitrary installed family names and quotes them safely",()=>{
  expect(textFontCss("Microsoft YaHei",32)).toBe('32px "Microsoft YaHei", sans-serif');
  expect(textFontCss('Private "Family"',32)).toBe('32px "Private \\"Family\\"", sans-serif');
  expect(textFontCss("toString",32)).toBe('32px "toString", sans-serif');
});
