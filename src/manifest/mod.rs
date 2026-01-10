//! Manifest parsing for multi-repository configuration
//!
//! Parses XML manifests compatible with Google's git-repo tool,
//! with AllBeads-specific annotations for agent personas, bead prefixes,
//! and external integration mappings.
//!
//! # Example Manifest
//!
//! ```xml
//! <manifest>
//!   <remote name="origin" fetch="https://github.com/org" />
//!   <default revision="main" remote="origin" />
//!
//!   <project path="services/auth" name="backend/auth-service">
//!     <annotation key="allbeads.persona" value="security-specialist" />
//!     <annotation key="allbeads.prefix" value="auth" />
//!     <annotation key="allbeads.jira-project" value="SEC" />
//!   </project>
//! </manifest>
//! ```

mod parser;

pub use parser::{Annotation, Manifest, ManifestDefault, Project, Remote};
