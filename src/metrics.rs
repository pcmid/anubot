use std::fmt::Write;
use std::sync::atomic::{AtomicU64, Ordering};

/// Hand-rolled Prometheus exposition counters. No new dependency.
/// All counters are monotonic AtomicU64 with Relaxed ordering — they
/// are operational gauges, not synchronization primitives.
pub struct Metrics {
    pub verifications_started: AtomicU64,
    pub verifications_succeeded: AtomicU64,
    pub verifications_expired: AtomicU64,
    pub spam_decisions_deleted: AtomicU64,
    pub spam_decisions_kicked: AtomicU64,
    pub ai_calls_ok: AtomicU64,
    pub ai_calls_error: AtomicU64,
}

impl Metrics {
    pub fn new() -> Self {
        Self {
            verifications_started: AtomicU64::new(0),
            verifications_succeeded: AtomicU64::new(0),
            verifications_expired: AtomicU64::new(0),
            spam_decisions_deleted: AtomicU64::new(0),
            spam_decisions_kicked: AtomicU64::new(0),
            ai_calls_ok: AtomicU64::new(0),
            ai_calls_error: AtomicU64::new(0),
        }
    }

    pub fn render_prometheus(&self) -> String {
        let mut out = String::with_capacity(1024);
        let _ = write!(
            out,
            "# HELP anubot_verifications_started_total Verification flows started\n\
             # TYPE anubot_verifications_started_total counter\n\
             anubot_verifications_started_total {}\n\
             # HELP anubot_verifications_succeeded_total Verification flows completed by user\n\
             # TYPE anubot_verifications_succeeded_total counter\n\
             anubot_verifications_succeeded_total {}\n\
             # HELP anubot_verifications_expired_total Verification sessions kicked after timeout\n\
             # TYPE anubot_verifications_expired_total counter\n\
             anubot_verifications_expired_total {}\n\
             # HELP anubot_spam_decisions_total Spam decisions taken by action\n\
             # TYPE anubot_spam_decisions_total counter\n\
             anubot_spam_decisions_total{{action=\"deleted\"}} {}\n\
             anubot_spam_decisions_total{{action=\"kicked\"}} {}\n\
             # HELP anubot_ai_calls_total Outcome of AI spam-check calls\n\
             # TYPE anubot_ai_calls_total counter\n\
             anubot_ai_calls_total{{outcome=\"ok\"}} {}\n\
             anubot_ai_calls_total{{outcome=\"error\"}} {}\n",
            self.verifications_started.load(Ordering::Relaxed),
            self.verifications_succeeded.load(Ordering::Relaxed),
            self.verifications_expired.load(Ordering::Relaxed),
            self.spam_decisions_deleted.load(Ordering::Relaxed),
            self.spam_decisions_kicked.load(Ordering::Relaxed),
            self.ai_calls_ok.load(Ordering::Relaxed),
            self.ai_calls_error.load(Ordering::Relaxed),
        );
        out
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_prometheus_format() {
        let m = Metrics::new();
        m.verifications_started.fetch_add(3, Ordering::Relaxed);
        m.spam_decisions_deleted.fetch_add(2, Ordering::Relaxed);
        let out = m.render_prometheus();
        assert!(out.contains("anubot_verifications_started_total 3"));
        assert!(out.contains("anubot_spam_decisions_total{action=\"deleted\"} 2"));
        assert!(out.contains("anubot_spam_decisions_total{action=\"kicked\"} 0"));
        assert!(out.contains("# TYPE anubot_verifications_started_total counter"));
    }
}
