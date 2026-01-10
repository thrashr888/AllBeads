//! XML manifest parser for git-repo compatible manifests

use crate::{AllBeadsError, Result};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use std::path::Path;

/// A parsed manifest file
#[derive(Debug, Clone, Default)]
pub struct Manifest {
    /// Remote repositories
    pub remotes: Vec<Remote>,

    /// Default settings for projects
    pub default: Option<ManifestDefault>,

    /// Projects (Rigs) in this manifest
    pub projects: Vec<Project>,
}

/// Remote repository definition
#[derive(Debug, Clone)]
pub struct Remote {
    /// Remote name (e.g., "origin")
    pub name: String,

    /// Fetch URL base (e.g., "https://github.com/org")
    pub fetch: String,

    /// Review URL for code review (optional)
    pub review: Option<String>,
}

/// Default settings for projects
#[derive(Debug, Clone)]
pub struct ManifestDefault {
    /// Default revision/branch (e.g., "main")
    pub revision: String,

    /// Default remote name
    pub remote: String,

    /// Default sync behavior
    pub sync_j: Option<u32>,
}

/// A project (Rig) in the manifest
#[derive(Debug, Clone)]
pub struct Project {
    /// Local path for the project
    pub path: String,

    /// Repository name (relative to remote fetch URL)
    pub name: String,

    /// Override revision for this project
    pub revision: Option<String>,

    /// Override remote for this project
    pub remote: Option<String>,

    /// AllBeads-specific annotations
    pub annotations: Vec<Annotation>,
}

/// AllBeads-specific annotation
#[derive(Debug, Clone)]
pub struct Annotation {
    /// Annotation key (e.g., "allbeads.persona")
    pub key: String,

    /// Annotation value
    pub value: String,
}

impl Project {
    /// Get the agent persona for this project
    pub fn persona(&self) -> Option<&str> {
        self.get_annotation("allbeads.persona")
    }

    /// Get the bead prefix for this project
    pub fn prefix(&self) -> Option<&str> {
        self.get_annotation("allbeads.prefix")
    }

    /// Get the JIRA project key for this project
    pub fn jira_project(&self) -> Option<&str> {
        self.get_annotation("allbeads.jira-project")
    }

    /// Get the GitHub repo for this project
    pub fn github_repo(&self) -> Option<&str> {
        self.get_annotation("allbeads.github-repo")
    }

    /// Get an annotation by key
    pub fn get_annotation(&self, key: &str) -> Option<&str> {
        self.annotations
            .iter()
            .find(|a| a.key == key)
            .map(|a| a.value.as_str())
    }

    /// Get the full repository URL given a remote
    pub fn full_url(&self, remote: &Remote) -> String {
        if self.name.starts_with("http://")
            || self.name.starts_with("https://")
            || self.name.starts_with("git@")
        {
            self.name.clone()
        } else {
            format!("{}/{}", remote.fetch.trim_end_matches('/'), &self.name)
        }
    }
}

