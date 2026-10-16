//! Integration test harness for Dust workspace discovery and configuration.

#[path = "workspace_tests/config.rs"]
mod config;
#[path = "workspace_tests/discovery.rs"]
mod discovery;
#[path = "workspace_tests/discovery_trivia.rs"]
mod discovery_trivia;

#[path = "workspace_tests/support.rs"]
mod support;
#[path = "workspace_tests/validation_discovery.rs"]
mod validation_discovery;
#[path = "workspace_tests/workspace.rs"]
mod workspace;
