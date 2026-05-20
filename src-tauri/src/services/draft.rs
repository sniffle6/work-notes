use std::sync::{Arc, Mutex};

use super::{ServiceError, ServiceResult};

#[derive(Clone, Default)]
pub struct DraftService {
    current: Arc<Mutex<Option<String>>>,
}

impl DraftService {
    pub fn save(&self, value: impl Into<String>) -> ServiceResult<()> {
        let mut current = self
            .current
            .lock()
            .map_err(|_| ServiceError::StatePoisoned { name: "draft" })?;
        *current = Some(value.into());
        Ok(())
    }

    pub fn get(&self) -> ServiceResult<Option<String>> {
        let current = self
            .current
            .lock()
            .map_err(|_| ServiceError::StatePoisoned { name: "draft" })?;
        Ok(current.clone())
    }

    pub fn clear(&self) -> ServiceResult<()> {
        let mut current = self
            .current
            .lock()
            .map_err(|_| ServiceError::StatePoisoned { name: "draft" })?;
        *current = None;
        Ok(())
    }
}
