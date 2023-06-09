use std::future::Future;

pub struct Retryer(usize);

/// Will try to retry passed function.
/// Will call function at least once
/// Maximum number of calls for passed fn is retry_count + 1
impl Retryer {
    pub fn new(retry_count: usize) -> Self {
        Retryer(retry_count)
    }

    pub async fn retry<T, E, Fut: Future<Output = Result<T, E>>, F: FnMut() -> Fut>(
        &self,
        mut f: F,
    ) -> Result<T, E> {
        for _ in 0..self.0 {
            let result = f().await;

            if result.is_ok() {
                return result;
            }
        }

        f().await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::Retryer;

    #[tokio::test]
    async fn should_call_function_at_least_once_ok() {
        let retryer = Retryer(0);
        let counter = Mutex::new(0);

        let res = retryer
            .retry(|| async {
                let mut counter = counter.lock().unwrap();

                *counter += 1;

                Result::<&str, &str>::Ok("test")
            })
            .await;

        assert_eq!(res, Ok("test"));
        assert_eq!(*counter.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn should_call_function_at_least_once_err() {
        let retryer = Retryer(0);
        let counter = Mutex::new(0);

        let res = retryer
            .retry(|| async {
                let mut counter = counter.lock().unwrap();

                *counter += 1;

                Result::<&str, &str>::Err("test")
            })
            .await;

        assert_eq!(res, Err("test"));
        assert_eq!(*counter.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn should_retry_function_until_ok() {
        let retryer = Retryer(3);
        let counter = Mutex::new(0);

        let res = retryer
            .retry(|| async {
                let mut counter = counter.lock().unwrap();

                *counter += 1;

                if *counter == 2 {
                    Ok("test")
                } else {
                    Err("test")
                }
            })
            .await;

        assert_eq!(res, Ok("test"));
        assert_eq!(*counter.lock().unwrap(), 2);
    }

    #[tokio::test]
    async fn should_retry_function_specified_number_of_times() {
        let retryer = Retryer(3);
        let counter = Mutex::new(0);

        let res = retryer
            .retry(|| async {
                *counter.lock().unwrap() += 1;

                Result::<&str, &str>::Err("test")
            })
            .await;

        assert_eq!(res, Err("test"));
        assert_eq!(*counter.lock().unwrap(), 4);
    }
}
