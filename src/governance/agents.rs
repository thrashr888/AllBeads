//! Agent detection for repository governance
//!
//! Detects AI agent configurations in repositories to track adoption
//! and enforce agent-related policies.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Supported AI agents
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentType {
    /// Anthropic Claude (Claude Code, claude.ai)
    Claude,
    /// GitHub Copilot
    Copilot,
    /// Cursor IDE
    Cursor,
    /// Aider CLI
    Aider,
    /// Sourcegraph Cody
    Cody,
    /// Continue.dev
    Continue,
    /// Windsurf (Codeium)
    Windsurf,
    /// Amazon Q Developer
    AmazonQ,
    /// AWS Kiro
    Kiro,
    /// OpenCode
    OpenCode,
    /// Droid (Factory)
    Droid,
    /// OpenAI Codex
    Codex,
    /// Google Gemini CLI
    Gemini,
    /// Generic Agent (.agent - used by VSCode, Speckit, OpenAI, Antigravity, etc.)
    GenericAgent,
    /// Unknown or custom agent
    Unknown,
}

impl AgentType {
    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            AgentType::Claude => "Claude",
            AgentType::Copilot => "GitHub Copilot",
            AgentType::Cursor => "Cursor",
            AgentType::Aider => "Aider",
            AgentType::Cody => "Sourcegraph Cody",
            AgentType::Continue => "Continue",
            AgentType::Windsurf => "Windsurf",
            AgentType::AmazonQ => "Amazon Q",
            AgentType::Kiro => "AWS Kiro",
            AgentType::OpenCode => "OpenCode",
            AgentType::Droid => "Droid",
            AgentType::Codex => "OpenAI Codex",
            AgentType::Gemini => "Google Gemini",
            AgentType::GenericAgent => "Generic Agent",
            AgentType::Unknown => "Unknown",
        }
    }

    /// Get short identifier
    pub fn id(&self) -> &'static str {
        match self {
            AgentType::Claude => "claude",
            AgentType::Copilot => "copilot",
            AgentType::Cursor => "cursor",
            AgentType::Aider => "aider",
            AgentType::Cody => "cody",
            AgentType::Continue => "continue",
            AgentType::Windsurf => "windsurf",
            AgentType::AmazonQ => "amazonq",
            AgentType::Kiro => "kiro",
            AgentType::OpenCode => "opencode",
            AgentType::Droid => "droid",
            AgentType::Codex => "codex",
            AgentType::Gemini => "gemini",
            AgentType::GenericAgent => "agent",
            AgentType::Unknown => "unknown",
        }
    }
}

impl std::fmt::Display for AgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Confidence level of agent detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DetectionConfidence {
    /// Low confidence - heuristic detection
    Low,
    /// Medium confidence - indirect markers
    Medium,
    /// High confidence - config file exists
    High,
}

impl DetectionConfidence {
    pub fn symbol(&self) -> &'static str {
        match self {
            DetectionConfidence::Low => "?",
            DetectionConfidence::Medium => "~",
            DetectionConfidence::High => "✓",
        }
    }
}

/// Result of detecting an agent in a repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDetection {
    /// The detected agent type
    pub agent: AgentType,
    /// Confidence level of detection
    pub confidence: DetectionConfidence,
    /// Path to the config file (if found)
    pub config_path: Option<PathBuf>,
    /// Evidence supporting the detection
    pub evidence: Vec<String>,
}

impl AgentDetection {
    /// Create a new agent detection
    pub fn new(agent: AgentType, confidence: DetectionConfidence) -> Self {
        Self {
            agent,
            confidence,
            config_path: None,
            evidence: Vec::new(),
        }
    }

    /// Add config path
    pub fn with_config_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.config_path = Some(path.into());
        self
    }

    /// Add evidence
    pub fn with_evidence(mut self, evidence: impl Into<String>) -> Self {
        self.evidence.push(evidence.into());
        self
    }
}

/// Aggregate result of scanning a repository for agents
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentScanResult {
    /// All detected agents
    pub detections: Vec<AgentDetection>,
    /// Path that was scanned
    pub scanned_path: PathBuf,
}

