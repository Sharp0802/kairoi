use std::error::Error;

pub type SendSyncError = Box<dyn Error + Send + Sync>;
