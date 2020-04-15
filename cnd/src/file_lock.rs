use anyhow::Context;
use fs2::FileExt;
use std::{fs::File, path::PathBuf};

pub trait TryLockExclusive {
    fn try_lock_exclusive(&self) -> anyhow::Result<File>;
}

impl TryLockExclusive for PathBuf {
    fn try_lock_exclusive(&self) -> anyhow::Result<File> {
        let file = File::open(self)?;
        file.try_lock_exclusive()
            .with_context(|| format!("Could not acquire file system lock on {}", self.display()))?;
        Ok(file)
    }
}
