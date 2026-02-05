# Code Review: Full Repository

**Date:** 2026-02-05
**Scope:** Entire repository

---


## Expo UI Reviewer

**N/A** - This is a Rust TUI application, not an Expo/React Native project. Expo UI guidelines review is not applicable.


## Guidelines Checker Review

### Summary

**No project guidelines found.**

The repository at `/Users/ryanday/repos/sherpa2` does not contain:
- `CLAUDE.md` - No project conventions file
- `.claude/settings.json` - No Claude settings
- `rustfmt.toml` / `.rustfmt.toml` - No Rust formatting configuration
- `clippy.toml` - No Clippy linting configuration
- `.editorconfig` - No editor configuration
- `CONTRIBUTING.md` - No contribution guidelines
- `README.md` - No readme file

### Files Scanned

The following source files exist in the repository:
- `src/main.rs`
- `src/lib.rs`
- `src/app.rs`
- `src/cli.rs`
- `src/error.rs`
- `src/git.rs`
- `src/ai/mod.rs`
- `src/ai/client.rs`
- `src/ai/claude.rs`
- `src/ai/types.rs`
- `src/models/mod.rs`
- `src/models/session.rs`
- `src/models/state.rs`
- `src/models/section.rs`
- `src/screens/mod.rs`
- `src/screens/loading.rs`
- `src/screens/pseudocode.rs`
- `src/screens/summary.rs`
- `src/screens/triage.rs`
- `src/screens/deep_review.rs`
- `src/session/mod.rs`
- `src/session/persistence.rs`
- `tests/e2e.rs`

### Recommendation

Without explicit project guidelines, the guidelines checker cannot verify compliance. Consider adding a `CLAUDE.md` file to define:

1. **Import conventions** - Ordering rules (std, external crates, internal modules)
2. **Naming conventions** - File naming, function naming, type naming patterns
3. **Error handling patterns** - When to use `thiserror` vs `anyhow`, Result conventions
4. **Module organization** - How to structure new features
5. **Rust idioms** - Preferred patterns for async, iterators, etc.
6. **Testing conventions** - Unit test location, integration test patterns

### Violations Found

**None** - No guidelines to check against.


## Performance Review

Analyzed: All Rust source files in the repository (17 files, ~3000 lines of application code)

### PERF-001: Synchronous blocking AI calls in main event loop
- Severity: P0
- Type: blocking
- File: /Users/ryanday/repos/sherpa2/src/app.rs:131
- Affects: src/app.rs, src/ai/claude.rs, src/ai/client.rs
- Current: The `retry_chunking()` and `retry_assessment()` methods call `self.ai_client.chunk_diff()` and `self.ai_client.assess_hypothesis()` synchronously. These shell out to the `claude` CLI via `Command::new("claude")`, which blocks the entire event loop.
- At Scale: With large diffs (100x current size), the Claude CLI call could take 30+ seconds. During this time, the UI freezes completely - no spinner animation, no keyboard response, no Ctrl+C handling.
- Caveat: Acceptable for small demos or if AI latency is under 1 second. The `busy` flag exists but does not prevent UI blocking.
- Fix: Use async/await with tokio or spawn a thread for AI calls. Update state via channel when complete. The TUI framework (ratatui + crossterm) supports async event loops.

### PERF-002: Repeated linear scan for sections needing review
- Severity: P2
- Type: complexity
- File: /Users/ryanday/repos/sherpa2/src/screens/deep_review.rs:26
- Affects: src/screens/deep_review.rs
- Current: `get_sections_needing_review()` iterates over all sections on every call. This method is called multiple times per render (in `render()`) and per input event.
- At Scale: With 1000 sections, each render/input triggers 3-4 O(n) scans. At 4 FPS, this is 12,000-16,000 iterations per second.
- Caveat: Acceptable for typical reviews (5-20 sections). Would only matter if AI chunking produced hundreds of sections.
- Fix: Cache the filtered list in a field and invalidate only when sections change (tag modified, assessment added).

### PERF-003: Full session serialization on every quit
- Severity: P1
- Type: resource
- File: /Users/ryanday/repos/sherpa2/src/session/persistence.rs:94
- Affects: src/session/persistence.rs, src/main.rs
- Current: `save_session()` calls `serde_json::to_string_pretty()` which serializes the entire Session including all diff text and code sections.
- At Scale: With 100 sections each containing 1KB of code, plus a 100KB diff_text, serialization could take 10-50ms and write several MB to disk.
- Caveat: Only called once on quit, not on every keystroke. Acceptable unless sessions grow very large.
- Fix: Consider incremental saves or compressing the diff_text. For very large sessions, stream to file rather than building the entire string in memory.

### PERF-004: String cloning in hypothesis retry path
- Severity: P2
- Type: memory
- File: /Users/ryanday/repos/sherpa2/src/app.rs:153
- Affects: src/app.rs
- Current: In `retry_assessment()`, both `section.code.clone()` and `self.state.ui.input_text.clone()` are performed even if the section doesn't exist or hypothesis is empty.
- At Scale: With large code sections (10KB+), unnecessary clones allocate significant memory before the early returns are checked.
- Caveat: Minor issue - clones are cheap for typical sizes and happen infrequently (only on retry).
- Fix: Move the clones after the existence/empty checks, or use references where possible.

### PERF-005: Unbounded diff_text storage in Session
- Severity: P1
- Type: memory
- File: /Users/ryanday/repos/sherpa2/src/models/session.rs:17
- Affects: src/models/session.rs, src/session/persistence.rs
- Current: The Session struct stores `diff_text: String` which is the raw git diff. This is stored in memory for the entire session lifetime and serialized to disk.
- At Scale: A diff of 10MB (large refactor touching many files) would consume 10MB RAM throughout the session, and double during serialization.
- Caveat: Necessary for re-chunking feature. Could be acceptable if diffs are typically under 1MB.
- Fix: Consider storing only file references and reading on-demand, or compressing the stored diff, or chunking and discarding the raw diff after processing.

### PERF-006: Repeated code line styling on every render
- Severity: P2
- Type: resource
- File: /Users/ryanday/repos/sherpa2/src/screens/triage.rs:251
- Affects: src/screens/triage.rs, src/screens/deep_review.rs, src/screens/pseudocode.rs
- Current: Code preview panels (in triage, deep_review, pseudocode screens) iterate through every line of code and create styled `Line` objects on every render frame (~4 FPS).
- At Scale: A section with 1000 lines of code generates 1000 Line allocations per frame, 4000 per second. This creates GC pressure and CPU overhead.
- Caveat: ratatui is designed for this pattern and Rust's allocator is efficient. Only problematic for very large code sections.
- Fix: Cache the styled lines when the section changes rather than rebuilding on every frame. Store alongside the Section or in a render cache.

### PERF-007: JSON extraction scans entire response multiple times
- Severity: P2
- Type: complexity
- File: /Users/ryanday/repos/sherpa2/src/ai/claude.rs:251
- Affects: src/ai/claude.rs
- Current: `extract_json_array()` and `extract_json_object()` call `text.find()` multiple times for different patterns (```json, ```, raw brackets). Each `find()` is O(n) where n is response length.
- At Scale: With a 100KB AI response, multiple linear scans perform redundant work. In the worst case (JSON at the end), 4-5 full scans occur.
- Caveat: AI responses are typically under 10KB and this happens once per API call, not per frame.
- Fix: Use a single-pass parser or regex with alternation. Or scan once to find all candidate positions, then validate.

### Summary

The most critical issue is **PERF-001** (blocking AI calls) which will cause UI freezes at any scale. This should be addressed before production use.

**PERF-003** and **PERF-005** (session size) become problematic with large diffs and should be monitored.

The remaining issues (P2) are optimization opportunities that only matter at extreme scale (hundreds of sections, very large code) and can be deferred.


## Reuse Detector

### RD-001: Duplicate diff syntax highlighting logic
- Severity: P0
- Type: duplicate-component
- File: /Users/ryanday/repos/sherpa2/src/screens/triage.rs:251-266
- Affects: src/screens/triage.rs, src/screens/deep_review.rs, src/screens/pseudocode.rs
- Existing: src/screens/deep_review.rs:285-302 (identical logic also in pseudocode.rs:275-290)
- Problem: The diff syntax highlighting logic that colors `+` lines green, `-` lines red, `@@` lines cyan, and `diff`/`index` lines blue is copy-pasted identically across three screen modules (triage.rs, deep_review.rs, pseudocode.rs). This creates maintenance burden when changing highlighting rules.
- Caveat: If screens need different highlighting behaviors in the future, keeping separate implementations might be justified.
- Fix: Extract a shared function `fn highlight_diff_lines(code: &str) -> Vec<Line>` into a new `src/screens/common.rs` module or into `src/screens/mod.rs`. All three screens should import and use this shared function.

### RD-002: Duplicate error rendering pattern
- Severity: P1
- Type: established-pattern
- File: /Users/ryanday/repos/sherpa2/src/screens/loading.rs:60-79
- Affects: src/screens/loading.rs, src/screens/triage.rs, src/screens/deep_review.rs, src/screens/pseudocode.rs
- Existing: src/screens/triage.rs:300-314 (similar pattern in deep_review.rs:358-374)
- Problem: Error rendering with a red-bordered block containing error message and retry hints is implemented similarly in multiple screens. The pattern is: create a red-bordered Block with error message and action hints (r to retry, q to quit), render as Paragraph with center alignment.
- Caveat: Error messages and available actions differ slightly per screen, so some customization is needed.
- Fix: Create a shared `render_error_panel(frame, area, error: &str, hints: &str)` function that accepts the error message and custom hint text. Each screen can call this with its specific hints.

