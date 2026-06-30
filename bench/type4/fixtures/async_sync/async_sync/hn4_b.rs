pub fn tally(xs: Vec<i32>) -> i32 {
    let mut acc = 0;
    for x in xs {
        let y = weigh(x);
        if y > 10 {
            acc -= y;
        }
    }
    acc
}
