# Concurrent Tools

my code used to learn future/stream and concurrency in rust. used unsafe without testing.

## Concurrent Fork Join (WIP)

see examples in [note/concurrent_fork_join.md](note/concurrent_fork_join.md)
or [bin/example_fork_join.rs](bin/example_fork_join.rs)

### use case

When you have heavy computation to do in async context,
basically it will starve current thread and run in sequential.

this is lightweight approach compare to spawn a task for each item and cheaper when task is more lightweight
