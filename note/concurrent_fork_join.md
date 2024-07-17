# Concurrent Fork Join
```rust
fn sequential() {
    let (sender, receiver) = kanal::unbounded_async();
    tokio::spawn(async move {
        for i in 0..count {
            sender.send(i).await.unwrap();
        }
    });
    let count = 100;

    let tasks = FuturesUnordered::new();
    for i in 0..count {
        let recv = receiver.clone();
        tasks.push(async move {
            let x = recv.recv().await.unwrap();
            // simulate computation
            std::thread::sleep(std::time::Duration::from_millis(1));
            println!("{}", x);
        });
    }
    tasks.collect::<Vec<_>>().await;
    // this will run about 105 ms in 32 cpus machine
}

fn parallel(){
    let (sender, receiver) = kanal::unbounded_async();

    tokio::spawn(async move {
        for i in 0..count {
            sender.send(i).await.unwrap();
        }
    });
    
    let mut tasks = Vec::new();
    // 64 = concurrency but limited by logical cpu count (32 in my machine)
    for _ in 0..64 {
        let receiver = receiver.clone();
        // spawn to let work stealing do its job
        let task = tokio::spawn(async move {
            // 4 allow 1 job to buffer 4 items
            // 1 = init size
            ConcurrentForkJoinTask::new(receiver, 4, 1, |x: i32| async move {
                // simulate computation
                std::thread::sleep(std::time::Duration::from_millis(1));
                println!("{}", x);
            }).collect::<Vec<_>>().await;
            ()
        });
        tasks.push(task);
    }
    // this only wait for task in other thread to finish
    futures_util::stream::iter(tasks)
        .for_each_concurrent(None, |it| async move { it.await.unwrap(); })
        .await;
    // this will run about 7.5ms in 32 cpus machine (not yield 32x probably cause by numad pin the process in 1ccd)
}
```
- ConcurrentForkJoinTask allow async code when compare to rayon
- ConcurrentForkJoinTask rely on mpmc channel to distribute work
- ConcurrentForkJoinTask work well with execution that heavy computation