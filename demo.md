# AllBeads CLI Demo

Config file location: `~/.config/allbeads/config.yaml`

## Setup alias for testing
```bash
alias ab='cargo run --quiet -- --cached'
```

## Commands

### Show statistics
```bash
ab stats
```

### List all beads
```bash
ab list
```

### Filter by status
```bash
ab list --status open
ab list --status closed
```

### Filter by priority
```bash
ab list --priority P1
ab list --priority 2
```

### Filter by context (when you have multiple)
```bash
ab list --context allbeads
```

### Show ready-to-work beads
```bash
ab ready
```

### Show details of a specific bead
```bash
ab show ab-oqy
```

### Clear cache (forces refresh next time)
```bash
ab clear-cache
```

## Debugging

To see INFO/DEBUG logs, set RUST_LOG:
```bash
RUST_LOG=info ab stats
RUST_LOG=debug ab list
```
