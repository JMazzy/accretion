//! Simulation-specific error types.
//!
//! Systems should propagate errors through these types rather than panicking
//! where practical, enabling graceful degradation instead of hard crashes.
//!
//! ## Usage
//!
//! ```rust
//! use crate::error::SimError;
//!
//! fn some_system() -> Result<(), SimError> {
//!     let hull = compute_hull(&points)
//!         .ok_or(SimError::HullComputation { vertex_count: points.len() })?;
//!     Ok(())
//! }
//! ```

// This module provides infrastructure types for future error propagation.
// Items are public API; dead_code lint is suppressed to avoid forcing premature wiring.
#![allow(dead_code)]
use std::fmt;

/// Top-level error enum for the grav-sim simulation.
#[derive(Debug)]
pub enum SimError {
    /// Convex hull computation failed, usually because fewer than 2 non-duplicate
    /// input points were available.
    HullComputation {
        /// Number of input vertices passed to the hull algorithm.
        vertex_count: usize,
    },

    /// A Rapier entity was referenced but could not be found in the world.
    /// Often caused by a despawn race between the formation system and the hit system.
    EntityNotFound {
        /// Human-readable description of where the lookup occurred.
        context: &'static str,
    },

    /// A spawned asteroid would have too few vertices to form a valid collider.
    InsufficientVertices {
        /// Actual vertex count provided.
        got: usize,
        /// Minimum required.
        required: usize,
    },

    /// Physics constant is outside its safe operating range.
    /// Returned by validation helpers; not triggered at runtime by default.
    UnsafeConstant {
        /// Name of the constant (for logging).
        name: &'static str,
        /// The value that was rejected.
        value: f32,
        /// Human-readable description of the safe range.
        safe_range: &'static str,
    },
}

impl fmt::Display for SimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SimError::HullComputation { vertex_count } => write!(
                f,
                "convex hull computation failed: only {} usable vertices after deduplication \
                 (need ≥ 2)",
                vertex_count
            ),
            SimError::EntityNotFound { context } => {
                write!(f, "entity not found during '{}'", context)
            }
            SimError::InsufficientVertices { got, required } => write!(
                f,
                "asteroid vertex count too low: got {}, need at least {}",
                got, required
            ),
            SimError::UnsafeConstant {
                name,
                value,
                safe_range,
            } => write!(
                f,
                "constant '{}' = {} is outside safe range {}",
                name, value, safe_range
            ),
        }
    }
}

impl std::error::Error for SimError {}

/// Convenience alias: a `Result` using `SimError` as the error type.
pub type SimResult<T> = Result<T, SimError>;

// ── Validation helpers ────────────────────────────────────────────────────────

/// Returns an error if `gravity_const` is outside its validated safe range.
///
/// Values above 20.0 have been observed to cause runaway acceleration at close range.
pub fn validate_gravity_const(value: f32) -> SimResult<()> {
    if value <= 0.0 || value > 20.0 {
        Err(SimError::UnsafeConstant {
            name: "GRAVITY_CONST",
            value,
            safe_range: "(0.0, 20.0]",
        })
    } else {
        Ok(())
    }
}

/// Returns an error if `cull_distance` is not strictly positive.
pub fn validate_cull_distance(value: f32) -> SimResult<()> {
    if value <= 0.0 {
        Err(SimError::UnsafeConstant {
            name: "CULL_DISTANCE",
            value,
            safe_range: "(0.0, ∞)",
        })
    } else {
        Ok(())
    }
}
