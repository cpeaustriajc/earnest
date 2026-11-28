module Literals {
  val zero = 0
  val big = 123456789012345678901234567890
  val underscored = 1_000_000
  val hex = 0xFF
  val hexBig = 0xFFFF_FFFF_FFFF_FFFF_FFFF_FFFF
  val hexLower = 0xdead_beef
  val yes = true
  val no = false
  val text = "hello"
  val emptyText = ""
  val unicode = "héllo ✓"
  val backslash = "a\nb"
  val multiline = "one
two"
  val keywordsAsNames = Set(1).map(from => from)
  val asName = Set(1).map(as => as)
}
