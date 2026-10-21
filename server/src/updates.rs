//! One updater shared by the authenticated API and Windows tray.
use crate::Control;
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};
#[cfg(windows)]
#[path = "update.rs"]
mod native;

#[derive(Clone, Serialize)]
pub struct Status {
    pub state: String,
    pub message: String,
    pub management: String,
    pub available_version: Option<String>,
    pub can_check: bool,
    pub can_install: bool,
}
#[derive(Clone)]
pub struct Context {
    pub data: PathBuf,
    pub config: PathBuf,
    pub address: String,
    pub native_windows: bool,
    pub container: bool,
}
struct State {
    context: Option<Context>,
    status: Status,
    #[cfg(windows)]
    candidate: Option<native::Candidate>,
}
#[derive(Clone)]
pub struct Manager(Arc<Mutex<State>>);
impl Default for Manager {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(State {
            context: None,
            status: Status {
                state: "unsupported".into(),
                message: "Updates are managed outside Companion.".into(),
                management: "external".into(),
                available_version: None,
                can_check: false,
                can_install: false,
            },
            #[cfg(windows)]
            candidate: None,
        })))
    }
}
impl Manager {
    pub fn configure(&self, context: Context) {
        let mut state = self.0.lock().unwrap();
        state.status.management = if context.container {
            "container"
        } else if context.native_windows {
            "native_windows"
        } else {
            "external"
        }
        .into();
        if context.container {
            state.status.message = "Update the Docker image through Container Station or your container manager, retaining the mounted data folders.".into();
        }
        #[cfg(windows)]
        if context.native_windows {
            state.status.state = if native::configured() {
                "idle"
            } else {
                "not_configured"
            }
            .into();
            state.status.message = if native::configured() {
                "Ready to check for updates."
            } else {
                "No signed update channel is configured for this development build."
            }
            .into();
            state.status.can_check = true;
        }
        state.context = Some(context);
    }
    pub fn status(&self, control: &Control) -> Status {
        let state = self.0.lock().unwrap();
        let mut status = state.status.clone();
        status.can_install = status.state == "available"
            && control.active_engines() == 0
            && control.registrations.available_permits() > 0
            && !control.stopping();
        status
    }
    pub fn check(&self) -> Result<(), &'static str> {
        #[cfg(windows)]
        {
            let mut state = self.0.lock().unwrap();
            if !state.status.can_check {
                return Err("Update checking is unavailable or already running.");
            }
            let data = state.context.as_ref().ok_or("Companion update context is unavailable.")?.data.clone();
            state.status.state = "checking".into();
            state.status.message = "Checking for a signed update…".into();
            state.status.can_check = false;
            state.status.available_version = None;
            state.candidate = None;
            drop(state);
            let manager = self.clone();
            std::thread::spawn(move || {
                let result = native::check(&data);
                let mut state = manager.0.lock().unwrap();
                state.status.can_check = true;
                match result {
                    Ok(Some(candidate)) => {
                        state.status.state = "available".into();
                        state.status.available_version = Some(candidate.version.clone());
                        state.status.message = format!(
                            "Version {} is ready. Finish any game or analysis and close engine sessions before updating.",
                            candidate.version
                        );
                        state.candidate = Some(candidate);
                    }
                    Ok(None) => {
                        state.status.state = "up_to_date".into();
                        state.status.message = "Companion is up to date.".into();
                    }
                    Err(error) => {
                        state.status.state = if native::configured() {
                            "error"
                        } else {
                            "not_configured"
                        }
                        .into();
                        state.status.message = error.to_string();
                    }
                }
            });
            Ok(())
        }
        #[cfg(not(windows))]
        {
            Err("Use your container manager or native package to update Companion.")
        }
    }
    pub fn install(&self, control: &Control) -> Result<(), &'static str> {
        #[cfg(windows)]
        {
            let mut state = self.0.lock().unwrap();
            if state.status.state != "available" {
                return Err("Check for an available update first.");
            }
            let Some(reservation) = control.reserve_update() else {
                return Err(
                    "Finish the game or analysis and close engine sessions and engine validation before updating.",
                );
            };
            let candidate = state
                .candidate
                .clone()
                .ok_or("No verified update is available.")?;
            let context = state.context.clone().ok_or("Updater is not configured.")?;
            state.status.state = "installing".into();
            state.status.can_check = false;
            state.status.message =
                "Preparing the verified update. Companion will restart when preparation completes…"
                    .into();
            drop(state);
            let manager = self.clone();
            let control = control.clone();
            std::thread::spawn(move || {
                let _reservation = reservation;
                let result =
                    native::install(&candidate, &context.data, &context.config, &context.address);
                if let Err(error) = result {
                    let mut state = manager.0.lock().unwrap();
                    state.status.state = "available".into();
                    state.status.can_check = true;
                    state.status.message = format!("Update preparation failed: {error}");
                } else {
                    control.shutdown();
                }
            });
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = control;
            Err("Update the container image or native package externally.")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn control() -> Control {
        let mut config = crate::Config::defaults(false);
        config.archive = None; config.token = "a".repeat(64);
        crate::app_with_control(config).unwrap().1
    }
    #[test]
    fn shared_status_blocks_engines_and_external_installation() {
        let control = control();
        let manager = control.updates.clone();
        manager.configure(Context { data: PathBuf::new(), config: PathBuf::new(), address: String::new(), native_windows: false, container: true });
        assert_eq!(manager.status(&control).management,"container");
        assert!(manager.check().is_err()); assert!(manager.install(&control).is_err());
        manager.0.lock().unwrap().status.state = "available".into();
        assert!(manager.status(&control).can_install);
        let engine = control.engines.clone().try_acquire_owned().unwrap();
        assert!(!manager.status(&control).can_install); drop(engine);
        let registration = control.registrations.clone().try_acquire_owned().unwrap();
        assert!(!manager.status(&control).can_install); drop(registration);
        control.shutdown(); assert!(!manager.status(&control).can_install);
    }
}
