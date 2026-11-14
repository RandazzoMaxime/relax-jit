#![cfg(target_arch = "x86_64")]
#![allow(deprecated)]
// Differential checks of the AVX/AVX2 256-bit integer intrinsics: every checksum below was
// produced by rustc on real hardware, and Relax's std must reproduce it.
use std::arch::x86_64::*;
use std::hint::black_box;

struct Fnv(u64);
impl Fnv {
    fn new() -> Self {
        Fnv(0xcbf29ce484222325)
    }
    fn bytes(&mut self, b: &[u8]) {
        for &x in b {
            self.0 ^= x as u64;
            self.0 = self.0.wrapping_mul(0x100000001b3);
        }
    }
    fn m256(&mut self, v: __m256i) {
        let b: [u8; 32] = unsafe { std::mem::transmute(v) };
        self.bytes(&b);
    }
    fn m128(&mut self, v: __m128i) {
        let b: [u8; 16] = unsafe { std::mem::transmute(v) };
        self.bytes(&b);
    }
    fn i32(&mut self, v: i32) {
        self.bytes(&v.to_le_bytes());
    }
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

fn v256(q: [u64; 4]) -> __m256i {
    unsafe { std::mem::transmute(q) }
}
fn v128(q: [u64; 2]) -> __m128i {
    unsafe { std::mem::transmute(q) }
}

/// zeros, ones, lane patterns of 0 / -1 / MIN / MAX / near-boundary values at each lane width, random
fn inputs() -> Vec<__m256i> {
    let e8: [u8; 8] = [0, 0xff, 0x80, 0x7f, 1, 0x81, 0xfe, 0x40];
    let e16: [u16; 8] = [0, 0xffff, 0x8000, 0x7fff, 1, 0x8001, 0xfffe, 0x4000];
    let e32: [u32; 8] = [0, !0, 0x8000_0000, 0x7fff_ffff, 1, 0x8000_0001, 0xffff_fffe, 0x4000_0000];
    let e64: [u64; 8] = [0, !0, 1 << 63, !0 >> 1, 1, (1 << 63) + 1, !0 - 1, 1 << 62];
    let mut v = vec![v256([0; 4]), v256([!0; 4])];
    for rot in 0..3usize {
        let (mut b8, mut b16, mut b32) = ([0u8; 32], [0u16; 16], [0u32; 8]);
        for i in 0..32 {
            b8[i] = e8[(i * (2 * rot + 1) + rot) % 8];
        }
        for i in 0..16 {
            b16[i] = e16[(i * (2 * rot + 1) + rot) % 8];
        }
        for i in 0..8 {
            b32[i] = e32[(i * (2 * rot + 1) + rot) % 8];
        }
        v.push(unsafe { std::mem::transmute(b8) });
        v.push(unsafe { std::mem::transmute(b16) });
        v.push(unsafe { std::mem::transmute(b32) });
        v.push(v256([e64[rot], e64[(rot + 3) % 8], e64[(rot * 3 + 2) % 8], e64[7 - rot]]));
    }
    let mut r = Rng(0x9e3779b97f4a7c15);
    for _ in 0..8 {
        v.push(v256([r.next(), r.next(), r.next(), r.next()]));
    }
    v
}

fn counts() -> Vec<__m128i> {
    let mut v = Vec::new();
    for c in [0u64, 1, 7, 8, 15, 16, 17, 31, 32, 33, 63, 64, 65, 1 << 32, 1 << 63, !0] {
        v.push(v128([c, 0]));
        v.push(v128([c, 0xdead_beef_0123_4567]));
    }
    v
}

fn check(got: &[(&'static str, u64)], want: &[(&'static str, u64)]) {
    if got != want {
        for (n, c) in got {
            eprintln!("        (\"{n}\", {c:#018x}),");
        }
    }
    assert_eq!(got.len(), want.len());
    for (g, w) in got.iter().zip(want) {
        assert_eq!(g.0, w.0);
        assert_eq!(g.1, w.1, "{}", g.0);
    }
}

macro_rules! un {
    ($out:ident, $xs:ident, $f:ident) => {{
        let mut h = Fnv::new();
        for a in &$xs {
            h.m256($f(black_box(*a)));
        }
        $out.push((stringify!($f), h.0));
    }};
}
macro_rules! bin {
    ($out:ident, $xs:ident, $f:ident) => {{
        let mut h = Fnv::new();
        for a in &$xs {
            for b in &$xs {
                h.m256($f(black_box(*a), black_box(*b)));
            }
        }
        $out.push((stringify!($f), h.0));
    }};
}
macro_rules! bin_i32 {
    ($out:ident, $xs:ident, $f:ident) => {{
        let mut h = Fnv::new();
        for a in &$xs {
            for b in &$xs {
                h.i32($f(black_box(*a), black_box(*b)));
            }
        }
        $out.push((stringify!($f), h.0));
    }};
}
macro_rules! un_imm {
    ($out:ident, $xs:ident, $f:ident, [$($imm:literal),*]) => {{
        let mut h = Fnv::new();
        for a in &$xs {
            $( h.m256($f::<$imm>(black_box(*a))); )*
        }
        $out.push((stringify!($f), h.0));
    }};
}
macro_rules! bin_imm {
    ($out:ident, $xs:ident, $f:ident, [$($imm:literal),*]) => {{
        let mut h = Fnv::new();
        for a in &$xs {
            for b in &$xs {
                $( h.m256($f::<$imm>(black_box(*a), black_box(*b))); )*
            }
        }
        $out.push((stringify!($f), h.0));
    }};
}
macro_rules! from128 {
    ($out:ident, $xs:ident, $f:ident) => {{
        let mut h = Fnv::new();
        for a in &$xs {
            h.m256($f(black_box(_mm256_castsi256_si128(*a))));
            h.m256($f(black_box(_mm256_extracti128_si256::<1>(*a))));
        }
        $out.push((stringify!($f), h.0));
    }};
}
macro_rules! shift {
    ($out:ident, $xs:ident, $cs:ident, $f:ident) => {{
        let mut h = Fnv::new();
        for a in &$xs {
            for c in &$cs {
                h.m256($f(black_box(*a), black_box(*c)));
            }
        }
        $out.push((stringify!($f), h.0));
    }};
}

type Sums = Vec<(&'static str, u64)>;

#[target_feature(enable = "avx2")]
unsafe fn g_arith() -> Sums {
    let (xs, mut o) = (inputs(), Vec::new());
    un!(o, xs, _mm256_abs_epi8);
    un!(o, xs, _mm256_abs_epi16);
    un!(o, xs, _mm256_abs_epi32);
    bin!(o, xs, _mm256_adds_epi8);
    bin!(o, xs, _mm256_adds_epi16);
    bin!(o, xs, _mm256_adds_epu8);
    bin!(o, xs, _mm256_adds_epu16);
    bin!(o, xs, _mm256_subs_epi8);
    bin!(o, xs, _mm256_subs_epi16);
    bin!(o, xs, _mm256_subs_epu16);
    bin!(o, xs, _mm256_avg_epu8);
    bin!(o, xs, _mm256_avg_epu16);
    o
}
#[test]
fn arith() {
    if !is_x86_feature_detected!("avx2") {
        return;
    }
    check(&unsafe { g_arith() }, &[
        ("_mm256_abs_epi8", 0x05ae9aad7c5db8c4),
        ("_mm256_abs_epi16", 0x4c04ebd6e59842f8),
        ("_mm256_abs_epi32", 0x0ae2df1b0cfa4309),
        ("_mm256_adds_epi8", 0xb76912924abc3f63),
        ("_mm256_adds_epi16", 0x30163da5d45e2913),
        ("_mm256_adds_epu8", 0xed0064048e5c327b),
        ("_mm256_adds_epu16", 0x05d1dba0bc6078e7),
        ("_mm256_subs_epi8", 0xcf8412815de5e791),
        ("_mm256_subs_epi16", 0x79512542b483c102),
        ("_mm256_subs_epu16", 0x82150edd0c239807),
        ("_mm256_avg_epu8", 0x018607ed1fab56b6),
        ("_mm256_avg_epu16", 0x4d23e8fbff7dc836),
    ]);
}

#[target_feature(enable = "avx2")]
unsafe fn g_minmax_mul() -> Sums {
    let (xs, mut o) = (inputs(), Vec::new());
    bin!(o, xs, _mm256_max_epi8);
    bin!(o, xs, _mm256_max_epi32);
    bin!(o, xs, _mm256_min_epi8);
    bin!(o, xs, _mm256_min_epi32);
    bin!(o, xs, _mm256_mul_epi32);
    bin!(o, xs, _mm256_mulhi_epi16);
    bin!(o, xs, _mm256_mulhrs_epi16);
    bin!(o, xs, _mm256_sign_epi8);
    bin!(o, xs, _mm256_sign_epi32);
    bin!(o, xs, _mm256_cmpeq_epi64);
    bin!(o, xs, _mm256_cmpgt_epi64);
    o
}
#[test]
fn minmax_mul() {
    if !is_x86_feature_detected!("avx2") {
        return;
    }
    check(&unsafe { g_minmax_mul() }, &[
        ("_mm256_max_epi8", 0x39d732a1a1b080de),
        ("_mm256_max_epi32", 0xb83148cc3b8ddd6e),
        ("_mm256_min_epi8", 0x47f9668a4cae1ae6),
        ("_mm256_min_epi32", 0x20f8293c1baaf902),
        ("_mm256_mul_epi32", 0x219c24666ee2a5db),
        ("_mm256_mulhi_epi16", 0xb500957ea7d632f3),
        ("_mm256_mulhrs_epi16", 0x96c4e10bb6beac53),
        ("_mm256_sign_epi8", 0xbcfc7d760c81ea54),
        ("_mm256_sign_epi32", 0xb427f5bdbcd2ca31),
        ("_mm256_cmpeq_epi64", 0x8f93c6054b785535),
        ("_mm256_cmpgt_epi64", 0xd272e3f1ec2b65dd),
    ]);
}

#[target_feature(enable = "avx2")]
unsafe fn g_horizontal_pack() -> Sums {
    let (xs, mut o) = (inputs(), Vec::new());
    bin!(o, xs, _mm256_hadd_epi16);
    bin!(o, xs, _mm256_hadd_epi32);
    bin!(o, xs, _mm256_hadds_epi16);
    bin!(o, xs, _mm256_hsub_epi16);
    bin!(o, xs, _mm256_hsub_epi32);
    bin!(o, xs, _mm256_hsubs_epi16);
    bin!(o, xs, _mm256_packs_epi16);
    bin_imm!(o, xs, _mm256_mpsadbw_epu8, [0, 1, 6, 11, 21, 37, 42, 63, 255]);
    o
}
#[test]
fn horizontal_pack() {
    if !is_x86_feature_detected!("avx2") {
        return;
    }
    check(&unsafe { g_horizontal_pack() }, &[
        ("_mm256_hadd_epi16", 0x310f443b6d10f321),
        ("_mm256_hadd_epi32", 0xd7d93e3bee8ab929),
        ("_mm256_hadds_epi16", 0xa65c7175f73a6479),
        ("_mm256_hsub_epi16", 0x5373c1fc61e1eb51),
        ("_mm256_hsub_epi32", 0x3f762991c229ac6d),
        ("_mm256_hsubs_epi16", 0x340eebc576c17d5d),
        ("_mm256_packs_epi16", 0x873096344b12e851),
        ("_mm256_mpsadbw_epu8", 0x9789b0c01dbac7ca),
    ]);
}

#[target_feature(enable = "avx2")]
unsafe fn g_blend_shuffle() -> Sums {
    let (xs, mut o) = (inputs(), Vec::new());
    bin_imm!(o, xs, _mm256_blend_epi16, [0, 1, 0x5a, 0x80, 0xa5, 0xff]);
    bin_imm!(o, xs, _mm256_permute2f128_si256, [0x00, 0x01, 0x20, 0x31, 0x13, 0x02, 0x08, 0x80, 0x88, 0x3b, 0xf7, 0x44]);
    un_imm!(o, xs, _mm256_shufflehi_epi16, [0x00, 0x1b, 0xe4, 0x4e, 0xff, 0x93]);
    un_imm!(o, xs, _mm256_shufflelo_epi16, [0x00, 0x1b, 0xe4, 0x4e, 0xff, 0x93]);
    un_imm!(o, xs, _mm256_bslli_epi128, [0, 1, 3, 8, 15, 16, 17, 100, 255]);
    un_imm!(o, xs, _mm256_bsrli_epi128, [0, 1, 3, 8, 15, 16, 17, 100, 255]);
    un_imm!(o, xs, _mm256_slli_si256, [0, 1, 5, 9, 15, 16, 31, 128, 255]);
    un_imm!(o, xs, _mm256_srli_si256, [0, 1, 5, 9, 15, 16, 31, 128, 255]);
    o
}
#[test]
fn blend_shuffle() {
    if !is_x86_feature_detected!("avx2") {
        return;
    }
    check(&unsafe { g_blend_shuffle() }, &[
        ("_mm256_blend_epi16", 0xaa6813ba47fb657d),
        ("_mm256_permute2f128_si256", 0x30c964f647370f45),
        ("_mm256_shufflehi_epi16", 0x1b6f87afe4a872c9),
        ("_mm256_shufflelo_epi16", 0x5d8df207905ee099),
        ("_mm256_bslli_epi128", 0x1d0af38e42277319),
        ("_mm256_bsrli_epi128", 0x77e15613aceabec3),
        ("_mm256_slli_si256", 0x4da377802b1b2f09),
        ("_mm256_srli_si256", 0xde7c3f58322cc1b3),
    ]);
}

#[target_feature(enable = "avx2")]
unsafe fn g_extend() -> Sums {
    let (xs, mut o) = (inputs(), Vec::new());
    from128!(o, xs, _mm256_broadcastb_epi8);
    from128!(o, xs, _mm256_broadcastw_epi16);
    from128!(o, xs, _mm256_broadcastq_epi64);
    from128!(o, xs, _mm256_cvtepi8_epi16);
    from128!(o, xs, _mm256_cvtepi8_epi64);
    from128!(o, xs, _mm256_cvtepi16_epi32);
    from128!(o, xs, _mm256_cvtepi16_epi64);
    from128!(o, xs, _mm256_cvtepi32_epi64);
    from128!(o, xs, _mm256_cvtepu8_epi32);
    from128!(o, xs, _mm256_cvtepu8_epi64);
    from128!(o, xs, _mm256_cvtepu16_epi32);
    from128!(o, xs, _mm256_cvtepu16_epi64);
    from128!(o, xs, _mm256_cvtepu32_epi64);
    o
}
#[test]
fn extend() {
    if !is_x86_feature_detected!("avx2") {
        return;
    }
    check(&unsafe { g_extend() }, &[
        ("_mm256_broadcastb_epi8", 0x89e33ac46316e185),
        ("_mm256_broadcastw_epi16", 0x274461b39a030285),
        ("_mm256_broadcastq_epi64", 0x4ac2ced8f2330ea5),
        ("_mm256_cvtepi8_epi16", 0x3081c25c8d15438e),
        ("_mm256_cvtepi8_epi64", 0x635408a0a1155556),
        ("_mm256_cvtepi16_epi32", 0xeab786184461a210),
        ("_mm256_cvtepi16_epi64", 0x74de43ecadb48570),
        ("_mm256_cvtepi32_epi64", 0xf23a874fb0d24dda),
        ("_mm256_cvtepu8_epi32", 0x76d7d42f27685a8e),
        ("_mm256_cvtepu8_epi64", 0xfdd098e7e9ba95e6),
        ("_mm256_cvtepu16_epi32", 0x766aad5641e8699e),
        ("_mm256_cvtepu16_epi64", 0xba73ca4a11b65408),
        ("_mm256_cvtepu32_epi64", 0x671f5a0ba0e83146),
    ]);
}

#[target_feature(enable = "avx2")]
unsafe fn g_insert_extract() -> Sums {
    let (xs, mut o) = (inputs(), Vec::new());
    let mut h = Fnv::new();
    for a in &xs {
        let a = black_box(*a);
        h.i32(_mm256_extract_epi8::<0>(a));
        h.i32(_mm256_extract_epi8::<7>(a));
        h.i32(_mm256_extract_epi8::<15>(a));
        h.i32(_mm256_extract_epi8::<16>(a));
        h.i32(_mm256_extract_epi8::<22>(a));
        h.i32(_mm256_extract_epi8::<31>(a));
    }
    o.push(("_mm256_extract_epi8", h.0));
    let mut h = Fnv::new();
    for a in &xs {
        let a = black_box(*a);
        h.i32(_mm256_extract_epi16::<0>(a));
        h.i32(_mm256_extract_epi16::<3>(a));
        h.i32(_mm256_extract_epi16::<7>(a));
        h.i32(_mm256_extract_epi16::<8>(a));
        h.i32(_mm256_extract_epi16::<12>(a));
        h.i32(_mm256_extract_epi16::<15>(a));
    }
    o.push(("_mm256_extract_epi16", h.0));
    let mut h = Fnv::new();
    for a in &xs {
        for v in [0i8, -1, i8::MIN, i8::MAX, 0x5a] {
            let (a, v) = (black_box(*a), black_box(v));
            h.m256(_mm256_insert_epi8::<0>(a, v));
            h.m256(_mm256_insert_epi8::<9>(a, v));
            h.m256(_mm256_insert_epi8::<16>(a, v));
            h.m256(_mm256_insert_epi8::<31>(a, v));
        }
    }
    o.push(("_mm256_insert_epi8", h.0));
    let mut h = Fnv::new();
    for a in &xs {
        for v in [0i16, -1, i16::MIN, i16::MAX, 0x5aa5] {
            let (a, v) = (black_box(*a), black_box(v));
            h.m256(_mm256_insert_epi16::<0>(a, v));
            h.m256(_mm256_insert_epi16::<5>(a, v));
            h.m256(_mm256_insert_epi16::<8>(a, v));
            h.m256(_mm256_insert_epi16::<15>(a, v));
        }
    }
    o.push(("_mm256_insert_epi16", h.0));
    let mut h = Fnv::new();
    for a in &xs {
        for v in [0i32, -1, i32::MIN, i32::MAX, 0x1234_5678] {
            let (a, v) = (black_box(*a), black_box(v));
            h.m256(_mm256_insert_epi32::<0>(a, v));
            h.m256(_mm256_insert_epi32::<3>(a, v));
            h.m256(_mm256_insert_epi32::<4>(a, v));
            h.m256(_mm256_insert_epi32::<7>(a, v));
        }
    }
    o.push(("_mm256_insert_epi32", h.0));
    let mut h = Fnv::new();
    for a in &xs {
        h.i32(_mm256_cvtsi256_si32(black_box(*a)));
    }
    o.push(("_mm256_cvtsi256_si32", h.0));
    let mut h = Fnv::new();
    let mut r = Rng(0x2545f4914f6cdd1d);
    for _ in 0..8 {
        let mut e = [0i16; 16];
        for x in e.iter_mut() {
            *x = black_box(r.next() as i16);
        }
        h.m256(_mm256_setr_epi16(
            e[0], e[1], e[2], e[3], e[4], e[5], e[6], e[7], e[8], e[9], e[10], e[11], e[12], e[13], e[14], e[15],
        ));
    }
    o.push(("_mm256_setr_epi16", h.0));
    bin_i32!(o, xs, _mm256_testc_si256);
    bin_i32!(o, xs, _mm256_testnzc_si256);
    o
}
#[test]
fn insert_extract() {
    if !is_x86_feature_detected!("avx2") {
        return;
    }
    check(&unsafe { g_insert_extract() }, &[
        ("_mm256_extract_epi8", 0xfc30f1f1911a2c54),
        ("_mm256_extract_epi16", 0x53adf5201661c6dc),
        ("_mm256_insert_epi8", 0x60c088f2e0093df2),
        ("_mm256_insert_epi16", 0x75f268f2a78af32a),
        ("_mm256_insert_epi32", 0x59d7ec6430d83311),
        ("_mm256_cvtsi256_si32", 0xf66e81c7ab169740),
        ("_mm256_setr_epi16", 0x58fb5d7d347c6755),
        ("_mm256_testc_si256", 0x803c0f22c1bb8834),
        ("_mm256_testnzc_si256", 0xfb2761ae3b98d335),
    ]);
    // the value is unspecified: only that it can be produced
    let _ = unsafe { g_undefined() };
}
#[target_feature(enable = "avx")]
unsafe fn g_undefined() -> __m256i {
    _mm256_undefined_si256()
}

#[target_feature(enable = "avx2")]
unsafe fn g_shifts() -> Sums {
    let (xs, cs, mut o) = (inputs(), counts(), Vec::new());
    shift!(o, xs, cs, _mm256_sll_epi16);
    shift!(o, xs, cs, _mm256_sll_epi32);
    shift!(o, xs, cs, _mm256_sll_epi64);
    shift!(o, xs, cs, _mm256_sra_epi16);
    shift!(o, xs, cs, _mm256_sra_epi32);
    shift!(o, xs, cs, _mm256_srl_epi32);
    shift!(o, xs, cs, _mm256_srl_epi64);
    let c64 = [
        v256([0, 1, 63, 64]),
        v256([65, !0, 1 << 63, 31]),
        v256([32, 33, 1 << 32, 7]),
        v256([62, 100, 0, (1 << 32) + 1]),
    ];
    shift!(o, xs, c64, _mm256_sllv_epi64);
    let c32 = [
        v256([1 << 32, 31 | 32 << 32, 33 | 0xffff_ffff << 32, 0x8000_0000 | 7 << 32]),
        v256([15 | 16 << 32, 30 | 100 << 32, 0x7fff_ffff | 2 << 32, 8 | 24 << 32]),
    ];
    shift!(o, xs, c32, _mm256_srav_epi32);
    shift!(o, xs, xs, _mm256_srav_epi32);
    o
}
#[test]
fn shifts() {
    if !is_x86_feature_detected!("avx2") {
        return;
    }
    check(&unsafe { g_shifts() }, &[
        ("_mm256_sll_epi16", 0xa4b15162f1e75511),
        ("_mm256_sll_epi32", 0x6f87148346bd2605),
        ("_mm256_sll_epi64", 0x6c750cbef0a30bb5),
        ("_mm256_sra_epi16", 0x67187bd7dddb28c9),
        ("_mm256_sra_epi32", 0x728d30aaf25ec8e1),
        ("_mm256_srl_epi32", 0x99bb12a8f7817559),
        ("_mm256_srl_epi64", 0x6c6e33473e5d2465),
        ("_mm256_sllv_epi64", 0xd32b08d39e35817d),
        ("_mm256_srav_epi32", 0xe5d4c211172dfc86),
        ("_mm256_srav_epi32", 0xf522b9932de34e6e),
    ]);
}

#[repr(align(32))]
struct A32([u8; 96]);

#[target_feature(enable = "avx2")]
unsafe fn g_memory() -> Sums {
    let (xs, mut o) = (inputs(), Vec::new());
    let mut h = Fnv::new();
    for a in &xs {
        let mut m = A32([0x77; 96]);
        _mm256_store_si256(m.0.as_mut_ptr().add(32) as *mut __m256i, black_box(*a));
        h.bytes(&m.0);
    }
    o.push(("_mm256_store_si256", h.0));
    let mut h = Fnv::new();
    for a in &xs {
        let mut m = A32([0x77; 96]);
        _mm256_stream_si256(m.0.as_mut_ptr().add(32) as *mut __m256i, black_box(*a));
        h.bytes(&m.0);
    }
    o.push(("_mm256_stream_si256", h.0));
    let mut h = Fnv::new();
    for a in &xs {
        let mut m = [0x33u8; 80];
        _mm256_storeu2_m128i(m.as_mut_ptr().add(51) as *mut __m128i, m.as_mut_ptr().add(3) as *mut __m128i, black_box(*a));
        h.bytes(&m);
        h.m256(_mm256_loadu2_m128i(m.as_ptr().add(47) as *const __m128i, m.as_ptr().add(1) as *const __m128i));
    }
    o.push(("_mm256_storeu2_m128i", h.0));
    o.push(("_mm256_loadu2_m128i", h.0));
    let mut r = Rng(0xd1b54a32d192ed03);
    let mut d32 = [0i32; 8];
    let mut d64 = [0i64; 4];
    for x in d32.iter_mut() {
        *x = r.next() as i32;
    }
    for x in d64.iter_mut() {
        *x = r.next() as i64;
    }
    let mut h = Fnv::new();
    for m in &xs {
        h.m256(_mm256_maskload_epi32(d32.as_ptr(), black_box(*m)));
    }
    o.push(("_mm256_maskload_epi32", h.0));
    let mut h = Fnv::new();
    for m in &xs {
        h.m256(_mm256_maskload_epi64(d64.as_ptr(), black_box(*m)));
    }
    o.push(("_mm256_maskload_epi64", h.0));
    let mut h = Fnv::new();
    for m in &xs {
        let mut out = [0x1111_1111i32; 10];
        _mm256_maskstore_epi32(out.as_mut_ptr().add(1), black_box(*m), xs[xs.len() - 1]);
        for x in out {
            h.i32(x);
        }
    }
    o.push(("_mm256_maskstore_epi32", h.0));
    let mut h = Fnv::new();
    for m in &xs {
        let mut out = [0x1111_1111_1111_1111i64; 6];
        _mm256_maskstore_epi64(out.as_mut_ptr().add(1), black_box(*m), xs[xs.len() - 1]);
        for x in out {
            h.bytes(&x.to_le_bytes());
        }
    }
    o.push(("_mm256_maskstore_epi64", h.0));
    // a masked-off lane is not accessed: the last lanes would lie past the end of the buffer
    let one = [42i32];
    let only0 = _mm256_setr_epi32(-1, 0, 0, 0, 0, 0, 0, 0);
    let mut h = Fnv::new();
    h.m256(_mm256_maskload_epi32(one.as_ptr(), black_box(only0)));
    o.push(("maskload_no_access", h.0));
    o
}
#[test]
fn memory() {
    if !is_x86_feature_detected!("avx2") {
        return;
    }
    check(&unsafe { g_memory() }, &[
        ("_mm256_store_si256", 0x3dd06fd267697576),
        ("_mm256_stream_si256", 0x3dd06fd267697576),
        ("_mm256_storeu2_m128i", 0x941d3e2ecea70733),
        ("_mm256_loadu2_m128i", 0x941d3e2ecea70733),
        ("_mm256_maskload_epi32", 0x0d5aee088b1711d4),
        ("_mm256_maskload_epi64", 0x76818785d0667d2d),
        ("_mm256_maskstore_epi32", 0x7f6fe212d92bc0b1),
        ("_mm256_maskstore_epi64", 0x55ac1d827a98bef1),
        ("maskload_no_access", 0x96e2b0a64c616a8f),
    ]);
}

#[target_feature(enable = "avx2")]
unsafe fn g_gather() -> Sums {
    let (xs, mut o) = (inputs(), Vec::new());
    let mut r = Rng(0x94d049bb133111eb);
    let mut d32 = [0i32; 64];
    let mut d64 = [0i64; 64];
    for x in d32.iter_mut() {
        *x = r.next() as i32;
    }
    for x in d64.iter_mut() {
        *x = r.next() as i64;
    }
    // offsets around the middle of the tables, negative ones included
    let (p32, p64) = (d32.as_ptr().add(32), d64.as_ptr().add(32));
    let mut o32 = Vec::new();
    let mut o64 = Vec::new();
    for _ in 0..6 {
        let mut a = [0i32; 8];
        let mut b = [0i64; 4];
        for x in a.iter_mut() {
            *x = (r.next() % 31) as i32 - 15;
        }
        for x in b.iter_mut() {
            *x = (r.next() % 31) as i64 - 15;
        }
        o32.push(std::mem::transmute::<[i32; 8], __m256i>(a));
        o64.push(std::mem::transmute::<[i64; 4], __m256i>(b));
    }
    let src = xs[xs.len() - 2];
    let (mut h1, mut h2, mut h3, mut h4) = (Fnv::new(), Fnv::new(), Fnv::new(), Fnv::new());
    let (mut m1, mut m2, mut m3, mut m4) = (Fnv::new(), Fnv::new(), Fnv::new(), Fnv::new());
    for k in 0..6 {
        let (a, b) = (black_box(o32[k]), black_box(o64[k]));
        let a128 = _mm256_castsi256_si128(a);
        h1.m256(_mm256_i32gather_epi32::<4>(p32, a));
        h1.m256(_mm256_i32gather_epi32::<8>(p32, a));
        h1.m256(_mm256_i32gather_epi32::<2>(p32, a));
        h1.m256(_mm256_i32gather_epi32::<1>(p32, a));
        h2.m256(_mm256_i32gather_epi64::<8>(p64, a128));
        h2.m256(_mm256_i32gather_epi64::<4>(p64, a128));
        h2.m256(_mm256_i32gather_epi64::<1>(p64, a128));
        h3.m128(_mm256_i64gather_epi32::<4>(p32, b));
        h3.m128(_mm256_i64gather_epi32::<8>(p32, b));
        h3.m128(_mm256_i64gather_epi32::<1>(p32, b));
        h4.m256(_mm256_i64gather_epi64::<8>(p64, b));
        h4.m256(_mm256_i64gather_epi64::<2>(p64, b));
        for m in &xs {
            let m = black_box(*m);
            let m128 = _mm256_castsi256_si128(m);
            let s128 = _mm256_castsi256_si128(src);
            m1.m256(_mm256_mask_i32gather_epi32::<4>(src, p32, a, m));
            m1.m256(_mm256_mask_i32gather_epi32::<1>(src, p32, a, m));
            m2.m256(_mm256_mask_i32gather_epi64::<8>(src, p64, a128, m));
            m2.m256(_mm256_mask_i32gather_epi64::<2>(src, p64, a128, m));
            m3.m128(_mm256_mask_i64gather_epi32::<4>(s128, p32, b, m128));
            m3.m128(_mm256_mask_i64gather_epi32::<2>(s128, p32, b, m128));
            m4.m256(_mm256_mask_i64gather_epi64::<8>(src, p64, b, m));
            m4.m256(_mm256_mask_i64gather_epi64::<1>(src, p64, b, m));
        }
    }
    o.push(("_mm256_i32gather_epi32", h1.0));
    o.push(("_mm256_i32gather_epi64", h2.0));
    o.push(("_mm256_i64gather_epi32", h3.0));
    o.push(("_mm256_i64gather_epi64", h4.0));
    o.push(("_mm256_mask_i32gather_epi32", m1.0));
    o.push(("_mm256_mask_i32gather_epi64", m2.0));
    o.push(("_mm256_mask_i64gather_epi32", m3.0));
    o.push(("_mm256_mask_i64gather_epi64", m4.0));
    o
}
#[test]
fn gather() {
    if !is_x86_feature_detected!("avx2") {
        return;
    }
    check(&unsafe { g_gather() }, &[
        ("_mm256_i32gather_epi32", 0x14a058a61b953822),
        ("_mm256_i32gather_epi64", 0x0b32b54a43bded37),
        ("_mm256_i64gather_epi32", 0x333f778ebd7e0244),
        ("_mm256_i64gather_epi64", 0x6bf3e2e737357bcf),
        ("_mm256_mask_i32gather_epi32", 0x4b5ed467eb9c42bb),
        ("_mm256_mask_i32gather_epi64", 0x88be694a14282861),
        ("_mm256_mask_i64gather_epi32", 0x99a2a70d3b24feb3),
        ("_mm256_mask_i64gather_epi64", 0x63a9d187d300a015),
    ]);
}

// out of line so that the conversion stays between the two MXCSR writes
#[inline(never)]
#[target_feature(enable = "avx")]
unsafe fn cvtps_opaque(v: __m256) -> __m256i {
    _mm256_cvtps_epi32(black_box(v))
}

#[target_feature(enable = "avx")]
unsafe fn g_cvtps() -> Sums {
    let mut o = Vec::new();
    let mut vals = vec![
        0.0f32, -0.0, 0.5, -0.5, 1.5, -1.5, 2.5, -2.5, 0.49999997, -0.49999997, 1.0000001, 8388607.5, -8388607.5,
        2147483520.0, 2147483648.0, -2147483648.0, -2147483904.0, 1e10, -1e10, f32::INFINITY, f32::NEG_INFINITY,
        f32::NAN, -f32::NAN, f32::MIN_POSITIVE, 1e-45, -1e-45, f32::MAX, f32::MIN, 3.4999998, -7.5000005, 1e-30, 123456.79,
    ];
    let mut r = Rng(0xbf58476d1ce4e5b9);
    for _ in 0..32 {
        vals.push((r.next() as i32 as f32) / 1024.0);
    }
    let saved = _mm_getcsr();
    // nearest-even, down, up, toward zero (MXCSR bits 13..15)
    for (name, rc) in [("_mm256_cvtps_epi32", 0u32), ("cvtps_down", 1), ("cvtps_up", 2), ("cvtps_trunc", 3)] {
        let mut h = Fnv::new();
        for c in vals.chunks(8) {
            let v = black_box(_mm256_loadu_ps(c.as_ptr()));
            _mm_setcsr((saved & !0x6000) | (rc << 13));
            let x = cvtps_opaque(v);
            _mm_setcsr(saved);
            h.m256(x);
        }
        o.push((name, h.0));
    }
    o
}
#[test]
fn cvtps() {
    if !is_x86_feature_detected!("avx") {
        return;
    }
    check(&unsafe { g_cvtps() }, &[
        ("_mm256_cvtps_epi32", 0xad697fb9c9fd938e),
        ("cvtps_down", 0x7669dfea98ae8b01),
        ("cvtps_up", 0xf92d3cdd58e056ab),
        ("cvtps_trunc", 0x88ed366428295781),
    ]);
}
