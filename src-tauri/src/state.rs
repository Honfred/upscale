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
    /// Перед вставкой удаляет из реестра все записи в терминальном состоянии
    /// (Done/Error/Cancelled) — иначе реестр рос бы неограниченно на каждый
    /// запуск джобы. Безопасно, т.к. insert вызывается только после проверки
    /// any_running()==false (см. commands::start_job), то есть все текущие
    /// записи на этот момент терминальны.
    pub fn insert(&mut self, job_id: String, cancel: CancellationToken) {
        self.jobs.retain(|_, h| h.status.state == JobState::Running);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_prunes_terminal_jobs() {
        let mut registry = JobRegistry::default();
        registry.insert("job1".to_string(), CancellationToken::new());
        registry.set_state("job1", JobState::Done);
        registry.insert("job2".to_string(), CancellationToken::new());
        registry.set_state("job2", JobState::Error);
        registry.insert("job3".to_string(), CancellationToken::new());
        registry.set_state("job3", JobState::Cancelled);

        // Перед этим insert реестр содержит 3 терминальные джобы — все они
        // должны быть удалены, останется только job4.
        registry.insert("job4".to_string(), CancellationToken::new());

        assert_eq!(registry.jobs.len(), 1);
        assert!(registry.status("job1").is_none());
        assert!(registry.status("job2").is_none());
        assert!(registry.status("job3").is_none());
        assert!(registry.status("job4").is_some());
    }

    #[test]
    fn insert_keeps_running_jobs() {
        let mut registry = JobRegistry::default();
        registry.insert("job1".to_string(), CancellationToken::new());
        // job1 остаётся Running — не должен быть удалён следующим insert
        // (в реальном использовании такой insert не произойдёт, т.к.
        // commands::start_job проверяет any_running() заранее, но
        // JobRegistry сама по себе не должна терять активные джобы).
        registry.insert("job2".to_string(), CancellationToken::new());

        assert_eq!(registry.jobs.len(), 2);
        assert!(registry.status("job1").is_some());
    }
}
