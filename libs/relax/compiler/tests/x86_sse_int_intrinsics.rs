#![cfg(target_arch = "x86_64")]
#![allow(deprecated)]
use std::arch::x86_64::*;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
}

const FNV0: u64 = 0xcbf29ce484222325;
fn fnv(h: &mut u64, bytes: &[u8]) {
    for &b in bytes {
        *h ^= b as u64;
        *h = h.wrapping_mul(0x100000001b3);
    }
}
trait Fold {
    fn fold(self, h: &mut u64);
}
impl Fold for i32 {
    fn fold(self, h: &mut u64) {
        fnv(h, &self.to_le_bytes())
    }
}
impl Fold for __m128i {
    fn fold(self, h: &mut u64) {
        let b: [u8; 16] = unsafe { std::mem::transmute(self) };
        fnv(h, &b)
    }
}
impl Fold for __m128 {
    fn fold(self, h: &mut u64) {
        let b: [u8; 16] = unsafe { std::mem::transmute(self) };
        fnv(h, &b)
    }
}

fn vi(lo: u64, hi: u64) -> __m128i {
    unsafe { std::mem::transmute([lo, hi]) }
}
fn inputs() -> Vec<__m128i> {
    let mut v = vec![
        vi(0, 0),
        vi(!0, !0),
        vi(0x8000_8000_8000_8000, 0x8000_8000_8000_8000),
        vi(0x7fff_7fff_7fff_7fff, 0x7fff_7fff_7fff_7fff),
        vi(0x8000_0000_8000_0000, 0x8000_0000_8000_0000),
        vi(0x7fff_ffff_7fff_ffff, 0x7fff_ffff_7fff_ffff),
        vi(0x8080_8080_8080_8080, 0x8080_8080_8080_8080),
        vi(0x7f7f_7f7f_7f7f_7f7f, 0x7f7f_7f7f_7f7f_7f7f),
        vi(0x8000_0000_0000_0000, 0x7fff_ffff_ffff_ffff),
        vi(0x0001_0001_0001_0001, 0x0101_0101_0101_0101),
        vi(0x8000_7fff_8000_7fff, 0x7fff_8000_ffff_0001),
        vi(0x0000_0001_ffff_ffff, 0xffff_ffff_0000_0001),
    ];
    let mut r = Rng(0x9e3779b97f4a7c15);
    for _ in 0..20 {
        v.push(vi(r.next(), r.next()));
    }
    v
}
fn counts() -> Vec<__m128i> {
    let mut v = Vec::new();
    for c in [0u64, 1, 7, 8, 15, 16, 17, 31, 32, 33, 63, 64, 65, 1 << 32, (1 << 32) + 3, !0, 1 << 63] {
        v.push(vi(c, 0));
        v.push(vi(c, 0x1234_5678_9abc_def0));
    }
    v
}
// per-lane shift counts: mostly small, sometimes at or past the lane width, sometimes huge
fn lane_counts() -> Vec<__m128i> {
    let mut r = Rng(0x243f6a8885a308d3);
    let mut v = vec![vi(0, 0), vi(!0, !0), vi(31 | 32 << 32, 33 | 64 << 32), vi(63, 64), vi(65, 1 << 63)];
    for _ in 0..40 {
        let mut l = [0u32; 4];
        for x in l.iter_mut() {
            let k = r.next();
            *x = match k & 7 {
                0 => (k >> 8) as u32,
                1 => 32 + ((k >> 8) % 40) as u32,
                _ => ((k >> 8) % 34) as u32,
            };
        }
        v.push(vi(l[0] as u64 | (l[1] as u64) << 32, l[2] as u64 | (l[3] as u64) << 32));
        v.push(vi((k64(&mut r)) as u64, k64(&mut r) as u64));
    }
    v
}
fn k64(r: &mut Rng) -> u64 {
    let k = r.next();
    match k & 7 {
        0 => k >> 8,
        1 => 64 + (k >> 8) % 10,
        _ => (k >> 8) % 66,
    }
}

