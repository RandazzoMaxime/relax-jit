// A chain of constants that each read the one before twice: the certificate of a value takes in
// the certificates of the values it read as a set. Taken in as lists they doubled per link
// (2^n entries: 40 links took gigabytes and never finished).

macro_rules! chain {
    ($first:ident = $v:expr; $($name:ident <- $prev:ident ^ $k:expr;)*) => {
        const $first: u64 = $v;
        $(const $name: u64 = $prev.wrapping_add($prev) ^ $k;)*
    };
}

chain! {
    C0 = 1;
    C1 <- C0 ^ 1; C2 <- C1 ^ 2; C3 <- C2 ^ 3; C4 <- C3 ^ 4; C5 <- C4 ^ 5;
    C6 <- C5 ^ 6; C7 <- C6 ^ 7; C8 <- C7 ^ 8; C9 <- C8 ^ 9; C10 <- C9 ^ 10;
    C11 <- C10 ^ 11; C12 <- C11 ^ 12; C13 <- C12 ^ 13; C14 <- C13 ^ 14; C15 <- C14 ^ 15;
    C16 <- C15 ^ 16; C17 <- C16 ^ 17; C18 <- C17 ^ 18; C19 <- C18 ^ 19; C20 <- C19 ^ 20;
    C21 <- C20 ^ 21; C22 <- C21 ^ 22; C23 <- C22 ^ 23; C24 <- C23 ^ 24; C25 <- C24 ^ 25;
    C26 <- C25 ^ 26; C27 <- C26 ^ 27; C28 <- C27 ^ 28; C29 <- C28 ^ 29; C30 <- C29 ^ 30;
    C31 <- C30 ^ 31; C32 <- C31 ^ 32; C33 <- C32 ^ 33; C34 <- C33 ^ 34; C35 <- C34 ^ 35;
    C36 <- C35 ^ 36; C37 <- C36 ^ 37; C38 <- C37 ^ 38; C39 <- C38 ^ 39; C40 <- C39 ^ 40;
}

#[test]
fn a_chain_of_constants_read_twice_each_evaluates() {
    assert_eq!(std::hint::black_box(C40), 1176584991222);
    assert_eq!(std::hint::black_box(C3), 11);
}
