---
description: Check and enforce governance policies for AI agent adoption
---

Manage governance policies for AI agent adoption across repositories.

Subcommands:
- `allbeads governance check` - Run policy checks against current repository
- `allbeads governance status` - View loaded policies and exemptions
- `allbeads governance violations` - List all policy violations
- `allbeads governance exempt <repo> <policy> --reason "..."` - Exempt a repo
- `allbeads governance unexempt <repo> <policy>` - Remove exemption

Policy enforcement levels (HCP Terraform-inspired):
- **Advisory**: Warn but don't block
- **SoftMandatory**: Block unless exempted
- **HardMandatory**: Always block, no exemptions

Built-in policy checks:
- `FileExists` - Required files (README, LICENSE, etc.)
- `FileExistsAny` - At least one of several files
- `OnboardingScore` - Minimum onboarding completeness
- `AgentAllowlist` - Permitted/denied agent types
- `PatternAbsent` - Ensure patterns don't exist in files

Configure policies in `.beads/policies.yaml` or `~/.config/allbeads/policies.yaml`.
