// Pure helpers over the types, imported by the spec: a chain spec -> lib -> types.
module lib {
  import types.* from "./types"
  pure val closed: Door = { open: false, count: 0 }
  pure def opened(d: Door): Door = { open: true, count: d.count + 1 }
  pure def shut(d: Door): Door = { ...d, open: false }
}
