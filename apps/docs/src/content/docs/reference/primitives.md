---
title: Primitives and ordering
description: Look up every Earnest primitive, its TypeScript and Rust operations, error rules, and deterministic set and map ordering.
---

Every primitive has the same semantics in TypeScript, in Rust, and in the TLA+ export. TypeScript names use `camelCase` and Rust names use `snake_case` (for example, `isEmpty` vs `is_empty()`). In TypeScript, names without parentheses below are properties; in Rust, every operation is a method call. All operations are pure and return a new value without mutating in place.

## Primitives

| Group | TypeScript / Rust declaration | Operations |
|---|---|---|
| Boolean | `m.boolean()` / `bool` | `&&`, `\|\|`, `!`, `===` (`==`) |
| Number | `m.int(lo, hi)` / integer with `#[range(lo, hi)]` or `#[one_of(...)]` | `+`, `-`, `*`, `m.rem(a, b)` / `%`, `m.div(a, b)` / `/`, `is_multiple_of(d)` (Rust), `u8::from(flag)` & lossless `From` (Rust), `<`, `<=`, `>`, `>=`, `===` (`==`) |
| Enum | `m.enum([...])`, `m.oneOf([...])` / `enum`, `#[one_of(...)]` | `===`, `!==` / `==`, `!=`, `match`, `matches!` |
| Optional | `.optional()` / `Option<T>` | `=== null`, `!== null`, `m.some(x)` / `is_some()`, `is_none()`, `is_some_and(f)`, `== Some(x)`, `if let Some(x)`, `map_or(d, f)`, `unwrap_or(d)`, `let x = v?;` in an action |
| Record | `m.record({...})` / `struct` | field read, `{ ...r, field: v }` / `R { field: v, ..r.clone() }` |
| Set | `m.set(T)` / `Set<T>` | `has(x)`, `add(x)`, `delete(x)`, `union(s)`, `intersect(s)`, `subtract(s)`, `isSubset(s)`, `size`, `isEmpty`, `every(f)`, `some(f)`, `filter(f)` |
| Map | `m.map(K, V)` / `Map<K, V>` | `has(k)`, `get(k)` (optional), `set(k, v)`, `delete(k)`, `size`, `isEmpty`, `keys` |
| Sequence | `m.seq(T, { max })` / `Seq<T, MAX>` | `size`, `isEmpty`, `get(i)`, `first`, `last` (optional), `push(x)`, `shift()`, `pop()`, `includes(x)`, `every(f)`, `some(f)`, `filter(f)` |
| Choice | `input` / action parameters | `m.forall(T, f)`, `m.exists(T, f)` over a declaration or collection / `forall::<T>(f)`, `exists::<T>(f)` |
| Queue | `m.queue(T, { capacity })` / `Queue<T, CAPACITY>` | `size`, `isEmpty`, `isFull`, `head` (optional), `send(x)`, `receive()`, `includes(x)`, `clear()`, `every(f)`, `some(f)` |
| Lock | `m.lock()`, `m.lock(Owner)` / `Lock`, `Lock<Owner>` | `isFree`, `isHeld`, `acquire()`, `release()`; with an owner: `acquire(o)`, `isHeldBy(o)`, `holder` (optional) |
| Timer | `m.timer()` / `Timer` | `isIdle`, `isRunning`, `hasFired`, `start()`, `fire()`, `cancel()`, `reset()` |
| Async request | `m.request()` / `Request` | `isIdle`, `isPending`, `hasSucceeded`, `hasFailed`, `isSettled`, `send()`, `succeed()`, `fail()`, `reset()` |
| Retry | `m.retry({ max })` / `Retry<MAX>` | `attempts`, `canRetry`, `isExhausted`, `again()`, `reset()` |

## Collection literals and value rules

- **Literals:** Write collection values with `m.setOf(a, b)`, `m.seqOf(a, b)`, and `m.mapOf([k, v])` in TypeScript, or `Set::of([a, b])`, `Seq::of([a, b])`, and `Map::of([(k, v)])` in Rust.
- **Out-of-range transitions:** Sending to a full queue, pushing past a sequence's `max`, or stepping a number outside its declared range is not a panic or syntax error. The checker reports the resulting state as `out of range` with the shortest path to it.
- **Owned locks in collections:** A lock with an owner (`m.lock(Owner)` / `Lock<Owner>`) cannot be a map value or a sequence/queue item in either language, because a free owned lock is `null` and could not be distinguished from a missing entry. Both the checker and `@earnestjs/model` refuse it. An unowned lock can be stored in collections, and an owned lock can be a state field, a record field, or a set item.
- **Ordering on optionals:** Optional values have no order. Comparing two optional values with `<`, `<=`, `>`, or `>=` makes the file `unreadable` (including in Rust, where `None < Some(5)` compiles; write `value.is_some_and(|n| n < 5)`). In TypeScript, comparing or adding an optional number or spreading an optional record requires `m.some(...)` (`{ ...m.some(s.chosen), qty: 1 }`), which faults if the value is `null`.
- **Field names vs primitive properties:** A record field may share a name with a primitive property (such as `isFull`). Earnest resolves `value.isFull` from the static type of `value`.

## Order of sets, maps, and records

The checker, Node (`@earnestjs/model`), Rust (`earnest`), and the TLA+ export all walk set items and map keys in one deterministic order, independent of insertion order, hash seeds, first-seen text order, or `--jobs`. Quantifiers (`m.exists`, `Set::some`), `filter`, printed paths, JSON output, statecharts, reports, and TLA+ export all follow this order:

| Kind | Order |
|---|---|
| Number | Ascending |
| Boolean | `false` before `true` |
| Text and enum variant | Unicode code-point (UTF-8 byte) order: `"B"` before `"a"`, and characters past `U+FFFF` after `U+E000..=U+FFFF`. Enum values sort by variant name, not declaration order (`enum Fruit { Pear, Apple }` walks `Apple` first) |
| Optional | `null` (`None`) before all present values, which sort in their own order |
| Record | Field by field, with field names sorted by code point (not struct declaration order) |
| Set, sequence, queue | Item by item in item order; a shorter prefix comes first |
| Map | Entry by entry, comparing key first, then value |
| Timer, request | By stage text: `"fired"`, `"idle"`, `"running"` (and `"failed"`, `"idle"`, `"pending"`, `"succeeded"` for requests) |
| Retry | By attempt count |
| Lock | Unowned: `false` (free) before `true` (held). Owned: by holder value (`null` first) |

Additional ordering rules:

- Records print fields in code-point order of field names (`{lit: true, row: 1}`).
- Quantifiers over a **declared type** (`m.forall(T, f)`, `forall::<T>(f)`) walk `T` in declaration order, not set order: declaration order for enums, `m.oneOf`, and `#[one_of]`; declaration field order with the first field varying slowest for records; subsets built one item at a time for sets.
- Sequences and queues walk first item first.
- In Rust, `Set` and `Map` enforce this order through `earnest::model::Walk`, derived automatically by `#[derive(Domain)]` (a custom `Ord` does not override it). `Report::report` prints collections in the same canonical order as `earnest verify`.
