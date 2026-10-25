//! PDF pixels must not inherit WinRT RenderToStreamAsync's host/display DIP scaling.
//! Render into bounded, explicitly sized Direct2D bitmaps on a software WARP device.
use windows::{
    core::{IUnknown, Interface},
    Data::Pdf::PdfPage,
    Win32::{
        Foundation::HMODULE,
        Graphics::{
            Direct2D::{Common::*, *},
            Direct3D::D3D_DRIVER_TYPE_WARP,
            Direct3D11::*,
            Dxgi::{Common::DXGI_FORMAT_B8G8R8A8_UNORM, IDXGIAdapter, IDXGIDevice},
        },
        System::WinRT::Pdf::{IPdfRendererNative, PdfCreateRenderer, PDF_RENDER_PARAMS},
    },
};

const TILE: u32 = 2048;
fn error(e: windows::core::Error) -> String {
    format!("Windows PDF pixel renderer: {e}")
}

pub struct Renderer {
    context: ID2D1DeviceContext,
    pdf: IPdfRendererNative,
}
struct Mapping<'a>(&'a ID2D1Bitmap1);
impl Drop for Mapping<'_> {
    fn drop(&mut self) {
        unsafe {
            let _ = self.0.Unmap();
        }
    }
}

impl Renderer {
    pub fn new() -> Result<Self, String> {
        unsafe {
            let mut device = None;
            D3D11CreateDevice(
                None::<&IDXGIAdapter>,
                D3D_DRIVER_TYPE_WARP,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                None,
            )
            .map_err(error)?;
            let device = device.ok_or("Windows did not return a PDF rendering device")?;
            let dxgi: IDXGIDevice = device.cast().map_err(error)?;
            let d2d = D2D1CreateDevice(&dxgi, None).map_err(error)?;
            let context = d2d
                .CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)
                .map_err(error)?;
            context.SetDpi(96., 96.);
            context.SetUnitMode(D2D1_UNIT_MODE_PIXELS);
            Ok(Self {
                context,
                pdf: PdfCreateRenderer(&dxgi).map_err(error)?,
            })
        }
    }

    pub fn render(&self, page: &PdfPage, width: u32, height: u32) -> Result<Vec<u8>, String> {
        if width == 0
            || height == 0
            || width > 30000
            || height > 30000
            || u64::from(width) * u64::from(height) > super::super::MAX_PIXELS
        {
            return Err("PDF raster exceeds the document pixel budget".into());
        }
        let size = page.Size().map_err(error)?;
        let count = width as usize * height as usize * 4;
        let mut rgba = Vec::new();
        rgba.try_reserve_exact(count)
            .map_err(|_| "Not enough memory for the PDF page")?;
        rgba.resize(count, 0);
        let unknown: IUnknown = page.cast().map_err(error)?;
        // A tiled target also supports 30,000-pixel pages beyond a D3D texture's side limit.
        for y in (0..height).step_by(TILE as usize) {
            for x in (0..width).step_by(TILE as usize) {
                let w = TILE.min(width - x);
                let h = TILE.min(height - y);
                unsafe {
                    let properties = D2D1_BITMAP_PROPERTIES1 {
                        pixelFormat: D2D1_PIXEL_FORMAT {
                            format: DXGI_FORMAT_B8G8R8A8_UNORM,
                            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
                        },
                        dpiX: 96.,
                        dpiY: 96.,
                        bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET,
                        ..Default::default()
                    };
                    let target = self
                        .context
                        .CreateBitmap(
                            D2D_SIZE_U {
                                width: w,
                                height: h,
                            },
                            None,
                            0,
                            &properties,
                        )
                        .map_err(error)?;
                    let readback = self
                        .context
                        .CreateBitmap(
                            D2D_SIZE_U {
                                width: w,
                                height: h,
                            },
                            None,
                            0,
                            &D2D1_BITMAP_PROPERTIES1 {
                                bitmapOptions: D2D1_BITMAP_OPTIONS_CPU_READ
                                    | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
                                ..properties
                            },
                        )
                        .map_err(error)?;
                    self.context.SetTarget(&target);
                    self.context.BeginDraw();
                    self.context.Clear(Some(&D2D1_COLOR_F {
                        r: 0.,
                        g: 0.,
                        b: 0.,
                        a: 0.,
                    }));
                    let params = PDF_RENDER_PARAMS {
                        SourceRect: D2D_RECT_F {
                            left: x as f32 * size.Width / width as f32,
                            top: y as f32 * size.Height / height as f32,
                            right: (x + w) as f32 * size.Width / width as f32,
                            bottom: (y + h) as f32 * size.Height / height as f32,
                        },
                        DestinationWidth: w,
                        DestinationHeight: h,
                        BackgroundColor: D2D_COLOR_F {
                            r: 0.,
                            g: 0.,
                            b: 0.,
                            a: 0.,
                        },
                        IgnoreHighContrast: true,
                    };
                    let drawn =
                        self.pdf
                            .RenderPageToDeviceContext(&unknown, &self.context, Some(&params));
                    let ended = self.context.EndDraw(None, None);
                    self.context.SetTarget(None::<&ID2D1Image>);
                    drawn.map_err(error)?;
                    ended.map_err(error)?;
                    readback
                        .CopyFromBitmap(None, &target, None)
                        .map_err(error)?;
                    let mapped = readback.Map(D2D1_MAP_OPTIONS_READ).map_err(error)?;
                    let _mapping = Mapping(&readback);
                    if mapped.bits.is_null() || mapped.pitch < w * 4 {
                        return Err("Invalid PDF readback buffer".into());
                    }
                    for row in 0..h as usize {
                        let source = std::slice::from_raw_parts(
                            mapped.bits.add(row * mapped.pitch as usize),
                            w as usize * 4,
                        );
                        let start = ((y as usize + row) * width as usize + x as usize) * 4;
                        for (from, to) in source
                            .chunks_exact(4)
                            .zip(rgba[start..start + w as usize * 4].chunks_exact_mut(4))
                        {
                            to.copy_from_slice(&[from[2], from[1], from[0], from[3]]);
                        }
                    }
                }
            }
        }
        compositor_engine::encode_png(
            &compositor_engine::Raster::from_premultiplied(width, height, rgba),
            300.,
        )
        .map_err(|e| e.to_string())
    }
}