impl AgentScanResult {
    /// Check if any agents were detected
    pub fn has_agents(&self) -> bool {
        !self.detections.is_empty()
    }

    /// Get agents with high confidence
    pub fn high_confidence_agents(&self) -> Vec<&AgentDetection> {
        self.detections
            .iter()
            .filter(|d| d.confidence == DetectionConfidence::High)
            .collect()
    }

    /// Check if a specific agent is detected
    pub fn has_agent(&self, agent: AgentType) -> bool {
        self.detections.iter().any(|d| d.agent == agent)
    }

    /// Get all agent types detected
    pub fn agent_types(&self) -> Vec<AgentType> {
        self.detections.iter().map(|d| d.agent).collect()
    }
}

/// Detect AI agents configured in a repository
pub fn detect_agents(repo_path: &Path) -> AgentScanResult {
    let mut result = AgentScanResult {
        detections: Vec::new(),
        scanned_path: repo_path.to_path_buf(),
    };

    // Claude detection
    if let Some(detection) = detect_claude(repo_path) {
        result.detections.push(detection);
    }

    // Copilot detection
    if let Some(detection) = detect_copilot(repo_path) {
        result.detections.push(detection);
    }

    // Cursor detection
    if let Some(detection) = detect_cursor(repo_path) {
        result.detections.push(detection);
    }

    // Aider detection
    if let Some(detection) = detect_aider(repo_path) {
        result.detections.push(detection);
    }

    // Cody detection
    if let Some(detection) = detect_cody(repo_path) {
        result.detections.push(detection);
    }

    // Continue detection
    if let Some(detection) = detect_continue(repo_path) {
        result.detections.push(detection);
    }

    // Windsurf detection
    if let Some(detection) = detect_windsurf(repo_path) {
        result.detections.push(detection);
    }

    // Amazon Q detection
    if let Some(detection) = detect_amazonq(repo_path) {
        result.detections.push(detection);
    }

    // Kiro detection
    if let Some(detection) = detect_kiro(repo_path) {
        result.detections.push(detection);
    }

    // OpenCode detection
    if let Some(detection) = detect_opencode(repo_path) {
        result.detections.push(detection);
    }

    // Droid detection
    if let Some(detection) = detect_droid(repo_path) {
        result.detections.push(detection);
    }

    // Codex detection
    if let Some(detection) = detect_codex(repo_path) {
        result.detections.push(detection);
    }

    // Gemini detection
    if let Some(detection) = detect_gemini(repo_path) {
        result.detections.push(detection);
    }

    // Generic Agent detection (last, as it's a catch-all)
    if let Some(detection) = detect_generic_agent(repo_path) {
        result.detections.push(detection);
    }

    result
}

/// Detect Claude configuration
fn detect_claude(repo_path: &Path) -> Option<AgentDetection> {
    let claude_md = repo_path.join("CLAUDE.md");
    let claude_dir = repo_path.join(".claude");
    let claude_plugin = repo_path.join(".claude-plugin");
    let claude_settings = repo_path.join(".claude/settings.json");

    if claude_md.exists() {
        return Some(
            AgentDetection::new(AgentType::Claude, DetectionConfidence::High)
                .with_config_path(&claude_md)
                .with_evidence("CLAUDE.md found"),
        );
    }

    if claude_settings.exists() {
        return Some(
            AgentDetection::new(AgentType::Claude, DetectionConfidence::High)
                .with_config_path(&claude_settings)
                .with_evidence(".claude/settings.json found"),
        );
    }

    if claude_dir.exists() {
        return Some(
            AgentDetection::new(AgentType::Claude, DetectionConfidence::High)
                .with_config_path(&claude_dir)
                .with_evidence(".claude/ directory found"),
        );
    }

    if claude_plugin.exists() {
        return Some(
            AgentDetection::new(AgentType::Claude, DetectionConfidence::High)
                .with_config_path(&claude_plugin)
                .with_evidence(".claude-plugin/ directory found"),
        );
    }

    None
}

