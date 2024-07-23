# Concurrent Tools

my code used to learn future/stream and concurrency in rust. used unsafe without testing.

## Concurrent Fork Join (WIP)

see examples in [note/concurrent_fork_join.md](note/concurrent_fork_join.md)
or [bin/example_fork_join.rs](bin/example_fork_join.rs)

### use case

When you have heavy computation to do in async context,
basically it will starve current thread and run in sequential.

this is lightweight approach compare to spawn a task for each item and cheaper when task is more lightweight

## Local Copy Map
Key-Value storage that have global map and local copy for each thread.

see examples in [benches/local_copy_map.rs](benches/local_copy_map.rs)

### use case
When you have to share data between thread and data is rarely updated,
normally doing Mutex or RwLock will have some overhead when doing CAS on atomic.
but this data structure allow lock free read via thread local storage.

### Benchmark
benchmark in single thread work load. [benches/local_copy_map.rs](benches/local_copy_map.rs)
```
test bench_concurrent_any_map_readonly ... bench:      42,772.33 ns/iter (+/- 16,496.18)
test bench_concurrent_map_readonly     ... bench:      45,793.85 ns/iter (+/- 26,147.29)
test bench_concurrent_map_readwrite    ... bench:      64,294.74 ns/iter (+/- 15,025.92)
test bench_std_mutex_map_readonly      ... bench:     236,347.61 ns/iter (+/- 25,887.72)
test bench_std_mutex_map_readwrite     ... bench:     333,435.14 ns/iter (+/- 30,818.49)
test bench_std_rwlock_map_readonly     ... bench:     138,909.14 ns/iter (+/- 54,793.16)
test bench_std_rwlock_map_readwrite    ... bench:     397,321.59 ns/iter (+/- 19,190.28)
```