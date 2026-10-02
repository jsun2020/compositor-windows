import {describe,it,expect} from "vitest";
import {DEFAULT_TEXT,colorText,fontText,replaceText,validText,textColorAt,textFontAt} from "../../src/tools/text-style";
describe("UTF-16 editable text runs",()=>{
  it("counts a supplementary emoji as two units and preserves later runs across a replacement",()=>{
    let s={...DEFAULT_TEXT,content:"A😀中文B"};s=colorText(s,{start:1,end:3},[1,0,0]);s=fontText(s,{start:3,end:5},"Microsoft YaHei");
    expect(s.colorRuns).toEqual([{location:1,length:2,red:1,green:0,blue:0}]);expect(s.fontRuns).toEqual([{location:3,length:2,fontName:"Microsoft YaHei"}]);
    s=replaceText(s,"A😀x中文B");expect(s.fontRuns).toEqual([{location:4,length:2,fontName:"Microsoft YaHei"}]);expect(textColorAt(s,3)).toEqual([1,0,0]);expect(textFontAt(s,4)).toBe("Microsoft YaHei");expect(validText(s)).toBe(true);
  });
  it("an empty or whole range updates the base style, a partial change preserves unaffected letters",()=>{
    let s={...DEFAULT_TEXT,content:"Hello"};s=colorText(s,{start:1,end:4},[0,0,1]);expect(textColorAt(s,0)).toEqual([0,0,0]);expect(textColorAt(s,2)).toEqual([0,0,1]);
    s=colorText(s,{start:2,end:2},[1,1,1]);expect(s.colorRuns).toBeNull();expect(textColorAt(s,0)).toEqual([1,1,1]);s=fontText(s,{start:0,end:5},"Georgia");expect(s.fontName).toBe("Georgia");expect(s.fontRuns).toBeNull();
  });
  it("rejects overlap, out-of-bounds runs, non-finite settings and oversized text boxes",()=>{
    const s={...DEFAULT_TEXT,content:"abc"};expect(validText({...s,colorRuns:[{location:1,length:3,red:1,green:0,blue:0}]})).toBe(false);expect(validText({...s,fontSize:NaN})).toBe(false);expect(validText({...s,boxSize:[10000,10001]})).toBe(false);expect(validText({...s,fontRuns:[]})).toBe(false);
  });
});