### RD-003: Duplicate footer/hints rendering pattern
- Severity: P1
- Type: established-pattern
- File: /Users/ryanday/repos/sherpa2/src/screens/triage.rs:276-288
- Affects: src/screens/triage.rs, src/screens/deep_review.rs, src/screens/pseudocode.rs, src/screens/summary.rs
- Existing: src/screens/deep_review.rs:321-328 (nearly identical in all screens)
- Problem: Footer hint rendering is duplicated across all screens. The pattern is always: create a Paragraph with hint text, center alignment, and DarkGray color.
- Caveat: Each screen has different hint text, so the function needs to accept the hints string.
- Fix: Create a shared `render_footer_hints(frame, area, hints: &str)` function in a common module. Each screen passes its specific hint string.

### RD-004: Duplicate error state handling in handle_input
- Severity: P1
- Type: established-pattern
- File: /Users/ryanday/repos/sherpa2/src/screens/triage.rs:64-78
- Affects: src/screens/triage.rs, src/screens/deep_review.rs, src/screens/pseudocode.rs, src/screens/loading.rs
- Existing: src/screens/deep_review.rs:106-126 (similar pattern)
- Problem: Multiple screens have the same error-state input handling pattern: check if error exists, then match on 'q' to quit and 'r' to clear error/retry. The core logic is duplicated.
- Caveat: Some screens have additional error-state keys (e.g., deep_review has 's' to skip). A shared helper would need to return whether the event was handled, allowing screens to add custom handlers.
- Fix: Consider a helper function `handle_error_state_input(key, state) -> Option<bool>` that handles common 'q' and 'r' keys, returning None if the key wasn't handled so the screen can add custom handling.

### RD-005: ChunkingError and AssessmentError are near-duplicates
- Severity: P2
- Type: duplicate-component
- File: /Users/ryanday/repos/sherpa2/src/ai/types.rs:31-49
- Affects: src/ai/types.rs
- Existing: src/ai/types.rs:74-91 (AssessmentError has identical structure)
- Problem: `ChunkingError` and `AssessmentError` have identical structure (message: String, raw_output: Option<String>) and identical implementations of `new()` and `with_output()` methods.
- Caveat: Having separate types provides clearer semantics and allows them to diverge in the future if needed. This is a minor reuse opportunity.
- Fix: Create a generic `AiOperationError` struct that both can use, or use a type alias. Alternatively, use the builder pattern with a single error type that includes an operation discriminant.

### RD-006: ChunkingResult and AssessmentResult follow identical pattern
- Severity: P2
- Type: established-pattern
- File: /Users/ryanday/repos/sherpa2/src/ai/types.rs:10-29
- Affects: src/ai/types.rs
- Existing: src/ai/types.rs:53-71 (AssessmentResult has identical structure)
- Problem: Both result enums have the same Success/Error pattern with identical `is_success()` methods. The only difference is the contained success type.
- Caveat: Rust's type system doesn't easily support this abstraction without macros. Keeping separate types is idiomatic and provides clarity.
- Fix: This could be unified with a generic `AiResult<T, E>` type, but the current approach is acceptable for a small codebase. Consider a macro if more AI operation types are added.

### RD-007: Scroll handling keybindings duplicated
- Severity: P2
- Type: established-pattern
- File: /Users/ryanday/repos/sherpa2/src/screens/triage.rs:127-143
- Affects: src/screens/triage.rs, src/screens/deep_review.rs
- Existing: src/screens/deep_review.rs:178-193 (identical scroll handling)
- Problem: The scroll handling for Ctrl+D, Ctrl+U, PageDown, PageUp is duplicated between triage and deep_review screens with identical logic (add/subtract 10 or 20 from scroll_offset).
- Caveat: Currently only two screens use this. If more screens need scrolling, a shared handler would be valuable.
- Fix: Extract a `handle_scroll_input(key, state) -> bool` helper that handles these four keys and returns whether it consumed the event.


## Correctness Review

### CR-001: Truncate function may panic on multi-byte UTF-8 characters
- Severity: P0
- Type: edge case
- File: /Users/ryanday/repos/sherpa2/src/ai/claude.rs:380
- Affects: src/ai/claude.rs
- Problem: The `truncate` function uses byte indexing `&s[..max_len]` which will panic if `max_len` falls in the middle of a multi-byte UTF-8 character. Claude responses often contain Unicode characters, emojis, or non-ASCII text which would trigger this panic.
- Caveat: Only affects error messages where Claude returns text with multi-byte characters in the first 200 bytes.
- Fix: Use `s.chars().take(max_len).collect::<String>()` or `s.char_indices().nth(max_len).map(|(i, _)| &s[..i]).unwrap_or(s)` for proper Unicode-aware truncation.

### CR-002: TriageScreen and DeepReviewScreen list_state not synchronized with AppState.ui
- Severity: P1
- Type: logic error
- File: /Users/ryanday/repos/sherpa2/src/screens/triage.rs:316-325
- Affects: src/screens/triage.rs, src/screens/deep_review.rs
- Problem: `TriageScreen.list_state` and `DeepReviewScreen.current_review_index` maintain their own selection state separate from `AppState.ui.selected_index`. When navigating away and returning to a screen, the selection could be out of sync. The `goto()` method in `AppState` resets `ui.selected_index` to 0, but the screen's internal state remains unchanged.
- Caveat: This may be intentional to preserve screen-specific selection, but creates confusion if one expects `ui.selected_index` to always reflect current selection.
- Fix: Either synchronize screen state with `AppState.ui.selected_index` on render/input, or remove `ui.selected_index` and use screen-specific state exclusively. Currently both are used inconsistently.

### CR-003: Potential index out of bounds in DeepReviewScreen Enter handler
- Severity: P1
- Type: logic error
- File: /Users/ryanday/repos/sherpa2/src/screens/deep_review.rs:163-166
- Affects: src/screens/deep_review.rs
- Problem: The Enter key handler calls `.unwrap()` on `.copied()` which can panic if `needs_review` is unexpectedly empty between the check and the access. While there is a guard `if !needs_review.is_empty()` before this code, the logic uses `current_review_index.min(needs_review.len() - 1)` which could underflow if `len()` is 0 (though guarded by the if-check). The `.unwrap()` call is still a code smell.
- Caveat: The guard check should prevent this, but race conditions in state mutation could theoretically cause issues.
- Fix: Replace `.unwrap()` with proper error handling or return early if the section cannot be retrieved.

### CR-004: Scroll offset may overflow when casting to u16 for Paragraph::scroll
- Severity: P2
- Type: edge case
- File: /Users/ryanday/repos/sherpa2/src/screens/triage.rs:271
- Affects: src/screens/triage.rs, src/screens/deep_review.rs, src/screens/pseudocode.rs
- Problem: `state.ui.scroll_offset as u16` will wrap around if scroll_offset exceeds 65535. With Ctrl+D adding 10 per press and PageDown adding 20, a user would need ~3000+ key presses, but there is no upper bound enforced.
- Caveat: Extremely unlikely to be hit in practice with real code diffs.
- Fix: Clamp scroll_offset to `u16::MAX` before casting, or use `.min(u16::MAX as usize) as u16`.

