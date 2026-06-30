func tally(_ xs: [Int]) -> Int {
  var acc = 0
  for x in xs {
    let y = weigh(x)
    if y > 10 {
      acc -= y
    }
  }
  return acc
}
