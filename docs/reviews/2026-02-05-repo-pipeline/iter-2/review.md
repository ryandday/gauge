# Code Review: Entire Repository

**Date**: 2026-02-05
**Scope**: All repository files

---


## Guidelines Checker Review

### Guidelines File Location

**No project guidelines found.**

The following locations were checked for project conventions and guidelines:
- `/Users/ryanday/repos/sherpa2/CLAUDE.md` - Does not exist
- `/Users/ryanday/repos/sherpa2/.claude/` - Directory does not exist
- `/Users/ryanday/repos/sherpa2/rustfmt.toml` - Does not exist
- `/Users/ryanday/repos/sherpa2/.rustfmt.toml` - Does not exist
- `/Users/ryanday/repos/sherpa2/clippy.toml` - Does not exist
- `/Users/ryanday/repos/sherpa2/.editorconfig` - Does not exist
- `/Users/ryanday/repos/sherpa2/README.md` - Does not exist

### Conclusion

This project does not have explicit project-specific conventions documented in a CLAUDE.md file or similar guidelines document. Without explicit project rules to verify against, this checker cannot identify convention violations.

The project planning document (`docs/plans/code-review-tui-review.md`) references "standard Rust conventions" which includes:
- Standard Cargo layout (`src/main.rs`, `src/lib.rs`, module organization)
- Snake_case for modules, PascalCase for types
- `tests/` directory for integration tests
- Standard error handling with `thiserror`

However, these are general Rust ecosystem conventions, not project-specific rules that would be documented in a CLAUDE.md file. The Guidelines Checker is designed to verify code against **explicit project rules** defined in project configuration files, not general language conventions.

### Recommendation

To enable guidelines checking for this project, create a `CLAUDE.md` file at the repository root documenting:
1. Import patterns and module organization rules
2. Naming conventions specific to this project
3. Error handling patterns to follow
4. Required file structure
5. Any framework-specific conventions (e.g., Ratatui screen patterns)

### Categories Checked

- **Convention violations found**: N/A - No guidelines file
- **CLAUDE.md compliance issues**: N/A - No CLAUDE.md file exists
- **Import pattern issues**: N/A - No import rules defined
- **Naming convention issues**: N/A - No naming rules defined
- **Framework rule violations**: N/A - No framework rules defined
- **Style violations**: N/A - No style rules defined


## Expo UI Reviewer

This is a Rust project, not an Expo/React Native project. Expo UI review is not applicable.

**Project Detection Results:**
- `package.json`: Not found
- `app.json` or `app.config.js`: Not found
- `*.tsx` or `*.jsx` files: Not found
- `Cargo.toml`: Found - indicates Rust project
- `.rs` files: Found (26+ Rust source files)

The codebase at `/Users/ryanday/repos/sherpa2` is a Rust TUI (Terminal User Interface) application called "sherpa" built with:
- `ratatui` - Rust TUI framework
- `crossterm` - Terminal manipulation
- `clap` - CLI argument parsing
- `serde` - Serialization

No Expo/React Native code exists in this repository.

## Reuse Detector

**Analysis Summary**: Reviewed unstaged changes across 22 modified files in the sherpa2 Rust codebase. The recent changes show significant consolidation work has already been completed - shared helpers for `highlight_diff_lines`, `render_footer_hints`, `render_error_panel`, `handle_scroll_input`, and `handle_error_state_input` have been extracted to `src/screens/mod.rs` and are being used by most screens.

### RD-001: Missed use of render_footer_hints helper
- Severity: P1
- Type: existing-utility
- File: /Users/ryanday/repos/sherpa2/src/screens/pseudocode.rs:408-413
- Affects: src/screens/pseudocode.rs
- Existing: /Users/ryanday/repos/sherpa2/src/screens/mod.rs:65-70 (render_footer_hints)
- Problem: The `render_assessment_panel` method manually creates a footer paragraph with the same pattern that `render_footer_hints` abstracts. Other screens (triage, deep_review, summary) already use the helper, but this location was missed.
- Caveat: The fix is trivial and low-risk; this is a consistency issue rather than a functional bug.
- Fix: Replace lines 410-413 with: `render_footer_hints(frame, layout[3], hints);`

### Previously Addressed Patterns (No Action Needed)

The following reuse patterns were already consolidated in the recent changes:

1. **Diff highlighting** - `highlight_diff_lines()` extracted to mod.rs, now used by triage.rs, deep_review.rs, pseudocode.rs
2. **Footer rendering** - `render_footer_hints()` extracted to mod.rs, used by most screens
3. **Error panel rendering** - `render_error_panel()` extracted to mod.rs, used by loading.rs, triage.rs, deep_review.rs
4. **Scroll input handling** - `handle_scroll_input()` extracted to mod.rs, used by triage.rs and deep_review.rs
5. **Error state input handling** - `handle_error_state_input()` and `ErrorInputResult` enum extracted, used across screens

### Deferred Items (Already Tracked)

The codebase contains `@review-defer` tags for items requiring human judgment:

- **RD-005/RD-006**: ChunkingError/AssessmentError and ChunkingResult/AssessmentResult type consolidation - valid caveat that types may diverge
- **AR-003**: Error type granularity decision deferred
- **AR-008**: Selection state ownership (UiState vs screen-owned) deferred

No additional reuse opportunities requiring immediate action were found beyond RD-001.

## Performance Review

### PERF-001: Synchronous AI calls block UI thread for up to 2 minutes
- Severity: P0
- Type: blocking
- File: /Users/ryanday/repos/sherpa2/src/ai/claude.rs:27-70
- Affects: src/ai/claude.rs, src/app.rs, src/main.rs
- Current: The `run_claude()` function spawns a subprocess and blocks waiting for it to complete with a 2-minute timeout. During this time, the entire TUI is frozen - no keyboard input is processed, no animations run.
- At Scale: With 100x users or larger diffs requiring longer AI processing, users experience unresponsive UI. If AI service is slow, users cannot cancel or navigate away.
- Caveat: Code has existing `@review-defer(PERF-001)` noting this is acceptable for demos. May be intentional for MVP simplicity.
- Fix: Move AI calls to async/background thread. Use channels to communicate results back to main loop. Allow user interaction during processing.

### PERF-002: Repeated O(n) section filtering on every render and input
- Severity: P2
- Type: complexity
- File: /Users/ryanday/repos/sherpa2/src/screens/deep_review.rs:31-62
- Affects: src/screens/deep_review.rs
- Current: `get_sections_needing_review()`, `total_needing_review()`, and `reviewed_count()` each iterate over all sections. These are called multiple times per render (lines 82, 83, 137, 138, 169, 199-200, 309, 347, 354).
- At Scale: With 100 sections, each render performs ~700+ iterations (7 calls x 100 sections). At 4 FPS, that is 2800 iterations/second.
- Caveat: Code has existing `@review-defer(PERF-002)` noting typical reviews have 5-20 sections where this is negligible.
- Fix: Cache filtered results in DeepReviewScreen struct, invalidate on section mutation. Or compute once at render start and pass down.

### PERF-003: Session serialization buffers entire session in memory before write
- Severity: P1
- Type: memory
- File: /Users/ryanday/repos/sherpa2/src/session/persistence.rs:105-111
- Affects: src/session/persistence.rs
- Current: `serde_json::to_string_pretty(session)` allocates a complete string representation before writing. For large diffs (which are stored in `diff_text`), this doubles memory usage temporarily.
- At Scale: A 10MB diff creates a 20MB+ JSON string. With 100x larger diffs (1GB), the serialization alone uses 2GB+ memory.
- Caveat: Code has existing `@review-defer(PERF-003)` noting this only runs on quit, not in hot path.
- Fix: Use `serde_json::to_writer()` to stream directly to file, avoiding intermediate string allocation.

### PERF-004: Large diff text stored twice in memory (Session and prompt)
- Severity: P1
- Type: memory
- File: /Users/ryanday/repos/sherpa2/src/ai/claude.rs:210-241
- Affects: src/ai/claude.rs
- Current: `build_chunking_prompt()` uses `format!()` to create a new string containing the entire diff. Combined with `Session.diff_text`, the diff exists in memory twice (plus once more when passed to the subprocess via command line).
- At Scale: A 10MB diff uses 30MB+ memory during AI chunking. With 100MB diffs, system may OOM.
- Caveat: Diffs are typically small (KB to low MB). This is only an issue for unusually large reviews.
- Fix: Pass diff via stdin to subprocess instead of command line argument. Consider streaming the diff rather than loading entirely into memory.

### PERF-005: Session retains full diff_text after chunking (for re-chunking feature)
- Severity: P1
- Type: memory
- File: /Users/ryanday/repos/sherpa2/src/models/session.rs:17-18
- Affects: src/models/session.rs
- Current: The raw `diff_text` is retained in Session even after chunking produces sections. This is documented as needed for potential re-chunking.
- At Scale: With 100x larger diffs, unnecessary memory consumption throughout the review session.
- Caveat: Code has existing `@review-defer(PERF-005)` noting this is required for re-chunking feature.
- Fix: If re-chunking is rare, consider lazy-loading from git on demand rather than keeping in memory. Or compress the diff_text after chunking.

### PERF-006: Diff highlighting re-computed on every render frame
- Severity: P2
- Type: resource
- File: /Users/ryanday/repos/sherpa2/src/screens/triage.rs:246
- Affects: src/screens/triage.rs, src/screens/deep_review.rs, src/screens/pseudocode.rs, src/screens/mod.rs
- Current: `highlight_diff_lines()` is called every render frame to convert code text to styled `Line` elements. At 4 FPS, this creates and discards styled objects 4 times per second.
- At Scale: With 100x larger code sections (10,000 lines), creating 40,000 `Line` objects per second causes allocation churn.
- Caveat: Code has existing `@review-defer(PERF-006)` noting ratatui is immediate-mode and caching adds complexity. The actual overhead is minimal for typical diff sizes.
- Fix: Cache styled lines in screen state, invalidate only when selected section changes. Ratatui's immediate-mode design makes this awkward but not impossible.