### CR-005: DefaultHasher is not guaranteed stable across Rust versions
- Severity: P2
- Type: edge case
- File: /Users/ryanday/repos/sherpa2/src/session/persistence.rs:20-24
- Affects: src/session/persistence.rs
- Problem: `std::collections::hash_map::DefaultHasher` is explicitly documented as not stable across Rust versions. If a user upgrades Rust and the hash algorithm changes, existing session files will become orphaned (new hash won't match old filename).
- Caveat: This would only affect users who upgrade Rust between sessions. The session would simply be treated as not found, not corrupted.
- Fix: Use a stable hashing algorithm like `std::hash::SipHasher` (deprecated but stable) or a crate like `siphasher` or `xxhash-rust` for deterministic hashing.

### CR-006: Assessment retry uses request_assessment_retry which doesn't trigger actual AI call
- Severity: P1
- Type: logic error
- File: /Users/ryanday/repos/sherpa2/src/screens/pseudocode.rs:416-419
- Affects: src/screens/pseudocode.rs, src/app.rs
- Problem: `submit_hypothesis` sets `state.ui.needs_assessment_retry = true` to trigger the AI call. However, looking at `App::retry_assessment()` in app.rs, it reads from `self.state.ui.selected_index` and `self.state.ui.input_text`. The `input_text` is correct, but `selected_index` may be stale if the user has navigated. The `selected_index` is only set when entering PseudocodeReview from DeepReview (line 167 in deep_review.rs).
- Caveat: If the user doesn't navigate away, this works correctly.
- Fix: Ensure `selected_index` is set correctly before triggering retry, or pass the section index directly in the retry mechanism.

### CR-007: Draft hypothesis not restored when returning to PseudocodeReview screen
- Severity: P1
- Type: logic error
- File: /Users/ryanday/repos/sherpa2/src/models/state.rs:129
- Affects: src/models/state.rs, src/screens/pseudocode.rs
- Problem: When `goto()` is called, `self.ui = UiState::default()` resets all UI state including `input_text`. The session stores `draft_hypothesis`, but nothing restores it to `ui.input_text` when entering PseudocodeReview. Users would lose their draft when navigating between screens.
- Caveat: The draft is saved in `session.draft_hypothesis`, but never loaded back into `ui.input_text`.
- Fix: Add logic to restore `state.ui.input_text = state.session.draft_hypothesis.clone().unwrap_or_default()` when entering PseudocodeReview screen.

### CR-008: Integer division in accuracy percentage can produce misleading results
- Severity: P2
- Type: logic error
- File: /Users/ryanday/repos/sherpa2/src/screens/summary.rs:152-156
- Affects: src/screens/summary.rs
- Problem: Integer division `(counts.confirmed * 100) / counts.total()` truncates. If 1 of 3 hypotheses is confirmed, the math is `(1 * 100) / 3 = 33%`. However, if 2 of 3 are confirmed, it shows `66%` not `67%`. More concerning, if total is large and confirmed is small, rounding errors compound.
- Caveat: Minor display issue, not a crash.
- Fix: Use floating point: `((counts.confirmed as f64 / counts.total() as f64) * 100.0).round() as usize`.

### CR-009: PseudocodeReviewScreen state not reset when entering screen
- Severity: P1
- Type: logic error
- File: /Users/ryanday/repos/sherpa2/src/screens/pseudocode.rs:28-32
- Affects: src/screens/pseudocode.rs
- Problem: `PseudocodeReviewScreen` maintains its own `state: PseudocodeState` field. When entering the screen fresh, this should be `Input`, but if the user previously entered, submitted, got an error, and navigated away, the screen state remains `Submitted`. On re-entry, the screen would show the wrong state.
- Caveat: The screen does check for assessment completion which may override, but the internal state machine is not reset.
- Fix: Reset `self.state = PseudocodeState::Input` at the start of `handle_input` or when screen is activated.

### CR-010: Atomic rename not guaranteed atomic on all file systems
- Severity: P2
- Type: edge case
- File: /Users/ryanday/repos/sherpa2/src/session/persistence.rs:100-102
- Affects: src/session/persistence.rs
- Problem: The comment states "POSIX guarantees atomicity" for `fs::rename`, but this is only true on POSIX systems with files on the same filesystem. On Windows or with cross-filesystem operations, the rename may not be atomic. If the app crashes during rename, data could be lost.
- Caveat: Most users will have session files on the same filesystem as temp files (both in ~/.sherpa/sessions/).
- Fix: Add a sync/flush before rename, or use a crate like `tempfile` with `persist()` for more robust atomic operations.


## Security Review

### Summary

This is a Rust TUI application for code review that shells out to the Claude CLI and git commands. Overall, the codebase demonstrates good Rust practices with no unsafe blocks. However, there are security concerns related to command execution and path handling that should be addressed.

### SEC-001: Prompt Injection via Git Diff Content
- Severity: P1
- Type: injection
- File: /Users/ryanday/repos/sherpa2/src/ai/claude.rs:180-211
- Affects: src/ai/claude.rs
- Attack Vector: A malicious git diff containing crafted content could influence the AI's behavior. The diff text is directly interpolated into the prompt sent to Claude CLI without any sanitization. An attacker who controls a git diff (e.g., a malicious contributor) could craft commit content that manipulates the AI's response, potentially causing it to emit malicious suggestions or ignore security-relevant changes.
- Impact: The AI could be manipulated to provide incorrect assessments, hide security issues in code reviews, or generate misleading outputs that influence developer decisions.
- Caveat: This requires the attacker to have write access to the repository being reviewed, and the impact depends on how much the user trusts the AI's output.
- Fix: Consider sanitizing or escaping special characters in the diff content before interpolation, or use a structured message format with clear delimiters that the AI is instructed to treat as data, not instructions.

### SEC-002: Command Execution with External CLI (Claude)
- Severity: P2
- Type: command-execution
- File: /Users/ryanday/repos/sherpa2/src/ai/claude.rs:27-32
- Affects: src/ai/claude.rs
- Attack Vector: The application shells out to the `claude` CLI with `--dangerously-skip-permissions` flag. While the prompt is passed via `-p` argument (not shell interpolation), the flag name itself indicates bypassing security controls. If the Claude CLI has vulnerabilities or if a malicious binary is placed in the PATH, it could lead to unintended behavior.
- Impact: Potential for unauthorized actions if the Claude CLI has security flaws, or complete compromise if a malicious `claude` binary is executed instead.
- Caveat: The prompt is passed as a separate argument (not shell-interpolated), reducing shell injection risk. The `--dangerously-skip-permissions` flag is intentional for automated tooling but warrants user awareness.
- Fix: Consider documenting this behavior prominently, verifying the claude binary path, or allowing users to configure whether to use this flag. Also consider using absolute paths for the claude binary.

### SEC-003: Session Data Stored in World-Readable Location
- Severity: P2
- Type: data-exposure
- File: /Users/ryanday/repos/sherpa2/src/session/persistence.rs:11-16
- Affects: src/session/persistence.rs
- Attack Vector: Session files are stored in `~/.sherpa/sessions/` without explicit permission restrictions. On multi-user systems, if the directory or files are created with permissive umask, other users could read session data containing git diffs, hypotheses, and AI assessments.
- Impact: Exposure of potentially sensitive code diffs, user hypotheses about code behavior, and AI-generated assessments to other local users.
- Caveat: Most single-user development machines have restrictive umasks by default. This is primarily a concern on shared systems.
- Fix: Set explicit restrictive permissions (0700 for directory, 0600 for files) when creating the session directory and files:
```rust
use std::os::unix::fs::PermissionsExt;
fs::create_dir_all(&dir)?;
fs::set_permissions(&dir, fs::Permissions::from_mode(0o700))?;
```

### SEC-004: No Input Validation on Commit Count
- Severity: P2
- Type: input-validation
- File: /Users/ryanday/repos/sherpa2/src/git.rs:63-77
- Affects: src/git.rs, src/cli.rs
- Attack Vector: The commit count is passed directly from CLI arguments to git diff command as `HEAD~N..HEAD`. While clap parses this as a `usize` (preventing non-numeric input), extremely large values could cause git to consume excessive resources or hang.
- Impact: Potential denial of service or resource exhaustion on the local machine.
- Caveat: This only affects the local user running the command and git typically handles large ranges gracefully by returning errors.
- Fix: Add an upper bound validation on the commit count:
```rust
#[arg(value_parser = clap::value_parser!(usize).range(1..1000))]
pub commits: Option<usize>,
```

### SEC-005: Markdown Export Path Not Sanitized
- Severity: P2
- Type: path-traversal
- File: /Users/ryanday/repos/sherpa2/src/screens/summary.rs:264-347
- Affects: src/screens/summary.rs
- Attack Vector: The export function creates files in `docs/` using a timestamp-based filename. While the filename itself is generated internally (not from user input), the session identifier displayed in the markdown content comes from user-controlled CLI arguments and is written directly to the file without sanitization.
- Impact: While not a direct path traversal (the path is controlled), the content written could contain malicious markdown if an attacker controls the session identifier (e.g., `commits:` prefix with injected content).
- Caveat: The session identifier is validated by clap to be either `None` (becomes "branch") or a `usize` (becomes "commits:N"), so direct injection is limited.
- Fix: Ensure the session identifier is properly escaped when written to the markdown file, or validate it matches expected patterns.

### Positive Findings

1. **No Unsafe Rust**: The codebase contains no `unsafe` blocks, relying entirely on safe Rust.

2. **Git Commands Use Argument Arrays**: All git commands use `.args([...])` instead of shell string interpolation, preventing shell injection.

3. **Atomic File Writes**: Session persistence uses atomic file writes (write to temp, then rename) to prevent corruption.

4. **No Hardcoded Secrets**: No credentials, API keys, or secrets found in the codebase.

5. **Input Validated by Type System**: CLI arguments are parsed through clap with type constraints, preventing many injection vectors.

6. **JSON Parsing Is Safe**: serde_json is used for deserialization which is memory-safe and handles malformed input gracefully.

### Recommendations

1. Add explicit file permissions when creating session directories and files
2. Document the `--dangerously-skip-permissions` flag usage for Claude CLI
3. Consider adding a maximum commit count limit
4. Review prompt construction to add clear data/instruction boundaries
5. Consider logging AI interactions for audit purposes (with appropriate privacy controls)

## Wiring Detector

### WD-001: anyhow dependency declared but never used
- Severity: P2
- Type: unused-dep
- Added: `anyhow = "1"` in Cargo.toml for error handling
- Location: /Users/ryanday/repos/sherpa2/Cargo.toml:28
- Affects: /Users/ryanday/repos/sherpa2/Cargo.toml
- Problem: The `anyhow` crate is listed as a dependency but is never imported or used anywhere in the codebase. The project uses `thiserror` for custom error types with a custom `Result` type alias instead.
- Evidence: No `use anyhow` or `anyhow::` patterns found anywhere in `/Users/ryanday/repos/sherpa2/src/`. The crate uses `thiserror::Error` in `src/error.rs` and defines `pub type Result<T> = std::result::Result<T, AppError>`.
- Caveat: May be planned for future use to simplify error propagation in main.rs or other modules.
- Fix: Remove `anyhow = "1"` from Cargo.toml dependencies if not planned for use, or integrate it for simpler error handling in places where detailed error context is not needed.

### WD-002: unidiff dependency declared but never used
- Severity: P1
- Type: unused-dep, incomplete-migration
- Added: `unidiff = "0.4"` in Cargo.toml for "Unified diff parsing"
- Location: /Users/ryanday/repos/sherpa2/Cargo.toml:34
- Affects: /Users/ryanday/repos/sherpa2/Cargo.toml, /Users/ryanday/repos/sherpa2/src/git.rs, /Users/ryanday/repos/sherpa2/src/ai/claude.rs
- Problem: The `unidiff` crate is declared for parsing unified diffs but is never imported or used. The AI chunking in `claude.rs` sends raw diff text directly to Claude CLI without parsing. The git module returns raw diff strings without structured parsing.
- Evidence: No `use unidiff` or `unidiff::` patterns found anywhere in the codebase. The `git::read_diff()` function returns `String` (raw output from `git diff`), and `ClaudeClient::chunk_diff()` passes this raw string directly to the Claude prompt.
- Caveat: The comment indicates this was intended for structured diff parsing. May be planned for future use to pre-process diffs before AI chunking, extract file names, or validate diff format.
- Fix: Either (1) remove the dependency if AI-based chunking is sufficient, or (2) integrate unidiff parsing to extract structured information (file paths, hunk boundaries) that could improve AI chunking accuracy or provide the `files` field in sections more reliably.

---

**Summary**: 2 unused dependencies found in Cargo.toml. Both `anyhow` and `unidiff` are declared but never imported or used. The `unidiff` case (WD-002) is more significant as it suggests an incomplete feature where structured diff parsing was planned but not implemented - the application currently relies entirely on AI to parse raw diff text.

**Dependencies verified as properly wired**:
- ratatui - used across all screen modules
- crossterm - used for terminal handling and events
- clap - used in CLI argument parsing
- serde/serde_json - used for session serialization
- thiserror - used for error type definitions
- dirs - used for home directory detection in session persistence
- throbber-widgets-tui - used in loading screen spinner
- chrono - used for export filename timestamps
- tempfile (dev-dependency) - used in summary screen tests


## Abstraction Reviewer

### AR-001: Duplicated diff syntax highlighting logic across screens
- Severity: P1
- Type: missing-abstraction
- File: /Users/ryanday/repos/sherpa2/src/screens/triage.rs:251-267, deep_review.rs:285-301, pseudocode.rs:275-290
- Affects: src/screens/triage.rs, src/screens/deep_review.rs, src/screens/pseudocode.rs
- Problem: The exact same diff syntax highlighting logic (checking for `+`, `-`, `@@`, `diff`, `index` line prefixes and applying colors) is copy-pasted in three different screen files.
- Why it matters: Changes to diff highlighting must be made in 3 places. Easy to introduce inconsistencies. The concept "diff line styling" deserves a name.
- Caveat: If screens need divergent highlighting in the future, keeping them separate may be intentional.
- Fix: Extract a `style_diff_line(line: &str) -> Line` or `diff_to_styled_lines(code: &str) -> Vec<Line>` function into a shared module (e.g., `src/ui/widgets.rs` or `src/screens/common.rs`).

### AR-002: Error/empty/completed state rendering logic duplicated across screens
- Severity: P2
- Type: missing-abstraction
- File: /Users/ryanday/repos/sherpa2/src/screens/triage.rs:290-314, deep_review.rs:330-375, loading.rs:61-79
- Affects: src/screens/triage.rs, src/screens/deep_review.rs, src/screens/loading.rs, src/screens/pseudocode.rs
- Problem: Each screen has similar `render_error`, `render_empty`, and `render_completed` methods that create a styled `Paragraph` with red border, centered text, and retry/quit hints.
- Why it matters: The pattern "show error with retry option" is repeated but unnamed. Adding a new error state requires copying the same boilerplate.
- Caveat: Screen-specific error messages may require some flexibility.
- Fix: Extract `render_error_panel(frame, area, title, error, hints)` helper widget or create an `ErrorPanel` struct that encapsulates the common pattern.

### AR-003: ChunkingError and AssessmentError are near-identical structs
- Severity: P2
- Type: over-abstracted
- File: /Users/ryanday/repos/sherpa2/src/ai/types.rs:31-49, 74-91
- Affects: src/ai/types.rs
- Problem: `ChunkingError` and `AssessmentError` have identical fields (`message`, `raw_output`) and identical methods (`new`, `with_output`). The only difference is the name.
- Why it matters: Two types that behave identically suggest over-abstraction. Either consolidate or differentiate.
- Caveat: If future versions need different error fields per operation, keeping them separate is justified.
- Fix: Either consolidate into a single `AiError` type, or add operation-specific fields that justify the separation.

### AR-004: `extract_json_array` and `extract_json_object` duplicate 90% of logic
- Severity: P1
- Type: missing-abstraction
- File: /Users/ryanday/repos/sherpa2/src/ai/claude.rs:251-334
- Affects: src/ai/claude.rs
- Problem: These two functions have nearly identical code for: (1) finding JSON in markdown code blocks, (2) finding raw JSON, (3) searching anywhere in text. Only the starting character (`[` vs `{`) and bracket type differ.
- Why it matters: Bug fixes or improvements must be applied twice. The shared concept "extract JSON from possibly-wrapped response" has no name.
- Fix: Extract `extract_json(text: &str, open_bracket: char, close_bracket: char) -> Option<String>` that both functions call, or create a generic `extract_json_delimited(text, start_char, end_char)`.

### AR-005: Scroll handling code duplicated across three screens
- Severity: P2
- Type: missing-abstraction
- File: /Users/ryanday/repos/sherpa2/src/screens/triage.rs:128-143, deep_review.rs:177-193
- Affects: src/screens/triage.rs, src/screens/deep_review.rs, src/screens/pseudocode.rs
- Problem: The same Ctrl+D, Ctrl+U, PageDown, PageUp scroll handling logic (adding/subtracting 10 or 20 to scroll_offset) appears in multiple screens.
- Why it matters: Inconsistent scroll amounts could be introduced. The scroll interaction pattern is unnamed.
- Caveat: Some screens may want different scroll behavior.
- Fix: Extract a `handle_scroll_keys(key, scroll_offset: &mut usize) -> bool` helper or add scroll handling to a base screen trait.

### AR-006: App holds all screen instances regardless of current state
- Severity: P2
- Type: over-abstracted
- File: /Users/ryanday/repos/sherpa2/src/app.rs:22-30
- Affects: src/app.rs
- Problem: The `App` struct holds all 5 screen instances simultaneously even though only one is active at a time. This eagerly allocates memory for all screens.
- Why it matters: Slight memory inefficiency and conceptual overhead. The relationship between App and screens is "owns all" rather than "has current".
- Caveat: Screens are lightweight (mostly stateless). Pre-allocation avoids creation cost on transitions. Acceptable trade-off.
- Fix: Could use an enum `CurrentScreen` that holds only the active screen, but current approach is pragmatic for a TUI. Consider documenting the design choice.

### AR-007: `run()` function in main.rs does too many things
- Severity: P1
- Type: solid-violation
- File: /Users/ryanday/repos/sherpa2/src/main.rs:30-99
- Affects: src/main.rs
- Problem: The `run()` function handles: CLI arg parsing, session loading/creation, user confirmation dialogs, AI chunking, error handling, screen transitions, app creation, and session saving. This violates Single Responsibility.
- Why it matters: Hard to test individual parts. Changes to session handling require modifying the same 70-line function as changes to AI integration.
- Caveat: For a small TUI, keeping orchestration in one place has value for readability.
- Fix: Extract at least: `load_or_create_session(args) -> Session` that handles the loading/creation/confirmation logic, and `run_with_session(session) -> Result<()>` for the app lifecycle.

### AR-008: Screen state should live in screen structs, not duplicated in UiState
- Severity: P2
- Type: missing-abstraction
- File: /Users/ryanday/repos/sherpa2/src/models/state.rs:40-66
- Affects: src/models/state.rs, src/screens/triage.rs, src/screens/deep_review.rs
- Problem: `UiState` holds `selected_index` and `scroll_offset`, but `TriageScreen` has its own `list_state` and `DeepReviewScreen` has `current_review_index`. The state is split between two places.
- Why it matters: Confusing to know where to look for state. Some screens use `state.ui.selected_index`, others use their internal state.
- Caveat: `ListState` is ratatui-specific and may need to live in the screen.
- Fix: Decide on one pattern: either all selection state in `UiState` (and screens are stateless renderers), or each screen owns its selection state (and `UiState` only has truly shared state).

### AR-009: `SectionResponse` struct defined inline inside method
- Severity: P2
- Type: missing-abstraction
- File: /Users/ryanday/repos/sherpa2/src/ai/claude.rs:54-62
- Affects: src/ai/claude.rs
- Problem: `SectionResponse` and `AssessmentResponse` are defined inside `parse_sections` and `parse_assessment` methods. These are proper domain types representing AI response format.
- Why it matters: Can not be reused or documented. The AI response schema is hidden inside implementation.
- Caveat: If only used in one place, inline is acceptable Rust style.
- Fix: Move to `types.rs` as `AiSectionResponse` and `AiAssessmentResponse` to make the AI contract explicit and enable schema validation tests.

### AR-010: `ScreenTrait` has non-optional methods that some screens ignore
- Severity: P2
- Type: solid-violation
- File: /Users/ryanday/repos/sherpa2/src/screens/mod.rs:22-28
- Affects: src/screens/mod.rs, all screen implementations
- Problem: `handle_input` returns `Result<bool>` but only the `bool` indicates if input was consumed - the `Result` is always `Ok`. Error handling happens via `state.ui.set_error()` instead.
- Why it matters: The return type promises error handling that doesn't exist. Callers must handle `Result` that never fails.
- Caveat: Keeping `Result` allows future error propagation if needed.
- Fix: Either use the `Result` for actual errors (not just setting `state.ui.error`), or change signature to `fn handle_input(&mut self, key: KeyEvent, state: &mut AppState) -> bool`.

### AR-011: Git operations use raw strings for error context
- Severity: P2
- Type: missing-abstraction
- File: /Users/ryanday/repos/sherpa2/src/git.rs:8-44
- Affects: src/git.rs
- Problem: Each git command has similar error handling: `map_err(|e| AppError::Git(format!("Failed to run git: {}", e)))`. The error message pattern is repeated but has no structured context (which git command failed, what arguments).
- Why it matters: Hard to debug which exact git invocation failed. Error messages are inconsistent.
- Caveat: For a CLI tool, string errors are often sufficient.
- Fix: Consider a helper `run_git(args: &[&str]) -> Result<String>` that handles command execution and error formatting consistently, or create a `GitCommandError` that captures the command and args.

### Summary

The codebase demonstrates generally good abstractions with clear module boundaries (ai, models, screens, session). The `AiClient` trait is a good example of a trustworthy abstraction - you know what `chunk_diff` does without drilling in.

Main patterns to address:
1. **Missing shared utilities**: Diff highlighting, error panels, scroll handling, JSON extraction are repeated patterns that deserve names
2. **State ownership clarity**: Selection state split between `UiState` and individual screens creates confusion
3. **`run()` orchestration bloat**: Main function does too much; extracting session lifecycle would improve testability

The architecture follows sensible Rust patterns. Most issues are P2 (could be cleaner) rather than P0/P1 (actively misleading).


## Test Auditor

### Executive Summary

The Sherpa2 codebase has **decent test coverage** for the core domain logic and UI interactions, but contains several **tests that provide false confidence** and **critical code paths that lack meaningful testing**. The test suite is well-organized with unit tests in modules and end-to-end tests in `tests/e2e.rs`, but the actual test assertions are often too weak to catch real bugs.

---

## Tests That Provide False Confidence

### TA-001: Tautological test - always passes regardless of outcome
- Severity: P1
- Type: false-confidence
- File: /Users/ryanday/repos/sherpa2/src/git.rs:137
- Affects: /Users/ryanday/repos/sherpa2/src/git.rs
- Problem: Test `test_detect_default_branch` asserts `result.is_ok() || result.is_err()` which is always true. This test can never fail and provides no value.
- Fix: Either mock the git commands to test specific scenarios, or remove the test and add meaningful integration tests that run in a controlled git environment.

### TA-002: Tautological test - always passes regardless of outcome
- Severity: P1
- Type: false-confidence
- File: /Users/ryanday/repos/sherpa2/src/git.rs:155
- Affects: /Users/ryanday/repos/sherpa2/src/git.rs
- Problem: Test `test_read_diff_commits_invalid_range` asserts `result.is_ok() || result.is_err()` which is always true. This test provides zero value.
- Fix: Remove this test or replace with a proper mock-based test that verifies error handling behavior.

### TA-003: No-op assertion test
- Severity: P1
- Type: false-confidence
- File: /Users/ryanday/repos/sherpa2/src/screens/loading.rs:137
- Affects: /Users/ryanday/repos/sherpa2/src/screens/loading.rs
- Problem: Test `test_loading_screen_new` only asserts `assert!(true)` which can never fail. The comment says "Basic construction test" but it verifies nothing about the constructed object.
- Fix: Add meaningful assertions about the initial state, e.g., verify throbber_state has expected default values.

### TA-004: No-op assertion test
- Severity: P1
- Type: false-confidence
- File: /Users/ryanday/repos/sherpa2/src/screens/loading.rs:145
- Affects: /Users/ryanday/repos/sherpa2/src/screens/loading.rs
- Problem: Test `test_loading_screen_tick` only asserts `assert!(true)`. This only verifies the method does not panic, not that it actually advances the animation state.
- Fix: Capture the throbber state before and after tick() and verify it changed, or verify the state index incremented.

### TA-005: Weak assertion - only checks non-null
- Severity: P2
- Type: false-confidence
- File: /Users/ryanday/repos/sherpa2/src/git.rs:144
- Affects: /Users/ryanday/repos/sherpa2/src/git.rs
- Problem: Test `test_get_current_branch` checks `if result.is_ok()` then asserts branch is not empty, but silently passes if the result is an error. This test provides no guarantee of correct behavior.
- Fix: Either mock git to ensure predictable results, or split into two tests: one that expects success in a git repo, one that expects failure outside a git repo.

---

## Critical Code Without Tests

### TA-006: AI response parsing edge cases not tested
- Severity: P0
- Type: missing-test
- File: /Users/ryanday/repos/sherpa2/src/ai/claude.rs:45
- Affects: /Users/ryanday/repos/sherpa2/src/ai/claude.rs
- Problem: The `parse_sections` method handles AI responses that may contain markdown, code blocks, or malformed JSON. While there are tests for happy paths, there are no tests for: malformed JSON with partial data, unicode/emoji in titles, extremely long responses, or responses with nested objects that look like sections but are not.
- Fix: Add tests for edge cases: truncated JSON, sections with special characters in code field, empty arrays inside code blocks, and responses that start with explanation text before JSON.

### TA-007: Session corruption recovery not fully tested
- Severity: P1
- Type: missing-test
- File: /Users/ryanday/repos/sherpa2/src/session/persistence.rs:51
- Affects: /Users/ryanday/repos/sherpa2/src/session/persistence.rs
- Problem: `load_session` handles corrupted files but only tests basic invalid JSON. Does not test: identifier mismatch (tested but weak), partial writes (truncated files), permission errors, or version migration scenarios.
- Fix: Add tests for truncated JSON files, files with valid JSON but wrong schema, and simulate partial write scenarios.

### TA-008: Git command failure scenarios not tested
- Severity: P1
- Type: missing-test
- File: /Users/ryanday/repos/sherpa2/src/git.rs:63
- Affects: /Users/ryanday/repos/sherpa2/src/git.rs
- Problem: `read_diff_commits` and `read_diff_branch` shell out to git but error handling is not tested. What happens when git is not installed? When the repo is corrupted? When stderr contains unexpected output?
- Fix: Create an abstraction for Command execution that can be mocked, then test various failure modes: command not found, non-zero exit with stderr, exit with mixed stdout/stderr.

### TA-009: Main run() function has no tests
- Severity: P1
- Type: missing-test
- File: /Users/ryanday/repos/sherpa2/src/main.rs:30
- Affects: /Users/ryanday/repos/sherpa2/src/main.rs
- Problem: The `run()` function contains critical logic for session loading/creation, AI chunking, and error handling. The comment says "Integration tests are in the tests/ directory" but there are no tests that exercise this flow without mocking.
- Fix: The e2e tests cover some flows but do not test the actual `run()` function. Add integration tests that exercise the session resume logic, corrupted session handling, and empty diff handling.

### TA-010: Terminal setup/restore error paths not tested
- Severity: P2
- Type: missing-test
- File: /Users/ryanday/repos/sherpa2/src/app.rs:204
- Affects: /Users/ryanday/repos/sherpa2/src/app.rs
- Problem: `setup_terminal` and `restore_terminal` have error handling code but no tests verify the error messages or that errors are properly propagated.
- Fix: These are difficult to test without mocking crossterm, but consider adding documentation tests or at minimum verify the error types are constructed correctly.

---

## Missing Scenarios

### TA-011: Boundary case - empty sections array
- Severity: P2
- Type: missing-scenario
- File: /Users/ryanday/repos/sherpa2/src/models/session.rs:79
- Affects: /Users/ryanday/repos/sherpa2/src/models/session.rs
- Problem: `sections_needing_review` and related methods handle empty sections, but no test verifies behavior when the session has zero sections. Edge cases like `hypothesis_counts()` with no assessments should be explicitly tested.
- Fix: Add tests for session with no sections, verifying counts return zeros and methods do not panic.

### TA-012: Boundary case - single section
- Severity: P2
- Type: missing-scenario
- File: /Users/ryanday/repos/sherpa2/src/screens/triage.rs:316
- Affects: /Users/ryanday/repos/sherpa2/src/screens/triage.rs
- Problem: Navigation tests use 2 sections. No test verifies wrap-around behavior with a single section (should stay on index 0).
- Fix: Add test with single-section state and verify next/previous navigation keeps selection at 0.

### TA-013: Concurrent AI call rejection tested but not behavior after
- Severity: P2
- Type: missing-scenario
- File: /Users/ryanday/repos/sherpa2/src/ai/client.rs:180
- Affects: /Users/ryanday/repos/sherpa2/src/ai/client.rs
- Problem: Tests verify concurrent calls are rejected, but do not verify that after a failed concurrent call, subsequent calls succeed once the first completes.
- Fix: Add test that simulates: call 1 starts, call 2 rejected, call 1 completes, call 3 succeeds.

### TA-014: JSON extraction with escaped characters
- Severity: P1
- Type: missing-scenario
- File: /Users/ryanday/repos/sherpa2/src/ai/claude.rs:251
- Affects: /Users/ryanday/repos/sherpa2/src/ai/claude.rs
- Problem: `extract_json_array` and `extract_json_object` handle escaped quotes in strings, but no test verifies handling of escaped backslashes (e.g., `\\n` vs `\n` in code strings), or JSON with nested quotes inside code fields.
- Fix: Add test with code containing `"path\\to\\file"` and verify extraction and parsing succeeds.

---

## Flaky Patterns

### TA-015: Test depends on system time
- Severity: P2
- Type: flaky
- File: /Users/ryanday/repos/sherpa2/src/screens/summary.rs:418
- Affects: /Users/ryanday/repos/sherpa2/src/screens/summary.rs
- Problem: Test `test_summary_export_markdown` creates a file with timestamp in the name and verifies it exists. While not actively flaky, if the test runs across a second boundary, the filename assertion could theoretically fail.
- Fix: Inject a clock abstraction or verify the file exists using a glob pattern rather than exact name match.

### TA-016: Tests may interfere via shared filesystem
- Severity: P2
- Type: flaky
- File: /Users/ryanday/repos/sherpa2/tests/e2e.rs:95
- Affects: /Users/ryanday/repos/sherpa2/tests/e2e.rs
- Problem: E2E tests use `std::process::id()` to create unique identifiers, which works for parallel test execution but leaves session files in `~/.sherpa/sessions/`. If tests fail before cleanup, subsequent runs could load stale sessions.
- Fix: Use a custom sessions directory for tests (via environment variable or config), or ensure cleanup runs even on test failure using a Drop guard.

---

## Well-Covered Areas

- **Session serialization/deserialization**: Good tests for Section and Session JSON round-tripping including special characters
- **CLI argument parsing**: Comprehensive tests for various argument combinations
- **Screen state transitions**: Tests verify navigation between screens and stage updates
- **Tag counts and statistics**: Well-tested with multiple scenarios
- **Input handling across screens**: Each screen has tests for key handling in various states
- **AI trait interface**: MockAiClient properly mirrors ClaudeClient behavior for testing

---

## Test Organization Observations

1. **Good**: Tests are co-located with code in `#[cfg(test)]` modules
2. **Good**: E2E tests are separated in `tests/e2e.rs`
3. **Improvement needed**: No test fixtures or shared test utilities - each test recreates state from scratch
4. **Improvement needed**: Git-dependent tests have no mocking strategy, leading to tautological assertions
5. **Missing**: No property-based testing for JSON parsing logic which handles complex string escaping


## Comment Analysis Review

**Summary**: Analyzed 20 Rust source files in the sherpa2 repository. The codebase is a TUI application for interactive code review. Overall comment quality is good with clear documentation comments on public APIs. Found several issues including aspirational TODOs without context, misleading phase references, and some missing documentation for complex logic.

---

### Critical Issues

### CA-001: Task comment references incomplete feature
- Severity: P1
- Type: aspirational-todo
- File: /Users/ryanday/repos/sherpa2/src/main.rs:1
- Affects: /Users/ryanday/repos/sherpa2/src/main.rs
- Problem: `@task(P1-T7) Implement session save on quit/Ctrl+C; detect corrupted JSON on load` - The comment describes "session save on quit/Ctrl+C" as incomplete, but the code at lines 94-97 already implements session save on quit. The "detect corrupted JSON on load" is also already implemented in `session/persistence.rs` (SessionLoadResult::Corrupted). This task marker is stale.
- Fix: Remove the task comment since the functionality is implemented, or update it to reflect any remaining work.

### CA-002: Aspirational TODO lacking context
- Severity: P2
- Type: aspirational-todo
- File: /Users/ryanday/repos/sherpa2/src/screens/summary.rs:2
- Affects: /Users/ryanday/repos/sherpa2/src/screens/summary.rs
- Problem: Comment says "Implement markdown export to docs/sherpa-review-<date>-<time>.md" but the `export_markdown` function at lines 264-348 already implements this feature. The comment is outdated.
- Fix: Remove the aspirational comment since the feature is implemented.

### CA-003: Misleading "PHASE" references in dead_code allows
- Severity: P1
- Type: misleading
- File: /Users/ryanday/repos/sherpa2/src/app.rs:56
- Affects: /Users/ryanday/repos/sherpa2/src/app.rs, /Users/ryanday/repos/sherpa2/src/error.rs, /Users/ryanday/repos/sherpa2/src/models/session.rs, /Users/ryanday/repos/sherpa2/src/models/state.rs, /Users/ryanday/repos/sherpa2/src/models/section.rs, /Users/ryanday/repos/sherpa2/src/ai/mod.rs
- Problem: Multiple `#[allow(dead_code)]` comments reference "PHASE-2", "PHASE-3", "PHASE-4" as if these are future phases. However, examining the code shows these features ARE already used. For example, `App::state()` is used in tests, `AppError::Ai` is used in the AI module, etc. These phase references are stale and misleading.
- Fix: Either remove the `#[allow(dead_code)]` annotations entirely if the code is used, or update comments to accurately describe current usage rather than referencing phases.

### CA-004: Module header describes types as "not fully used"
- Severity: P1
- Type: outdated
- File: /Users/ryanday/repos/sherpa2/src/ai/types.rs:2-4
- Affects: /Users/ryanday/repos/sherpa2/src/ai/types.rs
- Problem: Comment says "Note: These types are defined here for PHASE-2/3 parallel work but not fully used until those phases are implemented." However, `ChunkingResult` and `AssessmentResult` ARE fully used throughout the codebase (in `claude.rs`, `client.rs`, `app.rs`, `main.rs`).
- Fix: Remove or update the misleading note about types not being fully used.

### CA-005: Module header references future phases that are complete
- Severity: P1
- Type: outdated
- File: /Users/ryanday/repos/sherpa2/src/ai/claude.rs:1-5
- Affects: /Users/ryanday/repos/sherpa2/src/ai/claude.rs
- Problem: Comment says "It's implemented in PHASE-2 but not wired into the app until PHASE-4 (Integration). The MockAiClient is used in PHASE-3 for UI development, then replaced with ClaudeClient in PHASE-4." However, `ClaudeClient` IS wired into the app (used in `main.rs` and `app.rs`). The comment describes a development plan that has been executed.
- Fix: Update to simply describe what the module does without referencing historical phases: "This module provides the real AI client that shells out to the Claude CLI."

---

### Improvements

### CA-006: Missing explanation for hash-based session identifier
- Severity: P2
- Type: missing-comment
- File: /Users/ryanday/repos/sherpa2/src/session/persistence.rs:20-24
- Affects: /Users/ryanday/repos/sherpa2/src/session/persistence.rs
- Problem: The `hash_identifier` function converts session identifiers to hashes for filenames, but there's no comment explaining why hashing is used (likely to handle special characters in identifiers that wouldn't be valid in filenames).
- Fix: Add comment: `/// Hash the identifier to create a safe filename. This handles special characters like ':' in identifiers (e.g., "commits:5") that aren't allowed in filenames on some systems.`

