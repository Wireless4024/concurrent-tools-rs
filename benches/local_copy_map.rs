#![feature(test)]

extern crate test;

use std::fmt::Display;
use std::hash::RandomState;
use std::hint::black_box;

use rayon::prelude::*;

use concurrent_tools::local_copy_map::{LocalAnyMapType, LocalCopyMap, LocalMapType};

const N: i32 = 1024 << 4;
const SEARCH_LOOP: i32 = 1024;
const WRITE_MASK: i32 = 0x1;
#[bench]
fn bench_std_mutex_map_readonly(b: &mut test::Bencher) {
    let mut map = std::collections::HashMap::new();
    for i in 0..N {
        map.insert(i, i);
    }
    let map = std::sync::Mutex::new(map);
    b.iter(|| {
        (0..SEARCH_LOOP)
            .rev()
            .collect::<Vec<_>>()
            .into_par_iter()
            .for_each(|i| {
                black_box(map.lock().unwrap().get(&i).unwrap());
            });
    });
}

#[bench]
fn bench_std_rwlock_map_readonly(b: &mut test::Bencher) {
    let mut map = std::collections::HashMap::new();
    for i in 0..N {
        map.insert(i, i);
    }
    let map = std::sync::RwLock::new(map);
    b.iter(|| {
        (0..SEARCH_LOOP)
            .rev()
            .collect::<Vec<_>>()
            .into_par_iter()
            .for_each(|i| {
                black_box(map.read().unwrap().get(&i).unwrap());
            });
    });
}


thread_local! {
    static LOCAL_RO: LocalMapType<i32, i32, RandomState> = LocalCopyMap::init_local(32);
    static LOCAL_ANY_RO: LocalAnyMapType<RandomState> = LocalCopyMap::init_any_local(32);
}

#[bench]
fn bench_concurrent_map_readonly(b: &mut test::Bencher) {
    let map = LocalCopyMap::new(32, &LOCAL_RO);
    for i in 0..N {
        map.set(i, i);
    }
    b.iter(|| {
        (0..SEARCH_LOOP)
            .rev()
            .collect::<Vec<_>>()
            .into_par_iter()
            .for_each(|i| {
                black_box(map.get(&i, |x| *x).unwrap());
            });
    });
}

#[bench]
fn bench_concurrent_any_map_readonly(b: &mut test::Bencher) {
    let map = LocalCopyMap::new(32, &LOCAL_ANY_RO);
    map.set_any(String::from("hello"));
    map.set_any("hello");
    map.set_any(10i8);
    map.set_any(10u8);
    map.set_any(10i16);
    map.set_any(10u16);
    map.set_any(10i32);
    map.set_any(10u32);
    map.set_any(10i64);
    map.set_any(10u64);
    map.set_any(10i128);
    map.set_any(10u128);
    map.set_any(10u64);
    map.set_any(10f32);
    map.set_any(10f64);
    b.iter(|| {
        (0..SEARCH_LOOP)
            .rev()
            .collect::<Vec<_>>()
            .into_par_iter()
            .for_each(|i| {
                black_box(map.get_downcast(|x: &i64| {
                    *black_box(x)
                }).unwrap());
            });
    });
}

thread_local! {
    static LOCAL_RW: LocalMapType<i32, i32, RandomState> = LocalCopyMap::init_local(32);
    static LOCAL_ANY_RW: LocalAnyMapType<RandomState> = LocalCopyMap::init_any_local(32);
}

#[bench]
fn bench_concurrent_map_readwrite(b: &mut test::Bencher) {
    let map = LocalCopyMap::new(32, &LOCAL_RW);
    for i in 0..N {
        map.set(i, i);
    }
    b.iter(|| {
        (0..SEARCH_LOOP)
            .rev()
            .collect::<Vec<_>>()
            .into_par_iter()
            .for_each(|i| {
                if i & WRITE_MASK == WRITE_MASK {
                    black_box(map.set(i, i));
                } else {
                    black_box(map.get(&i, |x| *x).unwrap());
                }
            });
    });
}
#[bench]
fn bench_std_mutex_map_readwrite(b: &mut test::Bencher) {
    let mut map = std::collections::HashMap::new();
    for i in 0..N {
        map.insert(i, i);
    }
    let map = std::sync::Mutex::new(map);
    b.iter(|| {
        (0..SEARCH_LOOP)
            .rev()
            .collect::<Vec<_>>()
            .into_par_iter()
            .for_each(|i| {
                if i & WRITE_MASK == WRITE_MASK {
                    black_box(map.lock().unwrap().insert(i, i));
                } else {
                    black_box(map.lock().unwrap().get(&i).unwrap());
                }
            });
    });
}

#[bench]
fn bench_std_rwlock_map_readwrite(b: &mut test::Bencher) {
    let mut map = std::collections::HashMap::new();
    for i in 0..N {
        map.insert(i, i);
    }
    let map = std::sync::RwLock::new(map);
    b.iter(|| {
        (0..SEARCH_LOOP)
            .rev()
            .collect::<Vec<_>>()
            .into_par_iter()
            .for_each(|i| {
                if i & WRITE_MASK == WRITE_MASK {
                    black_box(map.write().unwrap().insert(i, i));
                } else {
                    black_box(map.read().unwrap().get(&i).unwrap());
                }
            });
    });
}