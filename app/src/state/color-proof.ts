import {create} from "zustand";
import {useEditor} from "./store";
import {validateProofLut,type ColorProof,type ColorProfile} from "../canvas/color-proof";
import type {ShellBridge} from "../shell/bridge";
interface State extends ColorProof {profiles:ColorProfile[];loading:boolean;error:string|null}
export const useColorProof=create<State>(()=>({profile:null,lut:null,proof:false,warning:false,revision:0,profiles:[],loading:false,error:null}));
let request=0,initialized=false;
export async function chooseColorProfile(bridge:ShellBridge,path:string,importProfile=false):Promise<void>{
  if(!bridge.proofColorProfile)return;const token=++request;useColorProof.setState({loading:true,error:null});
  try{
    const result=await bridge.proofColorProfile(path,importProfile);const lut=validateProofLut(result.lut);if(token!==request)return;
    const old=useColorProof.getState();useColorProof.setState({profile:result.profile,lut,revision:old.revision+1,profiles:[...old.profiles.filter(p=>p.path!==result.profile.path),result.profile]});
    try{localStorage.setItem("compositor.cmykProfile",result.profile.path);}catch{/* Session remains usable without storage. */}
    useEditor.getState().invalidate();
  }catch(e){if(token===request)useColorProof.setState({error:String(e)});}
  finally{if(token===request)useColorProof.setState({loading:false});}
}
export async function initializeColorProfiles(bridge:ShellBridge):Promise<void>{
  if(initialized||!bridge.listColorProfiles)return;initialized=true;
  try{
    const profiles=await bridge.listColorProfiles();useColorProof.setState({profiles});let chosen:string|null=null;try{chosen=localStorage.getItem("compositor.cmykProfile");}catch{}
    if(chosen&&profiles.some(p=>p.path===chosen))await chooseColorProfile(bridge,chosen);
    else if(profiles.length)await chooseColorProfile(bridge,profiles[0].path);
  }catch(e){initialized=false;useColorProof.setState({error:String(e)});}
}
export function setColorProof(patch:{proof?:boolean;warning?:boolean}):void {useColorProof.setState(patch);useEditor.getState().invalidate();}