### CA-007: Complex JSON extraction logic without rationale
- Severity: P2
- Type: missing-comment
- File: /Users/ryanday/repos/sherpa2/src/ai/claude.rs:251-292
- Affects: /Users/ryanday/repos/sherpa2/src/ai/claude.rs
- Problem: The `extract_json_array` function has complex logic to find JSON in various formats (code blocks, raw text), but no comment explains WHY Claude's responses might contain JSON in different formats (markdown code blocks, plain text, etc.).
- Fix: Add comment at the function start: `/// Claude responses may include JSON in markdown code blocks (\`\`\`json), plain code blocks (\`\`\`), or as raw text. This function handles all these formats to robustly extract the JSON array.`

### CA-008: Accuracy percentage thresholds lack explanation
- Severity: P2
- Type: missing-comment
- File: /Users/ryanday/repos/sherpa2/src/screens/summary.rs:168-176
- Affects: /Users/ryanday/repos/sherpa2/src/screens/summary.rs
- Problem: The accuracy display uses magic numbers (80%, 50%) for color thresholds without explaining why these values were chosen.
- Fix: Add comment: `// Color thresholds: green (>=80% excellent), yellow (>=50% needs work), red (<50% significant gaps)`

---

### Removals

### CA-009: Obvious test comment adds no value
- Severity: P2
- Type: unnecessary-comment
- File: /Users/ryanday/repos/sherpa2/src/main.rs:129-131
- Affects: /Users/ryanday/repos/sherpa2/src/main.rs
- Problem: The comment `// Integration tests are in the tests/ directory` in an empty test module provides minimal value - developers would expect to find tests in the tests/ directory by Rust convention.
- Fix: Remove the comment and the empty test module, or add actual unit tests.

