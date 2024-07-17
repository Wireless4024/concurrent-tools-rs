use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures::Stream;
use futures_util::stream::FuturesUnordered;
use futures_util::StreamExt;
use kanal::{AsyncReceiver, ReceiveError};
use pin_project_lite::pin_project;

pin_project! {
    pub struct ConcurrentForkJoinTask<T:'static, F, O> {
        receiver: AsyncReceiver<T>,
        #[pin]
        receiver_future:Option<Box<dyn Future<Output=Result<T, ReceiveError>>+Send>>,
        data_buffer: Vec<T>,
        running:u32,
        budget:u32,
        init_size:u32,
        is_first: bool,
        #[pin]
        tasks: FuturesUnordered<O>,
        map_fn: F,
    }
}

impl<T, F: Fn(T) -> O, O: Future<Output=R>, R> ConcurrentForkJoinTask<T, F, O> {
    pub fn new(receiver: AsyncReceiver<T>, budget: u32, init_size: u32, map_fn: F) -> Self {
        Self {
            receiver,
            receiver_future: None,
            data_buffer: Vec::new(),
            running: 0,
            budget,
            init_size,
            is_first: true,
            tasks: FuturesUnordered::new(),
            map_fn,
        }
    }
}

impl<T, F: Fn(T) -> O, O: Future<Output=R>, R> Stream for ConcurrentForkJoinTask<T, F, O> {
    type Item = R;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.is_first {
            self.is_first = false;
            for _ in 0..self.init_size {
                if let Ok(Some(value)) = self.receiver.try_recv() {
                    self.data_buffer.push(value);
                }
            }
        }

        let mut this = self.as_mut().project();

        if *this.running as usize + this.data_buffer.len() < *this.budget as usize {
            let budget = *this.budget as usize;
            let queue = *this.running as usize + this.data_buffer.len();
            if this.receiver_future.is_none() && queue < budget {
                if this.receiver.is_disconnected() {
                    if let Ok(Some(value)) = this.receiver.try_recv() {
                        this.data_buffer.push(value);
                    };
                } else {
                    unsafe {
                        let value: Box<dyn Future<Output=_>> = Box::new(this.receiver.recv());
                        let value = unsafe { std::mem::transmute(value) };
                        this.receiver_future.replace(value);
                    }
                }
            }
        }
        if let Some(receiver_future) = this.receiver_future.as_mut().as_pin_mut() {
            let fut = receiver_future.get_mut().as_mut();
            let mut fut = unsafe {
                Pin::new_unchecked(fut)
            };
            match fut.as_mut().poll(cx) {
                Poll::Ready(next) => {
                    drop(fut);
                    this.receiver_future.set(None);
                    //self.receiver_future.ta
                    if let Ok(next) = next {
                        this.data_buffer.push(next);
                    }
                }
                // continue to poll the receiver
                Poll::Pending => {}
            }
        }
        if this.running < this.budget {
            while let Some(value) = this.data_buffer.pop() {
                let fut = (this.map_fn)(value);
                this.tasks.push(fut);
                *this.running += 1;
            }
        }
        match ready!(this.tasks.poll_next_unpin(cx)) {
            Some(value) => {
                *this.running -= 1;
                Poll::Ready(Some(value))
            }
            None => {
                if *this.running == 0
                    && this.data_buffer.is_empty()
                    && this.receiver_future.is_none() {
                    Poll::Ready(None)
                } else {
                    Poll::Pending
                }
            }
        }
    }
}