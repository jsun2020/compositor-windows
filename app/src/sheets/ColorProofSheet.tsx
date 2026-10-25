import {useEffect} from "react";
import {useEditor} from "../state/store";
import {useColorProof,initializeColorProfiles,chooseColorProfile,setColorProof} from "../state/color-proof";
import {Sheet} from "./Sheet";
export function ColorProfilePicker({disabled=false}:{disabled?:boolean}){
  const bridge=useEditor(s=>s.bridge),color=useColorProof();
  useEffect(()=>{if(bridge)void initializeColorProfiles(bridge);},[bridge]);
  const load=async()=>{try{const path=await bridge?.pickColorProfile?.();if(path&&bridge)await chooseColorProfile(bridge,path,true);}catch(e){useColorProof.setState({error:String(e)});}};
  return <>
    <label>Print profile <select aria-label="CMYK profile" disabled={disabled||color.loading||!bridge?.proofColorProfile} value={color.profile?.path??""} onChange={e=>{if(bridge)void chooseColorProfile(bridge,e.target.value);}}><option value="" disabled>Choose a CMYK ICC profile</option>{color.profiles.map(p=><option value={p.path} key={p.path}>{p.name}</option>)}</select></label>
    <button type="button" disabled={disabled||color.loading||!bridge?.pickColorProfile} onClick={()=>void load()}>Load ICC Profile…</button>
    {color.loading&&<p role="status">Preparing color proof…</p>}{color.error&&<p role="alert">{color.error}</p>}
    {!color.profiles.length&&!color.loading&&<p>Load the CMYK output profile supplied by your printer. Windows may not include a generic CMYK profile.</p>}
  </>;
}
export function ColorProofSheet(){
  const close=useEditor(s=>s.closeSheet),color=useColorProof();
  return <Sheet title="CMYK Proof" primary="Done" canConfirm={!color.loading} onCancel={close} onConfirm={close}>
    <ColorProfilePicker/>
    <label><input type="checkbox" checked={color.proof} disabled={!color.lut||color.loading} onChange={e=>setColorProof({proof:e.target.checked})}/> Proof Colors</label>
    <label><input type="checkbox" checked={color.warning} disabled={!color.lut||color.loading} onChange={e=>setColorProof({warning:e.target.checked})}/> Gamut Warning</label>
    <p>Simulate the selected print profile. Gray marks colors outside its gamut. Your document stays RGB; choose CMYK in Export As for print output.</p>
  </Sheet>;
}
