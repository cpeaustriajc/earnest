// One `val` for each case of the operator table in crates/earnest_quint/src/table.rs.
module table {
  type V = V1 | V2(int)
  var x: int
  action init = x' = 0
  action step = x' = x
  pure val rec_build = { a: 1, b: "x" }
  pure val rec_field = { a: 1, b: 2 }.b
  pure val rec_with = { a: 1, b: 2 }.with("a", 5)
  pure val rec_spread = { ...{ a: 1, b: 2 }, b: 7 }
  pure val tup_build = (1, "a", true)
  pure val tup_item = (1, "a", true)._2
  pure val tup_unit = ()
  pure val tuples_two = tuples(Set(1, 2), Set("a"))
  pure val tuples_none = tuples(Set(1), Set())
  pure val variant_bare = V1
  pure val variant_value = V2(3)
  pure val match_value = match V2(3) { | V1 => 0 | V2(n) => n * 2 }
  pure val match_bare = match V1 { | V1 => 10 | V2(n) => n }
  pure val match_default = match V2(4) { | V1 => 0 | _ => 99 }
  pure val set_build = Set(3, 1, 2, 1)
  pure val set_empty = Set().union(Set(1))
  pure val set_contains = Set(1, 2).contains(2)
  pure val set_in = 3.in(Set(1, 2))
  pure val set_union = Set(1, 2).union(Set(2, 3))
  pure val set_intersect = Set(1, 2).intersect(Set(2, 3))
  pure val set_exclude = Set(1, 2, 3).exclude(Set(2))
  pure val set_subseteq = Set(1, 2).subseteq(Set(1, 2, 3))
  pure val set_not_subseteq = Set(1, 4).subseteq(Set(1, 2, 3))
  pure val set_size = Set(5, 6, 7).size()
  pure val set_filter = Set(1, 2, 3, 4).filter(x => x % 2 == 0)
  pure val set_exists = Set(1, 2).exists(x => x > 1)
  pure val set_exists_empty = Set().exists(x => x > 1)
  pure val set_forall = Set(1, 2).forall(x => x > 1)
  pure val set_forall_empty = Set().forall(x => x > 1)
  pure val set_map = Set(1, 2, 3).map(x => x % 2)
  pure val set_map_records = Set(1, 2).map(x => { n: x })
  pure val set_fold_sum = Set(1, 2, 3).fold(0, (acc, x) => acc + x)
  pure val set_fold_empty = Set().fold(7, (acc, x) => acc + x)
  pure val set_flatten = Set(Set(1), Set(2, 3), Set()).flatten()
  pure val set_powerset = Set(1, 2).powerset()
  pure val set_powerset_empty = Set().powerset()
  pure val set_is_finite = Set(1).isFinite()
  pure val set_to = 1.to(4)
  pure val set_to_empty = 3.to(1)
  pure val map_build = Map(1 -> "a", 2 -> "b")
  pure val map_empty = Map()
  pure val map_get = Map(1 -> 10, 2 -> 20).get(2)
  pure val map_get_missing = Map(1 -> 10).get(3)
  pure val map_keys = Map(1 -> 10, 2 -> 20).keys()
  pure val map_put_new = Map(1 -> 10).put(2, 20)
  pure val map_put_old = Map(1 -> 10).put(1, 5)
  pure val map_set = Map(1 -> 10).set(1, 5)
  pure val map_set_missing = Map(1 -> 10).set(2, 5)
  pure val map_set_by = Map(1 -> 10).setBy(1, v => v + 1)
  pure val map_set_by_missing = Map(1 -> 10).setBy(2, v => v + 1)
  pure val map_map_by = Set(1, 2).mapBy(x => x * 10)
  pure val map_set_to_map = Set((1, "a"), (2, "b")).setToMap()
  pure val list_build = [3, 1, 2]
  pure val list_append = [1, 2].append(3)
  pure val list_concat = [1, 2].concat([3, 1])
  pure val list_head = [5, 6].head()
  pure val list_head_empty = List().head()
  pure val list_tail = [5, 6, 7].tail()
  pure val list_tail_empty = List().tail()
  pure val list_length = [5, 6, 7].length()
  pure val list_nth = [5, 6, 7].nth(1)
  pure val list_nth_out = [5, 6].nth(2)
  pure val list_nth_negative = [5, 6].nth(-1)
  pure val list_indices = [5, 6, 7].indices()
  pure val list_indices_empty = List().indices()
  pure val list_replace_at = [1, 2, 3].replaceAt(1, 9)
  pure val list_replace_at_out = [1, 2, 3].replaceAt(3, 9)
  pure val list_slice = [1, 2, 3, 4].slice(1, 3)
  pure val list_slice_empty = [1, 2, 3].slice(3, 3)
  pure val list_slice_out = [1, 2, 3].slice(0, 4)
  pure val list_slice_backwards = [1, 2, 3].slice(2, 1)
  pure val list_range = range(1, 4)
  pure val list_range_empty = range(2, 2)
  pure val list_select = [1, 2, 3, 4].select(x => x > 2)
  pure val list_foldl = [1, 2, 3].foldl(0, (acc, x) => acc * 10 + x)
  pure val list_foldl_empty = List().foldl(5, (acc, x) => acc + x)
  pure val int_pow = 2^10
  pure val int_pow_zero = 7^0
  pure val int_pow_negative_base = (-3)^3
  pure val int_pow_negative = 2^(-1)
  pure val int_pow_zero_zero = 0^0
  pure val text_equal = "ab" == "ab"
  pure val text_set = Set("b", "a", "c")
  pure val nested_order = Set({ k: 2, v: "a" }, { k: 1, v: "b" })
}
