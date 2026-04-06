use std::future::Future;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU8, Ordering};
use tokio::sync::Notify;

struct AgentDispatchGate {
    limit: AtomicU8,
    in_flight: Mutex<usize>,
    notify: Notify,
}

impl AgentDispatchGate {
    fn new(limit: u8) -> Self {
        Self {
            limit: AtomicU8::new(limit.max(1)),
            in_flight: Mutex::new(0),
            notify: Notify::new(),
        }
    }

    fn set_limit(&self, limit: u8) {
        self.limit.store(limit.max(1), Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    async fn acquire(&self) -> AgentDispatchPermit<'_> {
        loop {
            let notified = self.notify.notified();
            {
                let mut in_flight = self
                    .in_flight
                    .lock()
                    .expect("agent dispatch gate lock poisoned");
                let limit = usize::from(self.limit.load(Ordering::SeqCst));

                if *in_flight < limit {
                    *in_flight += 1;
                    return AgentDispatchPermit { gate: self };
                }
            }

            notified.await;
        }
    }
}

struct AgentDispatchPermit<'a> {
    gate: &'a AgentDispatchGate,
}

impl Drop for AgentDispatchPermit<'_> {
    fn drop(&mut self) {
        let mut in_flight = self
            .gate
            .in_flight
            .lock()
            .expect("agent dispatch gate lock poisoned");
        *in_flight -= 1;
        drop(in_flight);
        self.gate.notify.notify_waiters();
    }
}

#[derive(Clone)]
pub struct AppRuntime {
    pub database_path: String,
    agent_dispatch_gate: Arc<AgentDispatchGate>,
}

impl AppRuntime {
    pub fn new(database_path: String) -> Self {
        Self {
            database_path,
            agent_dispatch_gate: Arc::new(AgentDispatchGate::new(4)),
        }
    }

    pub fn set_max_agent_concurrency(&self, limit: u8) {
        self.agent_dispatch_gate.set_limit(limit);
    }

    pub async fn with_agent_dispatch_permit<F, Fut, T>(&self, work: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = T>,
    {
        let _permit = self.agent_dispatch_gate.acquire().await;
        work().await
    }
}

#[cfg(test)]
mod tests {
    use super::AppRuntime;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::sync::Notify;
    use tokio::task::JoinSet;

    async fn run_tracked_gate_task(
        runtime: Arc<AppRuntime>,
        started: Arc<AtomicUsize>,
        in_flight: Arc<AtomicUsize>,
        max_seen: Arc<AtomicUsize>,
        release: Arc<Notify>,
    ) {
        runtime
            .with_agent_dispatch_permit(|| async {
                started.fetch_add(1, Ordering::SeqCst);
                let current = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                max_seen.fetch_max(current, Ordering::SeqCst);
                release.notified().await;
                in_flight.fetch_sub(1, Ordering::SeqCst);
            })
            .await;
    }

    #[tokio::test]
    async fn limit_one_forces_serial_execution() {
        let runtime = Arc::new(AppRuntime::new("/tmp/distilllab-runtime-gate-serial.db".to_string()));
        runtime.set_max_agent_concurrency(1);

        let started = Arc::new(AtomicUsize::new(0));
        let in_flight = Arc::new(AtomicUsize::new(0));
        let max_seen = Arc::new(AtomicUsize::new(0));
        let first_release = Arc::new(Notify::new());
        let second_release = Arc::new(Notify::new());

        let mut tasks = JoinSet::new();
        tasks.spawn(run_tracked_gate_task(
            runtime.clone(),
            started.clone(),
            in_flight.clone(),
            max_seen.clone(),
            first_release.clone(),
        ));

        tokio::task::yield_now().await;
        assert_eq!(started.load(Ordering::SeqCst), 1);

        tasks.spawn(run_tracked_gate_task(
            runtime.clone(),
            started.clone(),
            in_flight.clone(),
            max_seen.clone(),
            second_release.clone(),
        ));

        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        assert_eq!(started.load(Ordering::SeqCst), 1);
        assert_eq!(max_seen.load(Ordering::SeqCst), 1);

        first_release.notify_one();
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        assert_eq!(started.load(Ordering::SeqCst), 2);

        second_release.notify_one();
        while tasks.join_next().await.is_some() {}
    }

