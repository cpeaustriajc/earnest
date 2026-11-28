// A spec in another file, imported by `Plain.qnt`.
module Other {
  var m: int
  action init = m' = 0
  action step = m' = m + 1
  run otherTest = init.then(step).expect(m == 1)
}
