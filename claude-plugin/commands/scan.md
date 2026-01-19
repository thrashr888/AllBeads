---
description: Scan GitHub repositories for AI agent onboarding opportunities
---

Scan GitHub user or organization repositories to identify onboarding opportunities.

Subcommands:
- `allbeads scan user <username>` - Scan a user's repositories
- `allbeads scan org <org-name>` - Scan an organization's repositories
- `allbeads scan compare` - Compare scanned repos with managed contexts

Options:
- `--language <lang>` - Filter by programming language
- `--min-stars <n>` - Minimum star count
- `--include-archived` - Include archived repositories
- `--include-forks` - Include forked repositories

The scanner detects 14 AI agent types:
- Claude Code, GitHub Copilot, Cursor, Aider, Kiro
- OpenAI Codex, Google Gemini, Amazon CodeWhisperer
- Tabnine, Codeium, Sourcegraph Cody, Replit AI
- JetBrains AI, Windsurf

Each repo is assigned an onboarding priority (High, Medium, Low, Skip) based on:
- Number of stars and recent activity
- Existing agent configurations
- Language and project type

Requires `GITHUB_TOKEN` environment variable for API access.