### PERF-007: JSON extraction scans response string multiple times
- Severity: P2
- Type: complexity
- File: /Users/ryanday/repos/sherpa2/src/ai/claude.rs:283-324
- Affects: src/ai/claude.rs
- Current: `extract_json()` performs multiple passes: first looking for code blocks, then searching for raw JSON, then finding matching brackets. Each `find()` scans the string from the beginning.
- At Scale: For 100KB AI responses, this could mean 300KB+ of string scanning. With 1MB responses, scanning becomes noticeable.
- Caveat: Code has existing `@review-defer(PERF-007)` noting JSON extraction happens once per API call, not per frame. Typical responses are ~10KB.
- Fix: Single-pass parser that identifies structure type and extracts in one scan. Or use a streaming JSON parser.

### PERF-008: String cloning in retry_assessment hot path
- Severity: P2
- Type: resource
- File: /Users/ryanday/repos/sherpa2/src/app.rs:165-171
- Affects: src/app.rs
- Current: `retry_assessment()` clones `section.code` and `state.ui.input_text` even before checking if the operation will proceed. The clone happens on line 167 unconditionally.
- At Scale: With 100KB code sections, each retry attempt allocates and immediately may discard 100KB.
- Caveat: This only happens on user-initiated retry, not on every render. Frequency is low.
- Fix: Move clones inside the if-let after confirming the operation will proceed, or use references where possible.

### PERF-009: Misconception list in summary creates Vec per render
- Severity: P2
- Type: resource
- File: /Users/ryanday/repos/sherpa2/src/screens/summary.rs:191-236
- Affects: src/screens/summary.rs
- Current: `render_misconceptions()` creates a new `Vec<ListItem>` every render frame by iterating over all sections and assessments.
- At Scale: With 100 sections each having 10 misconceptions, 1000 `ListItem` allocations per render, 4000/second at 4 FPS.
- Caveat: Summary screen is viewed infrequently. This is unlikely to be a real bottleneck.
- Fix: Cache the misconception list, invalidate when assessments change.

### Summary

The codebase has several performance considerations, most of which have already been identified and documented with `@review-defer` comments indicating intentional design tradeoffs:

**Critical (P0):**
- Synchronous AI calls blocking the UI (acknowledged, requires async architecture)

**Important (P1):**
- Memory usage from storing full diff text and doubling during serialization
- Only relevant for unusually large diffs (>10MB)

**Optimization Opportunities (P2):**
- Repeated section filtering in DeepReviewScreen (negligible for typical 5-20 sections)
- Diff highlighting recomputation (typical diffs are small)
- JSON extraction multi-pass scanning (responses are small)
- Various allocation patterns (low frequency operations)

For a TUI code review tool processing typical git diffs (1KB-1MB), these performance characteristics are acceptable. The most impactful improvement would be async AI calls to maintain UI responsiveness, but this requires significant architectural changes.

## Correctness Review

### CR-001: Integer underflow in `find_matching_bracket` depth counter
- Severity: P1
- Type: logic error
- File: /Users/ryanday/repos/sherpa2/src/ai/claude.rs:365
- Affects: src/ai/claude.rs
- Problem: The `depth` variable is an `i32` (inferred from arithmetic) that starts at 0 and decrements when it sees a closing bracket. If malformed JSON has more closing brackets than opening brackets, `depth` will underflow to negative values. While this does not cause a panic (signed integer underflow wraps in debug, and the function will just never find `depth == 0`), it could mask parsing errors and return `None` when it should detect malformed input more explicitly.
- Caveat: This is defensive - the JSON coming from Claude API is likely well-formed, and returning `None` is acceptable behavior for malformed input.
- Fix: Use `usize` with `checked_sub()` or explicitly handle the case where depth would go negative.

### CR-002: Stale `current_review_index` after sections change
- Severity: P1
- Type: edge case
- File: /Users/ryanday/repos/sherpa2/src/screens/deep_review.rs:349
- Affects: src/screens/deep_review.rs
- Problem: The `current_review_index` field persists across the screen's lifetime but the list of sections needing review (`get_sections_needing_review`) can change dynamically as sections get reviewed. The code uses `min(needs_review.len().saturating_sub(1))` to clamp the index, but if a section is reviewed and the list shrinks, the user might unexpectedly jump to a different section. For example, if viewing section 2 of 3, and section 1 gets reviewed, the index stays at 2 but now points to what was section 3.
- Caveat: The clamping prevents out-of-bounds access. The UX issue is minor - user just sees a different section than expected.
- Fix: Reset `current_review_index` to 0 when transitioning to DeepReview screen, or track sections by ID rather than index.

### CR-003: Potential panic in `render_code_preview` with `unwrap_or(0)` and empty sections
- Severity: P2
- Type: edge case
- File: /Users/ryanday/repos/sherpa2/src/screens/triage.rs:224
- Affects: src/screens/triage.rs
- Problem: When `list_state.selected()` returns `None` and sections is empty, `state.session.sections.get(0)` returns `None`, which is handled. However, if somehow `list_state` has a selected index > 0 but sections is empty, the `get(selected_idx)` will return `None`. The code handles this case with the match statement, but the logic path is confusing.
- Caveat: The screen returns early if sections is empty before rendering content, so this path should never be hit in practice.
- Fix: Clarify the code path or add an assertion that selected_idx < sections.len() when sections is not empty.

### CR-004: Draft hypothesis not cleared after successful assessment
- Severity: P1
- Type: logic error
- File: /Users/ryanday/repos/sherpa2/src/app.rs:180
- Affects: src/app.rs
- Problem: In `retry_assessment()`, after a successful assessment, the code clears `draft_hypothesis` from the session (line 180). However, if the user navigates away after submitting but before the assessment completes, and then the assessment succeeds, the draft is cleared but the user might have started a new draft on a different section. The `draft_hypothesis` is a single field shared across all sections.
- Caveat: The workflow generally expects users to wait for assessment or explicitly navigate away. This is an edge case in async-like behavior in a synchronous app.
- Fix: Store draft per-section (e.g., in the Section struct) or only clear draft if it matches the submitted hypothesis.

### CR-005: `select_next`/`select_previous` in TriageScreen don't update `state.ui.selected_index`
- Severity: P2
- Type: logic error
- File: /Users/ryanday/repos/sherpa2/src/screens/triage.rs:281-301
- Affects: src/screens/triage.rs
- Problem: TriageScreen maintains its own `list_state` for selection, but `state.ui.selected_index` exists in AppState and is used by other parts of the code (e.g., `state.current_section()`). These are not kept in sync. If code elsewhere relies on `state.ui.selected_index`, it will have stale data.
- Caveat: There is a `@review-defer(CR-002)` comment acknowledging this architectural decision. The code currently works because TriageScreen reads from its own `list_state`, but this is a consistency concern.
- Fix: Either always use `state.ui.selected_index` as the source of truth, or remove `selected_index` from UiState if screens own their selection state.

### CR-006: Race condition in `busy` flag management in ClaudeClient
- Severity: P2
- Type: race condition
- File: /Users/ryanday/repos/sherpa2/src/ai/claude.rs:150-161
- Affects: src/ai/claude.rs
- Problem: The `busy` flag is set/cleared non-atomically around the synchronous AI call. While the current code is single-threaded and this is not an issue, if the code were ever made async or multi-threaded, this pattern would introduce a race condition.
- Caveat: The code is currently synchronous and single-threaded. The flag-based busy tracking works correctly in the current architecture.
- Fix: If threading is added, use `AtomicBool` or a mutex. For now, this is acceptable.

### CR-007: Scroll offset cast to u16 may truncate large values
- Severity: P2
- Type: edge case
- File: /Users/ryanday/repos/sherpa2/src/screens/triage.rs:251
- Affects: src/screens/triage.rs, src/screens/deep_review.rs:291, src/screens/pseudocode.rs:299
- Problem: `state.ui.scroll_offset as u16` truncates values larger than 65535. While practical usage is unlikely to hit this (requires ~3000+ page-down presses), it's technically incorrect.
- Caveat: There is a `@review-skip(CR-004)` comment acknowledging this is acceptable behavior. The truncation just wraps the scroll position.
- Fix: Use `min(scroll_offset, u16::MAX as usize) as u16` for defensive clamping.

### CR-008: `export_markdown` creates docs/ in current working directory
- Severity: P1
- Type: logic error
- File: /Users/ryanday/repos/sherpa2/src/screens/summary.rs:268
- Affects: src/screens/summary.rs
- Problem: The export creates `docs/` relative to the current working directory, which may not be the repository root. If the user runs the tool from a subdirectory, the exported file will be in an unexpected location.
- Caveat: Users typically run CLI tools from the repo root. This is a usability issue, not a correctness bug.
- Fix: Use `git rev-parse --show-toplevel` to find the repo root, or allow users to specify the output path.

### CR-009: Backspace at position 0 does not panic but input_text may be out of sync
- Severity: P2
- Type: edge case
- File: /Users/ryanday/repos/sherpa2/src/screens/pseudocode.rs:154-158
- Affects: src/screens/pseudocode.rs
- Problem: The backspace handling checks `cursor_position > 0` before decrementing and removing. If `cursor_position` is 0 but `input_text` is non-empty (which should not happen in normal flow), the character at position 0 would never be removable. However, since characters are only added at the end (push) and cursor_position is always set to `input_text.len()`, this inconsistency should not occur.
- Caveat: The invariant `cursor_position == input_text.len()` is maintained by the code, so this is a theoretical issue.
- Fix: Consider asserting the invariant or simplifying to always operate at the end of the string.

### CR-010: Missing bounds check in `tag_current`
- Severity: P2
- Type: edge case
- File: /Users/ryanday/repos/sherpa2/src/screens/triage.rs:304-307
- Affects: src/screens/triage.rs
- Problem: `tag_current` uses `unwrap_or(0)` on `list_state.selected()` and then does `.get_mut(selected)`. If somehow `list_state` contains an invalid index (e.g., set externally), this silently does nothing. While `get_mut` safely returns `None`, the user action (pressing 1/2/3 to tag) would silently fail.
- Caveat: The list_state is controlled internally and should never have an out-of-bounds index given the navigation logic.
- Fix: Log a warning or handle the case where the section cannot be found.

