#![cfg(target_arch = "x86_64")]
#![allow(non_snake_case)]
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

const EDGE: [u32; 16] = [
    0x0000_0000, 0x8000_0000, 0x3f80_0000, 0xbf80_0000, 0x7f80_0000, 0xff80_0000, 0x7fc0_0000, 0xffc0_0000,
    0x7fa0_1234, 0x0000_0001, 0x807f_ffff, 0x7f7f_ffff, 0xff7f_ffff, 0x0080_0000, 0x4049_0fdb, 0xc2f6_e979,
];

fn f32s(r: &mut Rng) -> f32 {
    let x = r.next();
    match x & 7 {
        0 | 1 => f32::from_bits(EDGE[((x >> 8) & 15) as usize]),
        2 => f32::from_bits((x >> 32) as u32),
        _ => ((x >> 40) as i32 - (1 << 23)) as f32 / 4096.0,
    }
}
fn f64s(r: &mut Rng) -> f64 {
    let x = r.next();
    match x & 7 {
        0 => f32::from_bits(EDGE[((x >> 8) & 15) as usize]) as f64,
        1 => f64::from_bits(r.next()),
        _ => ((x >> 20) as i64 - (1 << 43)) as f64 / 65536.0,
    }
}
fn v8(r: &mut Rng) -> [f32; 8] {
    let mut a = [0f32; 8];
    for x in a.iter_mut() {
        *x = f32s(r);
    }
    a
}
fn v4(r: &mut Rng) -> [f32; 4] {
    let mut a = [0f32; 4];
    for x in a.iter_mut() {
        *x = f32s(r);
    }
    a
}
fn d2(r: &mut Rng) -> [f64; 2] {
    [f64s(r), f64s(r)]
}
fn i8x(r: &mut Rng) -> [i32; 8] {
    let mut a = [0i32; 8];
    for x in a.iter_mut() {
        *x = r.next() as i32;
    }
    a
}