/// Detect GitHub Copilot configuration
fn detect_copilot(repo_path: &Path) -> Option<AgentDetection> {
    let copilot_instructions = repo_path.join(".github/copilot-instructions.md");
    let copilot_config = repo_path.join(".github/.copilot");

    if copilot_instructions.exists() {
        return Some(
            AgentDetection::new(AgentType::Copilot, DetectionConfidence::High)
                .with_config_path(&copilot_instructions)
                .with_evidence(".github/copilot-instructions.md found"),
        );
    }

    if copilot_config.exists() {
        return Some(
            AgentDetection::new(AgentType::Copilot, DetectionConfidence::High)
                .with_config_path(&copilot_config)
                .with_evidence(".github/.copilot found"),
        );
    }

    None
}

/// Detect Cursor IDE configuration
fn detect_cursor(repo_path: &Path) -> Option<AgentDetection> {
    let cursorrules = repo_path.join(".cursorrules");
    let cursor_dir = repo_path.join(".cursor");
    let cursor_rules_dir = repo_path.join(".cursor/rules");

    if cursorrules.exists() {
        return Some(
            AgentDetection::new(AgentType::Cursor, DetectionConfidence::High)
                .with_config_path(&cursorrules)
                .with_evidence(".cursorrules found"),
        );
    }

    if cursor_rules_dir.exists() {
        return Some(
            AgentDetection::new(AgentType::Cursor, DetectionConfidence::High)
                .with_config_path(&cursor_rules_dir)
                .with_evidence(".cursor/rules/ directory found"),
        );
    }

    if cursor_dir.exists() {
        return Some(
            AgentDetection::new(AgentType::Cursor, DetectionConfidence::Medium)
                .with_config_path(&cursor_dir)
                .with_evidence(".cursor/ directory found"),
        );
    }

    None
}

/// Detect Aider configuration
fn detect_aider(repo_path: &Path) -> Option<AgentDetection> {
    let aider_conf = repo_path.join(".aider.conf.yml");
    let aider_conf_alt = repo_path.join(".aider.yaml");
    let aiderignore = repo_path.join(".aiderignore");

    if aider_conf.exists() {
        return Some(
            AgentDetection::new(AgentType::Aider, DetectionConfidence::High)
                .with_config_path(&aider_conf)
                .with_evidence(".aider.conf.yml found"),
        );
    }

    if aider_conf_alt.exists() {
        return Some(
            AgentDetection::new(AgentType::Aider, DetectionConfidence::High)
                .with_config_path(&aider_conf_alt)
                .with_evidence(".aider.yaml found"),
        );
    }

    if aiderignore.exists() {
        return Some(
            AgentDetection::new(AgentType::Aider, DetectionConfidence::Medium)
                .with_config_path(&aiderignore)
                .with_evidence(".aiderignore found"),
        );
    }

    None
}

/// Detect Sourcegraph Cody configuration
fn detect_cody(repo_path: &Path) -> Option<AgentDetection> {
    let cody_dir = repo_path.join(".cody");
    let cody_json = repo_path.join("cody.json");

    if cody_json.exists() {
        return Some(
            AgentDetection::new(AgentType::Cody, DetectionConfidence::High)
                .with_config_path(&cody_json)
                .with_evidence("cody.json found"),
        );
    }

    if cody_dir.exists() {
        return Some(
            AgentDetection::new(AgentType::Cody, DetectionConfidence::High)
                .with_config_path(&cody_dir)
                .with_evidence(".cody/ directory found"),
        );
    }

    None
}

/// Detect Continue.dev configuration
fn detect_continue(repo_path: &Path) -> Option<AgentDetection> {
    let continue_dir = repo_path.join(".continue");
    let continue_config = repo_path.join(".continue/config.json");
    let continueignore = repo_path.join(".continueignore");

    if continue_config.exists() {
        return Some(
            AgentDetection::new(AgentType::Continue, DetectionConfidence::High)
                .with_config_path(&continue_config)
                .with_evidence(".continue/config.json found"),
        );
    }

    if continue_dir.exists() {
        return Some(
            AgentDetection::new(AgentType::Continue, DetectionConfidence::Medium)
                .with_config_path(&continue_dir)
                .with_evidence(".continue/ directory found"),
        );
    }

    if continueignore.exists() {
        return Some(
            AgentDetection::new(AgentType::Continue, DetectionConfidence::Medium)
                .with_config_path(&continueignore)
                .with_evidence(".continueignore found"),
        );
    }

    None
}