### CR-011: `DefaultHasher` is not stable across Rust versions
- Severity: P1
- Type: logic error
- File: /Users/ryanday/repos/sherpa2/src/session/persistence.rs:28
- Affects: src/session/persistence.rs
- Problem: `DefaultHasher` is explicitly documented as not guaranteeing the same hash values across different Rust versions or compilations. If a user upgrades their Rust toolchain, existing session files may become orphaned because the hash of the identifier will change, making it impossible to find the session file.
- Caveat: There is a `@review-defer(CR-005)` comment acknowledging this. For a personal tool, this is low impact. Sessions are not critical data.
- Fix: Use a stable hasher like `xxhash`, `highway`, or just use a simple deterministic encoding (e.g., base64 or percent-encoding of the identifier).

## Abstraction Reviewer

Reviewed the entire Rust codebase at `/Users/ryanday/repos/sherpa2` for abstraction quality, SOLID principles, and appropriate level of abstraction.

### Summary

The codebase demonstrates generally good abstraction design for a small TUI application. The code shows evidence of prior review iterations (many `@review-defer` and `@review-skip` tags) indicating conscious design decisions have been documented. The module structure is well-organized with clear separation of concerns.

**Key Strengths:**
- Clean trait-based AI client abstraction (`AiClient` trait with `ClaudeClient` and `MockAiClient`)
- Well-defined screen trait (`ScreenTrait`) enabling polymorphic screen handling
- Good separation between persistent state (`Session`) and ephemeral UI state (`UiState`)
- Shared utility functions in `screens/mod.rs` reduce duplication

**Areas Reviewed:**
- src/main.rs - Application entry point and orchestration
- src/app.rs - TUI application lifecycle
- src/cli.rs - Command line argument parsing
- src/error.rs - Error type definitions
- src/git.rs - Git operations
- src/models/ - Data models (Section, Session, State)
- src/ai/ - AI client abstraction layer
- src/screens/ - TUI screen implementations
- src/session/ - Session persistence

### Issues Found

---

### AR-001: Magic numbers for scroll increments scattered across code
- Severity: P2
- Type: missing-abstraction
- File: /Users/ryanday/repos/sherpa2/src/screens/mod.rs:80-93
- Affects: src/screens/mod.rs
- Problem: The scroll increment values (10 for Ctrl+D/U, 20 for PageDown/PageUp) are magic numbers without named constants.
- Why it matters: These values represent a user experience decision and should be tunable. Currently they're buried in input handling code.
- Caveat: For a small TUI app, inline constants may be acceptable if scroll behavior is unlikely to change.
- Fix: Extract constants like `const SMALL_SCROLL_LINES: usize = 10;` and `const LARGE_SCROLL_LINES: usize = 20;` at the module level.

---

### AR-002: Repeated session filtering patterns in DeepReviewScreen
- Severity: P2
- Type: missing-abstraction
- File: /Users/ryanday/repos/sherpa2/src/screens/deep_review.rs:31-62
- Affects: src/screens/deep_review.rs
- Problem: Three methods (`get_sections_needing_review`, `total_needing_review`, `reviewed_count`) perform similar iteration/filtering over sections. The filtering predicate `s.needs_review()` is repeated.
- Why it matters: The same filtering logic appears in multiple places, making it easy to introduce inconsistency.
- Caveat: These are already marked with `@review-defer(PERF-002)` and `@review-defer(CS-006)` noting the pattern is acceptable for typical section counts.
- Fix: Consider adding a `SectionFilter` iterator adapter or moving these methods to `Session` itself since they only depend on session data.

---

### AR-003: Duplicated error type structures (ChunkingError / AssessmentError)
- Severity: P2
- Type: missing-abstraction
- File: /Users/ryanday/repos/sherpa2/src/ai/types.rs:33-51, 76-94
- Affects: src/ai/types.rs
- Problem: `ChunkingError` and `AssessmentError` have identical structure (message: String, raw_output: Option<String>) and identical builder methods.
- Why it matters: Code duplication that may diverge over time. Adding a new field requires changes in two places.
- Caveat: Already marked with `@review-defer(AR-003)` and `@review-defer(RD-005)` noting they may need to diverge in the future.
- Fix: Extract a generic `AiError` struct or use a macro to generate both types. Alternatively, unify into single `AiOperationError` with an operation-type field.

---

### AR-004: Screen list state duplication between TriageScreen and UiState
- Severity: P1
- Type: solid-violation
- File: /Users/ryanday/repos/sherpa2/src/screens/triage.rs:19-21, /Users/ryanday/repos/sherpa2/src/models/state.rs:44-45
- Affects: src/screens/triage.rs, src/models/state.rs
- Problem: `TriageScreen` owns a `ListState` while `UiState` has `selected_index` and `scroll_offset`. This creates two sources of truth for selection state.
- Why it matters: Already noted in `@review-defer(AR-008)` and `@review-defer(CR-002)`. The inconsistency can lead to bugs where the two get out of sync during navigation.
- Caveat: This is partially due to ratatui's design requiring `ListState` for stateful widgets. The hybrid approach may be intentional.
- Fix: Choose one canonical source: either screens own all their state (removing selection from `UiState`), or centralize in `UiState` and have screens derive widget state on render.

---

### AR-005: Timeout constant embedded in run_claude method
- Severity: P2
- Type: missing-abstraction
- File: /Users/ryanday/repos/sherpa2/src/ai/claude.rs:36
- Affects: src/ai/claude.rs
- Problem: The 2-minute timeout (`Duration::from_secs(120)`) is a magic number in the middle of business logic.
- Why it matters: Timeout is a configuration decision that users may want to adjust. It's not discoverable at the top of the file.
- Caveat: For a CLI tool with no configuration file, inline constants may be acceptable.
- Fix: Extract to `const AI_TIMEOUT_SECS: u64 = 120;` at module level or make configurable via environment variable.

---

### AR-006: Layout constraints hardcoded throughout screens
- Severity: P2
- Type: missing-abstraction
- File: /Users/ryanday/repos/sherpa2/src/screens/triage.rs:54-61, src/screens/deep_review.rs:95-103, src/screens/pseudocode.rs:185-188
- Affects: src/screens/triage.rs, src/screens/deep_review.rs, src/screens/pseudocode.rs, src/screens/summary.rs
- Problem: Layout percentages and lengths (e.g., `Constraint::Percentage(40)`, `Constraint::Length(3)`) are scattered throughout render methods without named constants.
- Why it matters: Layout is a cross-cutting concern. Adjusting the overall look requires hunting through multiple files.
- Caveat: Ratatui's immediate-mode design makes centralized layout configuration awkward. The current approach is idiomatic for TUI apps.
- Fix: Consider a `layout.rs` module with named layout configurations, or at minimum use named constants for repeated values like header/footer heights.

---

### AR-007: No abstraction for terminal setup/restore lifecycle
- Severity: P2
- Type: missing-abstraction
- File: /Users/ryanday/repos/sherpa2/src/app.rs:213-241
- Affects: src/app.rs
- Problem: `setup_terminal()` and `restore_terminal()` are free functions that must be paired correctly. The pairing logic is in `App::run()`.
- Why it matters: If new entry points are added, they must remember to call both. This is a RAII pattern that could use a guard type.
- Caveat: Already marked with `@review-defer(TA-010)`. For a single entry point app, the current approach is sufficient.
- Fix: Create a `TerminalGuard` struct that implements `Drop` to ensure restore is always called, even on panic.

---

### AR-008: Prompts embedded in AI client implementation
- Severity: P1
- Type: missing-abstraction
- File: /Users/ryanday/repos/sherpa2/src/ai/claude.rs:210-278
- Affects: src/ai/claude.rs
- Problem: The AI prompts (`build_chunking_prompt`, `build_assessment_prompt`) are large multi-line strings embedded in the AI client module. These are domain-specific content mixed with infrastructure code.
- Why it matters: Prompts are likely to be iterated on frequently. Finding and editing them requires navigating infrastructure code. Prompt engineering is a separate concern from API integration.
- Caveat: For a small app, keeping prompts close to their usage is pragmatic and avoids indirection.
- Fix: Extract prompts to a separate `prompts.rs` module or even external template files. This separates prompt engineering from API plumbing.

---

### AR-009: Boolean return from handle_input masks error information
- Severity: P2
- Type: bad-abstraction
- File: /Users/ryanday/repos/sherpa2/src/screens/mod.rs:128
- Affects: src/screens/mod.rs, src/screens/loading.rs, src/screens/triage.rs, src/screens/deep_review.rs, src/screens/pseudocode.rs, src/screens/summary.rs
- Problem: `ScreenTrait::handle_input` returns `Result<bool>` where the bool indicates "event consumed". This masks the semantic meaning - was the key recognized? Was an action taken? Was it blocked?
- Why it matters: Already noted in `@review-defer(AR-010)`. The bool is overloaded - it can mean "key was handled" or "action was taken" depending on context.
- Caveat: The current design is simple and works. A richer return type adds complexity for marginal benefit.
- Fix: Consider an enum like `InputResult { Consumed, Ignored, Error(E) }` or keep boolean but document the exact contract.

---

### AR-010: Inconsistent error handling patterns across screens
- Severity: P2
- Type: solid-violation
- File: /Users/ryanday/repos/sherpa2/src/screens/loading.rs:100-112, src/screens/triage.rs:70-81, src/screens/deep_review.rs:113-134
- Affects: src/screens/loading.rs, src/screens/triage.rs, src/screens/deep_review.rs, src/screens/pseudocode.rs
- Problem: While `handle_error_state_input` is shared, screens handle the `ErrorInputResult::Retry` case differently. Loading screen calls `request_chunking_retry()`, Triage calls `clear_error()`, DeepReview also handles 's' for skip.
- Why it matters: Users may expect consistent retry behavior across screens. The shared helper suggests uniformity but the implementation diverges.
- Caveat: Different screens legitimately have different retry semantics (chunking vs assessment vs no retry).
- Fix: Document the expected retry behavior per screen or extend `ErrorInputResult` to include screen-specific retry types.

---

### No additional issues found requiring tag injection.

