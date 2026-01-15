//! State wrapper types for components.
//!
//! These wrappers make component state access patterns explicit and carry
//! app-level settings for convenient access throughout the component tree.

use std::ops::{Deref, DerefMut};

use crate::settings::Settings;

/// Immutable reference wrapper for component state.
///
/// Carries app-level settings and provides read-only access to component state.
/// Use in `view()` functions when components only need to read state.
#[derive(Debug, Clone, Copy)]
pub struct StateRef<'a, T> {
    inner: &'a T,
    settings: &'a Settings,
}

impl<'a, T> StateRef<'a, T> {
    /// Create a new immutable state wrapper.
    pub fn new(inner: &'a T, settings: &'a Settings) -> Self {
        Self { inner, settings }
    }

    /// Access the app-level settings.
    pub fn settings(&self) -> &Settings {
        self.settings
    }

    /// Map to a different state type while retaining settings.
    ///
    /// Useful for drilling into nested component state.
    pub fn map<U>(&self, f: impl FnOnce(&'a T) -> &'a U) -> StateRef<'a, U> {
        StateRef {
            inner: f(self.inner),
            settings: self.settings,
        }
    }

    /// Map with a closure that borrows self (shorter lifetime).
    pub fn map_ref<'b, U>(&'b self, f: impl FnOnce(&'b T) -> &'b U) -> StateRef<'b, U> {
        StateRef {
            inner: f(self.inner),
            settings: self.settings,
        }
    }
}

impl<T> Deref for StateRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner
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
}

impl<'a, T> StateMut<'a, T> {
    /// Create a new mutable state wrapper.
    pub fn new(inner: &'a mut T, settings: &'a mut Settings) -> Self {
        Self { inner, settings }
    }

    /// Access the app-level settings (immutable).
    pub fn settings(&self) -> &Settings {
        self.settings
    }

    /// Access the app-level settings (mutable).
    pub fn settings_mut(&mut self) -> &mut Settings {
        self.settings
    }

    /// Downgrade to an immutable reference.
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
        }
    }

    /// Reborrow as a shorter-lived StateMut.
    ///
    /// Useful when you need to pass mutable state to a child but keep using it after.
    pub fn reborrow(&mut self) -> StateMut<'_, T> {
        StateMut {
            inner: self.inner,
            settings: self.settings,
        }
    }
}

impl<T> Deref for StateMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<T> DerefMut for StateMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}
