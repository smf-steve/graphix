# Book Examples

This directory contains executable code examples from the Graphix book.

## Structure

- `tui/` - Terminal UI widget examples referenced in the book's TUI chapter

## Purpose

These examples serve two purposes:

1. **Documentation**: They are included in the mdbook via `{{#include ...}}` directives
2. **Verification**: They can be manually tested to ensure documentation stays accurate

## Testing Examples

Since these are visual TUI examples, they need to be tested manually:

```bash
# From the repository root
cargo run --bin graphix -- book/src/examples/tui/barchart_basic.gx
```

Some examples are code snippets that reference undefined variables (like `content`).
These are meant to illustrate specific concepts within a larger context and may not
run standalone. When updating the compiler, review these examples to ensure they
remain syntactically valid.

## Adding New Examples

When adding examples to the book:

1. Create the `.gx` file in the appropriate subdirectory under `book/src/examples/`
2. Reference it in the markdown using `{{#include ../../examples/.../filename.gx}}`
3. If the example should be runnable, test it manually with the graphix shell
4. If it's a code snippet, ensure it's syntactically valid for the current compiler