The codebase demonstrates good overall abstraction quality with appropriate use of traits, enums, and modules. Many potential issues have already been identified and deferred in prior reviews with thoughtful caveats about trade-offs. The issues above are mostly P2 (minor improvement opportunities) with a few P1 items that represent design decisions needing human judgment rather than clear bugs.

## Silent Failure Hunter

### SFH-001: Child process kill failure silently ignored
- Severity: P1
- Type: silent-failure
- File: /Users/ryanday/repos/sherpa2/src/ai/claude.rs:66
- Affects: src/ai/claude.rs
- Problem: When the Claude CLI times out, the `child.kill()` call result is silently discarded with `let _ = child.kill();`. If the kill fails, the orphan process continues running.
- Hidden Errors: Failed process termination, zombie processes, resource leaks
- User Impact: Orphan Claude processes may accumulate, consuming system resources. User has no visibility into failed process cleanup.
- Caveat: Kill failures are rare, and the user is already notified of the timeout. The orphan process may terminate naturally.
- Fix:
```rust
None => {
    // Timeout - kill the process
    if let Err(e) = child.kill() {
        eprintln!("Warning: Failed to kill timed-out Claude process: {}", e);
    }
    Err("Claude CLI timed out after 2 minutes".to_string())
}
```

### SFH-002: stdin read_line result silently ignored on save failure
- Severity: P2
- Type: silent-failure
- File: /Users/ryanday/repos/sherpa2/src/main.rs:113
- Affects: src/main.rs
- Problem: When session save fails, the code prompts user to press Enter to acknowledge, but ignores the read_line result with `let _ = std::io::stdin().read_line(...)`. If stdin is closed or fails, user acknowledgment is skipped.
- Hidden Errors: stdin read failures, EOF conditions
- User Impact: Minor - the warning is already printed. User might miss the pause if stdin is unavailable, but the message has been displayed.
- Caveat: This is a best-effort acknowledgment in an error recovery path. Failing to pause is acceptable since the error message was already shown.
- Fix:
```rust
if std::io::stdin().read_line(&mut String::new()).is_err() {
    // stdin unavailable - warning already shown, continue
}
```

### SFH-003: No retry mechanism for transient AI failures
- Severity: P1
- Type: missing-resilience
- File: /Users/ryanday/repos/sherpa2/src/ai/claude.rs:183-193
- Affects: src/ai/claude.rs, src/ai/client.rs, src/app.rs
- Problem: AI operations (chunk_diff, assess_hypothesis) have no automatic retry for transient failures like network timeouts or rate limits. Users must manually press 'r' to retry.
- Hidden Errors: Transient network errors, Claude CLI startup failures, rate limiting
- User Impact: Users experience repeated failures that could be automatically recovered. Manual retry requirement creates friction in the workflow.
- Caveat: The existing manual retry via 'r' key is documented and functional. Auto-retry could cause unexpected delays or infinite loops if the service is truly down. The TUI provides visual feedback.
- Fix: Add exponential backoff retry (2-3 attempts) before surfacing error to user:
```rust
fn chunk_diff_impl(&self, diff: &str) -> ChunkingResult {
    let prompt = build_chunking_prompt(diff);
    let mut attempts = 0;
    let max_attempts = 3;
    
    loop {
        match self.run_claude(&prompt) {
            Ok(response) => match self.parse_sections(&response) {
                Ok(sections) => return ChunkingResult::Success(sections),
                Err(e) => return ChunkingResult::Error(ChunkingError::new(e).with_output(response)),
            },
            Err(e) => {
                attempts += 1;
                if attempts >= max_attempts || !is_transient_error(&e) {
                    return ChunkingResult::Error(ChunkingError::new(e));
                }
                std::thread::sleep(Duration::from_millis(500 * attempts as u64));
            }
        }
    }
}
```

### SFH-004: Empty sections result returns Ok without user feedback mechanism
- Severity: P2
- Type: inadequate-handling
- File: /Users/ryanday/repos/sherpa2/src/main.rs:81-84
- Affects: src/main.rs, src/app.rs
- Problem: When AI returns empty sections, the code prints to stderr and returns `Ok(())`, which means a clean exit. The user sees an error message but the exit code is success (0), making it indistinguishable from normal completion in scripts.
- Hidden Errors: AI responses that fail to produce sections are treated as success
- User Impact: Scripted invocations cannot detect this failure mode. User sees message but exit code is misleading.
- Caveat: This is arguably a valid "no work to do" scenario rather than an error. The message is displayed to the user.
- Fix: Return a distinct error variant or non-zero exit code:
```rust
if sections.is_empty() {
    return Err(AppError::Ai("AI returned no sections. The diff may be too small or unclear.".to_string()));
}
```

### SFH-005: from_utf8_lossy silently replaces invalid UTF-8
- Severity: P2
- Type: silent-failure
- File: /Users/ryanday/repos/sherpa2/src/git.rs:16, 62, 80, 109, 121
- Affects: src/git.rs
- Problem: Multiple calls to `String::from_utf8_lossy()` silently replace invalid UTF-8 bytes with the Unicode replacement character. If git output contains binary data or corrupt encoding, this is hidden from the user.
- Hidden Errors: Binary data in diffs, encoding issues in file paths, corrupted git output
- User Impact: Users may see garbled output (replacement characters) without understanding why. The original error condition is masked.
- Caveat: Invalid UTF-8 in git output is rare. The replacement character is a reasonable fallback for display purposes. Strict UTF-8 validation would require error handling that might be overly aggressive for a TUI.
- Fix: Log when lossy conversion occurs:
```rust
let stdout_bytes = &output.stdout;
let stdout = match String::from_utf8(stdout_bytes.to_vec()) {
    Ok(s) => s,
    Err(_) => {
        eprintln!("Warning: Git output contains invalid UTF-8, some characters may be replaced");
        String::from_utf8_lossy(stdout_bytes).to_string()
    }
};
```

### SFH-006: Session deletion warning followed by continuation
- Severity: P2
- Type: inadequate-handling
- File: /Users/ryanday/repos/sherpa2/src/main.rs:44-46
- Affects: src/main.rs
- Problem: When `delete_session` fails during `--new` flag processing, the code prints a warning and continues. The old session data may still exist and could cause confusion on future runs.
- Hidden Errors: File permission errors, filesystem issues preventing deletion
- User Impact: User requested fresh session but old data may persist. Warning message may be missed in output.
- Caveat: The deletion is a "best effort" cleanup. Continuing with a new session is reasonable even if the old file remains. The old file would be overwritten on save anyway.
- Fix: Consider failing if deletion is critical:
```rust
if let Err(e) = delete_session(&identifier) {
    // Check if file actually exists to distinguish "already gone" from "can't delete"
    if session_exists(&identifier)? {
        return Err(AppError::Session(format!(
            "Failed to delete existing session for fresh start: {}. Use without --new or remove manually.",
            e
        )));
    }
}
```

### SFH-007: Terminal restore failure only logged, not propagated when main_loop succeeds
- Severity: P1
- Type: inadequate-handling
- File: /Users/ryanday/repos/sherpa2/src/app.rs:73-80
- Affects: src/app.rs
- Problem: The terminal restore error handling is correct - it returns the restore error if main_loop succeeded. However, if both fail, the main_loop error takes precedence and the restore failure is only logged via eprintln. The user may have a corrupted terminal state without knowing why.
- Hidden Errors: Terminal state not fully restored, raw mode still enabled
- User Impact: User's terminal may be left in a broken state (no echo, raw mode). They need to run `reset` but don't know why.
- Caveat: Returning the original error when both fail is reasonable - the main_loop error is likely more actionable. The warning is printed.
- Fix: Consider including both errors or prioritizing terminal restore:
```rust
if let Err(restore_err) = restore_terminal(&mut terminal) {
    eprintln!("CRITICAL: Failed to restore terminal: {}", restore_err);
    eprintln!("Your terminal may be in a broken state. Run 'reset' to fix.");
    if result.is_ok() {
        return Err(restore_err);
    }
    // Both failed - return main_loop error but user has been warned about terminal
}
```

### SFH-008: Scroll offset cast to u16 can wrap on large offsets
- Severity: P2
- Type: inadequate-handling
- File: /Users/ryanday/repos/sherpa2/src/screens/triage.rs:251
- Affects: src/screens/triage.rs, src/screens/deep_review.rs:291, src/screens/pseudocode.rs:299
- Problem: `state.ui.scroll_offset as u16` can wrap if offset exceeds 65535. While the existing @review-skip comment notes this requires ~3000+ key presses, the cast is unchecked.
- Hidden Errors: Integer overflow wrapping causes scroll position to jump unexpectedly
- User Impact: After extreme scrolling, the view may jump to an unexpected position. Non-catastrophic but confusing.
- Caveat: As noted in the code comment, this requires thousands of key presses and wrapping is non-catastrophic. The practical limit of file sizes makes this unlikely.
- Fix: Saturating conversion is safer:
```rust
.scroll((state.ui.scroll_offset.min(u16::MAX as usize) as u16, 0));
```

### SFH-009: ListState.selected().unwrap_or(0) hides None state
- Severity: P2
- Type: silent-failure
- File: /Users/ryanday/repos/sherpa2/src/screens/triage.rs:224, 287, 298, 304
- Affects: src/screens/triage.rs
- Problem: Multiple calls to `list_state.selected().unwrap_or(0)` silently default to index 0 when nothing is selected. This hides the distinction between "explicitly selected item 0" and "nothing selected".
- Hidden Errors: Uninitialized selection state, deselection events
- User Impact: Minor - the default to 0 is reasonable for this UI. User may not notice the distinction.
- Caveat: The list state is always initialized with `select(Some(0))` in `new()`, so None should not occur in practice. The default is a reasonable fallback.
- Fix: No change needed if initialization is guaranteed, but consider defensive handling:
```rust
let selected = self.list_state.selected().expect("list_state should always have selection");
```

