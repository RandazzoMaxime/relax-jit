// The pattern iterators of str implement Debug when the pattern's searcher does: a struct that
// holds one derives Debug (fluent-uri's `Split { inner: str::Split<'a, char>, .. }`).

#[derive(Clone, Debug)]
struct Holder<'a> {
    inner: std::str::Split<'a, char>,
}

#[test]
fn a_struct_holding_a_str_split_derives_debug() {
    let h = Holder { inner: "a,b".split(',') };
    assert_eq!(
        format!("{:?}", h.clone()),
        "Holder { inner: Split(SplitInternal { start: 0, end: 3, matcher: CharSearcher { haystack: \"a,b\", finger: 0, finger_back: 3, needle: ',', utf8_size: 1, utf8_encoded: [44, 0, 0, 0] }, allow_trailing_empty: true, finished: false }) }"
    );
    assert_eq!(h.inner.count(), 2);
}

#[test]
fn the_str_iterators_print_as_core_prints_them() {
    let cs = "CharSearcher { haystack: \"a,b\", finger: 0, finger_back: 3, needle: ',', utf8_size: 1, utf8_encoded: [44, 0, 0, 0] }";
    let ci = "char_indices: CharIndices { front_offset: 0, iter: Chars(['a', ',', 'b']) }";
    assert_eq!(format!("{:?}", "a,b".splitn(2, ',')), format!("SplitN(SplitNInternal {{ iter: SplitInternal {{ start: 0, end: 3, matcher: {cs}, allow_trailing_empty: true, finished: false }}, count: 2 }})"));
    assert_eq!(format!("{:?}", "a,b".rsplitn(2, ',')), format!("RSplitN(SplitNInternal {{ iter: SplitInternal {{ start: 0, end: 3, matcher: {cs}, allow_trailing_empty: true, finished: false }}, count: 2 }})"));
    assert_eq!(format!("{:?}", "a,b".matches(',')), format!("Matches(MatchesInternal({cs}))"));
    assert_eq!(format!("{:?}", "a,b".rmatches(',')), format!("RMatches(MatchesInternal({cs}))"));
    assert_eq!(format!("{:?}", "a,b".match_indices(',')), format!("MatchIndices(MatchIndicesInternal({cs}))"));
    assert_eq!(format!("{:?}", "a,b".rmatch_indices(',')), format!("RMatchIndices(MatchIndicesInternal({cs}))"));
    assert_eq!(format!("{:?}", "a,b".rsplit_terminator(',')), format!("RSplitTerminator(SplitInternal {{ start: 0, end: 3, matcher: {cs}, allow_trailing_empty: false, finished: false }})"));
    assert_eq!(format!("{:?}", "a,b".split_inclusive(',')), format!("SplitInclusive {{ 0: SplitInternal {{ start: 0, end: 3, matcher: {cs}, allow_trailing_empty: false, finished: false }} }}"));
    assert_eq!(format!("{:?}", "a,b".split(|c: char| c == ',')), format!("Split(SplitInternal {{ start: 0, end: 3, matcher: CharPredicateSearcher {{ haystack: \"a,b\", {ci} }}, allow_trailing_empty: true, finished: false }})"));
    assert_eq!(format!("{:?}", "a,b".split(&[',', ';'][..])), format!("Split(SplitInternal {{ start: 0, end: 3, matcher: CharSliceSearcher(MultiCharEqSearcher {{ char_eq: [',', ';'], haystack: \"a,b\", {ci} }}), allow_trailing_empty: true, finished: false }})"));
    assert_eq!(format!("{:?}", "a,b".split([',', ';'])), format!("Split(SplitInternal {{ start: 0, end: 3, matcher: CharArraySearcher(MultiCharEqSearcher {{ char_eq: [',', ';'], haystack: \"a,b\", {ci} }}), allow_trailing_empty: true, finished: false }})"));
    assert_eq!(format!("{:?}", "a\nb".lines()), "Lines(Map { iter: SplitInclusive { 0: SplitInternal { start: 0, end: 3, matcher: CharSearcher { haystack: \"a\\nb\", finger: 0, finger_back: 3, needle: '\\n', utf8_size: 1, utf8_encoded: [10, 0, 0, 0] }, allow_trailing_empty: false, finished: false } } })");
    assert_eq!(format!("{:?}", "a b".split_whitespace()), "SplitWhitespace { inner: Filter { iter: Split(SplitInternal { start: 0, end: 3, matcher: CharPredicateSearcher { haystack: \"a b\", char_indices: CharIndices { front_offset: 0, iter: Chars(['a', ' ', 'b']) } }, allow_trailing_empty: true, finished: false }) } }");
    assert_eq!(format!("{:?}", "a b".split_ascii_whitespace()), "SplitAsciiWhitespace { inner: Map { iter: Filter { iter: Split { v: [97, 32, 98], finished: false } } } }");
}
