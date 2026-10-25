//! Compositor 1.5.7 ShapeTool.swift: 60% cubic/arc/cubic corners and explicit dash geometry.
use crate::{Contour, Point, Rect};
use crate::selection::geometry::{cubic, polygon};
use super::shape::{ShapeLineCap, ShapeLineStyle};

pub(super) fn smoothed_rectangle(r: Rect, radius: f64) -> Contour {
    let smoothing = 0.6f64.min((r.width.min(r.height) / (2.0 * radius) - 1.0).max(0.0));
    if smoothing == 0.0 { return super::shape::rounded_rectangle(r, radius); }
    let reach = (1.0 + smoothing) * radius;
    let angle = smoothing * std::f64::consts::FRAC_PI_4;
    let tangent = radius * (angle / 2.0).tan();
    let (c, d) = (tangent * angle.cos(), tangent * angle.sin());
    let arc_length = radius * 2.0f64.sqrt() * ((1.0 - smoothing) * std::f64::consts::FRAC_PI_4).sin();
    let b = (reach - arc_length - c - d) / 3.0;
    let a = 2.0 * b;
    let mut points = Vec::new();
    for (i, (cx, cy)) in [(r.max_x(), r.y), (r.max_x(), r.max_y()), (r.x, r.max_y()), (r.x, r.y)].into_iter().enumerate() {
        // Exact quarter-turns keep axis-aligned edges on their original pixel grid.
        let p = |x:f64,y:f64| { let (x,y)=match i {0=>(x,y),1=>(-y,x),2=>(-x,-y),_=>(y,-x)}; Point{x:cx+x,y:cy+y} };
        points.push(p(-reach, 0.0));
        let end = p(-reach+a+b+c, d);
        cubic(&mut points, [p(-reach,0.0),p(-reach+a,0.0),p(-reach+a+b,0.0),end]);
        let start_angle = -std::f64::consts::FRAC_PI_2 + angle;
        let end_angle = -angle;
        let arc = |t:f64| p(-radius+radius*t.cos(), radius+radius*t.sin());
        let k = 4.0/3.0*((end_angle-start_angle)/4.0).tan();
        let cp1=p(-radius+radius*(start_angle.cos()-k*start_angle.sin()),radius+radius*(start_angle.sin()+k*start_angle.cos()));
        let cp2=p(-radius+radius*(end_angle.cos()+k*end_angle.sin()),radius+radius*(end_angle.sin()-k*end_angle.cos()));
        cubic(&mut points,[arc(start_angle),cp1,cp2,arc(end_angle)]);
        cubic(&mut points,[arc(end_angle),p(0.0,reach-a-b),p(0.0,reach-a),p(0.0,reach)]);
    }
    polygon(&points)
}

pub(super) fn line_reach(a:Point,b:Point,width:f64,cap:ShapeLineCap)->f64 {
    let length=(b.x-a.x).hypot(b.y-a.y);
    if cap==ShapeLineCap::Round || length==0.0 { return width/2.0; }
    let corner=width/2.0*((b.x-a.x).abs()+(b.y-a.y).abs())/length;
    width/2.0+(corner-width/2.0-0.0001).ceil()
}

pub(super) fn line_contours(a:Point,b:Point,width:f64,style:ShapeLineStyle,cap:ShapeLineCap)->Vec<Contour> {
    let length=(b.x-a.x).hypot(b.y-a.y);
    let (ux,uy)=if length>0.0 {((b.x-a.x)/length,(b.y-a.y)/length)}else{(1.0,0.0)};
    let along=|d:f64| Point{x:a.x+ux*(d-width/2.0),y:a.y+uy*(d-width/2.0)};
    let total=length+width;
    let (dash,gap)=match style {ShapeLineStyle::Solid=>(total,0.0),ShapeLineStyle::Dotted=>(width,width),ShapeLineStyle::Dashed=>(3.0*width,2.0*width),ShapeLineStyle::LongDash=>(7.0*width,2.0*width)};
    let mut out=Vec::new();let mut distance=0.0;
    while distance<total {
        let next=(distance+dash).min(total);
        if cap==ShapeLineCap::Square {
            let (p,q)=(along(distance),along(next));let(nx,ny)=(-uy*width/2.0,ux*width/2.0);
            out.push(polygon(&[Point{x:p.x+nx,y:p.y+ny},Point{x:q.x+nx,y:q.y+ny},Point{x:q.x-nx,y:q.y-ny},Point{x:p.x-nx,y:p.y-ny}]));
        } else if next-distance-width>0.001 {
            out.push(super::shape::capsule(along(distance+width/2.0),along(next-width/2.0),width));
        } else {
            let p=along((distance+next)/2.0);
            let radius=width/2.0;
            // 0.001 px sagitta: a subpixel dot needs finer tessellation than a large ellipse.
            let count=((std::f64::consts::PI/(1.0-0.001/radius).clamp(-1.0,1.0).acos()).ceil()as usize).max(32);
            let points:Vec<_>=(0..count).map(|i|{let a=i as f64/count as f64*std::f64::consts::TAU;Point{x:p.x+radius*a.cos(),y:p.y+radius*a.sin()}}).collect();
            out.push(polygon(&points));
        }
        distance=next+gap;
    }
    out
}
