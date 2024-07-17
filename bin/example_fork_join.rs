use futures_util::stream::FuturesUnordered;
use futures_util::StreamExt;
use tokio::time::sleep;

use concurrent_tools::concurrent_fork_join::ConcurrentForkJoinTask;

#[tokio::main]
async fn main() {
    let count = 100;
    {
        let (sender, receiver) = kanal::unbounded_async();
        tokio::spawn(async move {
            for i in 0..count {
                sender.send(i).await.unwrap();
            }
            //sleep(std::time::Duration::from_millis(100)).await;
        });
        sleep(std::time::Duration::from_millis(1000)).await;
        let start = std::time::Instant::now();
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
        let end = std::time::Instant::now();
        println!("time taken: {:?}", end.duration_since(start));
    }
    sleep(std::time::Duration::from_millis(1000)).await;
    {
        let (sender, receiver) = kanal::unbounded_async();

        tokio::spawn(async move {
            for i in 0..count {
                sender.send(i).await.unwrap();
            }
            //sleep(std::time::Duration::from_millis(500)).await;
        });
        sleep(std::time::Duration::from_millis(1000)).await;
        let start = std::time::Instant::now();
        let mut tasks = Vec::new();
        // more task have more opportunity to trigger work stealing but spawn all task have more overhead
        for _ in 0..64 {
            let receiver = receiver.clone();
            let task = tokio::spawn(async move {
                ConcurrentForkJoinTask::new(receiver, 4, 1, |x: i32| async move {
                    // simulate computation
                    std::thread::sleep(std::time::Duration::from_millis(1));
                    println!("{}", x);
                }).collect::<Vec<_>>().await;
                ()
            });
            tasks.push(task);
        }
        futures_util::stream::iter(tasks)
            .for_each_concurrent(None, |it| async move { it.await.unwrap(); })
            .await;
        let end = std::time::Instant::now();
        println!("time taken: {:?}", end.duration_since(start));
    }
}