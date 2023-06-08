use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::task::Poll;
use std::time::Instant;

use async_trait::async_trait;

use crate::statement::*;

#[async_trait]
pub trait Solution {
    async fn solve(repositories: Vec<ServerName>) -> Option<Binary>;
}

pub struct Solution0;

#[async_trait]
impl Solution for Solution0 {
    async fn solve(repositories: Vec<ServerName>) -> Option<Binary> {
        if repositories.is_empty() {
            return None;
        }

        let futures: Vec<_> = repositories
            .into_iter()
            .map(|repo| Box::pin(download(repo)))
            .collect();

        let future = SolutionFuture::new(futures);

        future.await.ok()
    }
}

struct SolutionFuture<F, T, E>
where
    F: Future<Output = Result<T, E>>,
    F::Output: Debug,
{
    futures: Vec<Pin<Box<F>>>,
    is_ready_future: Vec<bool>,
    pending_count: usize,
}

impl<F, T, E> SolutionFuture<F, T, E>
where
    F: Future<Output = Result<T, E>>,
    F::Output: Debug,
{
    fn new(futures: Vec<Pin<Box<F>>>) -> Self {
        assert!(
            !futures.is_empty(),
            "Futures vec to await should not be empty"
        );

        // it would be better to use bit arithmetic and store integers here
        // to minimize memory footprint
        // but I hope it is not critical for test task
        let is_ready_future = vec![false; futures.len()];

        SolutionFuture {
            pending_count: futures.len(),
            futures,
            is_ready_future,
        }
    }
}

impl<F, T, E> Future for SolutionFuture<F, T, E>
where
    F: Future<Output = Result<T, E>>,
    F::Output: Debug,
{
    type Output = Result<T, E>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let mut last_error = None;

        println!("Check in {:?}", Instant::now());

        let SolutionFuture {
            futures,
            is_ready_future,
            pending_count,
        } = &mut *self;

        for (future, is_ready) in futures.iter_mut().zip(is_ready_future.iter_mut()) {
            if *is_ready {
                continue;
            }

            if let Poll::Ready(result) = Future::poll(future.as_mut(), cx) {
                println!("{result:?}");

                *is_ready = true;

                *pending_count -= 1;

                if result.is_ok() {
                    return Poll::Ready(result);
                }

                last_error = Some(result);
            }
        }

        println!("________");

        if self.pending_count == 0 {
            // last_error should be initialized anyway in case we haven't succeeded
            if let Some(last_error) = last_error {
                return Poll::Ready(last_error);
            }
        }

        Poll::Pending
    }
}
