import {createRoot} from "react-dom/client";
import {Sheet} from "./Sheet";
import {useEditor} from "../state/store";

/** Conversion notices must be visible in both browsers and native WebView2.
 * Keep the import guard until the reader dismisses the complete report. */
export function showImportConversions(messages:string[]):Promise<void>{
  return new Promise(resolve=>{
    useEditor.getState().openSheet({kind:"importReport"});
    const host=document.createElement("div");host.dataset.importReport="true";document.body.append(host);const root=createRoot(host);let closed=false;
    const close=()=>{if(closed)return;closed=true;queueMicrotask(()=>{root.unmount();host.remove();if(useEditor.getState().sheet?.kind==="importReport")useEditor.getState().closeSheet();resolve();});};
    root.render(<Sheet title="Photoshop import conversions" primary="Close" canConfirm dismissOnly onConfirm={close} onCancel={close}><p>The imported project is open. Review these conversions before editing.</p><ul>{messages.map((message,i)=><li key={i}>{message}</li>)}</ul></Sheet>);
  });
}