fn check(got: Vec<(&'static str, u64)>, want: &[(&str, u64)]) {
    let mut bad = false;
    for (i, (name, h)) in got.iter().enumerate() {
        if i >= want.len() || want[i].0 != *name || want[i].1 != *h {
            bad = true;
        }
    }
    if bad || got.len() != want.len() {
        println!();
        for (name, h) in got.iter() {
            println!("        (\"{}\", {:#018x}),", name, h);
        }
    }
    assert_eq!(got.len(), want.len());
    for (i, (name, h)) in got.iter().enumerate() {
        assert_eq!(want[i].0, *name);
        assert_eq!(*h, want[i].1, "{name}");
    }
}

macro_rules! un {
    ($out:ident, $xs:ident, $f:ident) => {{
        let mut h = FNV0;
        for &a in $xs.iter() {
            $f(a).fold(&mut h);
        }
        $out.push((stringify!($f), h));
    }};
}
macro_rules! bin {
    ($out:ident, $xs:ident, $ys:ident, $f:ident) => {{
        let mut h = FNV0;
        for &a in $xs.iter() {
            for &b in $ys.iter() {
                $f(a, b).fold(&mut h);
            }
        }
        $out.push((stringify!($f), h));
    }};
}
macro_rules! imm_un {
    ($out:ident, $xs:ident, $f:ident, $($n:literal)*) => {{
        let mut h = FNV0;
        for &a in $xs.iter() {
            $( $f::<$n>(a).fold(&mut h); )*
        }
        $out.push((stringify!($f), h));
    }};
}
macro_rules! imm_bin {
    ($out:ident, $xs:ident, $f:ident, $($n:literal)*) => {{
        let mut h = FNV0;
        for &a in $xs.iter() {
            for &b in $xs.iter() {
                $( $f::<$n>(a, b).fold(&mut h); )*
            }
        }
        $out.push((stringify!($f), h));
    }};
}

#[target_feature(enable = "sse4.2,avx2")]
unsafe fn g_extend() -> Vec<(&'static str, u64)> {
    let xs = inputs();
    let mut out = Vec::new();
    un!(out, xs, _mm_cvtepi8_epi16);
    un!(out, xs, _mm_cvtepi8_epi32);
    un!(out, xs, _mm_cvtepi8_epi64);
    un!(out, xs, _mm_cvtepi16_epi32);
    un!(out, xs, _mm_cvtepi16_epi64);
    un!(out, xs, _mm_cvtepi32_epi64);
    un!(out, xs, _mm_cvtepu8_epi32);
    un!(out, xs, _mm_cvtepu8_epi64);
    un!(out, xs, _mm_cvtepu16_epi32);
    un!(out, xs, _mm_cvtepu16_epi64);
    un!(out, xs, _mm_cvtepu32_epi64);
    out
}
#[test]
fn extend() {
    if !is_x86_feature_detected!("sse4.2") || !is_x86_feature_detected!("avx2") {
        return;
    }
    check(unsafe { g_extend() }, &[
        ("_mm_cvtepi8_epi16", 0x80e77800e28a3706),
        ("_mm_cvtepi8_epi32", 0x9ecf78a3106d3a34),
        ("_mm_cvtepi8_epi64", 0xfc9c4174e0b1b9d5),
        ("_mm_cvtepi16_epi32", 0x7fe3c3044b3c5ad1),
        ("_mm_cvtepi16_epi64", 0xe3c188ad2733153c),
        ("_mm_cvtepi32_epi64", 0x49bdaa80c352f2a1),
        ("_mm_cvtepu8_epi32", 0xcd007ccc619a3914),
        ("_mm_cvtepu8_epi64", 0x4fa64419896e9335),
        ("_mm_cvtepu16_epi32", 0xb520f4136aff8889),
        ("_mm_cvtepu16_epi64", 0xe136e0f7f5269414),
        ("_mm_cvtepu32_epi64", 0x05c2aa6a80d16a71),
    ]);
}

#[target_feature(enable = "sse4.2,avx2")]
unsafe fn g_broadcast_blend() -> Vec<(&'static str, u64)> {
    let xs = inputs();
    let mut out = Vec::new();
    un!(out, xs, _mm_broadcastb_epi8);
    un!(out, xs, _mm_broadcastw_epi16);
    un!(out, xs, _mm_broadcastd_epi32);
    un!(out, xs, _mm_broadcastq_epi64);
    imm_bin!(out, xs, _mm_blend_epi32, 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15);
    imm_un!(out, xs, _mm_bslli_si128, 0 1 2 3 7 8 9 15 16 17 31 32 128 255);
    imm_un!(out, xs, _mm_bsrli_si128, 0 1 2 3 7 8 9 15 16 17 31 32 128 255);
    bin!(out, xs, xs, _mm_cmpgt_epi64);
    out
}
#[test]
fn broadcast_blend() {
    if !is_x86_feature_detected!("sse4.2") || !is_x86_feature_detected!("avx2") {
        return;
    }
    check(unsafe { g_broadcast_blend() }, &[
        ("_mm_broadcastb_epi8", 0x748e7c5d40c98d15),
        ("_mm_broadcastw_epi16", 0x6017359de3c4bc45),
        ("_mm_broadcastd_epi32", 0xd5467bf725eb973d),
        ("_mm_broadcastq_epi64", 0x89150e9b89f42331),
        ("_mm_blend_epi32", 0xcbec65db935c3cd5),
        ("_mm_bslli_si128", 0xd5fc392867826d66),
        ("_mm_bsrli_si128", 0x439e87be247234c1),
        ("_mm_cmpgt_epi64", 0x7337f683d7ef8e25),
    ]);
}

#[target_feature(enable = "sse4.2,avx2")]
unsafe fn g_insert_extract() -> Vec<(&'static str, u64)> {
    let xs = inputs();
    let mut out = Vec::new();
    imm_un!(out, xs, _mm_extract_epi8, 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15);
    let vals = [0i32, 1, -1, 0x7f, 0x80, 0xff, 0x100, 0x7fff, 0x8000, 0xffff, 0x12345678, i32::MIN, i32::MAX];
    let mut h = FNV0;
    for &a in xs.iter() {
        for &i in vals.iter() {
            _mm_insert_epi8::<0>(a, i).fold(&mut h);
            _mm_insert_epi8::<1>(a, i).fold(&mut h);
            _mm_insert_epi8::<5>(a, i).fold(&mut h);
            _mm_insert_epi8::<7>(a, i).fold(&mut h);
            _mm_insert_epi8::<8>(a, i).fold(&mut h);
            _mm_insert_epi8::<14>(a, i).fold(&mut h);
            _mm_insert_epi8::<15>(a, i).fold(&mut h);
        }
    }
    out.push(("_mm_insert_epi8", h));
    let mut h = FNV0;
    for &a in xs.iter() {
        for &i in vals.iter() {
            _mm_insert_epi16::<0>(a, i).fold(&mut h);
            _mm_insert_epi16::<1>(a, i).fold(&mut h);
            _mm_insert_epi16::<2>(a, i).fold(&mut h);
            _mm_insert_epi16::<3>(a, i).fold(&mut h);
            _mm_insert_epi16::<4>(a, i).fold(&mut h);
            _mm_insert_epi16::<5>(a, i).fold(&mut h);
            _mm_insert_epi16::<6>(a, i).fold(&mut h);
            _mm_insert_epi16::<7>(a, i).fold(&mut h);
        }
    }
    out.push(("_mm_insert_epi16", h));
    let mut h = FNV0;
    let mut r = Rng(0x13198a2e03707344);
    let edge = [0i16, -1, 1, i16::MIN, i16::MAX, 0x100, -0x100, 2];
    _mm_setr_epi16(edge[0], edge[1], edge[2], edge[3], edge[4], edge[5], edge[6], edge[7]).fold(&mut h);
    for _ in 0..32 {
        let k = r.next();
        let j = r.next();
        _mm_setr_epi16(k as i16, (k >> 16) as i16, (k >> 32) as i16, (k >> 48) as i16, j as i16, (j >> 16) as i16, (j >> 32) as i16, (j >> 48) as i16)
            .fold(&mut h);
    }
    out.push(("_mm_setr_epi16", h));
    out
}
#[test]
fn insert_extract() {
    if !is_x86_feature_detected!("sse4.2") || !is_x86_feature_detected!("avx2") {
        return;
    }
    check(unsafe { g_insert_extract() }, &[
        ("_mm_extract_epi8", 0x6b48826941c7412f),
        ("_mm_insert_epi8", 0x4006d060d1c9be8f),
        ("_mm_insert_epi16", 0x12ec89dbcc8bb5b1),
        ("_mm_setr_epi16", 0x435d285b81e9957c),
    ]);
}

#[target_feature(enable = "sse4.2,avx2")]
unsafe fn g_arith() -> Vec<(&'static str, u64)> {
    let xs = inputs();
    let mut out = Vec::new();
    bin!(out, xs, xs, _mm_mul_epi32);
    bin!(out, xs, xs, _mm_mulhi_epi16);
    bin!(out, xs, xs, _mm_mulhrs_epi16);
    bin!(out, xs, xs, _mm_hadd_epi16);
    bin!(out, xs, xs, _mm_hadd_epi32);
    bin!(out, xs, xs, _mm_hadds_epi16);
    bin!(out, xs, xs, _mm_hsub_epi16);
    bin!(out, xs, xs, _mm_hsub_epi32);
    bin!(out, xs, xs, _mm_hsubs_epi16);
    bin!(out, xs, xs, _mm_sign_epi8);
    bin!(out, xs, xs, _mm_sign_epi16);
    bin!(out, xs, xs, _mm_sign_epi32);
    un!(out, xs, _mm_minpos_epu16);
    let ties = [vi(0x0005_0003_0003_0007, 0x0003_0009_0003_0003), vi(0xffff_ffff_ffff_ffff, 0xfffe_ffff_fffe_ffff), vi(0x0001_0001_0001_0001, 0)];
    un!(out, ties, _mm_minpos_epu16);
    imm_bin!(out, xs, _mm_mpsadbw_epu8, 0 1 2 3 4 5 6 7);
    out
}
#[test]
fn arith() {
    if !is_x86_feature_detected!("sse4.2") || !is_x86_feature_detected!("avx2") {
        return;
    }
    check(unsafe { g_arith() }, &[
        ("_mm_mul_epi32", 0x9d0983a83d20948f),
        ("_mm_mulhi_epi16", 0x3b8cb3402bbca32b),
        ("_mm_mulhrs_epi16", 0x4a6e1c93d24548a8),
        ("_mm_hadd_epi16", 0x3accd2eb29f23e79),
        ("_mm_hadd_epi32", 0xd2ffb8182ccdd469),
        ("_mm_hadds_epi16", 0x193615a9de909cf5),
        ("_mm_hsub_epi16", 0x84831e14d9b6f341),
        ("_mm_hsub_epi32", 0xe47248e035854bbd),
        ("_mm_hsubs_epi16", 0x2bf0eec6cf4b82bd),
        ("_mm_sign_epi8", 0x3570f4ff2a8ae668),
        ("_mm_sign_epi16", 0x2b5926fe3de4e292),
        ("_mm_sign_epi32", 0xde628f9d865d02db),
        ("_mm_minpos_epu16", 0x16c15b2f825a460d),
        ("_mm_minpos_epu16", 0x964f9324fd651d1d),
        ("_mm_mpsadbw_epu8", 0xdae617d77b54e35c),
    ]);
}

#[target_feature(enable = "sse4.2,avx2")]
unsafe fn g_shifts() -> Vec<(&'static str, u64)> {
    let xs = inputs();
    let cs = counts();
    let ls = lane_counts();
    let mut out = Vec::new();
    bin!(out, xs, cs, _mm_sll_epi16);
    bin!(out, xs, cs, _mm_sll_epi64);
    bin!(out, xs, cs, _mm_srl_epi64);
    bin!(out, xs, cs, _mm_sra_epi16);
    bin!(out, xs, cs, _mm_sra_epi32);
    bin!(out, xs, ls, _mm_sllv_epi64);
    bin!(out, xs, ls, _mm_srlv_epi64);
    bin!(out, xs, ls, _mm_srlv_epi32);
    bin!(out, xs, ls, _mm_srav_epi32);
    out
}
#[test]
fn shifts() {
    if !is_x86_feature_detected!("sse4.2") || !is_x86_feature_detected!("avx2") {
        return;
    }
    check(unsafe { g_shifts() }, &[
        ("_mm_sll_epi16", 0x304a629403e0f6d5),
        ("_mm_sll_epi64", 0xd67df8d967a5f18d),
        ("_mm_srl_epi64", 0xae8bbb14ce0213b1),
        ("_mm_sra_epi16", 0x848780f63009a425),
        ("_mm_sra_epi32", 0x439605477018ccdd),
        ("_mm_sllv_epi64", 0x24748a72307c987c),
        ("_mm_srlv_epi64", 0x34bd92486526ea42),
        ("_mm_srlv_epi32", 0x3037c76c507ccdeb),
        ("_mm_srav_epi32", 0x106962dae3c660ae),
    ]);
}

#[target_feature(enable = "sse4.2,avx2")]
unsafe fn g_tests() -> Vec<(&'static str, u64)> {
    let mut xs = inputs();
    xs.push(vi(!0, 0));
    xs.push(vi(0, !0));
    xs.push(vi(0x00ff_00ff_00ff_00ff, 0x00ff_00ff_00ff_00ff));
    xs.push(vi(0xff00_ff00_ff00_ff00, 0xff00_ff00_ff00_ff00));
    xs.push(vi(0x00ff_00ff_00ff_00ff, 0));
    xs.push(vi(1, 0));
    xs.push(vi(0, 1 << 63));
    xs.push(vi(!0, !0 >> 1));
    let mut out = Vec::new();
    bin!(out, xs, xs, _mm_testc_si128);
    bin!(out, xs, xs, _mm_testnzc_si128);
    un!(out, xs, _mm_test_all_ones);
    bin!(out, xs, xs, _mm_test_all_zeros);
    bin!(out, xs, xs, _mm_test_mix_ones_zeros);
    out
}
#[test]
fn tests() {
    if !is_x86_feature_detected!("sse4.2") || !is_x86_feature_detected!("avx2") {
        return;
    }
    check(unsafe { g_tests() }, &[
        ("_mm_testc_si128", 0x9f73a00b6ec2f6d4),
        ("_mm_testnzc_si128", 0x82baa385f36de145),
        ("_mm_test_all_ones", 0x7d7ccbadcbec60b4),
        ("_mm_test_all_zeros", 0x41b8f906e0f8ee94),
        ("_mm_test_mix_ones_zeros", 0x82baa385f36de145),
    ]);
}

#[target_feature(enable = "sse4.2,avx2")]
unsafe fn g_float() -> Vec<(&'static str, u64)> {
    let mut out = Vec::new();
    let fs = [
        0.0f32, -0.0, 0.5, -0.5, 1.5, -1.5, 2.5, -2.5, 0.49999997, -0.49999997, 1.0e-40, -1.0e-40, 123456.7, -123456.7,
        2147483520.0, 2147483648.0, -2147483648.0, -2147483904.0, 4.0e9, -4.0e9, 1.0e30, f32::INFINITY, f32::NEG_INFINITY,
        f32::NAN, -f32::NAN, f32::MIN_POSITIVE, 16777217.0, 8388607.5, -8388607.5, 0.99999994, 3.5, -3.5,
    ];
    let is = [
        0i32, 1, -1, 16777215, 16777216, 16777217, 16777218, 16777219, -16777217, -16777219, 33554433, 33554435, 0x7fffffff,
        0x7fffffc0, 0x7fffff80, 0x7fffff40, -0x7fffffff, i32::MIN, 123456789, -123456789, 0x40000001, -0x40000001, 0x12345679,
    ];
    let base = _mm_setr_ps(1.0, -2.0, f32::NAN, 4.5);
    let csr = _mm_getcsr();
    let (mut h_ss, mut h_tss, mut h_si2ss) = (FNV0, FNV0, FNV0);
    for rm in [0u32, 1, 2, 3] {
        _mm_setcsr((csr & !0x6000) | (rm << 13));
        for &f in fs.iter() {
            let a = _mm_setr_ps(f, 9.0, -9.0, 1.0);
            _mm_cvtss_si32(a).fold(&mut h_ss);
            _mm_cvttss_si32(a).fold(&mut h_tss);
        }
        for &i in is.iter() {
            _mm_cvt_si2ss(base, i).fold(&mut h_si2ss);
        }
    }
    _mm_setcsr(csr);
    out.push(("_mm_cvtss_si32", h_ss));
    out.push(("_mm_cvttss_si32", h_tss));
    out.push(("_mm_cvt_si2ss", h_si2ss));
    out
}
#[test]
fn float() {
    if !is_x86_feature_detected!("sse4.2") || !is_x86_feature_detected!("avx2") {
        return;
    }
    check(unsafe { g_float() }, &[
        ("_mm_cvtss_si32", 0x81b645aa12f839df),
        ("_mm_cvttss_si32", 0x0650b5a43cfcab6d),
        ("_mm_cvt_si2ss", 0x080cd762a2b02351),
    ]);
}

#[repr(align(16))]
struct A16<T>(T);

macro_rules! gather {
    ($out:ident, $f:ident, $base:expr, $offs:ident, $($s:literal)*) => {{
        let mut h = FNV0;
        for &o in $offs.iter() {
            $( $f::<$s>($base, o).fold(&mut h); )*
        }
        $out.push((stringify!($f), h));
    }};
}
macro_rules! mask_gather {
    ($out:ident, $f:ident, $base:expr, $offs:ident, $wild:ident, $src:ident, $masks:ident, $gm:ident, $($s:literal)*) => {{
        let mut h = FNV0;
        for (k, &o) in $offs.iter().enumerate() {
            let m = $masks[k % $masks.len()];
            let src = $src[k % $src.len()];
            // lanes the mask leaves out carry an offset far outside the table: they must not be read
            let o = _mm_blendv_epi8($wild, o, m);
            let m = $gm[k % $gm.len()];
            $( $f::<$s>(src, $base, o, m).fold(&mut h); )*
        }
        $out.push((stringify!($f), h));
    }};
}

#[target_feature(enable = "sse4.2,avx2")]
unsafe fn g_gather() -> Vec<(&'static str, u64)> {
    let mut out = Vec::new();
    let mut r = Rng(0xa4093822299f31d0);
    let mut t32 = [0i32; 64];
    let mut t64 = [0i64; 64];
    for i in 0..64 {
        t32[i] = r.next() as i32;
        t64[i] = r.next() as i64;
    }
    let b32 = t32.as_ptr().add(32);
    let b64 = t64.as_ptr().add(32);
    // offsets in -12..=12: at scale 8 that stays inside both tables
    let mut o32 = vec![vi(0, 0), vi(0xffff_fff4_0000_000c, 0x0000_000c_ffff_fff4)];
    let mut o64 = vec![vi(0, 0), vi(12, (-12i64) as u64), vi((-12i64) as u64, 12)];
    for _ in 0..24 {
        let mut l = [0u32; 4];
        for x in l.iter_mut() {
            *x = ((r.next() % 25) as i32 - 12) as u32;
        }
        o32.push(vi(l[0] as u64 | (l[1] as u64) << 32, l[2] as u64 | (l[3] as u64) << 32));
        o64.push(vi(((r.next() % 25) as i64 - 12) as u64, ((r.next() % 25) as i64 - 12) as u64));
    }
    let xs = inputs();
    let mut masks = vec![vi(0, 0), vi(!0, !0), vi(1 << 63, 1 << 31), vi(1 << 31, 1 << 63), vi(0x7fff_ffff_7fff_ffff, 0x7fff_ffff_7fff_ffff)];
    for _ in 0..16 {
        masks.push(vi(r.next(), r.next()));
    }
    // 32-bit and 64-bit lane masks: every bit of a lane copies its sign, so blendv selects whole lanes
    let m32: Vec<__m128i> = masks.iter().map(|&m| _mm_srai_epi32::<31>(m)).collect();
    let m64: Vec<__m128i> = masks.iter().map(|&m| _mm_cmpgt_epi64(_mm_setzero_si128(), m)).collect();
    // VPGATHERDQ reads offsets from the low two 32-bit lanes, VPGATHERQD its mask from them
    let m32x: Vec<__m128i> = m64.iter().map(|&m| _mm_shuffle_epi32::<0b11_11_10_00>(m)).collect();
    let m64n: Vec<__m128i> = m32x.clone();
    let wild32 = vi(0x7fff_fff0_7fff_fff0, 0x7fff_fff0_7fff_fff0);
    let wild64 = vi(0x0fff_ffff_ffff_fff0, 0x0fff_ffff_ffff_fff0);
    gather!(out, _mm_i32gather_epi32, b32, o32, 1 2 4 8);
    gather!(out, _mm_i32gather_epi64, b64, o32, 1 2 4 8);
    gather!(out, _mm_i64gather_epi32, b32, o64, 1 2 4 8);
    gather!(out, _mm_i64gather_epi64, b64, o64, 1 2 4 8);
    mask_gather!(out, _mm_mask_i32gather_epi32, b32, o32, wild32, xs, m32, m32, 1 2 4 8);
    mask_gather!(out, _mm_mask_i32gather_epi64, b64, o32, wild32, xs, m32x, m64, 1 2 4 8);
    mask_gather!(out, _mm_mask_i64gather_epi32, b32, o64, wild64, xs, m64, m64n, 1 2 4 8);
    mask_gather!(out, _mm_mask_i64gather_epi64, b64, o64, wild64, xs, m64, m64, 1 2 4 8);
    // the same with raw masks: only the sign bit of a lane counts
    let mut h = FNV0;
    for (k, &m) in masks.iter().enumerate() {
        let o = o32[k % o32.len()];
        let q = o64[k % o64.len()];
        let src = xs[k % xs.len()];
        _mm_mask_i32gather_epi32::<4>(src, b32, o, m).fold(&mut h);
        _mm_mask_i32gather_epi64::<8>(src, b64, o, m).fold(&mut h);
        _mm_mask_i64gather_epi32::<4>(src, b32, q, m).fold(&mut h);
        _mm_mask_i64gather_epi64::<8>(src, b64, q, m).fold(&mut h);
    }
    out.push(("mask_gather_raw_masks", h));
    out
}
#[test]
fn gather() {
    if !is_x86_feature_detected!("sse4.2") || !is_x86_feature_detected!("avx2") {
        return;
    }
    check(unsafe { g_gather() }, &[
        ("_mm_i32gather_epi32", 0x2be1c862c956c4b5),
        ("_mm_i32gather_epi64", 0xfd91c4a9abf5db80),
        ("_mm_i64gather_epi32", 0x5fa3f419c9ebce6e),
        ("_mm_i64gather_epi64", 0x9d182491fd927574),
        ("_mm_mask_i32gather_epi32", 0x2dba9d5b9a2dce2f),
        ("_mm_mask_i32gather_epi64", 0x825f75cd30d3f417),
        ("_mm_mask_i64gather_epi32", 0x2d6ab049027ff710),
        ("_mm_mask_i64gather_epi64", 0x3fc973a2cab1a2b8),
        ("mask_gather_raw_masks", 0x96e154f8baf108d5),
    ]);
}

#[target_feature(enable = "sse4.2,avx2")]
unsafe fn g_memory() -> Vec<(&'static str, u64)> {
    let mut out = Vec::new();
    let xs = inputs();
    let mut r = Rng(0x082efa98ec4e6c89);
    let mut masks = vec![vi(0, 0), vi(!0, !0), vi(1 << 63, 1 << 31), vi(1 << 31, 1 << 63), vi(0x7fff_ffff_7fff_ffff, 0x7f7f_7f7f_7f7f_7f7f), vi(0x8080_8080_8080_8080, 0x0080_0080_0080_0080)];
    for _ in 0..16 {
        masks.push(vi(r.next(), r.next()));
    }

    let mut h = FNV0;
    let bytes: [u8; 24] = core::array::from_fn(|i| (i as u8).wrapping_mul(37).wrapping_add(11));
    for off in 0..16 {
        _mm_loadu_si64(bytes.as_ptr().add(off)).fold(&mut h);
    }
    out.push(("_mm_loadu_si64", h));

    let (mut h32, mut h64, mut hs32, mut hs64, mut hmm) = (FNV0, FNV0, FNV0, FNV0, FNV0);
    for (k, &m) in masks.iter().enumerate() {
        let a = xs[k % xs.len()];
        let d = xs[(k + 7) % xs.len()];
        let src32: [i32; 4] = std::mem::transmute(a);
        let src64: [i64; 2] = std::mem::transmute(a);
        _mm_maskload_epi32(src32.as_ptr(), m).fold(&mut h32);
        _mm_maskload_epi64(src64.as_ptr(), m).fold(&mut h64);
        let mut dst32: [i32; 4] = std::mem::transmute(d);
        _mm_maskstore_epi32(dst32.as_mut_ptr(), m, a);
        fnv(&mut hs32, &std::mem::transmute::<[i32; 4], [u8; 16]>(dst32));
        let mut dst64: [i64; 2] = std::mem::transmute(d);
        _mm_maskstore_epi64(dst64.as_mut_ptr(), m, a);
        fnv(&mut hs64, &std::mem::transmute::<[i64; 2], [u8; 16]>(dst64));
        let mut dst8 = [0x5ai8; 19];
        _mm_maskmoveu_si128(a, m, dst8.as_mut_ptr().add(k % 4));
        fnv(&mut hmm, &std::mem::transmute::<[i8; 19], [u8; 19]>(dst8));
    }
    out.push(("_mm_maskload_epi32", h32));
    out.push(("_mm_maskload_epi64", h64));
    out.push(("_mm_maskstore_epi32", hs32));
    out.push(("_mm_maskstore_epi64", hs64));
    out.push(("_mm_maskmoveu_si128", hmm));

    let (mut h128, mut hi32) = (FNV0, FNV0);
    for &a in xs.iter() {
        let mut dst = A16([vi(1, 2), vi(3, 4), vi(5, 6)]);
        _mm_stream_si128(dst.0.as_mut_ptr().add(1), a);
        let mut d32 = [7i32, 8, 9];
        _mm_stream_si32(d32.as_mut_ptr().add(1), _mm_cvtsi128_si32(a));
        for v in dst.0 {
            v.fold(&mut h128);
        }
        for v in d32 {
            v.fold(&mut hi32);
        }
    }
    out.push(("_mm_stream_si128", h128));
    out.push(("_mm_stream_si32", hi32));
    out
}
#[test]
fn memory() {
    if !is_x86_feature_detected!("sse4.2") || !is_x86_feature_detected!("avx2") {
        return;
    }
    check(unsafe { g_memory() }, &[
        ("_mm_loadu_si64", 0x0b47d3998e964825),
        ("_mm_maskload_epi32", 0x4134eccdc77667c8),
        ("_mm_maskload_epi64", 0x5b823728862dba36),
        ("_mm_maskstore_epi32", 0xaf96659885359632),
        ("_mm_maskstore_epi64", 0xa45a40c291fdd122),
        ("_mm_maskmoveu_si128", 0xbcfa18253d5aad9a),
        ("_mm_stream_si128", 0xf789719a0f8af899),
        ("_mm_stream_si32", 0x7eab91ba42a8b3dc),
    ]);
}

// every control byte: element size and signedness, aggregation, polarity, output selection
macro_rules! istr_one {
    ($out:ident, $strs:ident, $f:ident) => {{
        let mut h = FNV0;
        for &a in $strs.iter() {
            for &b in $strs.iter() {
                istr_one!(@ h, $f, a, b, 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32 33 34 35 36 37 38 39 40 41 42 43 44 45 46 47 48 49 50 51 52 53 54 55 56 57 58 59 60 61 62 63 64 65 66 67 68 69 70 71 72 73 74 75 76 77 78 79 80 81 82 83 84 85 86 87 88 89 90 91 92 93 94 95 96 97 98 99 100 101 102 103 104 105 106 107 108 109 110 111 112 113 114 115 116 117 118 119 120 121 122 123 124 125 126 127);
            }
        }
        $out.push((stringify!($f), h));
    }};
    (@ $h:ident, $f:ident, $a:ident, $b:ident, $($n:literal)*) => {
        $( $f::<$n>($a, $b).fold(&mut $h); )*
    };
}
macro_rules! estr_one {
    ($out:ident, $strs:ident, $lens:ident, $f:ident) => {{
        let mut h = FNV0;
        for (i, &a) in $strs.iter().enumerate() {
            for (j, &b) in $strs.iter().enumerate() {
                // every length pair on a few string pairs, a rotating one on the others
                for (k, &(la, lb)) in $lens.iter().enumerate() {
                    if i + j < 3 || (i * 7 + j * 3 + k) % 37 == 0 {
                        estr_one!(@ h, $f, a, la, b, lb, 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32 33 34 35 36 37 38 39 40 41 42 43 44 45 46 47 48 49 50 51 52 53 54 55 56 57 58 59 60 61 62 63 64 65 66 67 68 69 70 71 72 73 74 75 76 77 78 79 80 81 82 83 84 85 86 87 88 89 90 91 92 93 94 95 96 97 98 99 100 101 102 103 104 105 106 107 108 109 110 111 112 113 114 115 116 117 118 119 120 121 122 123 124 125 126 127);
                    }
                }
            }
        }
        $out.push((stringify!($f), h));
    }};
    (@ $h:ident, $f:ident, $a:ident, $la:ident, $b:ident, $lb:ident, $($n:literal)*) => {
        $( $f::<$n>($a, $la, $b, $lb).fold(&mut $h); )*
    };
}

fn bytes16(s: &[u8]) -> __m128i {
    let mut b = [0u8; 16];
    b[..s.len()].copy_from_slice(s);
    unsafe { std::mem::transmute(b) }
}
fn words8(s: &[u16]) -> __m128i {
    let mut b = [0u16; 8];
    b[..s.len()].copy_from_slice(s);
    unsafe { std::mem::transmute(b) }
}
fn strings() -> Vec<__m128i> {
    let mut v = vec![
        bytes16(b""),
        bytes16(b"a"),
        bytes16(b"az"),
        bytes16(b"azAZ09"),
        bytes16(b"abcabcabcabcabca"),
        bytes16(b"abc"),
        bytes16(b"bca"),
        bytes16(b"Hello, World 42!"),
        bytes16(b"hello\0world"),
        bytes16(b"\0abcdefgh"),
        bytes16(b"aaaaaaaaaaaaaaaa"),
        bytes16(b"aaaaaaaaaaaaaaa"),
        bytes16(b"aaaaaaab"),
        bytes16(&[0x80, 0xff, 0x7f, 0x01, 0xc3, 0xa9, 0x20, 0xfe, 0x81]),
        bytes16(&[0x01, 0x7f, 0x80, 0xff]),
        bytes16(&[0xff, 0x01, 0x90, 0xa0]),
        bytes16(&[0xc3, 0xa9, 0xc3, 0xa9, 0xc3, 0xa9, 0xc3, 0xa9, 0xc3, 0xa9, 0xc3, 0xa9, 0xc3, 0xa9, 0xc3, 0xa9]),
        // a null low byte inside a non-null word, and a null word
        words8(&[0x6100, 0x0061, 0x8000, 0xffff, 0x7fff]),
        words8(&[0x0061, 0x007a, 0x8000, 0xffff]),
        words8(&[0x0061, 0x0062, 0x0063, 0x0061, 0x0062, 0x0063, 0x0061, 0x0062]),
        words8(&[0x0061, 0x0062, 0x0063]),
        words8(&[0xff00, 0x00ff, 0x8001, 0x7ffe, 0x0101, 0x0061, 0x0062]),
    ];
    let mut r = Rng(0x452821e638d01377);
    for k in 0..8 {
        // small alphabets make matches likely; some strings stop early
        let mut b = [0u8; 16];
        let len = if k % 2 == 0 { 16 } else { (r.next() % 17) as usize };
        for x in b.iter_mut().take(len) {
            let c = r.next();
            *x = if k < 4 { b'a' + (c % 4) as u8 } else { [0x61, 0x62, 0x80, 0xff, 0x7f, 0x01][(c % 6) as usize] };
        }
        v.push(unsafe { std::mem::transmute(b) });
    }
    v
}
fn lengths() -> Vec<(i32, i32)> {
    let l = [0i32, 1, 2, 3, 7, 8, 9, 15, 16, 17, -1, -5, -8, -16, -20, 100, i32::MAX, i32::MIN];
    let mut v = Vec::new();
    for (i, &la) in l.iter().enumerate() {
        for (j, &lb) in l.iter().enumerate() {
            if i == j || (i + 2 * j) % 3 == 0 || la == 0 || lb == 16 {
                v.push((la, lb));
            }
        }
    }
    v
}

#[target_feature(enable = "sse4.2,avx2")]
unsafe fn g_cmpistr() -> Vec<(&'static str, u64)> {
    let strs = strings();
    let mut out = Vec::new();
    istr_one!(out, strs, _mm_cmpistri);
    istr_one!(out, strs, _mm_cmpistrm);
    istr_one!(out, strs, _mm_cmpistra);
    istr_one!(out, strs, _mm_cmpistrc);
    istr_one!(out, strs, _mm_cmpistro);
    istr_one!(out, strs, _mm_cmpistrs);
    istr_one!(out, strs, _mm_cmpistrz);
    out
}
#[test]
fn cmpistr() {
    if !is_x86_feature_detected!("sse4.2") || !is_x86_feature_detected!("avx2") {
        return;
    }
    check(unsafe { g_cmpistr() }, &[
        ("_mm_cmpistri", 0x16ca2f9f34e28dff),
        ("_mm_cmpistrm", 0xec0c44ed94de039d),
        ("_mm_cmpistra", 0x36bd2f4339e2f465),
        ("_mm_cmpistrc", 0xa02488ee88905225),
        ("_mm_cmpistro", 0x7db34b078a682c25),
        ("_mm_cmpistrs", 0x5105ef0c63d4ab25),
        ("_mm_cmpistrz", 0x6d5ec66dd6f4ab25),
    ]);
}

#[target_feature(enable = "sse4.2,avx2")]
unsafe fn g_cmpestr() -> Vec<(&'static str, u64)> {
    let strs = strings();
    let lens = lengths();
    let mut out = Vec::new();
    estr_one!(out, strs, lens, _mm_cmpestra);
    estr_one!(out, strs, lens, _mm_cmpestrc);
    estr_one!(out, strs, lens, _mm_cmpestro);
    estr_one!(out, strs, lens, _mm_cmpestrs);
    estr_one!(out, strs, lens, _mm_cmpestrz);
    out
}
#[test]
fn cmpestr() {
    if !is_x86_feature_detected!("sse4.2") || !is_x86_feature_detected!("avx2") {
        return;
    }
    check(unsafe { g_cmpestr() }, &[
        ("_mm_cmpestra", 0x1153dcf77ebb1925),
        ("_mm_cmpestrc", 0x59112e8c17367b45),
        ("_mm_cmpestro", 0xec0906828fdf9fa5),
        ("_mm_cmpestrs", 0x8c2e73ca5c62bf25),
        ("_mm_cmpestrz", 0x3d24dc75bed19725),
    ]);
}

// not in this family's list, but they share the core under test
#[target_feature(enable = "sse4.2,avx2")]
unsafe fn g_cmpestr_core() -> Vec<(&'static str, u64)> {
    let strs = strings();
    let lens = lengths();
    let mut out = Vec::new();
    estr_one!(out, strs, lens, _mm_cmpestri);
    estr_one!(out, strs, lens, _mm_cmpestrm);
    out
}
#[test]
fn cmpestr_core() {
    if !is_x86_feature_detected!("sse4.2") || !is_x86_feature_detected!("avx2") {
        return;
    }
    check(unsafe { g_cmpestr_core() }, &[
        ("_mm_cmpestri", 0x5ed2abb90cb9ca24),
        ("_mm_cmpestrm", 0x39b04c608609d339),
    ]);
}