### CA-010: Redundant task comments throughout codebase
- Severity: P2
- Type: unnecessary-comment
- File: /Users/ryanday/repos/sherpa2/src/cli.rs:32-36
- Affects: /Users/ryanday/repos/sherpa2/src/cli.rs, /Users/ryanday/repos/sherpa2/src/git.rs, /Users/ryanday/repos/sherpa2/src/models/mod.rs, /Users/ryanday/repos/sherpa2/src/models/section.rs, /Users/ryanday/repos/sherpa2/src/models/state.rs, /Users/ryanday/repos/sherpa2/src/models/session.rs, /Users/ryanday/repos/sherpa2/src/session/mod.rs, /Users/ryanday/repos/sherpa2/src/session/persistence.rs, /Users/ryanday/repos/sherpa2/src/screens/mod.rs, /Users/ryanday/repos/sherpa2/src/ai/mod.rs, /Users/ryanday/repos/sherpa2/src/ai/client.rs, /Users/ryanday/repos/sherpa2/src/ai/types.rs, /Users/ryanday/repos/sherpa2/src/app.rs
- Problem: Nearly every file has `@task(P1-TX)` comments at the top that describe what to implement. Since the implementation is complete, these task markers are historical artifacts rather than actionable items.
- Fix: Remove all `@task` comments or consolidate them into a project tracking system. If kept for historical reference, move them to a CHANGELOG or development notes file.

