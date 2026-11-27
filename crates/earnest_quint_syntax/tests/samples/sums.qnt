module Sums {
  type Color = Red | Green | Blue
  type Leading = | Up | Down
  type Payload = Num(int) | Text(str) | Nothing
  type Only = int
  type One = Only
  type OneWithPayload = Wrapped(int)
  type OneLeading = | Single
  type Generic[a] = Some(a) | None
  type Pairs[a, b] = Both((a, b)) | Neither
  type UnitPayload = Empty(())
  type lowercase = int
  type Lower = lowercase
}
