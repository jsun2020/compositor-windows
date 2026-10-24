use compositor_engine::{PixelRect,Raster};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};

struct Measured;
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
fn add(n: usize) { let now=LIVE.fetch_add(n,SeqCst)+n;PEAK.fetch_max(now,SeqCst); }
unsafe impl GlobalAlloc for Measured {
    unsafe fn alloc(&self,l:Layout)->*mut u8 {let p=System.alloc(l);if !p.is_null(){add(l.size());}p}
    unsafe fn alloc_zeroed(&self,l:Layout)->*mut u8 {let p=System.alloc_zeroed(l);if !p.is_null(){add(l.size());}p}
    unsafe fn dealloc(&self,p:*mut u8,l:Layout){System.dealloc(p,l);LIVE.fetch_sub(l.size(),SeqCst);}
    unsafe fn realloc(&self,p:*mut u8,l:Layout,n:usize)->*mut u8 {let q=System.realloc(p,l,n);if !q.is_null(){if n>=l.size(){add(n-l.size());}else{LIVE.fetch_sub(l.size()-n,SeqCst);}}q}
}
#[global_allocator]
static ALLOCATOR: Measured = Measured;

fn pattern(w:u32,h:u32)->Vec<u8>{
    let mut bytes=Vec::with_capacity(w as usize*h as usize*4);
    let mut seed=0x9e3779b9u32;
    for _ in 0..w*h {seed^=seed<<13;seed^=seed>>17;seed^=seed<<5;let a=(seed>>24)as u8;bytes.extend_from_slice(&[(seed as u8).min(a),((seed>>8)as u8).min(a),((seed>>16)as u8).min(a),a]);}
    bytes
}

// Independent complete-image reference with scalar channel sums. Every stage
// rounds before the next, rather than averaging a whole final block at once.
fn reference(mut bytes:Vec<u8>,mut w:u32,mut h:u32,level:u32)->Vec<u8>{
    for _ in 0..level {
        if w<=1||h<=1 {break;}
        let(nw,nh)=(w/2,h/2);let mut out=Vec::with_capacity(nw as usize*nh as usize*4);
        for y in 0..nh {for x in 0..nw {for c in 0..4 {
            let at=(((2*y)*w+2*x)*4+c)as usize;
            let sum=bytes[at]as u32+bytes[at+4]as u32+bytes[at+w as usize*4]as u32+bytes[at+w as usize*4+4]as u32;
            out.push(((sum+2)/4)as u8);
        }}}
        bytes=out;w=nw;h=nh;
    }
    bytes
}

#[test]
fn large_even_and_odd_reductions_are_exact_with_bounded_live_allocation(){
    for(w,h)in[(4096,4096),(4097,4099)]{
        let original=pattern(w,h);
        let raster=Raster::from_premultiplied(w,h,original.clone());
        let before=LIVE.load(SeqCst);PEAK.store(before,SeqCst);
        let shown=raster.reduced(3);
        let extra=PEAK.load(SeqCst)-before;
        assert!(extra<4*1024*1024,"{w}x{h}: temporary/output live allocation {extra} exceeds band budget");
        assert!(raster.memoized_half().is_none(),"no whole intermediate ladder was allocated");
        assert!(raster.reduced(3).same_pixels(&shown),"next frame reuses final buffer");
        assert_eq!(shown.bytes(),reference(original.clone(),w,h,3));
        assert_eq!(raster.bytes(),original,"stored pixels untouched");
        // The separate preview entrypoint must avoid the full intermediate
        // ladder even when the display has already cached a deeper level.
        let before_preview=LIVE.load(SeqCst);PEAK.store(before_preview,SeqCst);
        let preview=raster.reduced_pixels(2);
        let preview_extra=PEAK.load(SeqCst)-before_preview;
        assert!(preview_extra<5*1024*1024,"preview extra allocation {preview_extra}");
        assert!(raster.memoized_half().is_none());
        assert!(raster.reduced_pixels(2).same_pixels(&preview),"slider source cache reused");
        assert_eq!(preview.bytes(),reference(original.clone(),w,h,2));
        assert!(raster.reduced(3).same_pixels(&shown),"preview keeps display cache intact");
        println!("STREAMED_PREVIEW_ALLOCATION {{\"width\":{w},\"height\":{h},\"outputBytes\":{},\"peakAdditionalRequestedBytes\":{preview_extra}}}",preview.bytes().len());
        // A requested larger zoom still agrees with full scalar reduction.
        assert_eq!(raster.reduced(2).bytes(),reference(original.clone(),w,h,2));
        let mut modified=original.clone();
        for y in 0..8 {for x in 0..8 {let at=((y*w+x)*4)as usize;modified[at..at+4].copy_from_slice(&[9,19,29,97]);}}
        let changed=Raster::from_premultiplied(w,h,modified.clone());
        changed.seed_halvings(&raster,PixelRect{x:0,y:0,width:8,height:8});
        assert_eq!(changed.reduced(3).bytes(),reference(modified,w,h,3),"partial update of band-built cache is exact");
        assert_eq!(raster.bytes(),original,"partial edit does not change old image");
        // A separately supplied level keeps its pre-existing precedence, even
        // if its bytes differ from the source's computed display pyramid.
        let (aw,ah)=raster.size_at_level(2);
        let supplied=Raster::from_premultiplied(aw,ah,[17,31,43,97].repeat((aw*ah)as usize));
        assert!(raster.adopt(2,supplied.clone()));
        assert!(raster.reduced(2).same_pixels(&supplied));
        assert_eq!(raster.reduced(3).bytes(),reference(supplied.bytes().to_vec(),aw,ah,1));
        let separate=Raster::from_premultiplied(w,h,original.clone());
        assert!(separate.adopt(2,supplied.clone()));
        let raw_preview=separate.reduced_pixels(3);
        assert_eq!(raw_preview.bytes(),reference(original.clone(),w,h,3),"preview ignores authoritative display pixels as before");
        assert!(separate.reduced(2).same_pixels(&supplied),"preview did not replace authoritative display");
        assert!(separate.reduced_pixels(3).same_pixels(&raw_preview));
        separate.forget_halvings();
        assert!(!separate.reduced_pixels(3).same_pixels(&raw_preview),"forget drops separate preview cache");
        println!("STREAMED_ALLOCATION {{\"width\":{w},\"height\":{h},\"sourceBytes\":{},\"outputBytes\":{},\"peakAdditionalRequestedBytes\":{extra}}}",original.len(),shown.bytes().len());
    }
}