### CA-011: Redundant screen state comments
- Severity: P2
- Type: unnecessary-comment
- File: /Users/ryanday/repos/sherpa2/src/screens/triage.rs:1-5
- Affects: /Users/ryanday/repos/sherpa2/src/screens/triage.rs, /Users/ryanday/repos/sherpa2/src/screens/deep_review.rs, /Users/ryanday/repos/sherpa2/src/screens/pseudocode.rs
- Problem: Multi-line implementation comments at file headers (e.g., "TriageScreen section list panel: scrollable, badges for tags / TriageScreen code preview panel...") read like task descriptions rather than documentation. They describe WHAT is implemented rather than WHY or HOW.
- Fix: Convert to proper module documentation using `//!` that explains the screen's purpose in the application workflow, or remove if the code is self-documenting.

---

### Notes

**Well-documented areas:**
- The `AiClient` trait in `/Users/ryanday/repos/sherpa2/src/ai/client.rs` has excellent doc comments explaining each method's purpose, parameters, and return values.
- Public structs like `Session`, `Section`, and `Assessment` have good field-level documentation.
- The terminal setup/restore functions in `/Users/ryanday/repos/sherpa2/src/app.rs` have clear purpose comments.

**Test coverage:**
- Tests lack documentation comments explaining what scenarios they cover. Consider adding `///` docs to test functions describing the behavior being verified.


