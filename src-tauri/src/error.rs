//! Единый тип ошибки. Сериализуется на фронт как { code, message }.

use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

pub type Result<T> = std::result::Result<T, AppError>;

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("ffprobe: {0}")]
    Probe(String),
    #[error("{tool} завершился с кодом {code}: {stderr}")]
    Process {
        tool: String,
        code: i32,
        stderr: String,
    },
    #[error("ввод-вывод: {0}")]
    Io(#[from] std::io::Error),
    #[error("недостаточно места на диске: нужно ~{needed} байт, свободно {free}")]
    DiskSpace { needed: u64, free: u64 },
    #[error("Vulkan/GPU недоступен: {0}")]
    Gpu(String),
    #[error("компонент не найден: {0}")]
    SidecarMissing(String),
    #[error("джоба уже выполняется")]
    JobAlreadyRunning,
    #[error("джоба не найдена: {0}")]
    JobNotFound(String),
    #[error("джоба отменена")]
    Cancelled,
    #[error("неверная конфигурация: {0}")]
    Config(String),
    #[error("{0}")]
    Other(String),
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::Probe(_) => "probe",
            AppError::Process { .. } => "process",
            AppError::Io(_) => "io",
            AppError::DiskSpace { .. } => "disk_space",
            AppError::Gpu(_) => "gpu",
            AppError::SidecarMissing(_) => "sidecar_missing",
            AppError::JobAlreadyRunning => "job_already_running",
            AppError::JobNotFound(_) => "job_not_found",
            AppError::Cancelled => "cancelled",
            AppError::Config(_) => "config",
            AppError::Other(_) => "other",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        let mut s = serializer.serialize_struct("AppError", 2)?;
        s.serialize_field("code", self.code())?;
        s.serialize_field("message", &self.to_string())?;
        s.end()
    }
}
