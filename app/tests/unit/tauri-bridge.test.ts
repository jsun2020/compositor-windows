import {expect,it,vi} from "vitest";
vi.mock("@tauri-apps/api/core",()=>({invoke:vi.fn(async()=>{})}));
vi.mock("@tauri-apps/api/webview",()=>({getCurrentWebview:vi.fn()}));
vi.mock("@tauri-apps/plugin-dialog",()=>({open:vi.fn(),save:vi.fn()}));
import {invoke} from "@tauri-apps/api/core";
import {TauriBridge} from "../../src/shell/tauri-bridge";

it("sends image placement outside the browser-controlled Origin header",async()=>{
  const image=new Uint8Array([1,2,3]);
  await new TauriBridge().writeClipboardImage(image,[318,239],"11111111-1111-1111-1111-111111111111");
  expect(invoke).toHaveBeenCalledWith("write_clipboard_image",image,{headers:{
    "compositor-pixel-origin":"[318,239]","layer-token":"11111111-1111-1111-1111-111111111111",
  }});
  expect((vi.mocked(invoke).mock.calls[0][2] as {headers:Record<string,string>}).headers).not.toHaveProperty("origin");
});