## Silent Failure Hunter

### SFH-001: Silent deletion of session file on --new flag
- Severity: P2
- Type: silent-failure
- File: /Users/ryanday/repos/sherpa2/src/main.rs:37
- Affects: /Users/ryanday/repos/sherpa2/src/main.rs
- Problem: `delete_session(&identifier).ok()` silently swallows any errors from session deletion
- Hidden Errors: Permission denied, filesystem errors, path resolution failures
- User Impact: User may not realize their old session wasn't deleted, leading to confusion if the new session fails to save later
- Caveat: Intentional - comments say "Ignore errors if no existing session". However, permission errors differ from "file not found" and should be surfaced
- Fix:
```rust
// Distinguish "not found" from actual errors
if let Err(e) = delete_session(&identifier) {
    // Only warn on real errors, not "file not found"
    if !matches!(e, AppError::Session(ref msg) if msg.contains("not found")) {
        eprintln!("Warning: Could not delete old session: {}", e);
    }
}
```

### SFH-002: No timeout on Claude CLI subprocess
- Severity: P1
- Type: missing-resilience
- File: /Users/ryanday/repos/sherpa2/src/ai/claude.rs:27-33
- Affects: /Users/ryanday/repos/sherpa2/src/ai/claude.rs
- Problem: `Command::new("claude")...output()` has no timeout. The Claude CLI could hang indefinitely, blocking the entire application
- Hidden Errors: Network issues, Claude CLI hanging, rate limiting causing long waits
- User Impact: Application appears frozen with no way to recover except killing the process
- Caveat: None - external calls should always have timeouts
- Fix:
```rust
use std::time::Duration;
use wait_timeout::ChildExt;

fn run_claude(&self, prompt: &str) -> Result<String, String> {
    let mut child = Command::new("claude")
        .args(["--dangerously-skip-permissions", "-p", prompt])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn claude process: {}", e))?;
    
    let timeout = Duration::from_secs(120); // 2 minute timeout
    let status = child.wait_timeout(timeout)
        .map_err(|e| format!("Failed to wait on claude process: {}", e))?;
    
    match status {
        Some(exit_status) => {
            // Process completed within timeout
            let output = child.wait_with_output()
                .map_err(|e| format!("Failed to get output: {}", e))?;
            if !exit_status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Claude CLI failed: {}", stderr.trim()));
            }
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }
        None => {
            // Timeout - kill the process
            child.kill().ok();
            Err("Claude CLI timed out after 120 seconds".to_string())
        }
    }
}
```

### SFH-003: No retry logic for transient Claude CLI failures
- Severity: P2
- Type: missing-resilience
- File: /Users/ryanday/repos/sherpa2/src/ai/claude.rs:154-163
- Affects: /Users/ryanday/repos/sherpa2/src/ai/claude.rs
- Problem: AI operations fail immediately on first error. Network glitches, rate limits, and transient failures are not retried automatically
- Hidden Errors: Temporary network failures, rate limit responses, intermittent Claude service issues
- User Impact: User must manually retry (press 'r'), even for failures that would succeed on second attempt
- Caveat: UI provides manual retry which is acceptable for interactive use. Auto-retry would be better UX
- Fix:
```rust
fn chunk_diff_impl(&self, diff: &str) -> ChunkingResult {
    let prompt = build_chunking_prompt(diff);
    
    let max_retries = 2;
    let mut last_error = String::new();
    
    for attempt in 0..=max_retries {
        if attempt > 0 {
            // Exponential backoff: 1s, 2s
            std::thread::sleep(Duration::from_secs(1 << (attempt - 1)));
        }
        
        match self.run_claude(&prompt) {
            Ok(response) => match self.parse_sections(&response) {
                Ok(sections) => return ChunkingResult::Success(sections),
                Err(e) => {
                    // Parse errors shouldn't be retried
                    return ChunkingResult::Error(ChunkingError::new(e).with_output(response));
                }
            },
            Err(e) => {
                last_error = e;
                // Continue to retry
            }
        }
    }
    
    ChunkingResult::Error(ChunkingError::new(
        format!("Failed after {} retries: {}", max_retries + 1, last_error)
    ))
}
```

### SFH-004: Unwrap on assessment check could be cleaner
- Severity: P2
- Type: inadequate-handling
- File: /Users/ryanday/repos/sherpa2/src/screens/pseudocode.rs:52
- Affects: /Users/ryanday/repos/sherpa2/src/screens/pseudocode.rs
- Problem: Double unwrap pattern `section.is_some() && section.unwrap().assessment.is_some()` - while safe due to the is_some() check, this is fragile pattern that could become a bug during refactoring
- Hidden Errors: None currently, but risky pattern
- User Impact: None - but code maintenance risk
- Caveat: The pattern is technically safe due to short-circuit evaluation
- Fix:
```rust
// Use pattern matching instead
if let Some(section) = section {
    if section.assessment.is_some() {
        self.render_response(frame, area, state);
        return;
    }
}
```

### SFH-005: Process exit bypasses cleanup on empty diff
- Severity: P1
- Type: silent-failure
- File: /Users/ryanday/repos/sherpa2/src/main.rs:110-111
- Affects: /Users/ryanday/repos/sherpa2/src/main.rs
- Problem: `std::process::exit(0)` immediately exits without returning through main(). This bypasses any cleanup in run() or main() and doesn't call destructors
- Hidden Errors: Session state not saved, terminal not restored (if already in raw mode)
- User Impact: Abrupt exit could leave terminal in bad state if this code path changes in future
- Caveat: Currently happens before terminal setup, so safe today. But fragile if code is refactored
- Fix:
```rust
fn create_new_session(args: &Args) -> Result<Session> {
    let identifier = args.session_identifier();
    let diff_text = git::read_diff(args.commits)?;

    if diff_text.trim().is_empty() {
        // Return an error instead of exiting
        return Err(AppError::Git("No changes to review".to_string()));
    }

    Ok(Session::new(identifier, diff_text))
}

// In run(), handle this gracefully:
let session = match create_new_session(&args) {
    Ok(s) => s,
    Err(AppError::Git(msg)) if msg.contains("No changes") => {
        eprintln!("No changes to review.");
        return Ok(()); // Clean exit through normal flow
    }
    Err(e) => return Err(e),
};
```

### SFH-006: Restore terminal failure after main loop error
- Severity: P0
- Type: cleanup
- File: /Users/ryanday/repos/sherpa2/src/app.rs:70-75
- Affects: /Users/ryanday/repos/sherpa2/src/app.rs
- Problem: If `main_loop` returns an error, `restore_terminal` is called but its result is propagated. If restore_terminal ALSO fails, the original error is lost. More critically, if restore_terminal fails, the terminal could be left in raw mode
- Hidden Errors: Original main_loop error can be masked by restore_terminal error
- User Impact: User's terminal may be left in raw mode (no echo, no line buffering) requiring `reset` command
- Caveat: The current pattern does call restore, but doesn't handle double-failure well
- Fix:
```rust
pub fn run(&mut self) -> Result<()> {
    let mut terminal = setup_terminal()?;
    
    let result = self.main_loop(&mut terminal);
    
    // Always attempt restore, but don't let restore failure mask main_loop error
    if let Err(restore_err) = restore_terminal(&mut terminal) {
        eprintln!("Warning: Failed to restore terminal: {}", restore_err);
        // If main_loop succeeded but restore failed, return the restore error
        // If main_loop failed, return that original error
        if result.is_ok() {
            return Err(restore_err);
        }
    }
    
    result
}
```

### SFH-007: Session save failure only shows warning
- Severity: P1
- Type: inadequate-handling
- File: /Users/ryanday/repos/sherpa2/src/main.rs:95-97
- Affects: /Users/ryanday/repos/sherpa2/src/main.rs
- Problem: When session save fails, only a warning is printed to stderr. If the terminal was in alternate screen mode, the user won't see this message as it gets cleared
- Hidden Errors: Disk full, permission denied, serialization failures
- User Impact: User loses their session progress without clear notification - they think everything saved but it didn't
- Caveat: Don't want to fail the app after successful review, but user MUST know about data loss
- Fix:
```rust
// After restoring terminal in app.run(), the save happens:
let result = app.run();

// Save session on quit
if let Err(e) = save_session(app.session()) {
    // Make this very visible - pause before exiting
    eprintln!("\n==============================================");
    eprintln!("WARNING: Failed to save session: {}", e);
    eprintln!("Your progress has NOT been saved!");
    eprintln!("Session data: {:?}", app.session().identifier);
    eprintln!("==============================================");
    eprintln!("Press Enter to acknowledge...");
    let _ = std::io::stdin().read_line(&mut String::new());
}

result
```

