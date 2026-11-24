# earnest_threads

The one place in the workspace that starts threads, so that every thread is joined.

- `start` starts a thread of a `std::thread::scope`, with a stack of a given size or of the
  standard size, and hands back a `Joined`.
- `Joined::join` waits for the thread and gives what it returned or the payload of its panic.
  Dropping a `Joined` waits for the thread too, and passes its panic on.

Why: a standard handle that is dropped detaches its thread, and glibc's `pthread_detach` can read
the stack of a thread that has just exited and freed it. With a stack larger than glibc's cache of
stacks (40 MiB), such as the checker's workers of 256 MiB, that read crashes the process (issue
14, part 34). `just lint-library` refuses the standard calls that start a thread in the library and
binary code of every other crate; tests may still call them.
