# Graphix Book TODO List

This document tracks improvements and additions needed for the Graphix language documentation.

**Last Updated**: 2025-10-23

## Completed ✅

- [x] Create `style.md` documentation for TUI styling
- [x] Create `input.md` documentation for input handling
- [x] Create Getting Started tutorial (`getting_started.md`)
- [x] Create Reading Type Signatures guide (`core/reading_types.md`)

---

## Phase 1: Critical Fixes (Highest Priority)

These issues block usability or create confusion:

### 1.1 Fix Table of Contents Structure
- [ ] Move `error.md` to `./core/error.md` in SUMMARY.md
- **Rationale**: Error handling is a core language feature, not a top-level topic
- **Effort**: 5 minutes
- **Files**: `book/src/SUMMARY.md`, `book/src/error.md` → `book/src/core/error.md`

### 1.2 Complete Embedding Overview
- [ ] Write a proper introduction to the Embedding section
- **Content needed**:
  - Why embed Graphix? (scripting, DSL, config languages)
  - When to embed vs standalone
  - Brief overview of the three approaches (builtins, shell, rt)
  - Link to the three subsections
- **Effort**: 1 hour
- **File**: `book/src/embedding/overview.md`

### 1.3 Clarify Installation Instructions
- [ ] Update install.md with:
  - Explicit statement about crates.io publication status
  - If not published, add git installation: `cargo install --git https://github.com/...`
  - Add troubleshooting subsection for common issues
  - Verify platform-specific dependency names
- **Effort**: 30 minutes
- **File**: `book/src/install.md`

### 1.4 Sanitize Example Outputs
- [ ] Replace `/home/eric/` paths in error examples with generic paths
- **Find**: `grep -r "/home/eric" book/src/`
- **Replace with**: `/home/user/` or just `~/`
- **Effort**: 15 minutes
- **Files**: Multiple (mainly core language sections)

---

## Phase 2: Essential Content (High Priority)

Core documentation that users expect:

### 2.1 Add "Getting Started" Tutorial ✅
- [x] Create new section between "Installing" and "Core Language"
- [x] Content: REPL basics, first file, reactive model
- **Status**: COMPLETED

### 2.2 Add Type Signature Reading Guide ✅
- [x] Create guide for understanding function signatures
- [x] Content: All notation explained with examples
- **Status**: COMPLETED

### 2.3 Standardize API Documentation Format
- [ ] Review all widget docs and stdlib docs for consistent format
- **Standard**: All should use `mod name: sig { ... }` wrapper
- **Check**: All TUI widget files, stdlib files
- **Effort**: 2-3 hours (mostly mechanical)
- **Files**: All files in `book/src/ui/tui/` and `book/src/stdlib/`

### 2.4 Explain z32 and v32 Types
- [ ] Document these in fundamental_types.md
- **Research**: What are z32 and v32? (appear in core.md Sint/Uint but not documented)
- **Effort**: 30 minutes to 1 hour (depending on research needed)
- **File**: `book/src/core/fundamental_types.md`

### 2.5 Fix Intro Incomplete Sentence
- [ ] Complete "like the popular UI library" comparison
- **Suggestion**: "like React or Vue, except at the language level instead of just as a library"
- **Effort**: 2 minutes
- **File**: `book/src/intro.md`

---

## Phase 3: Quality Improvements (Medium Priority)

Polish and consistency:

### 3.1 Add Operators Quick Reference
- [ ] Create appendix with operator table
- **Content**:
  - `<-` (connect), `~` (sample), `?` (try/throw), `$` (or never)
  - `->` (lambda/function return), `=>` (select arm, map literal)
  - `..` (array slice)
  - All arithmetic, comparison, boolean operators
  - Precedence and associativity
- **Effort**: 2 hours
- **Files**: Create `book/src/appendix/operators.md`, update SUMMARY.md

### 3.2 Add Syntax Quick Reference
- [ ] Create appendix with syntax patterns
- **Content**:
  - Let binds, functions, select expressions
  - Structs, tuples, arrays, maps
  - Type annotations, modules
- **Effort**: 2 hours
- **Files**: Create `book/src/appendix/syntax_reference.md`

### 3.3 Standardize Code Block Language Tags
- [ ] Ensure all Graphix code uses ` ```graphix `
- **Find**: ` ``` ` followed by code that looks like Graphix
- **Effort**: 1 hour
- **Files**: All markdown files

### 3.4 Add Cross-References
- [ ] Add more internal links between sections
- **Priority links**:
  - connect.md → select.md and vice versa
  - error.md → functions (for throws)
  - fundamental_types.md → error handling
  - All core language sections should link to each other where relevant
- **Effort**: 1-2 hours
- **Files**: Core language sections primarily

### 3.5 Improve Graph Diagrams
- [ ] Consider enhancing ASCII diagrams
- **Options**:
  - Keep ASCII but ensure consistent formatting
  - Use Mermaid diagram syntax
  - Create actual images