    #[tokio::test]
    async fn limit_two_never_exceeds_two_in_flight_tasks() {
        let runtime = Arc::new(AppRuntime::new("/tmp/distilllab-runtime-gate-limit-two.db".to_string()));
        runtime.set_max_agent_concurrency(2);

        let started = Arc::new(AtomicUsize::new(0));
        let in_flight = Arc::new(AtomicUsize::new(0));
        let max_seen = Arc::new(AtomicUsize::new(0));
        let first_release = Arc::new(Notify::new());
        let second_release = Arc::new(Notify::new());
        let third_release = Arc::new(Notify::new());
        let mut tasks = JoinSet::new();

        tasks.spawn(run_tracked_gate_task(
            runtime.clone(),
            started.clone(),
            in_flight.clone(),
            max_seen.clone(),
            first_release.clone(),
        ));
        tasks.spawn(run_tracked_gate_task(
            runtime.clone(),
            started.clone(),
            in_flight.clone(),
            max_seen.clone(),
            second_release.clone(),
        ));
        tasks.spawn(run_tracked_gate_task(
            runtime.clone(),
            started.clone(),
            in_flight.clone(),
            max_seen.clone(),
            third_release.clone(),
        ));

        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        assert_eq!(started.load(Ordering::SeqCst), 2);
        assert_eq!(max_seen.load(Ordering::SeqCst), 2);

        first_release.notify_one();
        second_release.notify_one();
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        assert_eq!(started.load(Ordering::SeqCst), 3);

        third_release.notify_one();
        while tasks.join_next().await.is_some() {}
        assert_eq!(max_seen.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn queued_work_waits_and_runs_after_permits_release() {
        let runtime = Arc::new(AppRuntime::new("/tmp/distilllab-runtime-gate-queued.db".to_string()));
        runtime.set_max_agent_concurrency(1);

        let started = Arc::new(AtomicUsize::new(0));
        let in_flight = Arc::new(AtomicUsize::new(0));
        let max_seen = Arc::new(AtomicUsize::new(0));
        let first_release = Arc::new(Notify::new());
        let second_release = Arc::new(Notify::new());
        let mut tasks = JoinSet::new();

        tasks.spawn(run_tracked_gate_task(
            runtime.clone(),
            started.clone(),
            in_flight.clone(),
            max_seen.clone(),
            first_release.clone(),
        ));
        tasks.spawn(run_tracked_gate_task(
            runtime.clone(),
            started.clone(),
            in_flight.clone(),
            max_seen.clone(),
            second_release.clone(),
        ));

        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        assert_eq!(started.load(Ordering::SeqCst), 1);

        first_release.notify_one();
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        assert_eq!(started.load(Ordering::SeqCst), 2);
        assert_eq!(max_seen.load(Ordering::SeqCst), 1);

        second_release.notify_one();
        while tasks.join_next().await.is_some() {}
    }

    #[tokio::test]
    async fn changing_limit_only_affects_newly_started_work() {
        let runtime = Arc::new(AppRuntime::new("/tmp/distilllab-runtime-gate-limit-change.db".to_string()));
        runtime.set_max_agent_concurrency(1);

        let started = Arc::new(AtomicUsize::new(0));
        let in_flight = Arc::new(AtomicUsize::new(0));
        let max_seen = Arc::new(AtomicUsize::new(0));
        let first_release = Arc::new(Notify::new());
        let second_release = Arc::new(Notify::new());
        let mut tasks = JoinSet::new();

        tasks.spawn(run_tracked_gate_task(
            runtime.clone(),
            started.clone(),
            in_flight.clone(),
            max_seen.clone(),
            first_release.clone(),
        ));

        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        runtime.set_max_agent_concurrency(2);

        tasks.spawn(run_tracked_gate_task(
            runtime.clone(),
            started.clone(),
            in_flight.clone(),
            max_seen.clone(),
            second_release.clone(),
        ));

        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        assert_eq!(started.load(Ordering::SeqCst), 2);
        assert_eq!(max_seen.load(Ordering::SeqCst), 2);

        first_release.notify_one();
        second_release.notify_one();
        while tasks.join_next().await.is_some() {}
    }

    #[tokio::test]
    async fn lowering_limit_does_not_interrupt_already_running_work() {
        let runtime = Arc::new(AppRuntime::new("/tmp/distilllab-runtime-gate-lower.db".to_string()));
        runtime.set_max_agent_concurrency(2);

        let started = Arc::new(AtomicUsize::new(0));
        let in_flight = Arc::new(AtomicUsize::new(0));
        let max_seen = Arc::new(AtomicUsize::new(0));
        let first_release = Arc::new(Notify::new());
        let second_release = Arc::new(Notify::new());
        let third_release = Arc::new(Notify::new());
        let mut tasks = JoinSet::new();

        tasks.spawn(run_tracked_gate_task(
            runtime.clone(),
            started.clone(),
            in_flight.clone(),
            max_seen.clone(),
            first_release.clone(),
        ));
        tasks.spawn(run_tracked_gate_task(
            runtime.clone(),
            started.clone(),
            in_flight.clone(),
            max_seen.clone(),
            second_release.clone(),
        ));

        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        assert_eq!(started.load(Ordering::SeqCst), 2);

        runtime.set_max_agent_concurrency(1);
        tasks.spawn(run_tracked_gate_task(
            runtime.clone(),
            started.clone(),
            in_flight.clone(),
            max_seen.clone(),
            third_release.clone(),
        ));

        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        assert_eq!(started.load(Ordering::SeqCst), 2);
        assert_eq!(in_flight.load(Ordering::SeqCst), 2);

        first_release.notify_one();
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        assert_eq!(started.load(Ordering::SeqCst), 2);

        second_release.notify_one();
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
        assert_eq!(started.load(Ordering::SeqCst), 3);
        assert_eq!(max_seen.load(Ordering::SeqCst), 2);

        third_release.notify_one();
        while tasks.join_next().await.is_some() {}
    }
}
