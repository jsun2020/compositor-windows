//! Complete finite Scanlines lineSpacing 2..=32 parameter domain.
//! Direct observations from Mac149-glow-composite-20261015-075932:
//! all 31 public encode configurations and corresponding Float32 arguments.
//! These are input-independent algorithm parameters, not fitted values or
//! image answers. They must not be used as a general sigma/other-effects API.
//! Catalog SHA256: 4e24efb18e1409bd18e9578e904c47d8c32ded575f647b46fe41269d8aef70c0
use super::{Error, Kernel};

pub(crate) struct SpacingKernel {
    pub kernel: Kernel,
    pub shrink: usize,
    pub margin: i32,
    pub reduced_sigma: f32,
}

struct KernelBits {
    down: [u32; 3],
    fir: &'static [u32],
    up: u32,
}
const KERNELS: [KernelBits; 10] = [
    KernelBits {
        down: [0x3e88ac30, 0x3e3677fd, 0x3d60be8e],
        fir: &[
            0x3e444101, 0x3e2bbfdc, 0x3dfaf04d, 0x3d887d2d, 0x3d071676, 0x3c1b7971, 0x3b9d762a,
        ],
        up: 0x3f3e42d6,
    },
    KernelBits {
        down: [0x3e8ea015, 0x3e3091ce, 0x3d48b824],
        fir: &[
            0x3e5d3132, 0x3e4108f8, 0x3df2f66a, 0x3d716845, 0x3c987812, 0x3bef44c2,
        ],
        up: 0x3f3dbbd6,
    },
    KernelBits {
        down: [0x3e79a740, 0x3e42035c, 0x3d88aac9],
        fir: &[
            0x3e342bc8, 0x3e17c287, 0x3dffae80, 0x3d8ca3b8, 0x3d3fd5e5, 0x3c43f098, 0x3c3c9f5d,
        ],
        up: 0x3f3e33c0,
    },
    KernelBits {
        down: [0x3e8056f1, 0x3e3e4123, 0x3d8221f4],
        fir: &[
            0x3e4da549, 0x3e3f2b45, 0x3deb8724, 0x3d89df63, 0x3c9f26ea, 0x3c369f5f,
        ],
        up: 0x3f3defb0,
    },
    KernelBits {
        down: [0x3e7ca4a8, 0x3e4039ad, 0x3d864356],
        fir: &[
            0x3e467d0c, 0x3e3d320b, 0x3de8b4e0, 0x3d91b480, 0x3ca5859a, 0x3c5aa0b8,
        ],
        up: 0x3f3de81c,
    },
    KernelBits {
        down: [0x3e848388, 0x3e3a3bbe, 0x3d72f4ca],
        fir: &[
            0x3e55ae47, 0x3e404f10, 0x3def4f03, 0x3d812018, 0x3c9aa294, 0x3c14dec3,
        ],
        up: 0x3f3dcdb3,
    },
    KernelBits {
        down: [0x3e97dc28, 0x3e27d8e2, 0x3d21bb3b],
        fir: &[
            0x3e48c822, 0x3e31a479, 0x3df83093, 0x3d86e2bb, 0x3cedbf3d, 0x3c13c330, 0x3b5e6d37,
        ],
        up: 0x3f3e32a2,
    },
    KernelBits {
        down: [0x3e7e5b39, 0x3e3f624c, 0x3d8484f5],
        fir: &[
            0x3e49eb13, 0x3e3e4d22, 0x3de9f585, 0x3d8df071, 0x3ca23f6b, 0x3c4826ce,
        ],
        up: 0x3f3dee91,
    },
    KernelBits {
        down: [0x3e81c9e2, 0x3e3cdc8c, 0x3d7e3ec0],
        fir: &[
            0x3e518d67, 0x3e3fee83, 0x3ded211e, 0x3d85b8a1, 0x3c9c1219, 0x3c25ba6a,
        ],
        up: 0x3f3de2a9,
    },
    KernelBits {
        down: [0x3e89dc9f, 0x3e351fa6, 0x3d5c9c6e],
        fir: &[
            0x3e59b0e6, 0x3e4059a9, 0x3df1a5f6, 0x3d7949f3, 0x3c9a8483, 0x3c057dbe,
        ],
        up: 0x3f3dc3b0,
    },
];
// kernel, external shrink, halo on each side, exact public constructor sigma.
const STATES: [(usize, usize, i32, u32); 31] = [
    (0, 2, 14, 0x40900000),  // spacing 2
    (1, 3, 12, 0x40800000),  // spacing 3
    (2, 3, 15, 0x40a00000),  // spacing 4
    (0, 4, 14, 0x40900000),  // spacing 5
    (3, 5, 13, 0x40866666),  // spacing 6
    (1, 6, 12, 0x40800000),  // spacing 7
    (0, 6, 14, 0x40900000),  // spacing 8
    (4, 7, 13, 0x40892492),  // spacing 9
    (5, 8, 13, 0x40840000),  // spacing 10
    (1, 9, 12, 0x40800000),  // spacing 11
    (6, 9, 13, 0x408aaaab),  // spacing 12
    (3, 10, 13, 0x40866666), // spacing 13
    (5, 11, 13, 0x4082e8ba), // spacing 14
    (1, 12, 12, 0x40800000), // spacing 15
    (7, 12, 13, 0x40880000), // spacing 16
    (8, 13, 13, 0x4084ec4f), // spacing 17
    (9, 14, 13, 0x40824925), // spacing 18
    (1, 15, 12, 0x40800000), // spacing 19
    (3, 15, 13, 0x40866666), // spacing 20
    (5, 16, 13, 0x40840000), // spacing 21
    (9, 17, 13, 0x4081e1e2), // spacing 22
    (1, 18, 12, 0x40800000), // spacing 23
    (8, 18, 13, 0x40855555), // spacing 24
    (5, 19, 13, 0x40835e51), // spacing 25
    (9, 20, 13, 0x4081999a), // spacing 26
    (1, 21, 12, 0x40800000), // spacing 27
    (8, 21, 13, 0x40849249), // spacing 28
    (5, 22, 13, 0x4082e8ba), // spacing 29
    (9, 23, 13, 0x4081642d), // spacing 30
    (1, 24, 12, 0x40800000), // spacing 31
    (5, 24, 13, 0x40840000), // spacing 32
];

pub(crate) fn for_spacing(spacing: u8) -> Result<SpacingKernel, Error> {
    let index = spacing
        .checked_sub(2)
        .filter(|&v| v <= 30)
        .ok_or("Invalid normalized Scanlines spacing")?;
    let (id, shrink, margin, sigma) = STATES[usize::from(index)];
    let k = &KERNELS[id];
    Ok(SpacingKernel {
        kernel: Kernel {
            downsample: k.down.map(f32::from_bits),
            fir: k.fir.iter().map(|&v| f32::from_bits(v)).collect(),
            upsample: f32::from_bits(k.up),
        },
        shrink,
        margin,
        reduced_sigma: f32::from_bits(sigma),
    })
}
