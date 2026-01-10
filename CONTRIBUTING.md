# Contributing to AllBeads

Thank you for your interest in contributing to AllBeads! This document provides guidelines and instructions for contributing.

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment for everyone.

## Getting Started

### Prerequisites

- Rust 1.75 or later
- Git
- A GitHub account

### Development Setup

1. **Fork and clone the repository:**
   ```bash
   git clone https://github.com/YOUR_USERNAME/AllBeads.git
   cd AllBeads
   ```

2. **Build the project:**
   ```bash
   cargo build
   ```

3. **Run tests:**
   ```bash
   cargo test
   ```

4. **Run clippy (linter):**
   ```bash
   cargo clippy
   ```

5. **Format code:**
   ```bash
   cargo fmt
   ```

### Using AllBeads for Development (Dogfooding)

We use AllBeads to track issues for AllBeads itself! After building:

```bash
# Create an alias for convenience
alias ab='cargo run --quiet --'

# View current work
ab stats
ab ready
ab tui
```

## Making Changes

### Branch Naming

Use descriptive branch names:
- `feature/add-new-command` - New features
- `fix/resolve-cache-issue` - Bug fixes
- `docs/update-readme` - Documentation updates
- `refactor/simplify-graph` - Code refactoring

### Commit Messages

Follow conventional commit format:
```
type(scope): short description

Longer description if needed.

Co-Authored-By: Your Name <your.email@example.com>
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

### Pull Request Process

1. **Create a feature branch:**
   ```bash
   git checkout -b feature/your-feature
   ```

2. **Make your changes and test:**
   ```bash
   cargo test
   cargo clippy
   cargo fmt -- --check
   ```

3. **Commit your changes:**
   ```bash
   git add -A
   git commit -m "feat(scope): description"
   ```

4. **Push and create PR:**
   ```bash
   git push origin feature/your-feature
   ```

5. **Fill out the PR template** with:
   - Summary of changes
   - Related issues
   - Testing performed

## Code Style

### Rust Guidelines

- Follow Rust API guidelines
- Use `clippy` recommendations
- Prefer explicit types for public APIs
- Add documentation comments for public items
- Use `Result<T, E>` for fallible operations
- Avoid `unwrap()` in library code

### Error Handling

```rust
// Good: Use Result with context
fn load_config() -> Result<Config> {
    std::fs::read_to_string("config.yaml")
        .context("Failed to read config file")?;
    // ...
}

// Bad: Panicking
fn load_config() -> Config {
    std::fs::read_to_string("config.yaml").unwrap();
    // ...
}
```

### Testing

- Write unit tests for new functionality
- Integration tests go in `tests/` directory
- Use descriptive test names
- Test both success and error paths

```rust
#[test]
fn test_bead_creation_with_valid_input() {
    let bead = Bead::new("Test", Priority::P1);
    assert_eq!(bead.title, "Test");
    assert_eq!(bead.priority, Priority::P1);
}

#[test]
fn test_bead_creation_fails_with_empty_title() {
    let result = Bead::new("", Priority::P1);
    assert!(result.is_err());
}
```

## Architecture Overview

```
src/
├── main.rs              # CLI entry point
├── lib.rs               # Library root
├── aggregator/          # Multi-repo bead aggregation
├── cache/               # Local caching layer
├── config/              # Configuration management
├── graph/               # Bead graph data structures
├── integrations/        # JIRA, GitHub adapters
├── mail/                # Agent communication system
├── manifest/            # XML manifest parsing
├── sheriff/             # Background sync daemon
├── storage/             # Beads file format
├── swarm/               # Agent management
└── tui/                 # Terminal UI (ratatui)
```

## Adding New Features

### Adding a New CLI Command

1. Add the command to `Commands` enum in `src/main.rs`
2. Add a handler function `handle_X_command()`
3. Add the handler call in the `run()` match statement
4. Add tests if applicable
5. Update help text and documentation

### Adding a New TUI View

1. Create `src/tui/new_view.rs`
2. Add to `src/tui/mod.rs`
3. Add `Tab::NewView` variant
4. Update `App` struct with view state
5. Add drawing function in `ui.rs`
6. Handle keybindings in `mod.rs`

### Adding a New Integration

1. Create `src/integrations/new_service.rs`
2. Implement adapter trait pattern
3. Add configuration in `src/config/mod.rs`
4. Add CLI commands for the integration
5. Write integration tests

## Reporting Issues

### Bug Reports

Include:
- AllBeads version (`ab --version`)
- Operating system and version
- Steps to reproduce
- Expected vs actual behavior
- Relevant logs or error messages

### Feature Requests

Include:
- Use case description
- Proposed solution
- Alternative solutions considered
- Impact on existing functionality

## Getting Help

- **Documentation**: Check `CLAUDE.md` and `AGENTS.md`
- **Issues**: Search existing issues before creating new ones
- **Discussions**: Use GitHub Discussions for questions

## License

By contributing, you agree that your contributions will be licensed under the MIT License.

## Thank You!

Every contribution helps make AllBeads better. Whether it's fixing a typo, improving documentation, or adding new features - your help is appreciated!
