# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.2.x   | :white_check_mark: |
| < 0.2   | :x:                |

## Reporting a Vulnerability

If you discover a security vulnerability in AllBeads, please report it responsibly:

1. **Do NOT** open a public GitHub issue for security vulnerabilities
2. Email security concerns to the maintainer directly
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Any suggested fixes (optional)

### Response Timeline

- **Initial response**: Within 48 hours
- **Status update**: Within 7 days
- **Fix timeline**: Depends on severity
  - Critical: 24-48 hours
  - High: 7 days
  - Medium: 30 days
  - Low: Next release

## Security Considerations

### Authentication

AllBeads supports multiple authentication strategies for git operations:
- SSH Agent (recommended)
- Personal Access Tokens
- SSH Keys

**Best Practices:**
- Use SSH Agent for interactive use
- Use scoped Personal Access Tokens for CI/CD
- Never commit credentials to the repository

### Data Storage

- Beads data is stored in `.beads/` directories within git repositories
- Cache data is stored in `~/.config/allbeads/`
- SQLite databases contain cached bead data only (no credentials)

### External Integrations

When using JIRA or GitHub integrations:
- API tokens are stored in the user's config directory
- Tokens should have minimum required permissions
- Review token scopes before granting access

## Security Features

### Janitor Analysis

The `allbeads janitor` command scans for potential security issues:
- Hardcoded secrets in source code
- SQL injection patterns
- Unsafe eval usage

Run periodically to catch potential issues early.
