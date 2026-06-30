func total(_ xs: [Int]) async -> Int {
  var acc = 1
  for x in xs {
    let y = await bump(x)
    if y % 2 == 0 {
      acc *= y + 1
    }
  }
  return acc
}
