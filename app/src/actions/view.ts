import { useEditor } from "../state/store";

let changing = false;
export async function toggleFullscreen(): Promise<void> {
  const s=useEditor.getState();
  if(changing || (!s.activeId && !s.canvasOnly))return;
  changing=true;
  const next=!s.canvasOnly;
  try {
    if(s.bridge?.setFullscreen)await s.bridge.setFullscreen(next);
    useEditor.setState({canvasOnly:next});
  } catch(e) { s.setError(String(e)); }
  finally {changing=false;}
}
export function repeatLastFilter():void {
  const s=useEditor.getState(), d=s.activeId?s.documents[s.activeId]:null;
  if(!s.lastFilter || !d?.activeLayerId || !s.canAdjust(s.lastFilter.filter))return;
  const command={type:"ApplyFilter" as const,id:d.activeLayerId,params:structuredClone(s.lastFilter)};
  if(s.usesJob(command.id, command.params.filter === "Vignette"))void s.runEditJob(command,command.id);else s.run(command);
}
