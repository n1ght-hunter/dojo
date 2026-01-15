//! State wrapper types for components.
//!
//! These wrappers make component state access patterns explicit and carry
//! app-level settings for convenient access throughout the component tree.

use std::ops::{Deref, DerefMut};

use tokio::sync::mpsc;

use crate::settings::Settings;
use dojo_jj::WorkspaceCommand;

/// Immutable reference wrapper for component state.
///
/// Carries app-level settings and provides read-only access to component state.
/// Use in `view()` functions when components only need to read state.
#[allow(dead_code)]
#[derive(Debug)]
pub struct StateRef<'a, T> {
    state: &'a T,
    settings: &'a Settings,
}

impl<T> Clone for StateRef<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for StateRef<'_, T> {}

#[allow(dead_code)]
impl<'a, T> StateRef<'a, T> {
    /// Create a new immutable state wrapper.
    pub fn new(state: &'a T, settings: &'a Settings) -> Self {
        Self { state, settings }
    }

    /// Access the wrapped state with the original lifetime.
    pub fn state(&self) -> &'a T {
        self.state
    }

    /// Access the app-level settings.
    pub fn settings(&self) -> &'a Settings {
        self.settings
    }

    /// Map to a different state type while retaining settings.
    ///
    /// Useful for drilling into nested component state.
    pub fn map<U>(&self, f: impl FnOnce(&'a T) -> &'a U) -> StateRef<'a, U> {
        StateRef {
            state: f(self.state),
            settings: self.settings,
        }
    }

    /// Map with a closure that borrows self (shorter lifetime).
    pub fn map_ref<'b, U>(&'b self, f: impl FnOnce(&'b T) -> &'b U) -> StateRef<'b, U> {
        StateRef {
            state: f(self.state),
            settings: self.settings,
        }
    }
}

impl<T> Deref for StateRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.state
    }
}

/// Mutable reference wrapper for component state.
///
/// Carries mutable app-level settings and provides read-write access to component state.
/// Use in `update()` functions when components need to modify state.
#[derive(Debug)]
pub struct StateMut<'a, T> {
    inner: &'a mut T,
    settings: &'a mut Settings,
    /// Optional command sender for worker communication (cloned, not borrowed)
    worker_tx: Option<mpsc::Sender<WorkspaceCommand>>,
}

impl<'a, T> StateMut<'a, T> {
    /// Create a new mutable state wrapper (without worker).
    pub fn new(inner: &'a mut T, settings: &'a mut Settings) -> Self {
        Self {
            inner,
            settings,
            worker_tx: None,
        }
    }

    /// Add worker command sender (builder pattern). Clones the sender.
    pub fn with_worker(mut self, tx: &mpsc::Sender<WorkspaceCommand>) -> Self {
        self.worker_tx = Some(tx.clone());
        self
    }

    /// Send a command to the worker (fire and forget).
    /// Panics if no worker is attached.
    #[allow(dead_code)]
    pub fn send_command(&self, cmd: WorkspaceCommand) {
        let tx = self
            .worker_tx
            .as_ref()
            .expect("send_command called without worker attached");
        let _ = tx.try_send(cmd);
    }

    /// Get a clone of the worker sender for Task-based commands.
    /// Panics if no worker is attached.
    #[allow(dead_code)]
    pub fn worker_tx(&self) -> mpsc::Sender<WorkspaceCommand> {
        self.worker_tx
            .clone()
            .expect("worker_tx called without worker attached")
    }

    /// Access the app-level settings (immutable).
    #[allow(dead_code)]
    pub fn settings(&self) -> &Settings {
        self.settings
    }

    /// Access the app-level settings (mutable).
    #[allow(dead_code)]
    pub fn settings_mut(&mut self) -> &mut Settings {
        self.settings
    }

    /// Downgrade to an immutable reference.
    #[allow(dead_code)]
    pub fn as_ref(&self) -> StateRef<'_, T> {
        StateRef::new(self.inner, self.settings)
    }

    /// Map to a different state type while retaining settings (mutable).
    ///
    /// Consumes self because we can only have one mutable reference.
    pub fn map<U>(self, f: impl FnOnce(&'a mut T) -> &'a mut U) -> StateMut<'a, U> {
        StateMut {
            inner: f(self.inner),
            settings: self.settings,
            worker_tx: self.worker_tx,
        }
    }

    /// Reborrow as a shorter-lived StateMut.
    ///
    /// Useful when you need to pass mutable state to a child but keep using it after.
    pub fn reborrow(&mut self) -> StateMut<'_, T> {
        StateMut {
            inner: self.inner,
            settings: self.settings,
            worker_tx: self.worker_tx.clone(),
        }
    }
}

impl<'a, T> Deref for StateMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'a, T> DerefMut for StateMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}
