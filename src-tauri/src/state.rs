//! Реестр джобов: одна активная джоба (один GPU). Реализация — задача B.

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
}

#[derive(Default)]
pub struct AppState {
    pub jobs: Mutex<JobRegistry>,
}
