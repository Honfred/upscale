//! Реестр джобов: одна активная джоба (один GPU).

use serde::Serialize;
use std::collections::HashMap;
use std::sync::Mutex;
use tokio_util::sync::CancellationToken;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum JobState {
    Running,
    Done,
    Error,
    Cancelled,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobStatus {
    pub job_id: String,
    pub state: JobState,
    pub overall_progress: f32,
}

pub struct JobHandle {
    pub cancel: CancellationToken,
    pub status: JobStatus,
}

#[derive(Default)]
pub struct JobRegistry {
    pub jobs: HashMap<String, JobHandle>,
}

impl JobRegistry {
    pub fn any_running(&self) -> bool {
        self.jobs.values().any(|j| j.status.state == JobState::Running)
    }

    /// Регистрирует новую джобу в состоянии Running / overall_progress = 0.0.
    pub fn insert(&mut self, job_id: String, cancel: CancellationToken) {
        self.jobs.insert(
            job_id.clone(),
            JobHandle {
                cancel,
                status: JobStatus {
                    job_id,
                    state: JobState::Running,
                    overall_progress: 0.0,
                },
            },
        );
    }

    pub fn set_state(&mut self, job_id: &str, state: JobState) {
        if let Some(handle) = self.jobs.get_mut(job_id) {
            handle.status.state = state;
        }
    }

    pub fn set_progress(&mut self, job_id: &str, progress: f32) {
        if let Some(handle) = self.jobs.get_mut(job_id) {
            handle.status.overall_progress = progress;
        }
    }

    pub fn status(&self, job_id: &str) -> Option<JobStatus> {
        self.jobs.get(job_id).map(|h| h.status.clone())
    }

    /// Клонирует токен отмены (CancellationToken — дешёвый Arc-клон).
    pub fn cancel_token(&self, job_id: &str) -> Option<CancellationToken> {
        self.jobs.get(job_id).map(|h| h.cancel.clone())
    }
}

#[derive(Default)]
pub struct AppState {
    pub jobs: Mutex<JobRegistry>,
}
