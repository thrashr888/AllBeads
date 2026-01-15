---
description: Detect and track AI agent usage across repositories
---

Detect AI agents and track adoption metrics.

Subcommands:
- `allbeads agents detect` - Detect AI agents in current directory
- `allbeads agents track --context <name> --path <path>` - Record scan to metrics
- `allbeads agents stats` - View adoption statistics
- `allbeads agents stats --days <n>` - Stats for specific time period
- `allbeads agents stats --json` - Output as JSON

Detects 14 AI agent types via config files:
- Claude Code (CLAUDE.md, .claude/)
- GitHub Copilot (.github/copilot-instructions.md)
- Cursor (.cursorrules, .cursor/)
- Aider (.aider.conf.yml)
- Kiro (.kiro/)
- And more...

Detection confidence levels:
- **High**: Primary config file found
- **Medium**: Secondary indicators
- **Low**: Indirect evidence

Usage tracking stores metrics in SQLite for trend analysis:
- Adoption rate over time
- Agent distribution across repos
- Scan history per repository
