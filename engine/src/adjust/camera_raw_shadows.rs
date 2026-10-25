//! Region-adaptive Shadows, Compositor v1.5.7 CameraRawShadows.swift (MIT).
//! The immutable CRS2 resource is checked at build time; interpolation follows Swift Float order.
use std::sync::OnceLock;

struct Tables {
    normal_from: f32, amounts: Vec<f32>, coefficients: Vec<f32>,
    x_count: usize, k_count: usize, x_low: f32, x_step: f32, k_low: f32, k_step: f32,
}
fn tables() -> &'static Tables {
    static TABLES: OnceLock<Tables> = OnceLock::new();
    TABLES.get_or_init(|| {
        let bytes = include_bytes!("../../native/CameraRawShadows.bin");
        let u16_at = |i| u16::from_le_bytes([bytes[i], bytes[i+1]]) as usize;
        let f32_at = |i| f32::from_le_bytes(bytes[i..i+4].try_into().unwrap());
        let count = u16_at(6);
        Tables { normal_from: f32_at(14), x_count: u16_at(8), k_count: u16_at(10),
            x_low: f32_at(18), x_step: f32_at(22), k_low: f32_at(26), k_step: f32_at(30),
            amounts: (0..count).map(|i| f32_at(34 + i*4)).collect(),
            coefficients: (34 + count*4..bytes.len()).step_by(4).map(f32_at).collect() }
    })
}
extern "C" {
    fn adjust_camera_raw_shadows(pixels: *mut u8, width: usize, height: usize, stride: usize,
        coefficients: *const f32, x_count: i32, x_low: f32, x_step: f32,
        k_count: i32, k_low: f32, k_step: f32);
}
pub(super) fn apply(amount: f64, data: &mut [u8], width: usize, height: usize, scale: f64) {
    if amount == 0.0 { return; }
    assert_eq!(data.len(), width * height * 4);
    let t = tables();
    let set = usize::from(width.max(height) as f64 / scale >= t.normal_from as f64);
    let per = t.x_count * t.k_count * 3;
    let mut points: Vec<(f64, Option<usize>)> = t.amounts.iter().enumerate().map(|(i, a)| (*a as f64, Some(i))).collect();
    points.push((0.0, None));
    points.sort_by(|a,b| a.0.total_cmp(&b.0));
    let value = amount.clamp(points[0].0, points.last().unwrap().0);
    let mut index = 0;
    while index < points.len()-2 && value > points[index+1].0 { index += 1; }
    let (low, lower) = points[index];
    let (high, upper) = points[index+1];
    let blend = ((value-low)/(high-low)) as f32;
    let at = |index: Option<usize>, i| index.map_or(0.0, |a| t.coefficients[(set*t.amounts.len()+a)*per+i]);
    let coefficients: Vec<f32> = (0..per).map(|i| at(lower,i)*(1.0-blend)+at(upper,i)*blend).collect();
    unsafe { adjust_camera_raw_shadows(data.as_mut_ptr(), width, height, width*4, coefficients.as_ptr(),
        t.x_count as i32, t.x_low, t.x_step, t.k_count as i32, t.k_low, t.k_step); }
}
