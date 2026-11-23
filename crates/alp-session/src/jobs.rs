//! Cooperative background jobs.
//!
//! Work runs on a standard thread and reports progress through a channel. The
//! UI thread only drains events when a job is running, so decoding, peak
//! building and exports can never stall rendering. The audio callback is never
//! touched from this module.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};

/// Lifecycle of a background job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    /// Worker is active.
    Running,
    /// Worker returned success.
    Completed,
    /// Cancellation was requested and observed.
    Cancelled,
    /// Worker returned an error.
    Failed,
}

impl JobStatus {
    /// Whether the job still needs polling.
    #[must_use]
    pub fn is_running(self) -> bool {
        matches!(self, Self::Running)
    }
}

/// One visible job.
#[derive(Debug, Clone, PartialEq)]
pub struct Job {
    /// Stable id.
    pub id: u64,
    /// Display name.
    pub name: String,
    /// Progress in `0.0..=1.0`.
    pub progress: f32,
    /// Last detail line.
    pub detail: String,
    /// Lifecycle state.
    pub status: JobStatus,
}

impl Job {
    fn new(id: u64, name: String) -> Self {
        Self {
            id,
            name,
            progress: 0.0,
            detail: "Starting…".into(),
            status: JobStatus::Running,
        }
    }
}

/// Handle handed to a worker.
#[derive(Debug, Clone)]
pub struct JobReporter {
    id: u64,
    tx: Sender<JobEvent>,
    cancel: Arc<AtomicBool>,
}

impl JobReporter {
    /// Report progress in `0.0..=1.0`.
    pub fn progress(&self, progress: f32) {
        let value = (progress.clamp(0.0, 1.0) * 1_000.0) as u16;
        let _ = self.tx.send(JobEvent::Progress(self.id, value));
    }

    /// Replace the detail line.
    pub fn detail(&self, detail: impl Into<String>) {
        let _ = self.tx.send(JobEvent::Detail(self.id, detail.into()));
    }

    /// Cooperative cancellation flag.
    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::Relaxed)
    }
}

enum JobEvent {
    Progress(u64, u16),
    Detail(u64, String),
    Finished(u64, Result<String, String>),
}

/// Thread-safe queue of background jobs.
pub struct JobQueue {
    jobs: Vec<Job>,
    tx: Sender<JobEvent>,
    rx: Receiver<JobEvent>,
    next_id: u64,
    cancel: std::collections::HashMap<u64, Arc<AtomicBool>>,
}

