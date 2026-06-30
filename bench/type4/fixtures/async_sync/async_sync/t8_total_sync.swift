func total(_ xs: [Int]) -> Int {
  var acc = 0
  for x in xs {
    let y = bump(x)
    if y > 0 {
      acc += y
    }
  }
  return acc
}
