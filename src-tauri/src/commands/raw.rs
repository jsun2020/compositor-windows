//! Camera RAW is developed in an isolated, cancellable helper. It never edits the source.
use compositor_engine::{encode_png,encode_deep_png,Raster,MAX_PIXELS,MAX_SIDE};
use serde::{Deserialize,Serialize};
use std::{collections::HashMap,fs,io::Read,path::{Path,PathBuf},process::{Command,Stdio},sync::{Arc,Mutex,atomic::{AtomicBool,Ordering}},time::{Duration,Instant}};
use tauri::{ipc::Response,State};
use uuid::Uuid;
#[derive(Default)]
pub struct RawJobs { jobs:Mutex<HashMap<String,RawJob>>,queue:Arc<Mutex<()>> }
struct RawJob {cancel:Arc<AtomicBool>,started:bool,created:Instant}
impl RawJobs {
    fn register(&self,token:&str)->Result<Arc<AtomicBool>,String>{
        Uuid::parse_str(token).map_err(|_|"Invalid RAW request token")?;
        let mut jobs=self.jobs.lock().map_err(|_|"RAW job registry unavailable")?;
        jobs.retain(|_,job|job.started||job.created.elapsed()<Duration::from_secs(180));
        if jobs.len()>=128&&!jobs.contains_key(token){return Err("Too many RAW requests".into());}
        let job=jobs.entry(token.into()).or_insert_with(||RawJob{cancel:Arc::new(AtomicBool::new(false)),started:false,created:Instant::now()});
        if job.started{return Err("RAW request token is already running".into());}job.started=true;Ok(job.cancel.clone())
    }
    fn finish(&self,token:&str){if let Ok(mut jobs)=self.jobs.lock(){jobs.remove(token);}}
}
#[derive(Clone,Debug,Deserialize,Serialize)]
#[serde(rename_all="camelCase",deny_unknown_fields)]
pub struct DevelopSettings {pub exposure:f64,pub temperature:f64,pub tint:f64,pub tone:f64,#[serde(default)]pub noise_reduction:f64,#[serde(default)]pub highlight_recovery:bool}
impl DevelopSettings {
    fn validate(&self)->Result<(),String>{if !(-5.0..=5.0).contains(&self.exposure)||!(2000.0..=15000.0).contains(&self.temperature)||!(-150.0..=150.0).contains(&self.tint)||!(0.0..=1.0).contains(&self.tone)||!(0.0..=1.0).contains(&self.noise_reduction){Err("Invalid RAW develop settings".into())}else{Ok(())}}
}
struct TempFiles(Vec<PathBuf>);
impl Drop for TempFiles {fn drop(&mut self){for path in &self.0 {let _=fs::remove_file(path);}}}
fn helper()->Result<PathBuf,String>{let exe=std::env::current_exe().map_err(|e|e.to_string())?;let path=exe.parent().ok_or("Cannot locate RAW helper")?.join("CompositorRaw.exe");if !path.is_file(){return Err("The portable installation is missing CompositorRaw.exe. Extract the entire portable ZIP.".into());}Ok(path)}
fn checked_input(path:&str)->Result<PathBuf,String>{let p=PathBuf::from(path);let meta=fs::metadata(&p).map_err(|e|e.to_string())?;if !meta.is_file()||meta.len()>512*1024*1024 {return Err("RAW input must be a file under 512 MiB".into());}fs::canonicalize(p).map_err(|e|e.to_string())}
fn bounded_text(path:&Path)->Result<String,String>{let mut data=String::new();fs::File::open(path).map_err(|e|e.to_string())?.take(65537).read_to_string(&mut data).map_err(|e|e.to_string())?;if data.len()>65536 {return Err("RAW helper metadata exceeds its limit".into());}Ok(data)}
fn run(path:String,settings:Option<DevelopSettings>,preview:bool,cancel:Arc<AtomicBool>,queue:Arc<Mutex<()>>)->Result<Vec<u8>,String>{
    let started=Instant::now();let _queue=loop {if cancel.load(Ordering::Acquire){return Err("RAW development cancelled".into());}if started.elapsed()>Duration::from_secs(180){return Err("RAW development exceeded its time limit".into());}match queue.try_lock(){Ok(g)=>break g,Err(std::sync::TryLockError::Poisoned(_))=>return Err("RAW queue is unavailable".into()),Err(_)=>std::thread::sleep(Duration::from_millis(15))}};
    let input=checked_input(&path)?;if let Some(ref s)=settings {s.validate()?;}
    let temp=std::env::temp_dir().join(format!("compositor-raw-{}",Uuid::new_v4()));
    let pixels=temp.with_extension("rgba");let stdout=temp.with_extension("json");let stderr=temp.with_extension("log");let _cleanup=TempFiles(vec![pixels.clone(),stdout.clone(),stderr.clone()]);
    let mut command=Command::new(helper()?);
    #[cfg(windows)] {use std::os::windows::process::CommandExt;command.creation_flags(0x08000000);}
    command.arg(if settings.is_some(){"--develop"}else{"--inspect"}).arg(input);
    if let Some(ref s)=settings {command.arg(&pixels).args([s.exposure.to_string(),s.temperature.to_string(),s.tint.to_string(),s.tone.to_string(),if preview{"1"}else{"0"}.into(),s.noise_reduction.to_string(),if s.highlight_recovery{"1"}else{"0"}.into()]);}
    command.stdin(Stdio::null()).stdout(fs::OpenOptions::new().write(true).create_new(true).open(&stdout).map_err(|e|e.to_string())?).stderr(fs::OpenOptions::new().write(true).create_new(true).open(&stderr).map_err(|e|e.to_string())?);
    let mut child=command.spawn().map_err(|e|e.to_string())?;
    let status=loop {if cancel.load(Ordering::Acquire)||started.elapsed()>Duration::from_secs(180){let _=child.kill();let _=child.wait();return Err(if cancel.load(Ordering::Acquire){"RAW development cancelled"}else{"RAW development exceeded its time limit"}.into());}
        match child.try_wait(){Ok(Some(status))=>break status,Ok(None)=>std::thread::sleep(Duration::from_millis(15)),Err(e)=>{let _=child.kill();let _=child.wait();return Err(e.to_string());}}
    };
    if !status.success(){return Err(format!("Cannot develop RAW: {}",bounded_text(&stderr)?));}
    let metadata=bounded_text(&stdout)?;let _:serde_json::Value=serde_json::from_str(&metadata).map_err(|e|format!("Invalid RAW metadata: {e}"))?;
    if settings.is_none(){return Ok(metadata.into_bytes());}
    let png=read_pixels(&pixels,preview)?;if cancel.load(Ordering::Acquire){return Err("RAW development cancelled".into());}Ok(png)
}
fn read_pixels(path:&Path,preview:bool)->Result<Vec<u8>,String>{
    let mut file=fs::File::open(path).map_err(|e|e.to_string())?;let mut header=[0u8;16];file.read_exact(&mut header).map_err(|e|e.to_string())?;
    let deep=&header[..4]==b"RAW8"&&header[12..]==16u32.to_le_bytes();
    if !deep && (!preview || &header[..4]!=b"RAW7" || header[12..]!=[0;4]) {return Err("Invalid RAW helper output header; full imports require the current 16-bit helper".into());}
    let stride=if deep{8}else{4};
    let w=u32::from_le_bytes(header[4..8].try_into().unwrap());let h=u32::from_le_bytes(header[8..12].try_into().unwrap());let count=w as u64*h as u64;
    if w==0||h==0||w as i64>MAX_SIDE||h as i64>MAX_SIDE||count>MAX_PIXELS||file.metadata().map_err(|e|e.to_string())?.len()!=16+count*stride {return Err("RAW helper output dimensions or length are invalid".into());}
    let mut bytes=Vec::new();bytes.try_reserve_exact(count as usize*stride as usize).map_err(|_|"Not enough memory to import RAW")?;file.read_to_end(&mut bytes).map_err(|e|e.to_string())?;
    let mut raster=if deep {let samples=bytes.chunks_exact(2).map(|b|u16::from_le_bytes([b[0],b[1]])).collect();drop(bytes);Raster::from_rgba16(w,h,samples)}else{Raster::from_premultiplied(w,h,bytes)};
    if preview {while raster.width.max(raster.height)>2048 {raster=raster.halved();}}
    if preview {encode_png(&raster,72.0)}else{encode_deep_png(&raster,72.0)}.map_err(|e|e.to_string())
}
#[tauri::command]
pub async fn raw_inspect(path:String,token:String,state:State<'_,RawJobs>)->Result<serde_json::Value,String>{let cancel=state.register(&token)?;let queue=state.queue.clone();let result=tauri::async_runtime::spawn_blocking(move||run(path,None,false,cancel,queue)).await.map_err(|e|e.to_string());state.finish(&token);let bytes=result??;serde_json::from_slice(&bytes).map_err(|e|e.to_string())}
#[tauri::command]
pub async fn raw_develop(path:String,settings:DevelopSettings,preview:bool,token:String,state:State<'_,RawJobs>)->Result<Response,String>{
    settings.validate()?;
    let cancel=state.register(&token)?;
    let queue=state.queue.clone();let result=tauri::async_runtime::spawn_blocking(move||run(path,Some(settings),preview,cancel,queue)).await.map_err(|e|e.to_string());
    state.finish(&token);result?.map(Response::new)
}
#[tauri::command]
pub fn raw_cancel(token:String,state:State<'_,RawJobs>)->Result<(),String>{Uuid::parse_str(&token).map_err(|_|"Invalid RAW request token")?;let mut jobs=state.jobs.lock().map_err(|_|"RAW job registry unavailable")?;jobs.retain(|_,job|job.started||job.created.elapsed()<Duration::from_secs(180));if jobs.len()>=128&&!jobs.contains_key(&token){return Err("Too many RAW requests".into());}jobs.entry(token).or_insert_with(||RawJob{cancel:Arc::new(AtomicBool::new(true)),started:false,created:Instant::now()}).cancel.store(true,Ordering::Release);Ok(())}
#[cfg(test)]mod tests {use super::*;
    #[test]fn controls_refuse_nonfinite_and_out_of_range(){for s in [DevelopSettings{noise_reduction:0.,highlight_recovery:false,exposure:f64::NAN,temperature:5000.,tint:0.,tone:1.},DevelopSettings{noise_reduction:0.,highlight_recovery:false,exposure:0.,temperature:0.,tint:0.,tone:1.},DevelopSettings{noise_reduction:0.,highlight_recovery:false,exposure:0.,temperature:5000.,tint:151.,tone:1.}]{assert!(s.validate().is_err());}}
    #[test]fn pre_cancel_is_preserved_and_duplicate_requests_are_refused(){let jobs=RawJobs::default();let token=Uuid::new_v4().to_string();jobs.jobs.lock().unwrap().insert(token.clone(),RawJob{cancel:Arc::new(AtomicBool::new(true)),started:false,created:Instant::now()});let cancel=jobs.register(&token).unwrap();assert!(cancel.load(Ordering::Acquire));assert!(jobs.register(&token).is_err());assert!(run("missing.dng".into(),None,false,cancel,jobs.queue.clone()).unwrap_err().contains("cancelled"));jobs.finish(&token);assert!(jobs.jobs.lock().unwrap().is_empty());}
    #[test]fn output_refuses_huge_or_inconsistent_pixels_before_allocation(){let path=std::env::temp_dir().join(format!("raw-test-{}",Uuid::new_v4()));let _cleanup=TempFiles(vec![path.clone()]);let mut data=b"RAW7".to_vec();data.extend(30001u32.to_le_bytes());data.extend(1u32.to_le_bytes());data.extend([0;4]);fs::write(&path,&data).unwrap();assert!(read_pixels(&path,false).is_err());data[4..8].copy_from_slice(&2u32.to_le_bytes());fs::write(&path,data).unwrap();assert!(read_pixels(&path,false).is_err());}
    #[test]fn full_output_preserves_adjacent_16_bit_values_and_preview_stays_eight_bit(){
        let path=std::env::temp_dir().join(format!("raw-deep-test-{}",Uuid::new_v4()));let _cleanup=TempFiles(vec![path.clone()]);
        let values=[20000u16,30000,40000,65535,20001,30001,40001,65535];
        let mut data=b"RAW8".to_vec();data.extend(2u32.to_le_bytes());data.extend(1u32.to_le_bytes());data.extend(16u32.to_le_bytes());for v in values{data.extend(v.to_le_bytes());}fs::write(&path,&data).unwrap();
        let full=compositor_engine::decode_image(&read_pixels(&path,false).unwrap()).unwrap().raster;assert_eq!(full.depth(),compositor_engine::PixelDepth::Sixteen);assert_eq!(full.deep_pixels().unwrap().straight_u16(),values);
        let preview=compositor_engine::decode_image(&read_pixels(&path,true).unwrap()).unwrap().raster;assert_eq!(preview.depth(),compositor_engine::PixelDepth::Eight);
        data.truncate(16);data[..4].copy_from_slice(b"RAW7");data[12..16].fill(0);data.extend([255;8]);fs::write(&path,data).unwrap();assert!(read_pixels(&path,false).is_err());assert!(read_pixels(&path,true).is_ok());
    }

}
