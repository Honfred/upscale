//! Очистка промежуточных файлов джобы. Все функции — no-op при
//! keep_intermediate=true.
//!
//! Тайминг вызовов (см. pipeline/mod.rs):
//! - если интерполяция выполняется: после interpolate удаляются in/ и up/
//!   (оба уже потреблены), после encode удаляется rife/.
//! - если интерполяция пропущена: encode читает кадры из up/, поэтому in/
//!   удаляется сразу после upscale (раньше, чтобы освободить место), а up/ —
//!   после encode (симметрично случаю с интерполяцией, где потребитель
//!   последней стадии перед encode всегда удаляется после неё).

use std::path::Path;

use crate::error::Result;

fn remove_dir_if_exists(dir: &Path) -> Result<()> {
    if dir.exists() {
        std::fs::remove_dir_all(dir)?;
    }
    Ok(())
}

pub fn remove_in_dir(seg_dir: &Path, keep_intermediate: bool) -> Result<()> {
    if keep_intermediate {
        return Ok(());
    }
    remove_dir_if_exists(&seg_dir.join("in"))
}

pub fn remove_up_dir(seg_dir: &Path, keep_intermediate: bool) -> Result<()> {
    if keep_intermediate {
        return Ok(());
    }
    remove_dir_if_exists(&seg_dir.join("up"))
}

pub fn remove_rife_dir(seg_dir: &Path, keep_intermediate: bool) -> Result<()> {
    if keep_intermediate {
        return Ok(());
    }
    remove_dir_if_exists(&seg_dir.join("rife"))
}

/// Полная очистка temp-корня джобы (после успешного concat, либо на
/// отмену/ошибку — вызывается pipeline::run).
pub fn remove_job_temp(temp_root: &Path, keep_intermediate: bool) -> Result<()> {
    if keep_intermediate {
        return Ok(());
    }
    remove_dir_if_exists(temp_root)
}
