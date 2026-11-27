// `a()` is not a call here: the let `l` ends after `f - a`, its body is `()`, and `v2` is the next declaration.
module CallOrName {
  val a = 1
  val f = 2
  val v0 = val l = f - a () 3
  val v2 = match Some(1) { Some(x) => x | None => 0 }
  type Option = Some(int) | None
}
