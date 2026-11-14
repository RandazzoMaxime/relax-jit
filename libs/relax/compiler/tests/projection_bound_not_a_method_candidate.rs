mod cast {
    pub trait ToPrim { fn to_f64(&self) -> f64; }
    impl ToPrim for f64 { fn to_f64(&self) -> f64 { *self } }
    pub trait NumCast: Sized { fn from<T: ToPrim>(n: T) -> Option<Self>; }
    impl NumCast for f32 { fn from<T: ToPrim>(n: T) -> Option<f32> { Some(n.to_f64() as f32) } }
    impl NumCast for f64 { fn from<T: ToPrim>(n: T) -> Option<f64> { Some(n.to_f64()) } }
    pub trait Float: NumCast + Copy {}
    impl Float for f32 {}
    impl Float for f64 {}
}
use cast::Float;
trait Num: Copy + From<f32> + Into<f64> {}
impl Num for f32 {}
impl Num for f64 {}
trait HasPosition { type Scalar: Num; }
struct P32; struct P64;
impl HasPosition for P32 { type Scalar = f32; }
impl HasPosition for P64 { type Scalar = f64; }
trait Tri { type Vertex: HasPosition; }
struct T32; struct T64;
impl Tri for T32 { type Vertex = P32; }
impl Tri for T64 { type Vertex = P64; }
// `NumCast` (a supertrait of the `Float` bound) is not in scope: `from` is `From::from` alone.
fn one<T: Tri>() -> <T::Vertex as HasPosition>::Scalar
where <T::Vertex as HasPosition>::Scalar: Float {
    <T::Vertex as HasPosition>::Scalar::from(1.0)
}
#[test]
fn from_on_a_projection_takes_the_trait_in_scope() {
    assert_eq!(one::<T32>(), 1.0f32);
    assert_eq!(one::<T64>(), 1.0f64);
}