### SFH-008: Git command errors lack context
- Severity: P2
- Type: inadequate-handling
- File: /Users/ryanday/repos/sherpa2/src/git.rs:53-56
- Affects: /Users/ryanday/repos/sherpa2/src/git.rs
- Problem: `get_current_branch()` returns generic "Failed to get current branch name" without the actual git error output
- Hidden Errors: The actual reason git failed (detached HEAD, not a git repo, corrupted repo) is lost
- User Impact: User sees unhelpful error message, can't diagnose the real problem
- Caveat: None
- Fix:
```rust
pub fn get_current_branch() -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .map_err(|e| AppError::Git(format!("Failed to run git: {}", e)))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Git(format!(
            "Failed to get current branch: {}",
            stderr.trim()
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
```

### SFH-009: Truncate function can panic on non-ASCII
- Severity: P2
- Type: silent-failure
- File: /Users/ryanday/repos/sherpa2/src/ai/claude.rs:380
- Affects: /Users/ryanday/repos/sherpa2/src/ai/claude.rs
- Problem: `&s[..max_len]` can panic if max_len falls in the middle of a multi-byte UTF-8 character. The AI response could contain any Unicode
- Hidden Errors: Panic/crash on certain AI responses containing emoji or non-ASCII
- User Impact: Application crashes when truncating certain error messages
- Caveat: Unlikely if AI responds in ASCII, but not guaranteed
- Fix:
```rust
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        // Find a valid UTF-8 boundary at or before max_len
        let mut boundary = max_len;
        while boundary > 0 && !s.is_char_boundary(boundary) {
            boundary -= 1;
        }
        format!("{}...", &s[..boundary])
    }
}
```

### SFH-010: No graceful handling of Ctrl+C during AI call
- Severity: P1
- Type: missing-resilience
- File: /Users/ryanday/repos/sherpa2/src/app.rs:78-112
- Affects: /Users/ryanday/repos/sherpa2/src/app.rs, /Users/ryanday/repos/sherpa2/src/main.rs
- Problem: Ctrl+C is caught by the event loop, but if pressed during synchronous AI calls in `retry_chunking()` or `retry_assessment()`, it won't be processed until the call completes. There's no signal handler to interrupt long operations
- Hidden Errors: User's Ctrl+C ignored during blocking operations
- User Impact: Application appears unresponsive to quit commands during AI operations
- Caveat: Implementing proper signal handling in Rust is complex. The current synchronous design limits options
- Fix: Would require restructuring to use async/await or threads with proper cancellation. Minimum viable fix:
```rust
// At minimum, add a ctrlc handler that sets an atomic flag
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

static INTERRUPTED: AtomicBool = AtomicBool::new(false);

// In main:
ctrlc::set_handler(|| {
    INTERRUPTED.store(true, Ordering::SeqCst);
}).expect("Error setting Ctrl-C handler");

// In run_claude, check periodically or use non-blocking I/O
```

### Summary

Found **10 issues** in the Sherpa2 codebase:

| ID | Severity | Type | Location |
|----|----------|------|----------|
| SFH-001 | P2 | silent-failure | main.rs:37 |
| SFH-002 | P1 | missing-resilience | claude.rs:27-33 |
| SFH-003 | P2 | missing-resilience | claude.rs:154-163 |
| SFH-004 | P2 | inadequate-handling | pseudocode.rs:52 |
| SFH-005 | P1 | silent-failure | main.rs:110-111 |
| SFH-006 | P0 | cleanup | app.rs:70-75 |
| SFH-007 | P1 | inadequate-handling | main.rs:95-97 |
| SFH-008 | P2 | inadequate-handling | git.rs:53-56 |
| SFH-009 | P2 | silent-failure | claude.rs:380 |
| SFH-010 | P1 | missing-resilience | app.rs:78-112 |

**Critical (P0)**: 1 issue - terminal restore failure handling
**High (P1)**: 4 issues - timeouts, cleanup, data loss visibility
**Medium (P2)**: 5 issues - error context, retry logic, code quality


## Code Simplifier

### Simplification Opportunities

### CS-001: Duplicate JSON extraction logic
- Severity: P1
- Type: code-smell
- File: /Users/ryanday/repos/sherpa2/src/ai/claude.rs:251-334
- Affects: src/ai/claude.rs
- Problem: `extract_json_array` and `extract_json_object` are nearly identical functions (90% shared logic) differing only in the bracket characters they search for (`[`/`]` vs `{`/`}`). Both check for code blocks the same way, search for raw JSON the same way, and use the same bracket-matching helper.
- Caveat: The functions are well-tested and work correctly. May not be worth changing if no further JSON extraction variants are needed.
- Fix: Create a generic `extract_json(text: &str, open: char, close: char) -> Option<String>` function that both can delegate to, or use a single function with an enum parameter for the type.

### CS-002: Diff syntax highlighting duplicated across screens
- Severity: P1
- Type: code-smell
- File: /Users/ryanday/repos/sherpa2/src/screens/triage.rs:229-274
- Affects: src/screens/triage.rs, src/screens/deep_review.rs, src/screens/pseudocode.rs
- Problem: The diff syntax highlighting logic (lines starting with `+`, `-`, `@@`, `diff`, `index`) is copy-pasted in three screen modules: `triage.rs` (lines 251-266), `deep_review.rs` (lines 285-301), and `pseudocode.rs` (lines 275-290). Each has identical pattern matching for colorizing diff output.
- Caveat: The logic is simple and unlikely to change frequently.
- Fix: Extract a `highlight_diff_lines(code: &str) -> Vec<Line>` helper function into a shared module (perhaps `screens/mod.rs` or a new `screens/widgets.rs`). All three screens would call this helper.

### CS-003: Redundant is_some + unwrap pattern
- Severity: P2
- Type: complexity
- File: /Users/ryanday/repos/sherpa2/src/models/session.rs:64-77
- Affects: src/models/session.rs
- Problem: The `hypothesis_counts` method checks `if section.assessment.is_some()` then immediately calls `.unwrap()`. This is a common anti-pattern that can be simplified.
- Caveat: The current code is safe and works correctly.
- Fix: Use `if let Some(assessment) = &section.assessment { ... }` for cleaner idiomatic Rust.

### CS-004: Nested is_some + unwrap chains
- Severity: P2
- Type: complexity
- File: /Users/ryanday/repos/sherpa2/src/screens/pseudocode.rs:51-55, 104-105
- Affects: src/screens/pseudocode.rs
- Problem: Two locations use the pattern `if section.is_some() && section.unwrap().assessment.is_some()` which is verbose and requires multiple unwrap calls.
- Caveat: Safe code that works correctly.
- Fix: Use `if let Some(section) = state.current_section() { if let Some(assessment) = &section.assessment { ... } }` or Rust 1.65+ let-chains: `if let Some(section) = state.current_section() && section.assessment.is_some()`.

### CS-005: Repetitive bar width calculation
- Severity: P2
- Type: complexity
- File: /Users/ryanday/repos/sherpa2/src/screens/summary.rs:89-114
- Affects: src/screens/summary.rs
- Problem: The bar width calculation `if total > 0 { (count * bar_width) / total } else { 0 }` is repeated three times with only the count variable changing.
- Caveat: The code is readable and only appears once in the codebase.
- Fix: Extract a helper closure: `let calc_width = |count| if total > 0 { (count * bar_width) / total } else { 0 };` then call `calc_width(counts.got_it)`, etc.

### CS-006: Triple iteration over sections
- Severity: P2
- Type: complexity
- File: /Users/ryanday/repos/sherpa2/src/screens/deep_review.rs:26-57
- Affects: src/screens/deep_review.rs
- Problem: Three methods (`get_sections_needing_review`, `total_needing_review`, `reviewed_count`) each iterate over all sections independently. The `render` and `handle_input` methods call these multiple times per frame, causing redundant iterations.
- Caveat: With small section counts (typically under 20), performance impact is negligible.
- Fix: Create a `ReviewStats` struct computed once that contains all needed counts. Cache it per render frame or compute it once at the start of render/handle_input.

### CS-007: Repeated git command error handling
- Severity: P2
- Type: code-smell
- File: /Users/ryanday/repos/sherpa2/src/git.rs:7-44
- Affects: src/git.rs
- Problem: The pattern `.map_err(|e| AppError::Git(format!("Failed to run git: {}", e)))?` appears 5 times in `detect_default_branch` and similar patterns throughout the file.
- Caveat: The repetition is clear and explicit about what failed.
- Fix: Create a helper: `fn run_git(args: &[&str]) -> Result<Output>` that handles the common Command setup and error mapping.

### Additional Observations

**Positive patterns observed:**
- Clear module organization with single responsibility
- Good use of Rust enums for state machines (Screen, Tag, ReviewStage)
- Comprehensive test coverage throughout
- Atomic file operations for session persistence (temp file + rename)
- Clean separation between persistent Session data and ephemeral UiState

**Minor improvements (not worth individual issues):**
- `AppState::current_section()` and `current_section_mut()` could be consolidated using a generic with `AsRef`/`AsMut`, but the current approach is idiomatic Rust
- Some test assertions like `assert!(result.is_ok() || result.is_err())` are tautological (always true) - these should be removed or made meaningful
- Several `#[allow(dead_code)]` annotations suggest incremental development - these should be cleaned up when features are complete

**Technical debt indicators:**
- Multiple `@task` comments with phase references (P1-T2, P1-T3, etc.) indicate phased development. These should be cleaned up when phases complete.
- `#[allow(unused_imports)]` in ai/mod.rs suggests incomplete integration