impl Manifest {
    /// Parse a manifest from XML content
    pub fn parse(xml: &str) -> Result<Self> {
        let mut reader = Reader::from_str(xml);
        reader.config_mut().trim_text(true);

        let mut manifest = Manifest::default();
        let mut current_project: Option<Project> = None;

        loop {
            match reader.read_event() {
                Ok(Event::Empty(ref e)) => {
                    // Self-closing tags like <remote ... /> or <project ... />
                    match e.name().as_ref() {
                        b"remote" => {
                            manifest.remotes.push(parse_remote(e)?);
                        }
                        b"default" => {
                            manifest.default = Some(parse_default(e)?);
                        }
                        b"project" => {
                            // Self-closing project (no annotations)
                            manifest.projects.push(parse_project(e)?);
                        }
                        b"annotation" => {
                            if let Some(ref mut project) = current_project {
                                project.annotations.push(parse_annotation(e)?);
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::Start(ref e)) => {
                    // Opening tags like <project>
                    match e.name().as_ref() {
                        b"remote" => {
                            manifest.remotes.push(parse_remote(e)?);
                        }
                        b"default" => {
                            manifest.default = Some(parse_default(e)?);
                        }
                        b"project" => {
                            // Project with children (annotations)
                            current_project = Some(parse_project(e)?);
                        }
                        b"annotation" => {
                            if let Some(ref mut project) = current_project {
                                project.annotations.push(parse_annotation(e)?);
                            }
                        }
                        _ => {}
                    }
                }
                Ok(Event::End(ref e)) => {
                    if e.name().as_ref() == b"project" {
                        if let Some(project) = current_project.take() {
                            manifest.projects.push(project);
                        }
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    return Err(AllBeadsError::Parse(format!(
                        "Error parsing manifest XML: {}",
                        e
                    )));
                }
                _ => {}
            }
        }

        Ok(manifest)
    }

    /// Parse a manifest from a file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)?;
        Self::parse(&content)
    }

    /// Get the default remote
    pub fn default_remote(&self) -> Option<&Remote> {
        let remote_name = self.default.as_ref()?.remote.as_str();
        self.remotes.iter().find(|r| r.name == remote_name)
    }

    /// Get a remote by name
    pub fn get_remote(&self, name: &str) -> Option<&Remote> {
        self.remotes.iter().find(|r| r.name == name)
    }

    /// Get the effective remote for a project
    pub fn project_remote(&self, project: &Project) -> Option<&Remote> {
        if let Some(ref remote_name) = project.remote {
            self.get_remote(remote_name)
        } else {
            self.default_remote()
        }
    }

    /// Get the effective revision for a project
    pub fn project_revision(&self, project: &Project) -> Option<String> {
        project
            .revision
            .clone()
            .or_else(|| self.default.as_ref().map(|d| d.revision.clone()))
    }
}

fn get_attr(e: &BytesStart, name: &[u8]) -> Result<Option<String>> {
    for attr in e.attributes() {
        let attr = attr.map_err(|e| AllBeadsError::Parse(format!("Invalid attribute: {}", e)))?;
        if attr.key.as_ref() == name {
            let value = attr
                .unescape_value()
                .map_err(|e| AllBeadsError::Parse(format!("Invalid attribute value: {}", e)))?;
            return Ok(Some(value.to_string()));
        }
    }
    Ok(None)
}

fn require_attr(e: &BytesStart, name: &[u8]) -> Result<String> {
    get_attr(e, name)?.ok_or_else(|| {
        AllBeadsError::Parse(format!(
            "Missing required attribute: {}",
            String::from_utf8_lossy(name)
        ))
    })
}

fn parse_remote(e: &BytesStart) -> Result<Remote> {
    Ok(Remote {
        name: require_attr(e, b"name")?,
        fetch: require_attr(e, b"fetch")?,
        review: get_attr(e, b"review")?,
    })
}

fn parse_default(e: &BytesStart) -> Result<ManifestDefault> {
    Ok(ManifestDefault {
        revision: require_attr(e, b"revision")?,
        remote: require_attr(e, b"remote")?,
        sync_j: get_attr(e, b"sync-j")?.and_then(|s| s.parse().ok()),
    })
}

fn parse_project(e: &BytesStart) -> Result<Project> {
    Ok(Project {
        path: require_attr(e, b"path")?,
        name: require_attr(e, b"name")?,
        revision: get_attr(e, b"revision")?,
        remote: get_attr(e, b"remote")?,
        annotations: Vec::new(),
    })
}

fn parse_annotation(e: &BytesStart) -> Result<Annotation> {
    Ok(Annotation {
        key: require_attr(e, b"key")?,
        value: require_attr(e, b"value")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE_MANIFEST: &str = r#"
        <manifest>
            <remote name="origin" fetch="https://github.com/org" />
            <remote name="backup" fetch="https://gitlab.com/org" review="https://review.example.com" />
            <default revision="main" remote="origin" sync-j="4" />

            <project path="services/auth" name="backend/auth-service">
                <annotation key="allbeads.persona" value="security-specialist" />
                <annotation key="allbeads.prefix" value="auth" />
                <annotation key="allbeads.jira-project" value="SEC" />
            </project>

            <project path="services/api" name="backend/api-gateway" revision="develop">
                <annotation key="allbeads.persona" value="api-developer" />
                <annotation key="allbeads.prefix" value="api" />
            </project>

            <project path="frontend/web" name="frontend/web-app" remote="backup" />
        </manifest>
    "#;

    #[test]
    fn test_parse_manifest() {
        let manifest = Manifest::parse(EXAMPLE_MANIFEST).unwrap();

        // Check remotes
        assert_eq!(manifest.remotes.len(), 2);
        assert_eq!(manifest.remotes[0].name, "origin");
        assert_eq!(manifest.remotes[0].fetch, "https://github.com/org");
        assert_eq!(manifest.remotes[1].name, "backup");
        assert!(manifest.remotes[1].review.is_some());

        // Check default
        let default = manifest.default.as_ref().unwrap();
        assert_eq!(default.revision, "main");
        assert_eq!(default.remote, "origin");
        assert_eq!(default.sync_j, Some(4));

        // Check projects
        assert_eq!(manifest.projects.len(), 3);

        let auth = &manifest.projects[0];
        assert_eq!(auth.path, "services/auth");
        assert_eq!(auth.name, "backend/auth-service");
        assert_eq!(auth.persona(), Some("security-specialist"));
        assert_eq!(auth.prefix(), Some("auth"));
        assert_eq!(auth.jira_project(), Some("SEC"));

        let api = &manifest.projects[1];
        assert_eq!(api.revision, Some("develop".to_string()));

        let web = &manifest.projects[2];
        assert_eq!(web.remote, Some("backup".to_string()));
    }

    #[test]
    fn test_project_full_url() {
        let remote = Remote {
            name: "origin".to_string(),
            fetch: "https://github.com/org".to_string(),
            review: None,
        };

        let project = Project {
            path: "services/auth".to_string(),
            name: "backend/auth-service".to_string(),
            revision: None,
            remote: None,
            annotations: vec![],
        };

        assert_eq!(
            project.full_url(&remote),
            "https://github.com/org/backend/auth-service"
        );
    }

    #[test]
    fn test_default_remote() {
        let manifest = Manifest::parse(EXAMPLE_MANIFEST).unwrap();

        let default_remote = manifest.default_remote().unwrap();
        assert_eq!(default_remote.name, "origin");
    }

    #[test]
    fn test_project_remote() {
        let manifest = Manifest::parse(EXAMPLE_MANIFEST).unwrap();

        // First project uses default remote
        let auth = &manifest.projects[0];
        let auth_remote = manifest.project_remote(auth).unwrap();
        assert_eq!(auth_remote.name, "origin");

        // Third project overrides remote
        let web = &manifest.projects[2];
        let web_remote = manifest.project_remote(web).unwrap();
        assert_eq!(web_remote.name, "backup");
    }

    #[test]
    fn test_project_revision() {
        let manifest = Manifest::parse(EXAMPLE_MANIFEST).unwrap();

        // First project uses default revision
        let auth = &manifest.projects[0];
        assert_eq!(manifest.project_revision(auth), Some("main".to_string()));

        // Second project overrides revision
        let api = &manifest.projects[1];
        assert_eq!(manifest.project_revision(api), Some("develop".to_string()));
    }
}
