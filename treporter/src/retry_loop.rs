// RetryLoop — bounded self-refinement wrapper for LLM calls (Phase 3).
//
// Architecture: de-inductive reasoning pipeline.
// The deductive phase produces StructuredData; the inductive phase (this module)
// runs the LLM up to `max_retries` times, passing the previous failure hint on
// each retry so the model can self-correct. A pure-Rust fallback is always
// available so the overall pipeline never fails hard.

use std::future::Future;

/// Bounded retry wrapper for a single LLM call.
///
/// `max_retries` controls the upper limit on LLM invocations (default: 3).
/// After exhausting retries the caller's `fallback_value` is returned, ensuring
/// the pipeline always produces *some* output even when the model is degraded.
pub struct RetryLoop {
    max_retries: u32,
}

impl RetryLoop {
    pub fn new(max_retries: u32) -> Self {
        Self { max_retries }
    }

    /// Attempt `call_with_hint` up to `max_retries` times, validating the result
    /// after each call.
    ///
    /// # Parameters
    /// - `call_with_hint`: async closure `(Option<String>) -> Result<String>`.
    ///   Receives `None` on the first attempt. On subsequent attempts receives a
    ///   message describing why the previous output was rejected (up to 500 chars
    ///   of the failed output is included so the model can see its mistake).
    /// - `validate`: synchronous function `(&str) -> Result<(), String>`.
    ///   Return `Ok(())` to accept the output; return `Err(reason)` to retry.
    /// - `fallback_value`: always-valid Rust-generated string used when all
    ///   retries are exhausted.
    pub async fn run_str<F, Fut>(
        &self,
        mut call_with_hint: F,
        validate: impl Fn(&str) -> Result<(), String>,
        fallback_value: String,
    ) -> String
    where
        F: FnMut(Option<String>) -> Fut,
        Fut: Future<Output = anyhow::Result<String>>,
    {
        let mut hint: Option<String> = None;

        for attempt in 0..self.max_retries {
            match call_with_hint(hint.clone()).await {
                Ok(output) => match validate(&output) {
                    Ok(()) => return output,
                    Err(reason) => {
                        if attempt + 1 < self.max_retries {
                            hint = Some(format!(
                                "Your previous response was rejected because: {}\n\
                                 The first 500 chars of your rejected output:\n{}",
                                reason,
                                &output[..output.len().min(500)]
                            ));
                        }
                        eprintln!(
                            "[RetryLoop] attempt {}/{} rejected: {}",
                            attempt + 1,
                            self.max_retries,
                            reason
                        );
                    }
                },
                Err(e) => {
                    eprintln!(
                        "[RetryLoop] attempt {}/{} error: {}",
                        attempt + 1,
                        self.max_retries,
                        e
                    );
                    if attempt + 1 < self.max_retries {
                        hint = Some(format!("Previous call failed with: {}", e));
                    }
                }
            }
        }

        eprintln!(
            "[RetryLoop] all {} retries exhausted, using fallback",
            self.max_retries
        );
        fallback_value
    }
}

/// Simple output validator: reject if the string is shorter than `min_len` bytes.
pub fn validate_min_length(min_len: usize) -> impl Fn(&str) -> Result<(), String> {
    move |s: &str| {
        if s.len() >= min_len {
            Ok(())
        } else {
            Err(format!(
                "output too short ({} bytes, need at least {})",
                s.len(),
                min_len
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_succeeds_on_first_try() {
        let rl = RetryLoop::new(3);
        let count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let count2 = count.clone();
        let result = rl
            .run_str(
                move |_hint| {
                    count2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    async { Ok("hello world this is long enough".to_string()) }
                },
                validate_min_length(10),
                "fallback".to_string(),
            )
            .await;
        assert_eq!(result, "hello world this is long enough");
        assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retries_on_short_output() {
        let rl = RetryLoop::new(3);
        let count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
        let count2 = count.clone();
        let result = rl
            .run_str(
                move |_hint| {
                    let n = count2.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let s = if n < 2 { "short".to_string() } else { "a".repeat(200) };
                    async move { Ok(s) }
                },
                validate_min_length(100),
                "fallback".to_string(),
            )
            .await;
        assert_eq!(result.len(), 200);
        assert_eq!(count.load(std::sync::atomic::Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_uses_fallback_after_exhaustion() {
        let rl = RetryLoop::new(2);
        let result = rl
            .run_str(
                |_hint| async { Ok("tiny".to_string()) },
                validate_min_length(1000),
                "safe_fallback".to_string(),
            )
            .await;
        assert_eq!(result, "safe_fallback");
    }
}
