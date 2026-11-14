// is_x86_feature_detected! knows sse4a, tbm, xsaveopt, xsavec, xsaves and rtm (pulp probes
// them): each answers what CPUID says (std_detect x86.rs), not a compile error.
#![cfg(target_arch = "x86_64")]

use std::arch::x86_64::{__cpuid, __cpuid_count};

#[test]
fn the_amd_xsave_and_rtm_features_follow_cpuid() {
    let p = __cpuid(1);
    let ext = if __cpuid(0x8000_0000).eax >= 0x8000_0001 { __cpuid(0x8000_0001).ecx } else { 0 };
    let os_xsave = p.ecx & (3 << 26) == (3 << 26);
    let xs = if os_xsave && __cpuid(0).eax >= 0xd { __cpuid_count(0xd, 1).eax } else { 0 };
    let leaf7_ebx = if __cpuid(0).eax >= 7 { __cpuid_count(7, 0).ebx } else { 0 };
    assert_eq!(is_x86_feature_detected!("sse4a"), ext & (1 << 6) != 0);
    assert_eq!(is_x86_feature_detected!("tbm"), ext & (1 << 21) != 0);
    assert_eq!(is_x86_feature_detected!("xsaveopt"), xs & 1 != 0);
    assert_eq!(is_x86_feature_detected!("xsavec"), xs & (1 << 1) != 0);
    assert_eq!(is_x86_feature_detected!("xsaves"), xs & (1 << 3) != 0);
    assert_eq!(is_x86_feature_detected!("rtm"), leaf7_ebx & (1 << 11) != 0);
}