### SFH-010: No signal handling for graceful shutdown during AI operations
- Severity: P1
- Type: missing-resilience
- File: /Users/ryanday/repos/sherpa2/src/app.rs:86
- Affects: src/app.rs, src/main.rs
- Problem: During synchronous AI operations (run_claude), Ctrl+C is blocked until the operation completes or times out. The signal handling in the main loop only works between poll() cycles.
- Hidden Errors: User interrupts during AI calls are not processed
- User Impact: User cannot quit during the 2-minute AI timeout window. Application appears hung.
- Caveat: As noted in the existing @review-defer comment, proper Ctrl+C handling during synchronous AI calls requires async/threading architecture. The 2-minute timeout provides an upper bound.
- Fix: Requires architectural change to async or spawn AI calls in separate thread with channel-based cancellation. Deferred to human review.

### SFH-011: export_markdown creates docs/ directory silently
- Severity: P2
- Type: inadequate-handling
- File: /Users/ryanday/repos/sherpa2/src/screens/summary.rs:268-271
- Affects: src/screens/summary.rs
- Problem: `export_markdown` creates the `docs/` directory if it doesn't exist. While this is convenient, it modifies the filesystem without explicit user consent beyond pressing 'e'.
- Hidden Errors: Directory creation in unexpected location if cwd is wrong
- User Impact: User may be surprised to find a `docs/` folder created. If running from wrong directory, exports go to wrong location.
- Caveat: The user explicitly pressed 'e' to export. Creating `docs/` is reasonable for the export feature. The path is shown after export.
- Fix: Consider showing intended path before creation or using temp directory:
```rust
let docs_dir = PathBuf::from("docs");
let path = docs_dir.join(&filename);
if !docs_dir.exists() {
    eprintln!("Creating docs/ directory for export...");
    fs::create_dir_all(&docs_dir)
        .map_err(|e| format!("Failed to create docs directory: {}", e))?;
}
```

---

**Summary**: The codebase demonstrates generally good error handling practices with explicit Result types, meaningful error messages, and user-facing error display in the TUI. The identified issues fall into several categories:

1. **Silent process cleanup** (SFH-001): Orphan process handling should log failures
2. **Missing resilience patterns** (SFH-003, SFH-010): Auto-retry and signal handling would improve UX
3. **Encoding handling** (SFH-005): Lossy UTF-8 conversion should be logged
4. **Exit code semantics** (SFH-004): AI returning empty sections should be an error
5. **Minor edge cases** (SFH-002, SFH-006, SFH-008, SFH-009, SFH-011): Low-impact silent behaviors

The codebase already contains numerous `@review-defer` tags indicating awareness of architectural decisions that require human judgment, particularly around async handling and threading for responsive Ctrl+C during AI operations.

## Wiring Detector

### WD-001: unidiff dependency declared but never imported
- Severity: P2
- Type: unused-dep
- Added: `unidiff = "0.4"` in Cargo.toml
- Location: /Users/ryanday/repos/sherpa2/Cargo.toml:34
- Affects: Cargo.toml
- Problem: The `unidiff` crate is declared as a dependency but is never imported or used anywhere in the codebase. No `use unidiff`, `unidiff::`, or `extern crate unidiff` statements exist in any source file.
- Evidence: Searched entire src/ directory for any reference to unidiff - no matches found. All diff parsing is done manually via git subprocess output and custom JSON parsing in ai/claude.rs.
- Caveat: An existing `@review-defer(WD-002)` tag is already present on this line indicating human decision needed about whether to remove or implement the intended diff parsing feature. This is a known intentional gap, not an accidental oversight.
- Fix: Either remove the unused dependency from Cargo.toml, or implement the intended diff parsing feature that would use it. Since there's already a deferred review tag, this should be resolved during that human review.

### Summary

Analyzed the following dependencies from Cargo.toml for actual usage in the codebase:

