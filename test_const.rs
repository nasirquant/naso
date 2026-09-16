const fn foo<const N: usize, const arr: [usize; N]> -> usize {
    arr[0]
}
fn main() {
    let _ = foo::<3, [1,2,3]>();
}
