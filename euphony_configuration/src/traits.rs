//! This module contains a set of useful configuration validation traits.
//!
//! # Principle
//! All of the traits share one simple principle: a configuration should be loaded in two stages.
//!
//! In general, the first stage is done by using [`serde`] to load a raw configuration file into a private struct,
//! usually one with the prefix `Unresolved` (e.g. `UnresolvedConfiguration`).
//! That private (or `pub(crate)`, etc.) struct then implements `serde`'s
//! [`Deserialize`][serde::Deserialize] trait as well as one of the resolver traits, [`TryResolve`] for example.
//!
//! The [`TryResolve`] `impl` then specifies what output of the resolution step is, i.e. what struct
//! the original, unresolved configuration will attempt to resolve into and how (as well as what errors can pop up).
//!
//! The purpose of this two-step process is to allow more headroom for deserializing values from
//! the raw configuration file, potentially doing transformations and validation on the raw data
//! outside of the serde deserialization mechanism.
//!
//! # Traits
//! - [`Resolve`]: for infallible validation and transformation
//! - [`TryResolve`]: for fallible validation and transformation
//! - [`ResolveWithContext`]: for infallible validation and transformation, when additional context is required
//! - [`TryResolveWithContext`]: for fallible validation and transformation, when additional context is required
//!
//! # Example
//! See [`Configuration`][super::core::Configuration] for an implementation example.



/// A fallible validation and transformation trait.
///
/// See [module documentation][self] for more information.
pub trait TryResolve {
    /// The type an implementor of this trait (e.g. an unresolved configuration struct)
    /// resolves into.
    type Resolved;

    /// Error type that can be returned by the [`try_resolve`][Self::try_resolve] method.
    type Error;

    /// Attempts to resolve (validate, etc.) the provided "unresolved" configuration.
    fn try_resolve(self) -> Result<Self::Resolved, Self::Error>;
}

/// An infallible validation and transformation trait.
///
/// See [module documentation][self] for more information.
pub trait Resolve {
    /// The type an implementor of this trait (e.g. an unresolved configuration struct)
    /// resolves into.
    type Resolved;

    /// Resolves the provided "unresolved" type.
    fn resolve(self) -> Self::Resolved;
}


/// A fallible validation and transformation trait, with additional external context.
///
/// See [module documentation][self] for more information.
pub trait TryResolveWithContext {
    /// The type an implementor of this trait (e.g. an unresolved configuration struct)
    /// resolves into.
    type Resolved;

    /// Error type that can be returned by the [`try_resolve`] method.
    type Error;

    /// Type of the additional `context` argument passed to the [`Self::resolve`] method.
    type Context;

    /// Attempts to resolve (validate, etc.) the provided "unresolved" configuration.
    ///
    /// An additional argument named `context` of the `Self::Context` generic type is provided.
    ///
    /// The type and meaning of this argument are left up to the implementor.
    fn try_resolve(
        self,
        context: Self::Context,
    ) -> Result<Self::Resolved, Self::Error>;
}


/// An infallible validation and transformation trait, with additional external context.
///
/// See [module documentation][self] for more information.
pub trait ResolveWithContext {
    /// The type an implementor of this trait (e.g. an unresolved configuration struct)
    /// resolves into.
    type Resolved;

    /// Type of the additional `context` argument passed to the [`Self::resolve`] method.
    type Context;

    /// Resolves the provided "unresolved" type.
    ///
    /// An additional argument named `context` of the `Self::Context` generic type is provided.
    ///
    /// The type and meaning of this argument are left up to the implementor.
    fn resolve(self, context: Self::Context) -> Self::Resolved;
}
