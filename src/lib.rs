pub mod app;
pub mod cli;
pub mod model;
pub mod monitor;
pub mod processes;
pub mod query;
pub mod ui;

pub type AppResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
