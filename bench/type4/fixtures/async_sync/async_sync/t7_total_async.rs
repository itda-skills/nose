pub async fn total(xs: Vec<i32>) -> i32 {
    let mut acc = 0;
    for x in xs {
        let y = bump(x).await;
        if y > 0 {
            acc += y;
        }
    }
    acc
}