| Dependency | Status | Usage Files |
|------------|--------|-------------|
| ratatui | Used | app.rs, screens/*.rs, models/state.rs |
| crossterm | Used | app.rs, screens/*.rs |
| clap | Used | cli.rs |
| serde | Used | models/*.rs, session/persistence.rs, ai/claude.rs |
| serde_json | Used | models/section.rs, models/session.rs, session/persistence.rs, ai/claude.rs |
| thiserror | Used | error.rs |
| dirs | Used | session/persistence.rs |
| unidiff | **UNUSED** | No imports found |
| throbber-widgets-tui | Used | screens/loading.rs |
| chrono | Used | screens/summary.rs |
| wait-timeout | Used | ai/claude.rs |

**Incomplete Migrations:** None found. The codebase uses consistent patterns:
- All error handling uses `thiserror`
- All serialization uses `serde`/`serde_json`
- All TUI rendering uses `ratatui`/`crossterm`

**Dead Config:** None found. All environment variables and configuration paths are actively used.

**Unwired Components:** None found. All screen components are imported and rendered in app.rs.


## Comment Analysis Review

**Scope**: Full Rust codebase review at `/Users/ryanday/repos/sherpa2`
**Analyzed**: 19 source files, 1 test file
**Date**: 2026-02-04

### Summary

The codebase has relatively few traditional code comments, relying instead on:
1. Doc comments (`///`) for public APIs
2. Custom `@task`, `@review-defer`, and `@review-skip` annotations for project management
3. Inline comments primarily for non-obvious behavior

Overall comment quality is good with accurate doc comments and meaningful annotations. However, several issues were identified with misleading documentation, unnecessary comments, and missing explanations for complex logic.

---

### Critical Issues

### CA-001: Misleading doc comment on AiClient trait
- Severity: P1
- Type: misleading
- File: `/Users/ryanday/repos/sherpa2/src/ai/client.rs`:11-14
- Affects: `/Users/ryanday/repos/sherpa2/src/ai/client.rs`
- Problem: The comment states "This trait enables parallel development: PHASE-2 implements the real Claude subprocess client, PHASE-3 uses a mock implementation for UI testing" - however, reviewing the codebase shows that both ClaudeClient and MockAiClient are already implemented and in use. The PHASE references are outdated and no longer accurate.
- Fix: Update to reflect current state: "Trait defining the AI client interface for code review operations. ClaudeClient provides the real implementation via Claude CLI subprocess, MockAiClient is used for testing."

### CA-002: Incorrect footer hint message
- Severity: P1
- Type: incorrect
- File: `/Users/ryanday/repos/sherpa2/src/screens/deep_review.rs`:303-304
- Affects: `/Users/ryanday/repos/sherpa2/src/screens/deep_review.rs`
- Problem: The footer hint says "Enter: submit" but Enter actually navigates to the PseudocodeReview screen; it does not submit anything. The actual submission happens in PseudocodeReviewScreen.
- Fix: Change hint to "Enter: start review" or "Enter: write hypothesis"

### CA-003: Comment claims 4+ FPS but math shows otherwise
- Severity: P1
- Type: incorrect
- File: `/Users/ryanday/repos/sherpa2/src/app.rs`:99
- Affects: `/Users/ryanday/repos/sherpa2/src/app.rs`
- Problem: Comment says "Poll for events with timeout (allows 4+ FPS for smooth UI)" but a 250ms timeout means at most 4 FPS, not "4+ FPS". The comment implies more than 4 FPS is achieved when the timeout alone limits it to exactly 4 FPS maximum (assuming no processing overhead).
- Fix: Change to "Poll for events with timeout (allows ~4 FPS updates for responsive UI)"

---

### Improvements

### CA-004: Missing comment on complex filter chain
- Severity: P2
- Type: missing-comment
- File: `/Users/ryanday/repos/sherpa2/src/screens/deep_review.rs`:31-42
- Affects: `/Users/ryanday/repos/sherpa2/src/screens/deep_review.rs`
- Problem: The `get_sections_needing_review` method returns a tuple of `(usize, &Section)` but the significance of the index (original position in session.sections, not filtered index) is not explained. This is important for correctly updating the right section when navigating.
- Fix: Add comment: "Returns (original_index, section) pairs where original_index is the section's position in session.sections, not the filtered list. This index is needed for correct updates when the user selects a section."

### CA-005: Missing explanation for screen state reset behavior
- Severity: P2
- Type: missing-comment
- File: `/Users/ryanday/repos/sherpa2/src/screens/pseudocode.rs`:76-84
- Affects: `/Users/ryanday/repos/sherpa2/src/screens/pseudocode.rs`
- Problem: The complex state reset logic at the start of `handle_input` has no comment explaining why this is necessary. A future maintainer might not understand why the screen state needs to be reset under these specific conditions.
- Fix: Add comment: "Reset internal state when returning to this screen after navigation. This handles the case where user pressed Esc during Submitted state - we need to restore Input mode since no AI response is pending."

### CA-006: Missing rationale for hash-based session filenames
- Severity: P2
- Type: missing-comment
- File: `/Users/ryanday/repos/sherpa2/src/session/persistence.rs`:22-31
- Affects: `/Users/ryanday/repos/sherpa2/src/session/persistence.rs`
- Problem: The function comment explains that it handles special characters but does not explain why a hash is used instead of URL encoding or simple character replacement. The choice of hashing has implications (collision risk, cannot reverse lookup identifier from filename) that should be documented.
- Fix: Add to function doc comment: "Uses hashing rather than character escaping to ensure uniform filename length and avoid edge cases with very long identifiers. Trade-off: filenames are not human-readable, but collision risk is negligible for typical usage patterns."

### CA-007: Missing comment on submit_hypothesis behavior
- Severity: P2
- Type: missing-comment
- File: `/Users/ryanday/repos/sherpa2/src/screens/pseudocode.rs`:417-421
- Affects: `/Users/ryanday/repos/sherpa2/src/screens/pseudocode.rs`
- Problem: The `submit_hypothesis` method sets `needs_assessment_retry = true` which is confusing since this is a first submission, not a retry. The pattern of using the retry flag for initial submissions needs explanation.
- Fix: Add comment: "Uses the retry flag to signal app's main loop to perform assessment. The 'retry' naming is a misnomer - this flag triggers both initial submissions and actual retries. The main loop handles both cases identically."

---

### Removals

### CA-008: Outdated phase-based task comments
- Severity: P2
- Type: aspirational-todo
- File: Multiple files (see Affects)
- Affects: `/Users/ryanday/repos/sherpa2/src/app.rs`:1, `/Users/ryanday/repos/sherpa2/src/cli.rs`:1,41-46, `/Users/ryanday/repos/sherpa2/src/models/mod.rs`:1, `/Users/ryanday/repos/sherpa2/src/models/section.rs`:1, `/Users/ryanday/repos/sherpa2/src/models/session.rs`:1, `/Users/ryanday/repos/sherpa2/src/models/state.rs`:1-2, `/Users/ryanday/repos/sherpa2/src/ai/mod.rs`:1-2, `/Users/ryanday/repos/sherpa2/src/ai/types.rs`:1-3, `/Users/ryanday/repos/sherpa2/src/ai/client.rs`:1-4, `/Users/ryanday/repos/sherpa2/src/screens/mod.rs`:1-2, `/Users/ryanday/repos/sherpa2/src/session/mod.rs`:1, `/Users/ryanday/repos/sherpa2/src/session/persistence.rs`:1, `/Users/ryanday/repos/sherpa2/src/git.rs`:1
- Problem: Multiple `@task(P1-T*)` comments reference implementation tasks that are clearly complete. These provide no ongoing value and clutter the code. Examples: "@task(P1-T6) Create app shell: terminal setup, main event loop, screen routing" when the app shell is fully implemented.
- Fix: Remove all `@task` comments from production code. If task tracking is needed, use issues or a project management tool.

### CA-009: Unnecessary comment stating the obvious
- Severity: P2
- Type: unnecessary-comment
- File: `/Users/ryanday/repos/sherpa2/src/error.rs`:1
- Affects: `/Users/ryanday/repos/sherpa2/src/error.rs`
- Problem: Comment "// Error types for the application" is redundant - the file is named `error.rs` and contains `pub enum AppError`.
- Fix: Remove the comment or convert to module-level doc comment with more useful information.

### CA-010: Unnecessary comment on lib.rs
- Severity: P2
- Type: unnecessary-comment
- File: `/Users/ryanday/repos/sherpa2/src/lib.rs`:1
- Affects: `/Users/ryanday/repos/sherpa2/src/lib.rs`
- Problem: Comment "// Library crate for testing and reuse" adds minimal value - this is standard Rust crate structure.
- Fix: Either remove or expand to `//! Library crate exposing sherpa modules for testing and potential reuse as a library.` as a proper module doc comment.

### CA-011: Screen header comments are redundant with implementation
- Severity: P2
- Type: unnecessary-comment
- File: `/Users/ryanday/repos/sherpa2/src/screens/triage.rs`:2-6, `/Users/ryanday/repos/sherpa2/src/screens/deep_review.rs`:1-3, `/Users/ryanday/repos/sherpa2/src/screens/pseudocode.rs`:1-3, `/Users/ryanday/repos/sherpa2/src/screens/summary.rs`:1, `/Users/ryanday/repos/sherpa2/src/screens/loading.rs`:1
- Affects: `/Users/ryanday/repos/sherpa2/src/screens/triage.rs`, `/Users/ryanday/repos/sherpa2/src/screens/deep_review.rs`, `/Users/ryanday/repos/sherpa2/src/screens/pseudocode.rs`, `/Users/ryanday/repos/sherpa2/src/screens/summary.rs`, `/Users/ryanday/repos/sherpa2/src/screens/loading.rs`
- Problem: Comments like "TriageScreen section list panel: scrollable, badges for tags" and "TriageScreen keybindings: j/k navigate, 1/2/3 tag, Enter proceed, q quit" describe implementation details that are immediately visible in the code. These were likely planning notes that should be removed now that implementation is complete.
- Fix: Remove header comments or convert to module-level documentation (`//!`) with higher-level purpose description.

---

### Well-Documented Patterns (No Action Needed)

The following comment patterns in the codebase are well-done:

1. **Doc comments on public APIs** - Functions like `detect_default_branch()`, `session_path()`, and trait methods have accurate, helpful documentation.

2. **The `@review-defer` and `@review-skip` annotations** - While unusual, these provide valuable context for deferred technical decisions and explicitly skipped review items. The format includes issue IDs, rationale, and tracking tags.

3. **Error handling comments** - Comments like "// Make save failures very visible since user may lose progress" (main.rs:104) explain the "why" behind non-obvious behavior.

4. **Test function comments** - Tests have clear names and occasional comments explaining edge cases being tested.

---

### Statistics

- **Critical Issues (P0-P1)**: 3
- **Improvements (P2)**: 4  
- **Removals (P2)**: 4
- **Total Issues**: 11

## Test Audit Review

### Summary

This Rust codebase for a Code Review TUI application has **moderate test coverage** with 234 tests across multiple modules. However, several critical code paths lack meaningful testing, and some tests provide false confidence by only checking superficial properties.

**Overall Assessment**: The codebase demonstrates good testing discipline for data models and screen input handling, but has significant gaps in testing external system interactions (git commands, AI subprocess) and error recovery paths.

---

## Critical Code Needing Tests

### TA-001: CLI commit count boundary validation untested
- Severity: P1
- Type: missing-test
- File: /Users/ryanday/repos/sherpa2/src/cli.rs:4-9
- Affects: /Users/ryanday/repos/sherpa2/src/cli.rs, /Users/ryanday/repos/sherpa2/tests/e2e.rs
- Problem: The `parse_commit_count` function validates that commit count is between 1 and 999, but tests do not verify edge cases (0, 1, 999, 1000, negative after parse failure).
- Fix: Add tests for `parse_commit_count("0")`, `parse_commit_count("999")`, `parse_commit_count("1000")` to verify boundary enforcement.

### TA-002: ClaudeClient::run_claude subprocess execution completely untested
- Severity: P0
- Type: missing-test
- File: /Users/ryanday/repos/sherpa2/src/ai/claude.rs:27-70
- Affects: /Users/ryanday/repos/sherpa2/src/ai/claude.rs
- Problem: The core `run_claude` method that spawns the claude subprocess, handles timeouts, and reads output has zero test coverage. This is critical business logic that interacts with external processes.
- Fix: Add integration tests with a mock claude executable or use dependency injection to test timeout handling, exit code failures, and stdout/stderr reading logic.

### TA-003: Git command execution paths have environment-dependent tests
- Severity: P1
- Type: missing-test
- File: /Users/ryanday/repos/sherpa2/src/git.rs:67-81
- Affects: /Users/ryanday/repos/sherpa2/src/git.rs
- Problem: `read_diff_commits` and `read_diff_branch` rely on actual git state. Tests only work when run in the sherpa2 repo with commits. Error paths (invalid commit range, branch not found) are untested.
- Fix: Create tests that verify error handling for invalid ranges (e.g., HEAD~1000000) and test with mock git operations or isolated test repos.

### TA-004: Session persistence atomic write untested for failure cases
- Severity: P1
- Type: missing-test
- File: /Users/ryanday/repos/sherpa2/src/session/persistence.rs:90-119
- Affects: /Users/ryanday/repos/sherpa2/src/session/persistence.rs
- Problem: `save_session` uses atomic temp-file-then-rename pattern but tests do not verify: (1) failure during write to tmp, (2) failure during rename, (3) directory permission failures.
- Fix: Add tests that simulate filesystem failures (read-only directory, disk full) to verify error messages are correct and no partial files remain.

### TA-005: Main run() orchestration function lacks integration tests  
- Severity: P1
- Type: missing-test
- File: /Users/ryanday/repos/sherpa2/src/main.rs:36-117
- Affects: /Users/ryanday/repos/sherpa2/src/main.rs, /Users/ryanday/repos/sherpa2/tests/e2e.rs
- Problem: The `run()` function coordinates session loading, AI chunking, and app lifecycle but is not directly tested. E2E tests cover component behaviors in isolation but not the full startup flow.
- Fix: Add integration tests that exercise run() with mocked git/AI to verify session resume, --new flag, and corrupted session recovery.

---

## Tests That Provide False Confidence

### TA-006: Mock client test only checks is_success() not actual content
- Severity: P2
- Type: false-confidence
- File: /Users/ryanday/repos/sherpa2/src/ai/client.rs:164-171
- Affects: /Users/ryanday/repos/sherpa2/src/ai/client.rs
- Problem: `test_mock_client_assess_hypothesis` calls `assert!(result.is_success())` and `assert!(!assessment.correct.is_empty())` but the mock always returns the same hardcoded assessment. This tests the mock, not any real behavior.
- Fix: Since this is a mock, consider: (1) remove test as it provides no value, or (2) test that mock returns expected fixed values to document the mock's contract.

### TA-007: Git branch tests rely on execution environment
- Severity: P1
- Type: false-confidence
- File: /Users/ryanday/repos/sherpa2/src/git.rs:137-149
- Affects: /Users/ryanday/repos/sherpa2/src/git.rs
- Problem: `test_detect_default_branch` asserts branch is "main" or "master" - this is testing the repo's actual config, not the function's logic. If run in a repo with a different default branch, test fails.
- Fix: Tests should use isolated git repositories with known state, or mock the Command execution.

### TA-008: Loading screen tick test is trivial
- Severity: P2
- Type: false-confidence
- File: /Users/ryanday/repos/sherpa2/src/screens/loading.rs:140-146
- Affects: /Users/ryanday/repos/sherpa2/src/screens/loading.rs
- Problem: `test_loading_screen_tick` only verifies the throbber index changed, but doesn't test any meaningful behavior - it's testing the throbber_widgets_tui library, not app logic.
- Fix: Remove this test or document it as a sanity check that the throbber library is properly integrated.

---

## Missing Scenarios

### TA-009: Empty diff edge case in AI chunking
- Severity: P1
- Type: missing-scenario
- File: /Users/ryanday/repos/sherpa2/src/ai/claude.rs:150-161
- Affects: /Users/ryanday/repos/sherpa2/src/ai/claude.rs
- Problem: `chunk_diff` is called with diff text but there's no test for how it handles an empty or whitespace-only diff string passed to the AI.
- Fix: Add test verifying behavior when empty string passed to chunk_diff - should either error gracefully or return empty sections.

### TA-010: DeepReviewScreen current_review_index bounds checking
- Severity: P2
- Type: missing-scenario
- File: /Users/ryanday/repos/sherpa2/src/screens/deep_review.rs:346-362
- Affects: /Users/ryanday/repos/sherpa2/src/screens/deep_review.rs
- Problem: `advance_to_next` and `go_to_previous` wrap around, but no tests verify behavior when `current_review_index` starts out-of-bounds (e.g., if sections were removed).
- Fix: Add test where current_review_index is greater than needs_review.len() and verify it clamps correctly.

### TA-011: Session identifier hash collision handling
- Severity: P2
- Type: missing-scenario
- File: /Users/ryanday/repos/sherpa2/src/session/persistence.rs:27-31
- Affects: /Users/ryanday/repos/sherpa2/src/session/persistence.rs
- Problem: `hash_identifier` uses DefaultHasher which could theoretically have collisions. No test verifies behavior when two different identifiers hash to the same value.
- Fix: Document this as known limitation or add test with known colliding strings (if possible) to verify identifier mismatch detection catches it.

### TA-012: PseudocodeReviewScreen multi-byte character cursor handling
- Severity: P2
- Type: missing-scenario
- File: /Users/ryanday/repos/sherpa2/src/screens/pseudocode.rs:153-159
- Affects: /Users/ryanday/repos/sherpa2/src/screens/pseudocode.rs
- Problem: Backspace handling uses `input_text.remove(cursor_position)` which operates on byte indices. No tests verify behavior with multi-byte UTF-8 characters where cursor position may not be a valid byte boundary.
- Fix: Add test typing CJK or emoji characters then pressing backspace to verify no panic or corruption.

### TA-013: Export markdown special character escaping
- Severity: P2  
- Type: missing-scenario
- File: /Users/ryanday/repos/sherpa2/src/screens/summary.rs:263-347
- Affects: /Users/ryanday/repos/sherpa2/src/screens/summary.rs
- Problem: `export_markdown` writes user hypothesis text directly to markdown without escaping. If hypothesis contains markdown syntax (e.g., `**bold**`, `[link](url)`), output formatting may be broken.
- Fix: Add test with hypothesis containing markdown special characters and verify output is properly escaped or rendered as intended.

---

## Flaky Patterns

### TA-014: Session persistence test uses process ID for uniqueness
- Severity: P2
- Type: flaky
- File: /Users/ryanday/repos/sherpa2/tests/e2e.rs:96
- Affects: /Users/ryanday/repos/sherpa2/tests/e2e.rs
- Problem: Tests use `std::process::id()` to create unique identifiers, but this may collide if tests are run in parallel with fork or if PID wraps (unlikely but possible in long CI runs).
- Fix: Use UUID or timestamp+random for test session identifiers, or ensure tests clean up before running.

### TA-015: Export test changes current working directory
- Severity: P1
- Type: flaky
- File: /Users/ryanday/repos/sherpa2/src/screens/summary.rs:423-424
- Affects: /Users/ryanday/repos/sherpa2/src/screens/summary.rs
- Problem: `test_summary_export_markdown` calls `std::env::set_current_dir(&temp)` which affects global process state and can cause flaky parallel test execution.
- Fix: Modify export_markdown to accept an output directory parameter, or use test isolation (run in separate process).

---

## Well-Covered Areas

- **Data models** (Session, Section, Tag, Assessment): Comprehensive serialization and state transition tests
- **Screen input handling**: Good coverage of keyboard navigation, tagging, and error state handling
- **JSON parsing utilities**: Edge cases well tested (code blocks, nested JSON, escaped characters)
- **E2E workflow tests**: Good coverage of complete review flows and session resume scenarios

---

## Test Quality Metrics

| Module | Tests | Key Gaps |
|--------|-------|----------|
| src/ai/claude.rs | 26 | Subprocess execution untested |
| src/ai/client.rs | 10 | Mock tests provide limited value |
| src/session/persistence.rs | 10 | Failure paths untested |
| src/screens/*.rs | 45 | Multi-byte input, bounds checking |
| src/git.rs | 3 | Environment-dependent, error paths missing |
| tests/e2e.rs | 6 | Does not test main() orchestration |


## Code Simplifier Review

**Reviewer:** Code Simplifier Agent  
**Date:** 2026-02-04  
**Files Analyzed:** All 22 Rust source files in `/Users/ryanday/repos/sherpa2/src/`

### Summary

The codebase is well-structured overall with clear separation of concerns. The code already has extensive `@review-defer` annotations from prior reviews, indicating many architectural decisions are intentionally deferred. This review focuses on simplification opportunities not covered by existing annotations.

---

### CS-001: Duplicated Result Type Pattern in AI Types
- **Severity:** P2
- **Type:** code-smell
- **File:** `/Users/ryanday/repos/sherpa2/src/ai/types.rs`:9-94
- **Affects:** `/Users/ryanday/repos/sherpa2/src/ai/types.rs`
- **Problem:** `ChunkingResult` and `AssessmentResult` enums have identical structure (Success/Error variants), and `ChunkingError` and `AssessmentError` structs are identical (both have `message: String` and `raw_output: Option<String>`). This is classic DRY violation.
- **Caveat:** Already noted in existing `@review-defer(AR-003)` and `@review-defer(RD-005)` comments. The types may need to diverge in future, and consolidation adds a crate dependency or macro complexity.
- **Fix:** Already deferred - no action needed. This is flagged for completeness of the simplification audit.

---

### CS-002: Repeated Section-Filtering Logic in DeepReviewScreen
- **Severity:** P2
- **Type:** complexity
- **File:** `/Users/ryanday/repos/sherpa2/src/screens/deep_review.rs`:28-62
- **Affects:** `/Users/ryanday/repos/sherpa2/src/screens/deep_review.rs`
- **Problem:** Three separate methods (`get_sections_needing_review`, `total_needing_review`, `reviewed_count`) each iterate over `state.session.sections` with similar filter logic. In `render()` and `handle_input()`, `get_sections_needing_review()` is called multiple times per frame.
- **Caveat:** Already noted in `@review-defer(CS-006)` and `@review-defer(PERF-002)`. Section counts are typically small (5-20), making this negligible. Caching would add complexity.
- **Fix:** Already deferred - the repeated iteration is a micro-optimization for typical workloads.

---

### CS-003: Inconsistent Selection State Ownership
- **Severity:** P1
- **Type:** tech-debt
- **File:** `/Users/ryanday/repos/sherpa2/src/models/state.rs`:42-66 and `/Users/ryanday/repos/sherpa2/src/screens/triage.rs`:19-21
- **Affects:** `/Users/ryanday/repos/sherpa2/src/models/state.rs`, `/Users/ryanday/repos/sherpa2/src/screens/triage.rs`, `/Users/ryanday/repos/sherpa2/src/screens/deep_review.rs`
- **Problem:** Selection state is split between `UiState.selected_index` and individual screens (`TriageScreen.list_state`, `DeepReviewScreen.current_review_index`). The code has to synchronize these in non-obvious ways (e.g., `state.ui.selected_index = section_idx` in `deep_review.rs:175`).
- **Caveat:** Already noted in `@review-defer(AR-008)` and `@review-defer(CR-002)`. Ratatui's `ListState` is widget-specific, creating tension between centralized state and ratatui's patterns.
- **Fix:** Architectural decision needed - either centralize all selection in `UiState` (screens become pure views) or document the hybrid approach clearly.

---

### CS-004: Long Function in `git.rs::detect_default_branch`
- **Severity:** P2
- **Type:** complexity
- **File:** `/Users/ryanday/repos/sherpa2/src/git.rs`:7-45
- **Affects:** `/Users/ryanday/repos/sherpa2/src/git.rs`
- **Problem:** `detect_default_branch()` has three sequential git command invocations with repetitive error handling patterns. Each block does: spawn command, check success, extract string.
- **Caveat:** The repetition is visible but the logic is simple and linear. Extracting a helper would require handling the varied return types (config value vs. existence check).
- **Fix:** Consider a helper like `fn run_git_command(args: &[&str]) -> Result<Option<String>>` that returns `Ok(Some(stdout))` on success, `Ok(None)` if command fails (for existence checks), or `Err` for spawn failures. This would reduce the three 10-line blocks to three 2-line calls.

---

### CS-005: Render Methods in SummaryScreen Could Share Bar Calculation
- **Severity:** P2
- **Type:** code-smell
- **File:** `/Users/ryanday/repos/sherpa2/src/screens/summary.rs`:89-142
- **Affects:** `/Users/ryanday/repos/sherpa2/src/screens/summary.rs`
- **Problem:** `render_confidence_breakdown` has three nearly identical calculations for bar widths:
  ```rust
  let got_it_width = if total > 0 { (counts.got_it * bar_width) / total } else { 0 };
  let shaky_width = if total > 0 { (counts.shaky * bar_width) / total } else { 0 };
  let lost_width = if total > 0 { (counts.lost * bar_width) / total } else { 0 };
  ```
- **Caveat:** Already noted in `@review-defer(CS-005)`. The code is localized and readable; extracting to a helper is subjective style preference.
- **Fix:** A simple `fn proportion(count: usize, total: usize, width: usize) -> usize` helper would reduce repetition, but this is minor.

---

### CS-006: JSON Extraction Functions Have Overlapping Logic
- **Severity:** P2
- **Type:** complexity
- **File:** `/Users/ryanday/repos/sherpa2/src/ai/claude.rs`:280-334
- **Affects:** `/Users/ryanday/repos/sherpa2/src/ai/claude.rs`
- **Problem:** The `extract_json` function handles multiple formats (code blocks, plain code blocks, raw JSON) in a single 40-line function with nested conditionals and early returns.
- **Caveat:** Already noted in `@review-defer(PERF-007)`. The function runs once per API call, not per frame. It's also well-tested with edge cases.
- **Fix:** The function could be split into separate extractors (`try_json_code_block`, `try_plain_code_block`, `try_raw_json`) that are tried in sequence. However, the current implementation is readable and the complexity is inherent to the problem.

---

### CS-007: Repetitive Screen Input Handling Patterns
- **Severity:** P1
- **Type:** code-smell
- **File:** Multiple screen files
- **Affects:** `/Users/ryanday/repos/sherpa2/src/screens/loading.rs`, `/Users/ryanday/repos/sherpa2/src/screens/triage.rs`, `/Users/ryanday/repos/sherpa2/src/screens/deep_review.rs`, `/Users/ryanday/repos/sherpa2/src/screens/pseudocode.rs`
- **Problem:** Each screen's `handle_input` starts with the same pattern:
  ```rust
  if state.ui.error.is_some() {
      return match handle_error_state_input(&key) {
          ErrorInputResult::Quit => { state.quit(); Ok(true) }
          ErrorInputResult::Retry => { /* screen-specific */ }
          ErrorInputResult::NotHandled => { /* screen-specific */ }
      };
  }
  ```
  The quit handling is identical across all screens.
- **Caveat:** The `handle_error_state_input` helper already exists but screens still need boilerplate to wire it up. Some screens have custom behavior (DeepReview has 's' to skip, Pseudocode has Esc handling).
- **Fix:** Consider a trait default method or helper that handles the common Quit case, leaving screens to override only Retry and NotHandled. Alternatively, document the pattern as intentional for explicitness.

---

### CS-008: Test Helper Duplication in Screen Tests
- **Severity:** P2
- **Type:** code-smell
- **File:** `/Users/ryanday/repos/sherpa2/src/screens/triage.rs`:316-333, `/Users/ryanday/repos/sherpa2/src/screens/deep_review.rs`:370-397
- **Affects:** `/Users/ryanday/repos/sherpa2/src/screens/triage.rs`, `/Users/ryanday/repos/sherpa2/src/screens/deep_review.rs`, `/Users/ryanday/repos/sherpa2/src/screens/pseudocode.rs`, `/Users/ryanday/repos/sherpa2/src/screens/summary.rs`
- **Problem:** Each screen module has its own `create_test_state()` helper that creates similar test sessions. The helpers differ slightly (e.g., different tags, different section counts) but share significant setup code.
- **Caveat:** Test helpers being local to each module keeps tests self-contained and readable. Shared test fixtures can create coupling.
- **Fix:** Consider a `tests/common.rs` or `src/test_helpers.rs` module with flexible builder functions like `SessionBuilder::new().with_sections(2).with_tags([GotIt, Shaky]).build()`. This is optional - current approach works.

---

### Issues Not Flagged (Justified by Existing Annotations)

The following were considered but not flagged because existing `@review-defer` annotations with valid caveats already cover them:

1. **Session memory usage** (`PERF-005`): Storing full diff_text for re-chunking is intentional
2. **Blocking AI calls** (`PERF-001`, `SFH-010`): Requires async architecture, deferred
3. **Terminal setup/restore testing** (`TA-010`): Requires crossterm mocking, deferred
4. **Hash stability** (`CR-005`): Trade-off between crate dependency and rare session orphaning
5. **Atomic file rename** (`CR-010`): Same-filesystem rename is atomic on POSIX/NTFS

---

### Positive Observations

1. **Good use of shared helpers**: `render_error_panel`, `render_footer_hints`, `handle_scroll_input`, `highlight_diff_lines` reduce duplication across screens
2. **Clear error type hierarchy**: `AppError` enum with `#[from]` for io::Error shows good thiserror usage
3. **Comprehensive test coverage**: Each module has unit tests, plus integration tests in `tests/e2e.rs`
4. **Well-documented defer decisions**: The `@review-defer` annotations provide clear rationale for deferred complexity decisions

