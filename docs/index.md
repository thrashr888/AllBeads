# AllBeads Documentation

AllBeads is a meta-orchestration system that federates issue tracking (beads) across multiple git repositories, enabling AI agents to coordinate work across distributed microservices with unified dependency management.

## Documentation

### [Getting Started](./getting-started.md)

Installation, initial setup, and basic commands to get you up and running.

- Installing via Homebrew, binary, or source
- Initializing AllBeads
- Adding your first context
- Essential commands (stats, list, ready)

### [Core Concepts](./core-concepts.md)

Understand the fundamental concepts behind AllBeads.

- The Boss Repository pattern
- Beads and Shadow Beads
- Rigs (member repositories)
- Federated Graph
- Agent Mail system
- Sheriff daemon
- Health checks

### [CLI Reference](./cli-reference.md)

Complete reference for all AllBeads commands.

- Initialization
- Context management
- Viewing and searching beads
- TUI dashboard
- Sheriff daemon
- Agent mail
- Enterprise integration (JIRA, GitHub)
- Plugin system
- Coding agents and handoff
- Governance
- Sync and cache

### [Tutorials](./tutorials.md)

Step-by-step guides for common workflows.

1. Onboarding an existing repository
2. Setting up JIRA integration
3. Setting up GitHub integration
4. Using agent handoff
5. Running the Sheriff daemon
6. Using the TUI dashboard
7. Multi-repository coordination

## Quick Links

- **GitHub**: https://github.com/thrashr888/AllBeads
- **Issues**: Use `bd ready` to find work
- **Beads**: https://github.com/steveyegge/beads

## Getting Help

```bash
# Show all commands
ab --help

# Get help for a specific command
ab <command> --help
```