/// Detect Windsurf (Codeium) configuration
fn detect_windsurf(repo_path: &Path) -> Option<AgentDetection> {
    let windsurf_dir = repo_path.join(".windsurf");
    let windsurf_rules = repo_path.join(".windsurf/rules.md");
    let codeium_config = repo_path.join(".codeium");

    if windsurf_rules.exists() {
        return Some(
            AgentDetection::new(AgentType::Windsurf, DetectionConfidence::High)
                .with_config_path(&windsurf_rules)
                .with_evidence(".windsurf/rules.md found"),
        );
    }

    if windsurf_dir.exists() {
        return Some(
            AgentDetection::new(AgentType::Windsurf, DetectionConfidence::High)
                .with_config_path(&windsurf_dir)
                .with_evidence(".windsurf/ directory found"),
        );
    }

    if codeium_config.exists() {
        return Some(
            AgentDetection::new(AgentType::Windsurf, DetectionConfidence::Medium)
                .with_config_path(&codeium_config)
                .with_evidence(".codeium config found (Windsurf/Codeium)"),
        );
    }

    None
}

/// Detect Amazon Q Developer configuration
fn detect_amazonq(repo_path: &Path) -> Option<AgentDetection> {
    let amazonq_dir = repo_path.join(".amazonq");
    let amazonq_rules = repo_path.join(".amazonq/rules");

    if amazonq_rules.exists() {
        return Some(
            AgentDetection::new(AgentType::AmazonQ, DetectionConfidence::High)
                .with_config_path(&amazonq_rules)
                .with_evidence(".amazonq/rules found"),
        );
    }

    if amazonq_dir.exists() {
        return Some(
            AgentDetection::new(AgentType::AmazonQ, DetectionConfidence::High)
                .with_config_path(&amazonq_dir)
                .with_evidence(".amazonq/ directory found"),
        );
    }

    None
}

/// Detect AWS Kiro configuration
fn detect_kiro(repo_path: &Path) -> Option<AgentDetection> {
    let kiro_dir = repo_path.join(".kiro");

    if kiro_dir.exists() {
        return Some(
            AgentDetection::new(AgentType::Kiro, DetectionConfidence::High)
                .with_config_path(&kiro_dir)
                .with_evidence(".kiro/ directory found"),
        );
    }

    None
}

/// Detect OpenCode configuration
fn detect_opencode(repo_path: &Path) -> Option<AgentDetection> {
    let opencode_json = repo_path.join("opencode.json");
    let opencode_dir = repo_path.join(".opencode");

    if opencode_json.exists() {
        return Some(
            AgentDetection::new(AgentType::OpenCode, DetectionConfidence::High)
                .with_config_path(&opencode_json)
                .with_evidence("opencode.json found"),
        );
    }

    if opencode_dir.exists() {
        return Some(
            AgentDetection::new(AgentType::OpenCode, DetectionConfidence::High)
                .with_config_path(&opencode_dir)
                .with_evidence(".opencode/ directory found"),
        );
    }

    None
}

/// Detect Droid (Factory) configuration
fn detect_droid(repo_path: &Path) -> Option<AgentDetection> {
    let factory_dir = repo_path.join(".factory");

    if factory_dir.exists() {
        return Some(
            AgentDetection::new(AgentType::Droid, DetectionConfidence::High)
                .with_config_path(&factory_dir)
                .with_evidence(".factory/ directory found"),
        );
    }

    None
}

