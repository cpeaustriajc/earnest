module types_crash_wildcard_only {
  def f(e) = match e { | _ => 1 }
}
