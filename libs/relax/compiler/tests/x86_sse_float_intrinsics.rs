#![cfg(target_arch = "x86_64")]
#![allow(unused_unsafe, deprecated, unexpected_cfgs)]
use std::arch::x86_64::*;
use std::hint::black_box;

const FNV: u64 = 0xcbf2_9ce4_8422_2325;

fn fold(h: &mut u64, bytes: &[u8]) {
    for b in bytes {
        *h = (*h ^ *b as u64).wrapping_mul(0x0000_0100_0000_01b3);
    }
}
fn fold_i32(h: &mut u64, v: i32) {
    fold(h, &v.to_le_bytes());
}
fn bits(v: __m128) -> [u32; 4] {
    let mut r = [0u32; 4];
    unsafe { _mm_storeu_ps(r.as_mut_ptr() as *mut f32, v) };
    r
}
fn fold_ps(h: &mut u64, v: __m128) {
    for x in bits(v) {
        fold(h, &x.to_le_bytes());
    }
}
fn ld(x: &[u32; 4]) -> __m128 {
    unsafe { _mm_loadu_ps(x.as_ptr() as *const f32) }
}
fn ldi(x: &[u32; 4]) -> __m128i {
    unsafe { _mm_loadu_si128(x.as_ptr() as *const __m128i) }
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 16) as u32
    }
    // a finite float of moderate magnitude (2^-20 .. 2^20), either sign
    fn sane(&mut self) -> u32 {
        let r = self.next();
        (r & 0x807f_ffff) | ((107 + (r >> 8) % 41) << 23)
    }
}

const NAN: u32 = 0x7fc0_0000;
// one NaN pattern only: the payload an arithmetic instruction propagates then does not depend on
// operand order
const ARITH: [u32; 24] = [
    0x0000_0000, 0x8000_0000, 0x3f80_0000, 0xbf80_0000, 0x3f00_0000, 0xbf00_0000, 0x3fc0_0000, 0x4020_0000,
    0xc020_0000, 0x7f80_0000, 0xff80_0000, NAN, 0x0080_0000, 0x0000_0001, 0x807f_ffff, 0x7f7f_ffff,
    0xff7f_ffff, 0x4f00_0000, 0xcf00_0000, 0x4eff_ffff, 0x3eff_ffff, 0x4b80_0000, 0x406c_cccd, 0xc04c_cccd,
];
const ZOO: [u32; 28] = [
    0x0000_0000, 0x8000_0000, 0x3f80_0000, 0xbf80_0000, 0x3f00_0000, 0xbf00_0000, 0x3fc0_0000, 0x4020_0000,
    0xc020_0000, 0x7f80_0000, 0xff80_0000, NAN, 0xffc0_0000, 0x7fa0_0000, 0xff80_0001, 0x7fc1_2345,
    0x0080_0000, 0x0000_0001, 0x807f_ffff, 0x7f7f_ffff, 0xff7f_ffff, 0x4f00_0000, 0xcf00_0000, 0x4eff_ffff,
    0x3eff_ffff, 0x4b80_0000, 0x406c_cccd, 0xc04c_cccd,
];

fn vectors(edges: &[u32], seed: u64, raw: bool) -> Vec<[u32; 4]> {
    let n = edges.len();
    let mut v = Vec::new();
    for i in 0..n {
        v.push([edges[i], edges[(i + 3) % n], edges[(i * 7 + 1) % n], edges[(i + 11) % n]]);
    }
    let mut r = Rng(seed);
    for _ in 0..12 {
        if raw {
            v.push([r.next(), r.next(), r.next(), r.next()]);
        } else {
            v.push([r.sane(), r.sane(), r.sane(), r.sane()]);
        }
    }
    black_box(v)
}
fn arith() -> Vec<[u32; 4]> {
    vectors(&ARITH, 0x9e37_79b9_7f4a_7c15, false)
}
fn zoo() -> Vec<[u32; 4]> {
    vectors(&ZOO, 0x2545_f491_4f6c_dd1d, true)
}

