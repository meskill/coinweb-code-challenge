use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::task::Poll;
use std::time::Instant;

use async_trait::async_trait;

use crate::retry::Retryer;
use crate::statement::*;

#[async_trait]
pub trait Solution<T, E, D: Download<T, E> + Send + Sync> {
    async fn solve(repositories: Vec<D>) -> Option<T>;
}

pub struct Solution0;

#[async_trait]
impl<T: Debug, E: Debug, D: Download<T, E> + Send + Sync + 'static + Clone> Solution<T, E, D>
    for Solution0
{
    async fn solve(repositories: Vec<D>) -> Option<T> {
        if repositories.is_empty() {
            return None;
        }

        let retryer = Retryer::new(3);

        let futures: Vec<_> = repositories
            .iter()
            .map(|repo| {
                Box::pin(retryer.retry(move || {
                    // I'm not happy with cloning here, but we need to pass ownership from closure
                    // to async block multiple times
                    let repo = repo.clone();

                    async move { repo.download().await }
                }))
            })
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

#[cfg(test)]
mod tests {
    use async_trait::async_trait;
    use tokio::time;

    use crate::statement::Download;

    use super::{Solution, Solution0};

    #[derive(Clone)]
    struct MockRepo {
        value: String,
        tick_times: usize,
        failing: bool,
        panic: bool,
    }

    #[async_trait]
    impl Download<String, String> for MockRepo {
        async fn download(self) -> Result<String, String> {
            let mut interval = time::interval(time::Duration::from_millis(1));

            for _i in 0..self.tick_times {
                interval.tick().await;
            }

            if self.panic {
                unreachable!();
            }

            if self.failing {
                Err(self.value.clone())
            } else {
                Ok(self.value.clone())
            }
        }
    }

    #[tokio::test]
    async fn should_return_none_for_empty_vec() {
        let repos: Vec<MockRepo> = vec![];

        assert_eq!(Solution0::solve(repos).await, None);
    }

    #[tokio::test]
    async fn should_wait_success_for_single_repo() {
        let repos = vec![MockRepo {
            failing: false,
            panic: false,
            tick_times: 3,
            value: "1".to_owned(),
        }];

        assert_eq!(Solution0::solve(repos).await, Some("1".to_owned()));
    }

    #[tokio::test]
    async fn should_wait_error_for_single_repo() {
        let repos = vec![MockRepo {
            failing: true,
            panic: false,
            tick_times: 3,
            value: "1".to_owned(),
        }];

        assert_eq!(Solution0::solve(repos).await, None);
    }

    #[tokio::test]
    async fn should_not_call_future_again_after_it_is_failed() {
        let repos = vec![
            MockRepo {
                failing: false,
                panic: false,
                tick_times: 3,
                value: "1".to_owned(),
            },
            MockRepo {
                failing: true,
                panic: false,
                tick_times: 1,
                value: "2".to_owned(),
            },
        ];

        assert_eq!(Solution0::solve(repos).await, Some("1".to_owned()));
    }

    #[tokio::test]
    async fn should_not_call_future_when_another_future_succeeded() {
        let repos = vec![
            MockRepo {
                failing: false,
                panic: false,
                tick_times: 3,
                value: "1".to_owned(),
            },
            MockRepo {
                failing: true,
                panic: true,
                tick_times: 4,
                value: "2".to_owned(),
            },
        ];

        assert_eq!(Solution0::solve(repos).await, Some("1".to_owned()));
    }

    #[tokio::test]
    async fn should_pick_first_succeeded_repo() {
        let repos = vec![
            MockRepo {
                failing: true,
                panic: false,
                tick_times: 3,
                value: "1".to_owned(),
            },
            MockRepo {
                failing: false,
                panic: false,
                tick_times: 4,
                value: "2".to_owned(),
            },
            MockRepo {
                failing: false,
                panic: false,
                tick_times: 4,
                value: "3".to_owned(),
            },
        ];

        assert_eq!(Solution0::solve(repos).await, Some("2".to_owned()));
    }

    #[tokio::test]
    async fn should_return_none_if_none_repo_succeeded() {
        let repos = vec![
            MockRepo {
                failing: true,
                panic: false,
                tick_times: 3,
                value: "1".to_owned(),
            },
            MockRepo {
                failing: true,
                panic: false,
                tick_times: 4,
                value: "2".to_owned(),
            },
            MockRepo {
                failing: true,
                panic: false,
                tick_times: 5,
                value: "3".to_owned(),
            },
        ];

        assert_eq!(Solution0::solve(repos).await, None);
    }
}
