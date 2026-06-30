pub async fn total(xs: Vec<i32>) -> i32 {
    let mut acc = 1;
    for x in xs {
        let y = bump(x).await;
        if y % 2 == 0 {
            acc *= y + 1;
        }
    }
    acc
}