type Got = Vec<(&'static str, u64)>;

fn check(got: Got, want: &[(&str, u64)]) {
    if cfg!(a1_print) {
        for (name, h) in &got {
            println!("        (\"{name}\", 0x{h:016x}),");
        }
        return;
    }
    assert_eq!(got.len(), want.len());
    for ((name, h), (wname, w)) in got.iter().zip(want) {
        assert_eq!(name, wname);
        assert_eq!(h, w, "{name}");
    }
}

macro_rules! bin_ps {
    ($v:ident, $out:ident, $($f:ident),*) => {$(
        let mut h = FNV;
        for a in $v.iter() {
            for b in $v.iter() {
                fold_ps(&mut h, $f(ld(a), ld(b)));
            }
        }
        $out.push((stringify!($f), h));
    )*};
}
macro_rules! bin_i32 {
    ($v:ident, $out:ident, $($f:ident),*) => {$(
        let mut h = FNV;
        for a in $v.iter() {
            for b in $v.iter() {
                fold_i32(&mut h, $f(ld(a), ld(b)));
            }
        }
        $out.push((stringify!($f), h));
    )*};
}
macro_rules! un_ps {
    ($v:ident, $out:ident, $($f:ident),*) => {$(
        let mut h = FNV;
        for a in $v.iter() {
            fold_ps(&mut h, $f(ld(a)));
        }
        $out.push((stringify!($f), h));
    )*};
}
macro_rules! tern_ps {
    ($v:ident, $out:ident, $($f:ident),*) => {$(
        let mut h = FNV;
        for a in $v.iter().step_by(3) {
            for b in $v.iter().skip(1).step_by(3) {
                for c in $v.iter().skip(2).step_by(3) {
                    fold_ps(&mut h, $f(ld(a), ld(b), ld(c)));
                }
            }
        }
        $out.push((stringify!($f), h));
    )*};
}
// one checksum over every listed immediate
macro_rules! bin_ps_imm {
    ($v:ident, $out:ident, $f:ident, $($imm:literal),*) => {
        let mut h = FNV;
        for a in $v.iter() {
            for b in $v.iter() {
                $( fold_ps(&mut h, $f::<$imm>(ld(a), ld(b))); )*
            }
        }
        $out.push((stringify!($f), h));
    };
}

#[target_feature(enable = "sse3")]
unsafe fn run_arith_ss() -> Got {
    let mut out = Got::new();
    let v = arith();
    bin_ps!(v, out, _mm_sub_ss, _mm_mul_ss, _mm_div_ss);
    un_ps!(v, out, _mm_sqrt_ss);
    let v = zoo();
    bin_ps!(v, out, _mm_min_ss, _mm_max_ss, _mm_move_ss, _mm_movelh_ps);
    un_ps!(v, out, _mm_moveldup_ps);
    out
}
#[test]
fn arith_ss() {
    if !is_x86_feature_detected!("sse3") {
        return;
    }
    check(unsafe { run_arith_ss() }, &[
        ("_mm_sub_ss", 0x16cfb7181d7e1d34),
        ("_mm_mul_ss", 0x31b2840f3d2b37ad),
        ("_mm_div_ss", 0x495859920376826b),
        ("_mm_sqrt_ss", 0x5bb6a4255c372dc7),
        ("_mm_min_ss", 0xcad1b1b758ee5f09),
        ("_mm_max_ss", 0xf2e181f540eaff41),
        ("_mm_move_ss", 0x0789f11871ac75ad),
        ("_mm_movelh_ps", 0x29b17aa235827c35),
        ("_mm_moveldup_ps", 0x9253802cde2a2e41),
    ]);
}

#[target_feature(enable = "sse")]
unsafe fn run_cmp_ps() -> Got {
    let mut out = Got::new();
    let v = zoo();
    bin_ps!(v, out, _mm_cmpnlt_ps, _mm_cmpnle_ps, _mm_cmpngt_ps, _mm_cmpnge_ps);
    out
}
#[test]
fn cmp_ps() {
    check(unsafe { run_cmp_ps() }, &[
        ("_mm_cmpnlt_ps", 0x6a65a725ac017261),
        ("_mm_cmpnle_ps", 0xbc805bbcf9e45f59),
        ("_mm_cmpngt_ps", 0x267e308bc4eb9761),
        ("_mm_cmpnge_ps", 0x924dc4c01a0ad059),
    ]);
}

#[target_feature(enable = "sse")]
unsafe fn run_cmp_ss() -> Got {
    let mut out = Got::new();
    let v = zoo();
    bin_ps!(v, out, _mm_cmpeq_ss, _mm_cmplt_ss, _mm_cmple_ss, _mm_cmpgt_ss, _mm_cmpge_ss, _mm_cmpneq_ss);
    bin_ps!(v, out, _mm_cmpnlt_ss, _mm_cmpnle_ss, _mm_cmpngt_ss, _mm_cmpnge_ss, _mm_cmpord_ss, _mm_cmpunord_ss);
    out
}
#[test]
fn cmp_ss() {
    check(unsafe { run_cmp_ss() }, &[
        ("_mm_cmpeq_ss", 0x58307c80f35ed999),
        ("_mm_cmplt_ss", 0x6f5e3226d71179ed),
        ("_mm_cmple_ss", 0xc674810a5872df21),
        ("_mm_cmpgt_ss", 0x5a2396aa85c417fd),
        ("_mm_cmpge_ss", 0x0f92ca34e01491f1),
        ("_mm_cmpneq_ss", 0xc4e1ec62e481f6f1),
        ("_mm_cmpnlt_ss", 0xf71c3b494305e0dd),
        ("_mm_cmpnle_ss", 0x2b75ac1264beaae9),
        ("_mm_cmpngt_ss", 0xe1e99cf2c1a2b1ed),
        ("_mm_cmpnge_ss", 0x6ca7a685ad2db2b9),
        ("_mm_cmpord_ss", 0xcf5297791c187859),
        ("_mm_cmpunord_ss", 0xb530a38bcdbc7491),
    ]);
}

#[target_feature(enable = "avx")]
unsafe fn run_cmp_ss_imm() -> Got {
    let mut out = Got::new();
    let v = zoo();
    bin_ps_imm!(v, out, _mm_cmp_ss, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21,
        22, 23, 24, 25, 26, 27, 28, 29, 30, 31);
    // each predicate on its own: equal, less, greater, unordered
    let mut h = FNV;
    let one = _mm_set1_ps(black_box(1.0));
    let two = _mm_set1_ps(black_box(2.0));
    let nan = _mm_set1_ps(black_box(f32::NAN));
    macro_rules! each {
        ($($imm:literal),*) => {$(
            for (a, b) in [(one, one), (one, two), (two, one), (one, nan)] {
                fold_i32(&mut h, _mm_movemask_ps(_mm_cmp_ss::<$imm>(a, b)) & 1);
            }
        )*};
    }
    each!(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28,
        29, 30, 31);
    out.push(("_mm_cmp_ss predicates", h));
    out
}
#[test]
fn cmp_ss_imm() {
    if !is_x86_feature_detected!("avx") {
        return;
    }
    check(unsafe { run_cmp_ss_imm() }, &[
        ("_mm_cmp_ss", 0x96f6e6a2a5b3b5a5),
        ("_mm_cmp_ss predicates", 0x97f396b6f690aaa5),
    ]);
}

#[target_feature(enable = "sse")]
unsafe fn run_comi() -> Got {
    let mut out = Got::new();
    let v = zoo();
    bin_i32!(v, out, _mm_comieq_ss, _mm_comilt_ss, _mm_comile_ss, _mm_comigt_ss, _mm_comige_ss, _mm_comineq_ss);
    bin_i32!(v, out, _mm_ucomieq_ss, _mm_ucomilt_ss, _mm_ucomile_ss, _mm_ucomigt_ss, _mm_ucomige_ss, _mm_ucomineq_ss);
    out
}
#[test]
fn comi() {
    check(unsafe { run_comi() }, &[
        ("_mm_comieq_ss", 0x57feb210a62e1424),
        ("_mm_comilt_ss", 0x7b969595161a49f5),
        ("_mm_comile_ss", 0x9db6edc90c47d2f4),
        ("_mm_comigt_ss", 0xc868a4c986420e65),
        ("_mm_comige_ss", 0x277eef216037e244),
        ("_mm_comineq_ss", 0xf56c48345c787ac4),
        ("_mm_ucomieq_ss", 0x57feb210a62e1424),
        ("_mm_ucomilt_ss", 0x7b969595161a49f5),
        ("_mm_ucomile_ss", 0x9db6edc90c47d2f4),
        ("_mm_ucomigt_ss", 0xc868a4c986420e65),
        ("_mm_ucomige_ss", 0x277eef216037e244),
        ("_mm_ucomineq_ss", 0xf56c48345c787ac4),
    ]);
}

const HALVES: [f32; 14] = [
    0.5, -0.5, 1.5, -1.5, 2.5, -2.5, 0.25, -0.75, 2147483520.0, -2147483648.0, -2147483904.0, 1e-30, -1e-30, 7.75,
];

#[target_feature(enable = "sse")]
unsafe fn run_conv() -> Got {
    let mut out = Got::new();
    let mut v = arith();
    for x in HALVES {
        v.push(black_box([x.to_bits(), 0x3f80_0000, NAN, 0]));
    }
    // MXCSR rounding: nearest, down, up, toward zero
    let csr = _mm_getcsr();
    let mut h = FNV;
    for rm in 0..4u32 {
        _mm_setcsr((csr & !0x6000) | (rm << 13));
        let mut r = [0i32; 64];
        for (i, a) in v.iter().enumerate() {
            r[i] = _mm_cvt_ss2si(ld(a));
        }
        _mm_setcsr(csr);
        for x in &r[..v.len()] {
            fold_i32(&mut h, *x);
        }
    }
    out.push(("_mm_cvt_ss2si", h));
    let mut h = FNV;
    for a in v.iter() {
        fold_i32(&mut h, _mm_cvtt_ss2si(ld(a)));
    }
    out.push(("_mm_cvtt_ss2si", h));
    let mut ints = vec![0, 1, -1, i32::MIN, i32::MAX, 16777217, -16777217, 33554435, 2147483583, -2147483585];
    let mut r = Rng(0x1234_5678_9abc_def1);
    for _ in 0..16 {
        ints.push(r.next() as i32);
    }
    let ints = black_box(ints);
    let mut h = FNV;
    for a in v.iter().step_by(5) {
        for b in ints.iter() {
            fold_ps(&mut h, _mm_cvtsi32_ss(ld(a), *b));
        }
    }
    out.push(("_mm_cvtsi32_ss", h));
    out
}
#[test]
fn conv() {
    check(unsafe { run_conv() }, &[
        ("_mm_cvt_ss2si", 0xd996df52d5b786d4),
        ("_mm_cvtt_ss2si", 0x84dbecdc6f7aa89a),
        ("_mm_cvtsi32_ss", 0xe059160a21b2d521),
    ]);
}

#[target_feature(enable = "sse3")]
unsafe fn run_sse3() -> Got {
    let mut out = Got::new();
    let v = arith();
    bin_ps!(v, out, _mm_addsub_ps, _mm_hadd_ps, _mm_hsub_ps);
    out
}
#[test]
fn sse3() {
    if !is_x86_feature_detected!("sse3") {
        return;
    }
    check(unsafe { run_sse3() }, &[
        ("_mm_addsub_ps", 0xd2f6090acf6f651c),
        ("_mm_hadd_ps", 0xff582a9e27cadde5),
        ("_mm_hsub_ps", 0x3575e767378c4a75),
    ]);
}

#[target_feature(enable = "sse4.1")]
unsafe fn run_sse41() -> Got {
    let mut out = Got::new();
    let z = zoo();
    bin_ps_imm!(z, out, _mm_blend_ps, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15);
    bin_ps_imm!(z, out, _mm_insert_ps, 0x00, 0x10, 0x20, 0x30, 0x40, 0x80, 0xc0, 0xff, 0x1d, 0xe2, 0x5a, 0xa5, 0x0f,
        0x71);
    let mut h = FNV;
    for a in z.iter() {
        fold_i32(&mut h, _mm_extract_ps::<0>(ld(a)));
        fold_i32(&mut h, _mm_extract_ps::<1>(ld(a)));
        fold_i32(&mut h, _mm_extract_ps::<2>(ld(a)));
        fold_i32(&mut h, _mm_extract_ps::<3>(ld(a)));
    }
    out.push(("_mm_extract_ps", h));
    let mut v = arith();
    for x in HALVES {
        v.push(black_box([x.to_bits(), 0x3f80_0000, NAN, 0]));
    }
    bin_ps!(v, out, _mm_floor_ss, _mm_ceil_ss);
    bin_ps_imm!(v, out, _mm_round_ss, 0, 1, 2, 3, 8, 9, 10, 11);
    // _MM_FROUND_CUR_DIRECTION under each MXCSR rounding mode
    let csr = _mm_getcsr();
    let mut h = FNV;
    for rm in 0..4u32 {
        let mut r = [[0u32; 4]; 64];
        _mm_setcsr((csr & !0x6000) | (rm << 13));
        for (i, a) in v.iter().enumerate() {
            r[i] = bits(_mm_round_ss::<4>(ld(&v[(i + 1) % v.len()]), ld(a)));
            r[i][1] ^= bits(_mm_round_ss::<12>(ld(&v[(i + 1) % v.len()]), ld(a)))[0];
        }
        _mm_setcsr(csr);
        for x in &r[..v.len()] {
            fold_ps(&mut h, ld(x));
        }
    }
    out.push(("_mm_round_ss mxcsr", h));
    // a NaN lane of the result is folded as one pattern: which NaN dpps keeps among several is
    // not a property of the scalar sum
    let v = arith();
    let mut h = FNV;
    macro_rules! dp {
        ($($imm:literal),*) => {
            for a in v.iter() {
                for b in v.iter() {
                    $(
                        let mut r = bits(_mm_dp_ps::<$imm>(ld(a), ld(b)));
                        for x in r.iter_mut() {
                            if f32::from_bits(*x).is_nan() {
                                *x = NAN;
                            }
                        }
                        fold_ps(&mut h, ld(&r));
                    )*
                }
            }
        };
    }
    dp!(0x00, 0xff, 0xf1, 0x71, 0x55, 0xaa, 0x3c, 0x80, 0x1f, 0xe8, 0x33);
    out.push(("_mm_dp_ps", h));
    out
}
#[test]
fn sse41() {
    if !is_x86_feature_detected!("sse4.1") {
        return;
    }
    check(unsafe { run_sse41() }, &[
        ("_mm_blend_ps", 0xb905c033c5740cd5),
        ("_mm_insert_ps", 0x02bcdfc22c7d0991),
        ("_mm_extract_ps", 0x73bf8bd960d0a0a3),
        ("_mm_floor_ss", 0xc30a6483b2649d35),
        ("_mm_ceil_ss", 0x356d8152f568990d),
        ("_mm_round_ss", 0x195f5ba46a3bf1b5),
        ("_mm_round_ss mxcsr", 0xab22fac92127ae0d),
        ("_mm_dp_ps", 0x2adc0ec857f4f00e),
    ]);
}

#[target_feature(enable = "fma")]
unsafe fn run_fma() -> Got {
    let mut out = Got::new();
    let v = arith();
    tern_ps!(v, out, _mm_fmadd_ss, _mm_fmsub_ss, _mm_fnmadd_ss, _mm_fnmsub_ss, _mm_fmaddsub_ps, _mm_fmsubadd_ps);
    out
}
#[test]
fn fma() {
    if !is_x86_feature_detected!("fma") {
        return;
    }
    check(unsafe { run_fma() }, &[
        ("_mm_fmadd_ss", 0x2095e631f68558f5),
        ("_mm_fmsub_ss", 0x0f629f870982ca23),
        ("_mm_fnmadd_ss", 0x820cdaab2c90e523),
        ("_mm_fnmsub_ss", 0xf02e30117c2125f5),
        ("_mm_fmaddsub_ps", 0x697adc4721984658),
        ("_mm_fmsubadd_ps", 0x1dd9c0f1c73b2015),
    ]);
}

#[target_feature(enable = "avx2")]
unsafe fn run_avx() -> Got {
    let mut out = Got::new();
    let z = zoo();
    let mut h = FNV;
    for a in z.iter() {
        let f = f32::from_bits(a[0]);
        fold_ps(&mut h, _mm_broadcast_ss(&f));
    }
    out.push(("_mm_broadcast_ss", h));
    un_ps!(z, out, _mm_broadcastss_ps);
    let mut h = FNV;
    for a in z.iter() {
        fold_ps(&mut h, _mm_permute_ps::<0x00>(ld(a)));
        fold_ps(&mut h, _mm_permute_ps::<0x1b>(ld(a)));
        fold_ps(&mut h, _mm_permute_ps::<0xe4>(ld(a)));
        fold_ps(&mut h, _mm_permute_ps::<0xff>(ld(a)));
        fold_ps(&mut h, _mm_permute_ps::<0x4e>(ld(a)));
        fold_ps(&mut h, _mm_permute_ps::<0xb1>(ld(a)));
        fold_ps(&mut h, _mm_permute_ps::<0x93>(ld(a)));
    }
    out.push(("_mm_permute_ps", h));
    let mut h = FNV;
    for a in z.iter() {
        for b in z.iter() {
            fold_ps(&mut h, _mm_permutevar_ps(ld(a), ldi(b)));
        }
    }
    out.push(("_mm_permutevar_ps", h));
    bin_i32!(z, out, _mm_testz_ps, _mm_testc_ps, _mm_testnzc_ps);
    // the mask lanes that are off are never read or written: the buffers end right after lane 1
    let mut h = FNV;
    for (i, a) in z.iter().enumerate() {
        for m in z.iter() {
            let src = [f32::from_bits(a[0]), f32::from_bits(a[1]), f32::from_bits(a[2]), f32::from_bits(a[3])];
            fold_ps(&mut h, _mm_maskload_ps(src.as_ptr(), ldi(m)));
            let short = black_box([src[0], src[1]]);
            let m2 = [m[0], m[1], m[2] & 0x7fff_ffff, (i as u32) & 0x7fff_ffff];
            fold_ps(&mut h, _mm_maskload_ps(short.as_ptr(), ldi(&m2)));
        }
    }
    out.push(("_mm_maskload_ps", h));
    let mut h = FNV;
    for (i, a) in z.iter().enumerate() {
        for m in z.iter() {
            let mut dst = [f32::from_bits(0x1111_1111), f32::from_bits(0x2222_2222), 3.0, 4.0];
            _mm_maskstore_ps(dst.as_mut_ptr(), ldi(m), ld(a));
            fold_ps(&mut h, _mm_loadu_ps(dst.as_ptr()));
            let mut short = black_box([5.0f32, 6.0]);
            let m2 = [m[0], m[1], m[2] & 0x7fff_ffff, (i as u32) & 0x7fff_ffff];
            _mm_maskstore_ps(short.as_mut_ptr(), ldi(&m2), ld(a));
            fold(&mut h, &short[0].to_bits().to_le_bytes());
            fold(&mut h, &short[1].to_bits().to_le_bytes());
        }
    }
    out.push(("_mm_maskstore_ps", h));
    out
}
#[test]
fn avx() {
    if !is_x86_feature_detected!("avx2") {
        return;
    }
    check(unsafe { run_avx() }, &[
        ("_mm_broadcast_ss", 0xd7d25bae1d5991f5),
        ("_mm_broadcastss_ps", 0xd7d25bae1d5991f5),
        ("_mm_permute_ps", 0x172fcc999b41762f),
        ("_mm_permutevar_ps", 0xc5bb6ebfdc9491d5),
        ("_mm_testz_ps", 0x7639bda5d6f56d34),
        ("_mm_testc_ps", 0x06a91c425a7a50b4),
        ("_mm_testnzc_ps", 0x29bafa3c5a9f4565),
        ("_mm_maskload_ps", 0x04e808382f8e9b38),
        ("_mm_maskstore_ps", 0x1b5b98cc7394b188),
    ]);
}

#[target_feature(enable = "avx2")]
unsafe fn run_gather() -> Got {
    let mut out = Got::new();
    let mut r = Rng(0xdead_beef_cafe_f00d);
    let mut data = [0u32; 64];
    for (i, d) in data.iter_mut().enumerate() {
        *d = if i < ZOO.len() { ZOO[i] } else { r.next() };
    }
    let data = black_box(data);
    // offsets from the middle of the table, negative ones included
    let mid = (data.as_ptr() as *const f32).add(32);
    let (mut h32, mut h64, mut m32, mut m64) = (FNV, FNV, FNV, FNV);
    for _ in 0..48 {
        let idx = [r.next() % 64, r.next() % 64, r.next() % 64, r.next() % 64].map(|x| x as i32 - 32);
        let mask = [r.next(), r.next(), r.next(), r.next()];
        let src = ld(&[r.next(), r.next(), r.next(), r.next()]);
        let i4 = ldi(&idx.map(|x| x as u32));
        let i8_ = ldi(&idx.map(|x| (x * 2) as u32));
        // byte offsets, odd ones included, kept inside the table
        let i1 = ldi(&idx.map(|x| (x * 4 + if x < 31 { x & 1 } else { 0 }) as u32));
        fold_ps(&mut h32, _mm_i32gather_ps::<4>(mid, i4));
        fold_ps(&mut h32, _mm_i32gather_ps::<2>(mid, i8_));
        fold_ps(&mut h32, _mm_i32gather_ps::<1>(mid, i1));
        let q = _mm_set_epi64x(idx[1] as i64, idx[0] as i64);
        let q2 = _mm_set_epi64x(idx[3] as i64 * 4, idx[2] as i64 * 4);
        let q8 = _mm_set_epi64x((idx[2] / 2) as i64, (idx[3] / 2) as i64);
        fold_ps(&mut h64, _mm_i64gather_ps::<4>(mid, q));
        fold_ps(&mut h64, _mm_i64gather_ps::<1>(mid, q2));
        fold_ps(&mut h64, _mm_i64gather_ps::<8>(mid, q8));
        // lanes whose mask is off carry an offset far outside the table
        let wild = [0, 1, 2, 3].map(|i| if mask[i] >> 31 != 0 { idx[i] as u32 } else { 0x7000_0000 + mask[i] % 4096 });
        fold_ps(&mut m32, _mm_mask_i32gather_ps::<4>(src, mid, ldi(&wild), ld(&mask)));
        fold_ps(&mut m32, _mm_mask_i32gather_ps::<4>(src, mid, i4, _mm_set1_ps(-1.0)));
        fold_ps(&mut m32, _mm_mask_i32gather_ps::<8>(src, mid, ldi(&wild), _mm_setzero_ps()));
        let w = |i: usize| if mask[i] >> 31 != 0 { idx[i] as i64 } else { 0x7000_0000_0000 + mask[i] as i64 };
        fold_ps(&mut m64, _mm_mask_i64gather_ps::<4>(src, mid, _mm_set_epi64x(w(1), w(0)), ld(&mask)));
        fold_ps(&mut m64, _mm_mask_i64gather_ps::<4>(src, mid, q, _mm_set1_ps(-0.0)));
        fold_ps(&mut m64, _mm_mask_i64gather_ps::<2>(src, mid, _mm_set_epi64x(w(3), w(2)), _mm_setzero_ps()));
    }
    out.push(("_mm_i32gather_ps", h32));
    out.push(("_mm_i64gather_ps", h64));
    out.push(("_mm_mask_i32gather_ps", m32));
    out.push(("_mm_mask_i64gather_ps", m64));
    out
}
#[test]
fn gather() {
    if !is_x86_feature_detected!("avx2") {
        return;
    }
    check(unsafe { run_gather() }, &[
        ("_mm_i32gather_ps", 0x4a16266abdd02b62),
        ("_mm_i64gather_ps", 0x389e54f219375cb8),
        ("_mm_mask_i32gather_ps", 0x404bc1ac946b0276),
        ("_mm_mask_i64gather_ps", 0x953faa977915d4e5),
    ]);
}

#[repr(C, align(16))]
struct Al([f32; 4]);

#[target_feature(enable = "sse")]
unsafe fn run_mem() -> Got {
    let mut out = Got::new();
    let z = zoo();
    let (mut h_ss, mut h1, mut hp1, mut hr) = (FNV, FNV, FNV, FNV);
    let (mut s_ss, mut s1, mut sp1, mut sr, mut st) = (FNV, FNV, FNV, FNV, FNV);
    for a in z.iter() {
        let m = black_box(Al([f32::from_bits(a[0]), f32::from_bits(a[1]), f32::from_bits(a[2]), f32::from_bits(a[3])]));
        // one f32 read: the unaligned last element of the array
        fold_ps(&mut h_ss, _mm_load_ss(m.0.as_ptr().add(3)));
        fold_ps(&mut h1, _mm_load1_ps(m.0.as_ptr().add(3)));
        fold_ps(&mut hp1, _mm_load_ps1(m.0.as_ptr().add(1)));
        fold_ps(&mut hr, _mm_loadr_ps(m.0.as_ptr()));
        let fresh = || Al([f32::from_bits(0x0bad_f00d); 4]);
        let mut d = fresh();
        _mm_store_ss(d.0.as_mut_ptr().add(3), ld(a));
        fold_ps(&mut s_ss, _mm_loadu_ps(d.0.as_ptr()));
        let mut d = fresh();
        _mm_store1_ps(d.0.as_mut_ptr(), ld(a));
        fold_ps(&mut s1, _mm_loadu_ps(d.0.as_ptr()));
        let mut d = fresh();
        _mm_store_ps1(d.0.as_mut_ptr(), ld(a));
        fold_ps(&mut sp1, _mm_loadu_ps(d.0.as_ptr()));
        let mut d = fresh();
        _mm_storer_ps(d.0.as_mut_ptr(), ld(a));
        fold_ps(&mut sr, _mm_loadu_ps(d.0.as_ptr()));
        let mut d = fresh();
        _mm_stream_ps(d.0.as_mut_ptr(), ld(a));
        _mm_sfence();
        fold_ps(&mut st, _mm_loadu_ps(d.0.as_ptr()));
    }
    out.push(("_mm_load_ss", h_ss));
    out.push(("_mm_load1_ps", h1));
    out.push(("_mm_load_ps1", hp1));
    out.push(("_mm_loadr_ps", hr));
    out.push(("_mm_store_ss", s_ss));
    out.push(("_mm_store1_ps", s1));
    out.push(("_mm_store_ps1", sp1));
    out.push(("_mm_storer_ps", sr));
    out.push(("_mm_stream_ps", st));
    out
}
#[test]
fn mem() {
    check(unsafe { run_mem() }, &[
        ("_mm_load_ss", 0x71ec40b6898f5a5a),
        ("_mm_load1_ps", 0x904f3de61386ca0d),
        ("_mm_load_ps1", 0x107fed11e94b2645),
        ("_mm_loadr_ps", 0x31bf0a58443a643f),
        ("_mm_store_ss", 0xa7280ce9bc51955c),
        ("_mm_store1_ps", 0xd7d25bae1d5991f5),
        ("_mm_store_ps1", 0xd7d25bae1d5991f5),
        ("_mm_storer_ps", 0x31bf0a58443a643f),
        ("_mm_stream_ps", 0x73bf8bd960d0a0a3),
    ]);
}

// rcpss / rsqrtss are approximations on hardware: relative error at most 1.5 * 2^-12
#[target_feature(enable = "sse")]
unsafe fn run_approx() {
    let mut r = Rng(0x0123_4567_89ab_cdef);
    let tol = 1.5 / 4096.0;
    for i in 0..200 {
        let x = f32::from_bits(black_box(r.sane()) & 0x7fff_ffff);
        let up = [r.next(), r.next(), r.next()];
        let a = ld(&[x.to_bits(), up[0], up[1], up[2]]);
        let got = bits(_mm_rcp_ss(a));
        let want = 1.0 / x as f64;
        assert!(((f32::from_bits(got[0]) as f64 - want) / want).abs() <= tol, "_mm_rcp_ss {i}");
        assert_eq!(&got[1..], &up[..], "_mm_rcp_ss upper {i}");
        let n = ld(&[x.to_bits() | 0x8000_0000, up[0], up[1], up[2]]);
        let got = bits(_mm_rcp_ss(n));
        assert!(((f32::from_bits(got[0]) as f64 + want) / want).abs() <= tol, "_mm_rcp_ss neg {i}");
        let got = bits(_mm_rsqrt_ss(a));
        let want = 1.0 / (x as f64).sqrt();
        assert!(((f32::from_bits(got[0]) as f64 - want) / want).abs() <= tol, "_mm_rsqrt_ss {i}");
        assert_eq!(&got[1..], &up[..], "_mm_rsqrt_ss upper {i}");
    }
    let v = |x: f32| _mm_setr_ps(black_box(x), 2.0, 3.0, 4.0);
    let lo = |v: __m128| bits(v)[0];
    assert_eq!(lo(_mm_rcp_ss(v(0.0))), 0x7f80_0000);
    assert_eq!(lo(_mm_rcp_ss(v(-0.0))), 0xff80_0000);
    assert_eq!(lo(_mm_rcp_ss(v(f32::INFINITY))), 0);
    assert_eq!(lo(_mm_rcp_ss(v(f32::NEG_INFINITY))), 0x8000_0000);
    assert!(f32::from_bits(lo(_mm_rcp_ss(v(f32::NAN)))).is_nan());
    assert_eq!(lo(_mm_rsqrt_ss(v(0.0))), 0x7f80_0000);
    assert_eq!(lo(_mm_rsqrt_ss(v(-0.0))), 0xff80_0000);
    assert_eq!(lo(_mm_rsqrt_ss(v(f32::INFINITY))), 0);
    assert!(f32::from_bits(lo(_mm_rsqrt_ss(v(-1.0)))).is_nan());
    assert!(f32::from_bits(lo(_mm_rsqrt_ss(v(f32::NAN)))).is_nan());
    assert_eq!(bits(_mm_rsqrt_ss(v(4.0)))[1..], [2.0f32.to_bits(), 3.0f32.to_bits(), 4.0f32.to_bits()]);
}
#[test]
fn approx() {
    unsafe { run_approx() }
}

#[target_feature(enable = "sse2")]
unsafe fn run_fences() -> u32 {
    let mut cell = black_box([1u8; 64]);
    let mut n = 0;
    for i in 0..8 {
        cell[i] = i as u8;
        _mm_sfence();
        _mm_lfence();
        _mm_mfence();
        _mm_pause();
        _mm_clflush(cell.as_ptr().add(i));
        n += cell[i] as u32;
    }
    n
}
#[test]
fn fences() {
    assert_eq!(unsafe { run_fences() }, 28);
}
