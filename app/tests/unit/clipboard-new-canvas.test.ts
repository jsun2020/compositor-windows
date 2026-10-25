import {beforeEach,describe,expect,it,vi} from "vitest";
const state=vi.hoisted(()=>({activeId:null as string|null,working:false,busy:false,cropRect:null,engine:{newDocument:vi.fn(),pastePixels:vi.fn(),closeDocument:vi.fn()},bridge:{readClipboardImage:vi.fn()},jobs:{run:vi.fn()},panelOwnsDocument:()=>false,commitTransform:()=>{},commitGradient:()=>{},openDocument:vi.fn(),refresh:vi.fn(),revealActiveLayer:vi.fn(),error:null as string|null}));
vi.mock("../../src/state/store",()=>({useEditor:{getState:()=>state,setState:(patch:object)=>Object.assign(state,patch)}}));
vi.mock("../../src/actions/layers",()=>({duplicateSelected:vi.fn()}));
import {pastePixels} from "../../src/actions/clipboard";
const pixels=new Uint8Array(48).buffer;
beforeEach(()=>{vi.clearAllMocks();Object.assign(state,{activeId:null,working:false,busy:false,error:null});state.engine.newDocument.mockReturnValue("new");state.engine.pastePixels.mockImplementation(()=>{});state.bridge.readClipboardImage.mockResolvedValue({bytes:new Uint8Array([1]),origin:[9,12]});state.jobs.run.mockResolvedValue({pixels,header:JSON.stringify({width:3,height:2,pixelDepth:"Sixteen"})});});
describe("paste as new canvas",()=>{
 it("makes a canvas at clipboard dimensions and preserves deep pixels at origin",async()=>{
  await pastePixels();expect(state.engine.newDocument).toHaveBeenCalledWith(3,2,false);
  expect(state.engine.pastePixels).toHaveBeenCalledWith("new",3,2,pixels,[0,0],false,"Sixteen");expect(state.openDocument).toHaveBeenCalledWith("new");expect(state.working).toBe(false);
 });
 it("keeps normal paste in the active document and explicitly opens a separate canvas",async()=>{
  state.activeId="existing";await pastePixels();expect(state.engine.newDocument).not.toHaveBeenCalled();expect(state.engine.pastePixels).toHaveBeenLastCalledWith("existing",3,2,pixels,[9,12],false,"Sixteen");
  await pastePixels(true);expect(state.openDocument).toHaveBeenCalledWith("new");
 });
 it("never creates a canvas on decode failure and closes one if insertion fails",async()=>{
  state.jobs.run.mockRejectedValueOnce(new Error("Invalid image"));await pastePixels();expect(state.engine.newDocument).not.toHaveBeenCalled();expect(state.error).toBe("Invalid image");
  state.engine.pastePixels.mockImplementationOnce(()=>{throw new Error("Too large");});await pastePixels(true);expect(state.engine.closeDocument).toHaveBeenCalledWith("new");expect(state.openDocument).not.toHaveBeenCalled();expect(state.error).toBe("Too large");expect(state.working).toBe(false);
 });
});
