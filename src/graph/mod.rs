//! Core graph data structures
//!
//! Defines Bead, ShadowBead, Rig, and FederatedGraph types.

mod bead;
mod federated_graph;
mod ids;
mod rig;
mod shadow_bead;

pub use bead::Bead;
pub use federated_graph::FederatedGraph;
pub use ids::{BeadId, RigId};
pub use rig::Rig;
pub use shadow_bead::ShadowBead;