- **Effort**: Variable (2-8 hours depending on approach)
- **Files**: `intro.md`, `connect.md`, other sections with diagrams

---

## Phase 4: Content Expansion (Lower Priority)

Nice-to-have additions:

### 4.1 Add Examples Gallery
- [ ] Document what's in `graphix-shell/examples/`
- **Content**:
  - List all examples with brief descriptions
  - Categorize (beginner, intermediate, advanced)
  - Note which demonstrate which features
- **Effort**: 2-3 hours
- **Files**: Create `book/src/examples_gallery.md` or appendix section

### 4.2 Add Module System Practical Guide
- [ ] Expand modules section with real examples
- **Content**:
  - Multi-file project example
  - Directory structure recommendations
  - Common patterns
  - Module reuse strategies
- **Effort**: 3-4 hours
- **Files**: `book/src/modules/practical.md` or expand existing files

### 4.3 Add Performance Guide
- [ ] Create new section about performance
- **Content**:
  - Memory pooling explained in depth
  - Array operations complexity (already mentioned, expand)
  - Map operations complexity
  - When to use which data structure
  - Lazy evaluation implications
  - Profiling techniques
- **Effort**: 4-6 hours
- **Files**: Create `book/src/performance.md` or appendix

### 4.4 Add Common Patterns Section
- [ ] For each stdlib module, add "Common Patterns"
- **Content**:
  - Array transformations
  - Map manipulations
  - String processing
  - Error handling patterns
  - Reactive patterns with connect
- **Effort**: 4-6 hours
- **Files**: All stdlib module files

### 4.5 Add FAQ
- [ ] Create FAQ section
- **Content**: Gather common questions from users
- **Effort**: 2-3 hours (plus ongoing maintenance)
- **Files**: Create `book/src/faq.md`

### 4.6 Add Best Practices Guide
- [ ] Document Graphix idioms
- **Content**:
  - When to use select vs if
  - Naming conventions
  - Module organization
  - Error handling strategy
  - Anti-patterns to avoid
- **Effort**: 3-4 hours
- **Files**: Create `book/src/best_practices.md`

---

## Phase 5: Validation (Ongoing)

Ensure quality:

### 5.1 Verify All Example Files Exist
- [ ] Check every `{{#include ...}}` reference
- **Script**: Could write a script to validate
- **Action**: Create missing examples or mark as conceptual
- **Effort**: 3-4 hours
- **Files**: All TUI examples primarily

### 5.2 Test All Example Code
- [ ] Run `cargo run --bin graphix --check` on all examples
- **Action**: Fix any that don't compile
- **Effort**: 2-4 hours (depending on issues found)
- **Files**: All .gx example files

### 5.3 Review for Consistency
- [ ] Read through entire book for tone, terminology, formatting
- **Check**:
  - Consistent capitalization (Graphix vs graphix)
  - Consistent terminology
  - Consistent formatting
  - Proper grammar/spelling
- **Effort**: 4-6 hours
- **Files**: All files

### 5.4 External Review
- [ ] Have someone unfamiliar with Graphix read through
- **Goal**: Identify confusing sections, gaps in understanding
- **Effort**: Variable (depends on reviewer availability)

---

## Timeline Summary

| Phase | Priority | Duration | Can Start |
|-------|----------|----------|-----------|
| Phase 1 | Critical | 2-3 days | Immediately |
| Phase 2 | High | 3-4 days | After Phase 1 |
| Phase 3 | Medium | 2-3 days | Parallel with Phase 2 |
| Phase 4 | Lower | 3-5 days | After Phase 2 |
| Phase 5 | Ongoing | 2-3 days | Throughout |

**Total estimated effort**: 12-18 days of focused work

---

## Quick Wins (Can do in 1-2 hours)

If you want to make immediate progress:
1. [ ] Fix ToC structure (5 min)
2. [ ] Sanitize paths (15 min)
3. [ ] Fix intro sentence (2 min)
4. [ ] Add embedding overview intro (1 hour)
5. [ ] Improve install.md (30 min)

**Total**: ~2 hours for massive improvement in book quality

---

## Specific Issues Found

### Typos/Errors
- [ ] `intro.md` line 6: "like the popular UI library" - incomplete comparison
- [ ] `fundamental_types.md`: "z32" and "v32" appear in core.md but not explained
- [ ] Error example outputs show home directory paths `/home/eric/` - should be sanitized

### Missing Documentation
- [ ] `core/block.md` - what does this cover? (Scoping blocks? `{}`?)
- [ ] `core/use.md` - module imports presumably, but no preview in TOC
- [ ] Several stdlib modules need brief descriptions in the TOC

---

## Notes

- The book structure is solid and flows well
- TUI documentation is excellent and comprehensive
- Core language explanations are clear (especially connect and select)
- Main gaps are in getting started materials and consistency polish
- After Phase 1-2 completion, the book will be publication-ready