---

### Recommendations

1. **CS-003 (P1)**: Address the selection state inconsistency - either document the current hybrid approach or refactor to centralize
2. **CS-007 (P1)**: Consider extracting more of the error-handling boilerplate into the ScreenTrait or a helper
3. **CS-004 (P2)**: Extract git command helper to reduce repetition in `git.rs`
4. **CS-008 (P2)**: Consider shared test fixtures if adding more screen tests


## Security Review

### SEC-001: Potential Command Injection via Claude CLI
- Severity: P1
- Type: command-injection
- File: /Users/ryanday/repos/sherpa2/src/ai/claude.rs:27-34
- Affects: src/ai/claude.rs
- Attack Vector: An attacker with write access to a git repository could craft a malicious diff containing shell metacharacters. When the diff content is passed to the `run_claude` function as part of the prompt, although it is passed as a single argument to `-p`, the content could potentially influence Claude's behavior or, depending on how the `claude` CLI processes its arguments, could have unintended effects.
- Impact: If exploitable, could lead to arbitrary command execution or manipulation of AI responses. However, the `-p` flag passes the entire prompt as a single argument, which mitigates direct shell injection.
- Caveat: The `Command::new("git")` and `Command::new("claude")` calls use argument arrays rather than shell string interpolation, which prevents most command injection attacks. The prompt is passed as a single argument to `-p`, not through a shell. The actual risk depends on how the `claude` CLI handles the prompt internally. This is more of a defense-in-depth concern.
- Fix: Consider sanitizing or escaping special characters in the diff content before including it in prompts, or add input length limits to prevent excessively large prompts. The codebase already has a `@review-defer(SEC-001)` comment acknowledging this concern.

