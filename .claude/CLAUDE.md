# Claude Code Guidelines

## Git Commits

Keep commit messages concise and essential:

- Use conventional commit format (feat, fix, refactor, etc.)
- Focus on the "what" and "why", not implementation details
- Do NOT include:
  - Footer lines like "Generated with..."
  - "Co-Authored-By" lines
  - Excessive bullet points listing every file changed

### Good commit message example:

```
refactor: consolidate examples and remove unused crates

Delete mcp-cli crates (moved helpers to mcp-server).
Reduce examples from 19 to 5.

Closes #66, #68
```

### Bad commit message example:

```
refactor: major crate cleanup and example consolidation

## Deleted Crates
- mcp-cli and mcp-cli-derive: Thin wrappers around clap...
[20 more lines of details]

Footer: ...
Co-Authored-By: ...
```