fn fnv(h: &mut u64, bytes: &[u8]) {
    for &b in bytes {
        *h ^= b as u64;
        *h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
}
const H0: u64 = 0xcbf2_9ce4_8422_2325;

#[target_feature(enable = "avx")]
unsafe fn ld(a: &[f32; 8]) -> __m256 {
    _mm256_loadu_ps(a.as_ptr())
}
#[target_feature(enable = "avx")]
unsafe fn ldi(a: &[i32; 8]) -> __m256i {
    _mm256_loadu_si256(a.as_ptr() as *const __m256i)
}
#[target_feature(enable = "avx")]
unsafe fn h256(h: &mut u64, v: __m256) {
    let mut o = [0u8; 32];
    _mm256_storeu_ps(o.as_mut_ptr() as *mut f32, v);
    fnv(h, &o);
}
#[target_feature(enable = "avx")]
unsafe fn h256d(h: &mut u64, v: __m256d) {
    let mut o = [0u8; 32];
    _mm256_storeu_pd(o.as_mut_ptr() as *mut f64, v);
    fnv(h, &o);
}
#[target_feature(enable = "sse2")]
unsafe fn h128(h: &mut u64, v: __m128) {
    let mut o = [0u8; 16];
    _mm_storeu_ps(o.as_mut_ptr() as *mut f32, v);
    fnv(h, &o);
}
fn hf(h: &mut u64, a: &[f32]) {
    for x in a {
        fnv(h, &x.to_bits().to_le_bytes());
    }
}
fn hd(h: &mut u64, a: &[f64]) {
    for x in a {
        fnv(h, &x.to_bits().to_le_bytes());
    }
}

fn check(table: &[(&str, u64, u64)]) {
    for &(name, got, want) in table {
        if std::env::var_os("X86_MISSING_PRINT").is_some() {
            println!("CK {name} {got:#018x}");
        } else {
            assert_eq!(got, want, "{name}");
        }
    }
}

const N: usize = 48;

// ---- arithmetic ----
#[target_feature(enable = "avx,fma")]
unsafe fn ck_arith(which: u32) -> u64 {
    let mut r = Rng(0x9e37_79b9_7f4a_7c15 ^ which as u64);
    let mut h = H0;
    for _ in 0..N {
        let (aa, mut ba) = (v8(&mut r), v8(&mut r));
        if which == 5 {
            // dpps is commutable for the compiler, so which NaN a NaN * NaN product keeps is not fixed
            for i in 0..8 {
                if aa[i].is_nan() && ba[i].is_nan() {
                    ba[i] = -2.5;
                }
            }
        }
        let (a, b, c) = (ld(&aa), ld(&ba), ld(&v8(&mut r)));
        match which {
            0 => h256(&mut h, _mm256_addsub_ps(a, b)),
            1 => h256(&mut h, _mm256_hadd_ps(a, b)),
            2 => h256(&mut h, _mm256_hsub_ps(a, b)),
            3 => h256(&mut h, _mm256_fmaddsub_ps(a, b, c)),
            4 => h256(&mut h, _mm256_fmsubadd_ps(a, b, c)),
            _ => {
                h256(&mut h, _mm256_dp_ps::<0x00>(a, b));
                h256(&mut h, _mm256_dp_ps::<0xff>(a, b));
                h256(&mut h, _mm256_dp_ps::<0xf1>(a, b));
                h256(&mut h, _mm256_dp_ps::<0x7f>(a, b));
                h256(&mut h, _mm256_dp_ps::<0x35>(a, b));
                h256(&mut h, _mm256_dp_ps::<0xa8>(a, b));
                h256(&mut h, _mm256_dp_ps::<0x1e>(a, b));
            }
        }
    }
    h
}
#[test]
fn a4_arith() {
    if !(is_x86_feature_detected!("avx") && is_x86_feature_detected!("fma")) {
        return;
    }
    unsafe {
        check(&[
            ("_mm256_addsub_ps", ck_arith(0), 0x13f983f87a99459d),
            ("_mm256_hadd_ps", ck_arith(1), 0xe9034b8b8bbd714a),
            ("_mm256_hsub_ps", ck_arith(2), 0x019da510312aadb2),
            ("_mm256_fmaddsub_ps", ck_arith(3), 0x57335069c1e4b746),
            ("_mm256_fmsubadd_ps", ck_arith(4), 0x261f6f3986c4bb99),
            ("_mm256_dp_ps", ck_arith(5), 0x86a2d45152129985),
        ]);
    }
}

// ---- shuffles ----
#[target_feature(enable = "avx")]
unsafe fn ck_shuffle(which: u32) -> u64 {
    let mut r = Rng(0x1234_5678_9abc_def1 ^ which as u64);
    let mut h = H0;
    for _ in 0..N {
        let (a, b) = (ld(&v8(&mut r)), ld(&v8(&mut r)));
        let x = _mm_loadu_ps(v4(&mut r).as_ptr());
        let k = ldi(&i8x(&mut r));
        match which {
            0 => {
                h256(&mut h, _mm256_blend_ps::<0x00>(a, b));
                h256(&mut h, _mm256_blend_ps::<0xff>(a, b));
                h256(&mut h, _mm256_blend_ps::<0xaa>(a, b));
                h256(&mut h, _mm256_blend_ps::<0x35>(a, b));
                h256(&mut h, _mm256_blend_ps::<0x81>(a, b));
            }
            1 => {
                h256(&mut h, _mm256_permute_ps::<0x00>(a));
                h256(&mut h, _mm256_permute_ps::<0xff>(a));
                h256(&mut h, _mm256_permute_ps::<0x1b>(a));
                h256(&mut h, _mm256_permute_ps::<0xe4>(a));
                h256(&mut h, _mm256_permute_ps::<0x6c>(a));
            }
            2 => h256(&mut h, _mm256_permutevar_ps(a, k)),
            3 => h256(&mut h, _mm256_movehdup_ps(a)),
            4 => h256(&mut h, _mm256_moveldup_ps(a)),
            5 => {
                h256(&mut h, _mm256_insertf128_ps::<0>(a, x));
                h256(&mut h, _mm256_insertf128_ps::<1>(a, x));
            }
            _ => {
                let (mut r0, mut r1) = (_mm256_castps256_ps128(a), _mm256_extractf128_ps::<1>(a));
                let (mut r2, mut r3) = (_mm256_castps256_ps128(b), x);
                _MM_TRANSPOSE4_PS(&mut r0, &mut r1, &mut r2, &mut r3);
                h128(&mut h, r0);
                h128(&mut h, r1);
                h128(&mut h, r2);
                h128(&mut h, r3);
            }
        }
    }
    h
}
#[test]
fn a4_shuffle() {
    if !is_x86_feature_detected!("avx") {
        return;
    }
    unsafe {
        check(&[
            ("_mm256_blend_ps", ck_shuffle(0), 0x553cdfbb48b9bcd8),
            ("_mm256_permute_ps", ck_shuffle(1), 0xcf796048192232c7),
            ("_mm256_permutevar_ps", ck_shuffle(2), 0xd83682d6032428bc),
            ("_mm256_movehdup_ps", ck_shuffle(3), 0x76d6254aa8c70dd9),
            ("_mm256_moveldup_ps", ck_shuffle(4), 0x3f8452b85a0ccb91),
            ("_mm256_insertf128_ps", ck_shuffle(5), 0xb077a65ce9b02dfa),
            ("_MM_TRANSPOSE4_PS", ck_shuffle(6), 0x241b095ea3a2346c),
        ]);
    }
}

// ---- broadcasts, casts, sets ----
#[target_feature(enable = "avx,avx2")]
unsafe fn ck_set(which: u32) -> u64 {
    let mut r = Rng(0x0bad_cafe_f00d_1235 ^ which as u64);
    let mut h = H0;
    for _ in 0..N {
        let a = ld(&v8(&mut r));
        let (xa, ya) = (v4(&mut r), v4(&mut r));
        let (x, y) = (_mm_loadu_ps(xa.as_ptr()), _mm_loadu_ps(ya.as_ptr()));
        let (p, q) = (_mm_loadu_pd(d2(&mut r).as_ptr()), _mm_loadu_pd(d2(&mut r).as_ptr()));
        match which {
            0 => h256(&mut h, _mm256_broadcast_ps(&x)),
            1 => h256(&mut h, _mm256_broadcast_ss(&xa[1])),
            2 => h256(&mut h, _mm256_broadcastss_ps(x)),
            // the upper half of the cast is unspecified: only the lower half is checked
            3 => h128(&mut h, _mm256_castps256_ps128(_mm256_castps128_ps256(x))),
            4 => h256(&mut h, _mm256_zextps128_ps256(x)),
            5 => hd(&mut h, &[_mm256_cvtsd_f64(_mm256_set_m128d(q, p))]),
            6 => hf(&mut h, &[_mm256_cvtss_f32(a)]),
            7 => h256(&mut h, _mm256_set_m128(x, y)),
            8 => h256d(&mut h, _mm256_set_m128d(p, q)),
            9 => h256d(&mut h, _mm256_setr_m128d(p, q)),
            _ => h256(&mut h, _mm256_undefined_ps()),
        }
    }
    h
}
#[test]
fn a4_set() {
    if !(is_x86_feature_detected!("avx") && is_x86_feature_detected!("avx2")) {
        return;
    }
    unsafe {
        check(&[
            ("_mm256_broadcast_ps", ck_set(0), 0xed203bd4f5f4aeed),
            ("_mm256_broadcast_ss", ck_set(1), 0x2b74d78191be6415),
            ("_mm256_broadcastss_ps", ck_set(2), 0xcec21a0e27c11b75),
            ("_mm256_castps128_ps256", ck_set(3), 0xca7fced26da9ebf8),
            ("_mm256_zextps128_ps256", ck_set(4), 0x1880be143c572f21),
            ("_mm256_cvtsd_f64", ck_set(5), 0xbf3ec15c48f9a438),
            ("_mm256_cvtss_f32", ck_set(6), 0x751da2ae5265a0c6),
            ("_mm256_set_m128", ck_set(7), 0x5e30be0a1728b579),
            ("_mm256_set_m128d", ck_set(8), 0x12db23e3024bf3ae),
            ("_mm256_setr_m128d", ck_set(9), 0x9c74265bb50b0f80),
            ("_mm256_undefined_ps", ck_set(10), 0x6ab05ef9aa8b9b25),
        ]);
    }
}

// ---- memory ----
#[repr(align(32))]
struct A32([f32; 8]);

#[target_feature(enable = "avx")]
unsafe fn ck_mem(which: u32) -> u64 {
    let mut r = Rng(0x7777_1111_3333_5559 ^ which as u64);
    let mut h = H0;
    for _ in 0..N {
        let av = v8(&mut r);
        let a = ld(&av);
        let (lo, hi) = (d2(&mut r), d2(&mut r));
        let mut m = i8x(&mut r);
        if r.next() & 3 == 0 {
            m = [-1, 0, i32::MIN, i32::MAX, 1, -2, 0x4000_0000, -0x4000_0000];
        }
        match which {
            0 => h256d(&mut h, _mm256_loadu2_m128d(hi.as_ptr(), lo.as_ptr())),
            1 => {
                let (mut o0, mut o1) = ([7f32; 5], [9f32; 5]);
                _mm256_storeu2_m128(o1.as_mut_ptr(), o0.as_mut_ptr().add(1), a);
                hf(&mut h, &o0);
                hf(&mut h, &o1);
            }
            2 => {
                let (mut o0, mut o1) = ([7f64; 3], [9f64; 3]);
                _mm256_storeu2_m128d(o1.as_mut_ptr().add(1), o0.as_mut_ptr(), _mm256_loadu2_m128d(hi.as_ptr(), lo.as_ptr()));
                hd(&mut h, &o0);
                hd(&mut h, &o1);
            }
            3 => h256(&mut h, _mm256_maskload_ps(av.as_ptr(), ldi(&m))),
            4 => {
                let mut o = [5f32; 8];
                _mm256_maskstore_ps(o.as_mut_ptr(), ldi(&m), a);
                hf(&mut h, &o);
            }
            _ => {
                let mut o = A32([5f32; 8]);
                _mm256_stream_ps(o.0.as_mut_ptr(), a);
                hf(&mut h, &o.0);
            }
        }
    }
    // masked-off lanes are not accessed: only lane 0 lies inside the buffer
    if which == 3 || which == 4 {
        let mut one = [3.5f32];
        let m = _mm256_setr_epi32(-1, 0, 0, 0, 0, 0, 0, 0);
        if which == 3 {
            h256(&mut h, _mm256_maskload_ps(one.as_ptr(), m));
        } else {
            _mm256_maskstore_ps(one.as_mut_ptr(), m, _mm256_set1_ps(-8.25));
            hf(&mut h, &one);
        }
    }
    h
}
#[test]
fn a4_mem() {
    if !is_x86_feature_detected!("avx") {
        return;
    }
    unsafe {
        check(&[
            ("_mm256_loadu2_m128d", ck_mem(0), 0xb149cac0d69e5814),
            ("_mm256_storeu2_m128", ck_mem(1), 0x5f1148afebfc8ddf),
            ("_mm256_storeu2_m128d", ck_mem(2), 0x402fec95229e8763),
            ("_mm256_maskload_ps", ck_mem(3), 0xefbd757e4dc62f0f),
            ("_mm256_maskstore_ps", ck_mem(4), 0x2dd7f4cd6be3d39b),
            ("_mm256_stream_ps", ck_mem(5), 0xfeb311ff8ede66a5),
        ]);
    }
}

// ---- gathers ----
#[target_feature(enable = "avx,avx2")]
unsafe fn ck_gather(which: u32) -> u64 {
    let mut r = Rng(0x2468_ace0_1357_9bdf ^ which as u64);
    let mut h = H0;
    let mut table = [0f32; 80];
    for x in table.iter_mut() {
        *x = f32s(&mut r);
    }
    // base in the middle, so offsets are negative too; byte offsets stay within [-128, 124]
    let base = table.as_ptr().add(32);
    for _ in 0..N {
        let mut o1 = [0i32; 8];
        let mut q1 = [0i64; 4];
        for x in o1.iter_mut() {
            *x = (r.next() % 253) as i32 - 128;
        }
        for x in q1.iter_mut() {
            *x = (r.next() % 253) as i64 - 128;
        }
        let div = |o: &[i32; 8], s: i32| ldi(&o.map(|x| x.div_euclid(s)));
        let divq = |o: &[i64; 4], s: i64| _mm256_loadu_si256(o.map(|x| x.div_euclid(s)).as_ptr() as *const __m256i);
        let src = ld(&v8(&mut r));
        let mut mk = v8(&mut r);
        mk[(r.next() & 7) as usize] = -1.0;
        mk[(r.next() & 7) as usize] = 1.0;
        let mask = ld(&mk);
        // masked-off lanes get an offset far outside the table: they are not read
        let mut wild = o1;
        let mut wildq = q1;
        for i in 0..8 {
            if !mk[i].is_sign_negative() {
                wild[i] = i32::MAX - 7;
            }
        }
        for i in 0..4 {
            if !mk[i].is_sign_negative() {
                wildq[i] = i64::MAX / 16;
            }
        }
        let (src4, mask4) = (_mm256_castps256_ps128(src), _mm256_castps256_ps128(mask));
        match which {
            0 => {
                h256(&mut h, _mm256_i32gather_ps::<1>(base, ldi(&o1)));
                h256(&mut h, _mm256_i32gather_ps::<2>(base, div(&o1, 2)));
                h256(&mut h, _mm256_i32gather_ps::<4>(base, div(&o1, 4)));
                h256(&mut h, _mm256_i32gather_ps::<8>(base, div(&o1, 8)));
            }
            1 => {
                h128(&mut h, _mm256_i64gather_ps::<1>(base, divq(&q1, 1)));
                h128(&mut h, _mm256_i64gather_ps::<2>(base, divq(&q1, 2)));
                h128(&mut h, _mm256_i64gather_ps::<4>(base, divq(&q1, 4)));
                h128(&mut h, _mm256_i64gather_ps::<8>(base, divq(&q1, 8)));
            }
            2 => {
                h256(&mut h, _mm256_mask_i32gather_ps::<1>(src, base, ldi(&wild), mask));
                h256(&mut h, _mm256_mask_i32gather_ps::<2>(src, base, div(&o1, 2), mask));
                h256(&mut h, _mm256_mask_i32gather_ps::<4>(src, base, div(&o1, 4), mask));
                h256(&mut h, _mm256_mask_i32gather_ps::<8>(src, base, div(&o1, 8), mask));
            }
            _ => {
                h128(&mut h, _mm256_mask_i64gather_ps::<1>(src4, base, divq(&wildq, 1), mask4));
                h128(&mut h, _mm256_mask_i64gather_ps::<2>(src4, base, divq(&q1, 2), mask4));
                h128(&mut h, _mm256_mask_i64gather_ps::<4>(src4, base, divq(&q1, 4), mask4));
                h128(&mut h, _mm256_mask_i64gather_ps::<8>(src4, base, divq(&q1, 8), mask4));
            }
        }
    }
    h
}
#[test]
fn a4_gather() {
    if !(is_x86_feature_detected!("avx") && is_x86_feature_detected!("avx2")) {
        return;
    }
    unsafe {
        check(&[
            ("_mm256_i32gather_ps", ck_gather(0), 0x8faf03102c5bdf13),
            ("_mm256_i64gather_ps", ck_gather(1), 0x4c98a61d4c49a2cf),
            ("_mm256_mask_i32gather_ps", ck_gather(2), 0x3e25546df3341ca0),
            ("_mm256_mask_i64gather_ps", ck_gather(3), 0xc5ed987fca2225b6),
        ]);
    }
}

// ---- sign-bit tests ----
#[target_feature(enable = "avx")]
unsafe fn ck_test(which: u32) -> u64 {
    let mut r = Rng(0x5151_aeae_0f0f_c3c3 ^ which as u64);
    let mut h = H0;
    for n in 0..4 * N {
        let (mut a, mut b) = (v8(&mut r), v8(&mut r));
        // steer towards the three outcomes: few sign bits set, b a subset of a, disjoint signs
        let bits = r.next();
        for i in 0..8 {
            let (sa, sb) = ((bits >> i) & 1 != 0, (bits >> (8 + i)) & 1 != 0);
            match n % 4 {
                0 => {}
                1 => {
                    a[i] = f32::from_bits(a[i].to_bits() & 0x7fff_ffff | ((sa as u32) << 31));
                    b[i] = f32::from_bits(b[i].to_bits() & 0x7fff_ffff | (((sa && sb) as u32) << 31));
                }
                2 => {
                    a[i] = f32::from_bits(a[i].to_bits() & 0x7fff_ffff | ((sa as u32) << 31));
                    b[i] = f32::from_bits(b[i].to_bits() & 0x7fff_ffff | (((!sa && sb) as u32) << 31));
                }
                _ => {
                    a[i] = f32::from_bits(a[i].to_bits() & 0x7fff_ffff | (((sa && sb) as u32) << 31));
                    b[i] = f32::from_bits(b[i].to_bits() & 0x7fff_ffff);
                }
            }
        }
        let (a, b) = (ld(&a), ld(&b));
        let v = match which {
            0 => _mm256_testc_ps(a, b),
            1 => _mm256_testz_ps(a, b),
            _ => _mm256_testnzc_ps(a, b),
        };
        fnv(&mut h, &v.to_le_bytes());
    }
    h
}
#[test]
fn a4_test() {
    if !is_x86_feature_detected!("avx") {
        return;
    }
    unsafe {
        check(&[
            ("_mm256_testc_ps", ck_test(0), 0x35a7ca360a1b2825),
            ("_mm256_testz_ps", ck_test(1), 0xc03cfd91dfe38814),
            ("_mm256_testnzc_ps", ck_test(2), 0x0d46a252a8a1e165),
        ]);
    }
}

// ---- rcp / rsqrt: hardware approximates (relative error <= 1.5 * 2^-12), so a tolerance and the special values ----
#[target_feature(enable = "avx")]
unsafe fn approx(rsqrt: bool) {
    let name = if rsqrt { "_mm256_rsqrt_ps" } else { "_mm256_rcp_ps" };
    let mut r = Rng(0xfeed_beef_0123_4567);
    for _ in 0..N {
        let mut a = [0f32; 8];
        for x in a.iter_mut() {
            let v = r.next();
            let mag = f32::from_bits(((v >> 9) as u32 & 0x007f_ffff) | ((87 + (v % 80) as u32) << 23));
            *x = if rsqrt || v & 0x100 == 0 { mag } else { -mag };
        }
        let mut o = [0f32; 8];
        _mm256_storeu_ps(o.as_mut_ptr(), if rsqrt { _mm256_rsqrt_ps(ld(&a)) } else { _mm256_rcp_ps(ld(&a)) });
        for i in 0..8 {
            let exact = if rsqrt { 1.0 / (a[i] as f64).sqrt() } else { 1.0 / a[i] as f64 };
            let rel = ((o[i] as f64 - exact) / exact).abs();
            assert!(rel <= 1.5 / 4096.0, "{name}({}) = {} (exact {exact})", a[i], o[i]);
        }
    }
    let sp = [0.0f32, -0.0, f32::INFINITY, f32::NEG_INFINITY, f32::NAN, -4.0, 1.0, 4.0];
    let mut o = [0f32; 8];
    _mm256_storeu_ps(o.as_mut_ptr(), if rsqrt { _mm256_rsqrt_ps(ld(&sp)) } else { _mm256_rcp_ps(ld(&sp)) });
    assert_eq!(o[0].to_bits(), f32::INFINITY.to_bits(), "{name}(+0)");
    assert_eq!(o[1].to_bits(), f32::NEG_INFINITY.to_bits(), "{name}(-0)");
    assert_eq!(o[2].to_bits(), 0, "{name}(+inf)");
    assert!(o[4].is_nan(), "{name}(nan)");
    if rsqrt {
        assert!(o[3].is_nan(), "{name}(-inf)");
        assert!(o[5].is_nan(), "{name}(-4)");
    } else {
        assert_eq!(o[3].to_bits(), (-0.0f32).to_bits(), "{name}(-inf)");
        assert!((o[5] + 0.25).abs() < 1e-4, "{name}(-4)");
    }
}
#[test]
fn a4_approx() {
    if !is_x86_feature_detected!("avx") {
        return;
    }
    unsafe {
        approx(false);
        approx(true);
    }
}
