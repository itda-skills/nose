func total(_ xs: [Int]) async -> Int {
  var acc = 0
  for x in xs {
    let y = await bump(x)
    if y > 0 {
      acc += y
    }
  }
  return acc
}