### SEC-002: Dangerous Skip Permissions Flag in Claude CLI
- Severity: P2
- Type: auth-bypass
- File: /Users/ryanday/repos/sherpa2/src/ai/claude.rs:29
- Affects: src/ai/claude.rs
- Attack Vector: The `--dangerously-skip-permissions` flag bypasses Claude CLI's permission system. While this is intentional for automation, it means the application runs without the normal permission checks that would protect against certain operations.
- Impact: The Claude CLI may perform actions without user confirmation that would normally require explicit consent. This is a design decision trade-off for automation, but reduces defense-in-depth.
- Caveat: This is an intentional design choice for automation workflows. The codebase already acknowledges this with `@review-defer(SEC-002)`. The risk is primarily that users may not realize the CLI is running with elevated permissions.
- Fix: Document this behavior prominently for users, or add a configuration option to allow users to opt into the permission-skipping behavior rather than having it as the default.

### SEC-003: Session Files Stored Without Encryption
- Severity: P2
- Type: data-exposure
- File: /Users/ryanday/repos/sherpa2/src/session/persistence.rs:90-119
- Affects: src/session/persistence.rs, src/models/session.rs
- Attack Vector: Session files are stored as plain JSON in `~/.sherpa/sessions/`. If a user's home directory is accessible to other users or processes, the session data (including git diffs which may contain sensitive code) could be read.
- Impact: Exposure of code review data including git diffs, user hypotheses, and AI assessments. This could reveal sensitive intellectual property or security-related code changes.
- Caveat: The code correctly sets directory permissions to 0700 (owner-only) on Unix systems, which mitigates this risk significantly. The concern is primarily on non-Unix systems or in shared computing environments where file permissions may not be enforced as expected.
- Fix: The current implementation with 0700 permissions is reasonable for most use cases. For higher security environments, consider adding optional encryption of session files using a user-provided key or system keychain integration.

### SEC-004: Git Diff Contains Potentially Sensitive Data
- Severity: P2
- Type: data-exposure
- File: /Users/ryanday/repos/sherpa2/src/git.rs:67-81
- Affects: src/git.rs, src/ai/claude.rs, src/session/persistence.rs
- Attack Vector: The application reads git diffs which may contain sensitive information (credentials, API keys, secrets) if a developer accidentally commits such data. This data is then: (1) stored in session files, (2) sent to the Claude CLI, and (3) potentially exported to markdown files.
- Impact: Secrets embedded in code diffs could be persisted to disk and sent to external AI services.
- Caveat: This is inherent to any code review tool that processes git diffs. The diff content comes from the user's own repository. However, users may not realize their diffs are being persisted and sent externally.
- Fix: Consider adding a warning to users about what data will be processed, or implementing a secret-scanning pass on diffs before processing (similar to git-secrets or trufflehog patterns) to warn users if potential secrets are detected.

### SEC-005: Export Creates Files in Current Working Directory
- Severity: P2
- Type: path-traversal
- File: /Users/ryanday/repos/sherpa2/src/screens/summary.rs:263-347
- Affects: src/screens/summary.rs
- Attack Vector: The export function creates files in a `docs/` subdirectory relative to the current working directory. If the application is run from an unexpected directory, files could be created in unintended locations.
- Impact: Low - files are only created in `docs/` subdirectory with controlled filenames (timestamp-based). No user-controlled path components are used in the file path.
- Caveat: The filename is generated from a timestamp and a constant prefix ("sherpa-review-"). Session identifier content is written inside the file but not used in the path. The codebase has a `@review-skip(SEC-005)` comment noting the identifier is validated by clap.
- Fix: Current implementation is acceptable. For additional safety, could use an absolute path (e.g., relative to the git repository root) rather than cwd.

### SEC-006: No Rate Limiting on AI Calls
- Severity: P2
- Type: access-control
- File: /Users/ryanday/repos/sherpa2/src/ai/claude.rs:149-178
- Affects: src/ai/claude.rs, src/ai/client.rs
- Attack Vector: There is no rate limiting on AI calls. A user (or automated process) could rapidly trigger many AI requests, potentially exhausting API quotas or incurring unexpected costs.
- Impact: Resource exhaustion, unexpected billing, potential denial of service to the user's own Claude API access.
- Caveat: The `busy` flag prevents concurrent calls, but does not limit the rate of sequential calls. This is primarily a cost/resource concern rather than a security vulnerability.
- Fix: Consider implementing rate limiting (e.g., minimum delay between calls, maximum calls per time window) or usage tracking/warnings.

### Summary

The codebase demonstrates good security practices overall:

1. **Command execution**: Uses `Command` with argument arrays rather than shell strings, avoiding shell injection.
2. **File permissions**: Session directory is created with 0700 permissions on Unix.
3. **Input validation**: CLI arguments are validated by clap with proper type constraints.
4. **Atomic file operations**: Session saves use temporary files with atomic rename.
5. **No unsafe code**: No `unsafe` blocks were found in the source files.
6. **Dependency hygiene**: Dependencies are mainstream, well-maintained crates (ratatui, clap, serde, etc.).

The main security considerations are:
- The `--dangerously-skip-permissions` flag is a documented trade-off for automation
- Diff content is sent to external AI services and persisted to disk
- Session data is stored unencrypted (but with restricted permissions)

No critical (P0) vulnerabilities were identified that would allow remote code execution, authentication bypass, or data breaches in the typical threat model for a local CLI tool.