impl JobQueue {
    /// Empty queue.
    #[must_use]
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            jobs: Vec::new(),
            tx,
            rx,
            next_id: 1,
            cancel: std::collections::HashMap::new(),
        }
    }

    /// Start a worker. The closure must return a short result string or error.
    pub fn spawn(
        &mut self,
        name: impl Into<String>,
        work: impl FnOnce(JobReporter) -> Result<String, String> + Send + 'static,
    ) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.saturating_add(1);
        let cancel = Arc::new(AtomicBool::new(false));
        self.cancel.insert(id, Arc::clone(&cancel));
        self.jobs.push(Job::new(id, name.into()));
        let tx = self.tx.clone();
        let worker_tx = tx.clone();
        std::thread::spawn(move || {
            let reporter = JobReporter {
                id,
                tx: worker_tx,
                cancel,
            };
            let result = work(reporter);
            let _ = tx.send(JobEvent::Finished(id, result));
        });
        id
    }

    /// Drain pending events and update job state.
    pub fn poll(&mut self) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                JobEvent::Progress(id, value) => {
                    if let Some(job) = self.jobs.iter_mut().find(|job| job.id == id)
                        && job.status.is_running()
                    {
                        job.progress = f32::from(value) / 1_000.0;
                    }
                }
                JobEvent::Detail(id, detail) => {
                    if let Some(job) = self.jobs.iter_mut().find(|job| job.id == id)
                        && job.status.is_running()
                    {
                        job.detail = detail;
                    }
                }
                JobEvent::Finished(id, result) => {
                    if let Some(job) = self.jobs.iter_mut().find(|job| job.id == id) {
                        match result {
                            Ok(detail) => {
                                job.progress = 1.0;
                                job.detail = detail;
                                job.status = JobStatus::Completed;
                            }
                            Err(error) if error == "cancelled" => {
                                job.detail = "Cancelled".into();
                                job.status = JobStatus::Cancelled;
                            }
                            Err(error) => {
                                job.detail = error;
                                job.status = JobStatus::Failed;
                            }
                        }
                    }
                    self.cancel.remove(&id);
                }
            }
        }
    }

    /// Request cancellation.
    pub fn cancel(&mut self, id: u64) {
        if let Some(flag) = self.cancel.get(&id) {
            flag.store(true, Ordering::Relaxed);
        }
        if let Some(job) = self.jobs.iter_mut().find(|job| job.id == id)
            && job.status.is_running()
        {
            job.detail = "Cancelling…".into();
        }
    }

    /// All jobs, oldest first.
    #[must_use]
    pub fn jobs(&self) -> &[Job] {
        &self.jobs
    }

    /// Whether any job is still running.
    #[must_use]
    pub fn has_running(&self) -> bool {
        self.jobs.iter().any(|job| job.status.is_running())
    }

    /// Number of running jobs.
    #[must_use]
    pub fn running_count(&self) -> usize {
        self.jobs
            .iter()
            .filter(|job| job.status.is_running())
            .count()
    }

    /// Remove finished jobs.
    pub fn clear_finished(&mut self) {
        self.jobs.retain(|job| job.status.is_running());
    }
}

impl Default for JobQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn wait_until_done(queue: &mut JobQueue) {
        let deadline = Instant::now() + Duration::from_secs(3);
        while queue.has_running() && Instant::now() < deadline {
            queue.poll();
            std::thread::sleep(Duration::from_millis(5));
        }
        queue.poll();
    }

    #[test]
    fn worker_reports_progress_and_completes() {
        let mut queue = JobQueue::new();
        let id = queue.spawn("test", |reporter| {
            reporter.progress(0.25);
            reporter.detail("half");
            reporter.progress(1.0);
            Ok("done".into())
        });
        wait_until_done(&mut queue);
        let job = queue.jobs().iter().find(|job| job.id == id).expect("job");
        assert_eq!(job.status, JobStatus::Completed);
        assert_eq!(job.progress, 1.0);
        assert_eq!(job.detail, "done");
        assert!(!queue.has_running());
    }

    #[test]
    fn cooperative_cancel_is_observed() {
        let mut queue = JobQueue::new();
        let id = queue.spawn("cancel-me", |reporter| {
            for _ in 0..200 {
                if reporter.is_cancelled() {
                    return Err("cancelled".into());
                }
                std::thread::sleep(Duration::from_millis(2));
            }
            Ok("finished anyway".into())
        });
        queue.cancel(id);
        wait_until_done(&mut queue);
        let job = queue.jobs().iter().find(|job| job.id == id).expect("job");
        assert_eq!(job.status, JobStatus::Cancelled);
        assert_eq!(job.detail, "Cancelled");
    }

    #[test]
    fn failed_worker_is_reported() {
        let mut queue = JobQueue::new();
        let id = queue.spawn("fail", |_| Err("disk full".into()));
        wait_until_done(&mut queue);
        let job = queue.jobs().iter().find(|job| job.id == id).expect("job");
        assert_eq!(job.status, JobStatus::Failed);
        assert_eq!(job.detail, "disk full");
    }

    #[test]
    fn finished_jobs_can_be_cleared() {
        let mut queue = JobQueue::new();
        let _ = queue.spawn("done", |_| Ok("ok".into()));
        wait_until_done(&mut queue);
        assert_eq!(queue.jobs().len(), 1);
        queue.clear_finished();
        assert!(queue.jobs().is_empty());
    }
}
