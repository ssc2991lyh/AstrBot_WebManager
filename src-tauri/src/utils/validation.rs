use crate::error::{AppError, Result};

pub(crate) fn validate_instance_id(instance_id: &str) -> Result<()> {
    if uuid::Uuid::parse_str(instance_id).is_err() {
        return Err(AppError::other("Invalid instance id"));
    }
    Ok(())
}