/// Detect OpenAI Codex configuration
fn detect_codex(repo_path: &Path) -> Option<AgentDetection> {
    let codex_dir = repo_path.join(".codex");

    if codex_dir.exists() {
        return Some(
            AgentDetection::new(AgentType::Codex, DetectionConfidence::High)
                .with_config_path(&codex_dir)
                .with_evidence(".codex/ directory found"),
        );
    }

    None
}

/// Detect Google Gemini CLI configuration
fn detect_gemini(repo_path: &Path) -> Option<AgentDetection> {
    let gemini_dir = repo_path.join(".gemini");

    if gemini_dir.exists() {
        return Some(
            AgentDetection::new(AgentType::Gemini, DetectionConfidence::High)
                .with_config_path(&gemini_dir)
                .with_evidence(".gemini/ directory found"),
        );
    }

    None
}

/// Detect Generic Agent configuration (.agent)
/// Used by VSCode, Speckit, OpenAI, Google Antigravity, and others
fn detect_generic_agent(repo_path: &Path) -> Option<AgentDetection> {
    let agent_dir = repo_path.join(".agent");

    if agent_dir.exists() {
        return Some(
            AgentDetection::new(AgentType::GenericAgent, DetectionConfidence::High)
                .with_config_path(&agent_dir)
                .with_evidence(".agent/ directory found (VSCode/Speckit/OpenAI/Antigravity)"),
        );
    }

    None
}

/// Print agent detection results
pub fn print_agent_scan(result: &AgentScanResult) {
    if result.detections.is_empty() {
        println!("  No AI agents detected");
        return;
    }

    println!("  Detected {} agent(s):", result.detections.len());
    for detection in &result.detections {
        let conf = detection.confidence.symbol();
        let agent = detection.agent.name();
        print!("    [{conf}] {agent}");

        if let Some(ref path) = detection.config_path {
            // Show relative path if possible
            if let Ok(rel) = path.strip_prefix(&result.scanned_path) {
                print!(" - {}", rel.display());
            } else {
                print!(" - {}", path.display());
            }
        }
        println!();

        for evidence in &detection.evidence {
            println!("        {}", evidence);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_detect_claude_md() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("CLAUDE.md"), "# Claude Config").unwrap();

        let result = detect_agents(dir.path());
        assert!(result.has_agent(AgentType::Claude));
        assert_eq!(result.detections[0].confidence, DetectionConfidence::High);
    }

    #[test]
    fn test_detect_claude_dir() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join(".claude")).unwrap();

        let result = detect_agents(dir.path());
        assert!(result.has_agent(AgentType::Claude));
    }

    #[test]
    fn test_detect_copilot() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".github")).unwrap();
        fs::write(
            dir.path().join(".github/copilot-instructions.md"),
            "# Copilot",
        )
        .unwrap();

        let result = detect_agents(dir.path());
        assert!(result.has_agent(AgentType::Copilot));
    }

    #[test]
    fn test_detect_cursor() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".cursorrules"), "rules").unwrap();

        let result = detect_agents(dir.path());
        assert!(result.has_agent(AgentType::Cursor));
    }

    #[test]
    fn test_detect_aider() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join(".aider.conf.yml"), "config: true").unwrap();

        let result = detect_agents(dir.path());
        assert!(result.has_agent(AgentType::Aider));
    }

    #[test]
    fn test_detect_multiple_agents() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("CLAUDE.md"), "# Claude").unwrap();
        fs::write(dir.path().join(".cursorrules"), "rules").unwrap();

        let result = detect_agents(dir.path());
        assert_eq!(result.detections.len(), 2);
        assert!(result.has_agent(AgentType::Claude));
        assert!(result.has_agent(AgentType::Cursor));
    }

    #[test]
    fn test_no_agents() {
        let dir = TempDir::new().unwrap();

        let result = detect_agents(dir.path());
        assert!(!result.has_agents());
        assert!(result.detections.is_empty());
    }

    #[test]
    fn test_agent_type_display() {
        assert_eq!(AgentType::Claude.name(), "Claude");
        assert_eq!(AgentType::Copilot.name(), "GitHub Copilot");
        assert_eq!(AgentType::Claude.id(), "claude");
    }
}
