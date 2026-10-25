//! Opt-in tests of the real Windows codecs and ICC engine, using local acceptance fixtures.
use compositor_engine::{decode_image,open_package,Package,PixelDepth};
use serde_json::{json,Value};
use std::{fs,path::Path};
fn split(bytes:&[u8],magic:&[u8])->(Value,Vec<(String,Vec<u8>)>){
    assert_eq!(&bytes[..4],magic);let n=u32::from_le_bytes(bytes[4..8].try_into().unwrap())as usize;
    let header:Value=serde_json::from_slice(&bytes[8..8+n]).unwrap();let mut at=8+n;let mut images=Vec::new();
    for image in header["images"].as_array().unwrap(){let len=image["length"].as_u64().unwrap()as usize;images.push((image["name"].as_str().unwrap().into(),bytes[at..at+len].to_vec()));at+=len;}
    assert_eq!(at,bytes.len());(header,images)
}
fn path(folder:&Path,name:&str)->String{folder.join(name).to_string_lossy().into_owned()}
#[test]
#[ignore = "Requires Windows system codecs and explicit COMPOSITOR_NATIVE157_FIXTURES/OUTPUT"]
fn windows157_real_codecs_and_icc(){
    let fixtures=std::path::PathBuf::from(std::env::var_os("COMPOSITOR_NATIVE157_FIXTURES").expect("explicit fixtures"));
    let output=std::path::PathBuf::from(std::env::var_os("COMPOSITOR_NATIVE157_OUTPUT").expect("explicit output"));
    fs::create_dir(&output).expect("fresh output directory");let mut rows=Vec::new();
    let fonts=crate::commands::fonts::list_fonts().unwrap();assert!(fonts.len()>5);assert!(fonts.iter().all(|f|!f.is_empty()));rows.push(json!({"case":"Installed Windows font families","count":fonts.len()}));
    for fitting in [None,Some([64,64])]{
        let wire=crate::native_images::decode(&path(&fixtures,"two-pages.pdf"),fitting,100_000_000).unwrap();
        let(h,pages)=split(&wire,b"NIM1");assert_eq!(pages.len(),2);assert_eq!(h["resolution"],300.);
        for(i,(_,png))in pages.iter().enumerate(){let r=decode_image(png).unwrap().raster;
            let size=if fitting.is_some(){[[64,43],[32,64]][i]}else{[[300,200],[150,300]][i]};assert_eq!([r.width,r.height],size);
            assert_eq!(r.pixel(r.width/2,r.height/2),if i==0{[255,0,0,255]}else{[0,0,255,255]});
            fs::write(output.join(format!("pdf-{}-{i}.png",if fitting.is_some(){"fit"}else{"print"})),png).unwrap();
        }
        rows.push(json!({"case":"PDF real system rendering","fitting":fitting,"pages":2}));
    }
    // Exercise both horizontal and vertical tile boundaries in the fixed-pixel renderer.
    let wire=crate::native_images::decode(&path(&fixtures,"two-pages.pdf"),Some([2300,2300]),100_000_000).unwrap();
    let(_,pages)=split(&wire,b"NIM1");
    for(i,(_,png))in pages.iter().enumerate(){
        let r=decode_image(png).unwrap().raster;
        assert_eq!([r.width,r.height],[[2300,1533],[1150,2300]][i]);
        let color=if i==0{[255,0,0,255]}else{[0,0,255,255]};
        for n in [0,1024,2047,2048,2299]{assert_eq!(if i==0{r.pixel(n,700)}else{r.pixel(500,n)},color);}
        fs::write(output.join(format!("pdf-tiled-{i}.png")),png).unwrap();
    }
    rows.push(json!({"case":"PDF horizontal and vertical 2048-pixel tile boundaries","pages":2}));
    assert!(crate::native_images::decode(&path(&fixtures,"two-pages.pdf"),None,10).is_err());
    let wire=crate::native_images::decode(&path(&fixtures,"rgb.heic"),None,100_000_000).unwrap();let(_,pages)=split(&wire,b"NIM1");assert_eq!(pages.len(),1);
    let heif=decode_image(&pages[0].1).unwrap().raster;assert_eq!([heif.width,heif.height],[48,32]);assert_eq!(heif.depth(),PixelDepth::Sixteen);
    // The fixture is lossy HEIF. Check distinct colors and opacity, not original bytes.
    assert!(heif.pixel(3,16)[0]>180&&heif.pixel(3,16)[1]<80);assert!(heif.pixel(19,16)[2]>150);assert_eq!(heif.pixel(45,16)[3],255);
    fs::write(output.join("heif.png"),&pages[0].1).unwrap();rows.push(json!({"case":"HEIF bundled 16-bit output","width":48,"height":32}));
    let wire=crate::native_images::decode(&path(&fixtures,"gradient10.heic"),None,100_000_000).unwrap();
    let(_,pages)=split(&wire,b"NIM1");let deep=decode_image(&pages[0].1).unwrap().raster;
    assert_eq!([deep.width,deep.height],[512,64]);assert_eq!(deep.depth(),PixelDepth::Sixteen);
    assert!(deep.deep_pixels().unwrap().straight_u16().chunks_exact(4).any(|p|p[..3].iter().any(|v|v%257!=0)),"HEIF precision was reduced to eight bits");
    fs::write(output.join("heif10.png"),&pages[0].1).unwrap();rows.push(json!({"case":"Bundled HEVC 10-bit detail retained"}));
    let wire=crate::native_images::decode(&path(&fixtures,"icc-adobe.heic"),None,100_000_000).unwrap();let(_,pages)=split(&wire,b"NIM1");
    let converted=decode_image(&pages[0].1).unwrap().raster;assert_eq!(converted.depth(),PixelDepth::Sixteen);
    let reference=decode_image(&fs::read(fixtures.join("icc-adobe-reference.png")).unwrap()).unwrap().raster;
    let mut max=0u8;let mut sum=0u64;let mut count=0u64;
    for x in [3,11,19,27,35,43]{for y in 8..24{for c in 0..3{let d=converted.pixel(x,y)[c].abs_diff(reference.pixel(x,y)[c]);max=max.max(d);sum+=d as u64;count+=1;}}}
    let mean=sum as f64/count as f64;
    fs::write(output.join("heif-icc.png"),&pages[0].1).unwrap();
    fs::write(output.join("heif-icc-comparison.json"),serde_json::to_vec_pretty(&json!({"maxDifference":max,"meanDifference":mean,"maxAllowed":12,"meanAllowed":3})).unwrap()).unwrap();
    assert!(max<=12&&mean<=3.,"HEIF ICC vs Apple reference: max {max}, mean {mean}");
    rows.push(json!({"case":"HEIF embedded Adobe RGB to sRGB at 16 bits","maxDifference":max,"meanDifference":mean}));
    let profile=path(&fixtures,"test-print.icc");let profiles=output.join("profiles");
    let proof=crate::color_profiles::proof(&profile,&profiles,true).unwrap();assert_eq!(proof.lut.len(),33*33*33*3);assert!(proof.lut.iter().any(|v|*v>200));assert!(proof.lut.iter().any(|v|*v<40));
    fs::write(output.join("proof-lut.rgb"),&proof.lut).unwrap();assert!(crate::color_profiles::list(&profiles).iter().any(|p|p.path==proof.profile.path));
    let rgb=fs::read(fixtures.join("rgb.png")).unwrap();
    for(format,ext)in[("JPEG","jpg"),("TIFF","tif"),("PDF","pdf")]{let bytes=crate::color_profiles::export(&rgb,&profile,format,1.,300.).unwrap();fs::write(output.join(format!("export.{ext}")),bytes).unwrap();}
    let reference=decode_image(&fs::read(fixtures.join("cmyk-reference.png")).unwrap()).unwrap().raster;
    for name in ["cmyk.tif","cmyk.jpg","cmyk.psd","unprofiled.tif"]{
        let fallback=if name=="unprofiled.tif"{profile.as_str()}else{"missing-old-profile.icc"};
        let wire=crate::color_profiles::import_file(&path(&fixtures,name),Some(fallback),&profiles).unwrap_or_else(|e|panic!("CMYK import {name}: {e}"));let(h,images)=split(&wire,b"NCP1");
        let document=open_package(&Package{manifest_json:h["manifest"].as_str().unwrap().into(),images}).unwrap();assert_eq!([document.width,document.height],[48,32]);
        let raster=document.layers[0].pixels.as_ref().unwrap();let mut max=0u8;let mut sum=0u64;let mut count=0u64;
        // Sample the interiors of each color stripe so JPEG ringing is excluded.
        for x in [3,11,19,27,35,43]{for y in 8..24{for c in 0..3{let d=raster.pixel(x,y)[c].abs_diff(reference.pixel(x,y)[c]);max=max.max(d);sum+=d as u64;count+=1;}}}
        let mean=sum as f64/count as f64;
    fs::write(output.join(format!("{name}.png")),compositor_engine::encode_png(raster,300.).unwrap()).unwrap();
    fs::write(output.join(format!("{name}-comparison.json")),serde_json::to_vec_pretty(&json!({"maxDifference":max,"meanDifference":mean,"maxAllowed":12,"meanAllowed":3})).unwrap()).unwrap();
    assert!(max<=12&&mean<=3.,"{name} WCS/WIC vs independent LittleCMS: max {max}, mean {mean}");
        rows.push(json!({"case":"CMYK native import","file":name,"maxDifference":max,"meanDifference":mean}));
    }
    assert!(crate::color_profiles::import_file(&path(&fixtures,"rgb.jpg"),Some("missing-old-profile.icc"),&profiles).unwrap().is_empty());
    assert!(crate::color_profiles::import_file(&path(&fixtures,"unprofiled.tif"),None,&profiles).is_err());
    rows.push(json!({"case":"ICC proof/export, RGB stale preference and missing-profile rejection"}));
    fs::write(output.join("result.json"),serde_json::to_vec_pretty(&json!({"passed":true,"rows":rows,"runtime":"Actual Windows WIC/WCS/WinRT PDF and bundled libheif/libde265 HEVC"})).unwrap()).unwrap();
}
