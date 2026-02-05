# Plan Review: Code Review TUI
**Date**: 2026-02-04
**Plan**: docs/plans/code-review-tui.yaml
**Spec**: docs/specifications/ux-code-review-tui.yaml

---

## Reuse Review

**Status**: No reuse issues found - this is a greenfield project.

### Codebase Analysis

The following searches were performed to verify this is a greenfield project:

| Search Type | Pattern | Result |
|-------------|---------|--------|
| Rust source files | `**/*.rs` | No files found |
| Cargo manifest | `**/Cargo.toml` | No files found |
| Go source files | `**/*.go` | No files found |
| TypeScript files | `**/*.ts` | No files found |
| JavaScript files | `**/*.js` | No files found |
| Python files | `**/*.py` | No files found |
| Package manifests | `**/package.json` | No files found |

### Project Contents

The repository contains only:
- `/Users/ryanday/repos/sherpa2/docs/specifications/ux-code-review-tui.yaml` - UX specification
- `/Users/ryanday/repos/sherpa2/docs/plans/code-review-tui.yaml` - Implementation plan
- `/Users/ryanday/repos/sherpa2/docs/plans/code-review-tui-review.md` - This review document
- `.git/` - Git repository metadata

### Conclusion

**No reuse opportunities exist** because there is no existing source code in this repository. The plan appropriately describes building all components from scratch for a new Rust TUI application using:
- Ratatui (terminal UI framework)
- Crossterm (terminal manipulation)
- Clap (CLI argument parsing)
- Serde/serde_json (serialization)

All 4 phases and 34 tasks are justified as new development work.


## Conventions Review

### Project Context

This is a **greenfield Rust project** with no existing source code. The repository currently contains only planning and specification documents under `docs/`. There is no `CLAUDE.md`, `Cargo.toml`, or existing Rust code to establish project-specific conventions.

The review evaluates the implementation plan against **standard Rust project conventions** as established by the Rust ecosystem (Cargo, rustfmt, clippy, and community best practices).

### Conventions Checked

1. **Project structure**: Standard Cargo layout (`src/main.rs`, `src/lib.rs`, module organization)
2. **File naming**: Snake_case for modules, PascalCase for types
3. **Test organization**: `tests/` directory for integration tests, `#[cfg(test)]` modules for unit tests
4. **Dependency management**: Standard Cargo.toml structure
5. **Error handling patterns**: Result/Option idioms
6. **Module organization**: Flat vs nested modules, re-exports

### Issues Found

**No convention violations detected.**

The implementation plan is appropriately abstract for a greenfield project. It specifies:

- **P1-T1**: Initialize Rust project with Cargo.toml - This correctly establishes the standard Rust project structure
- **Dependencies**: ratatui, crossterm, clap, serde, serde_json - All are standard, well-maintained crates appropriate for this use case
- **Session storage**: `~/.sherpa/sessions/<hash>.json` - Follows XDG-like conventions for user data

### Recommendations (P2 - Minor)

While the plan has no violations, these standard Rust conventions should be followed during implementation:

#### PCV-001: Module organization for screens
- Severity: P2
- Plan Task: P3-T1 through P3-T14 (Screen implementations)
- Convention: Rust projects typically organize related code in modules. For a TUI with multiple screens, the standard pattern is:
  ```
  src/
    main.rs           # Entry point, CLI parsing
    lib.rs            # Core library exports
    screens/          # Screen modules
      mod.rs
      loading.rs
      triage.rs
      deep_review.rs
      pseudocode_review.rs
      summary.rs
    models/           # Data structures
      mod.rs
      section.rs
      session.rs
    ai/               # AI client
      mod.rs
      client.rs
      prompts.rs
  ```
- Problem: The plan does not specify file organization, leaving it implicit
- Fix: Consider adding a task or note specifying the module structure to ensure consistency

#### PCV-002: Test organization
- Severity: P2
- Plan Task: P1-T8, P2-T6, P3-T15, P4-T10 (Test tasks)
- Convention: Rust projects use:
  - `#[cfg(test)] mod tests { ... }` for unit tests (same file as implementation)
  - `tests/` directory for integration tests
  - E2E tests often in `tests/e2e/` or similar
- Problem: The plan mentions "tests" without specifying whether they are unit tests (in-file) or integration tests (`tests/` directory)
- Fix: Clarify test organization in task descriptions. For example, P4-T10 "Write E2E tests" should specify `tests/e2e/` location

#### PCV-003: Error handling consistency
- Severity: P2
- Plan Task: P2-T4 (Handle AI errors gracefully)
- Convention: Rust projects typically define a custom error type using `thiserror` or similar, with consistent error propagation via `?` operator
- Problem: The plan does not specify error handling strategy
- Fix: Add a task or note for defining `src/error.rs` with project-wide error types early in Phase 1

### Summary

The implementation plan is **consistent with standard Rust conventions** for a greenfield project. The plan appropriately defers implementation details while establishing correct dependencies and architectural boundaries. The P2 recommendations above are suggestions for implementation-time decisions, not plan deficiencies.

**Verdict: Plan approved - no blocking issues (P0/P1) found.**

## Spec Coverage Review

**Plan**: `/Users/ryanday/repos/sherpa2/docs/plans/code-review-tui.yaml`
**Spec**: `/Users/ryanday/repos/sherpa2/docs/specifications/ux-code-review-tui.yaml`
**Reviewed**: 2026-02-04

### Summary

The plan provides reasonable coverage of the specification. All 5 screens are explicitly listed in PHASE-3 implements. Most user stories and flows are addressed through task descriptions. However, several gaps exist in explicit coverage of acceptance criteria, screen components, and edge cases.

---

### PSC-001: Missing explicit coverage for user_stories[0] acceptance criteria - specific file path review
- Severity: P1 (important)
- Spec Item: user_stories[0].acceptance_criteria[2] - "Running `sherpa <file-path>` reviews a specific file's changes"
- Expected: Task in PHASE-1 covering file-path input handling
- Problem: P1-T3 mentions "file-path inputs" but no verification or acceptance criteria explicitly tests single-file review mode
- Fix: Add explicit test case in P1-T8 for file-path input mode; add acceptance verification in PHASE-1

---

### PSC-002: Missing explicit coverage for user_stories[0] acceptance criteria - invalid input error handling
- Severity: P1 (important)
- Spec Item: user_stories[0].acceptance_criteria[4] - "Invalid input shows helpful error message"
- Expected: Task in PHASE-1 for CLI error handling
- Problem: No task explicitly covers invalid input error messaging
- Fix: Add task or expand P1-T2 to explicitly handle and test invalid input error messages

---

### PSC-003: Missing explicit coverage for user_stories[1] acceptance criteria - section ordering
- Severity: P1 (important)
- Spec Item: user_stories[1].acceptance_criteria[2] - "Sections are ordered by importance (core types first, utilities last)"
- Expected: Task in PHASE-2 covering section ordering in AI chunking prompt
- Problem: P2-T2 mentions "receive sections with titles/descriptions" but not ordering requirement
- Fix: Expand P2-T2 description to include "sections ordered by importance (core types first)"

---

### PSC-004: Missing explicit coverage for user_stories[2] acceptance criteria - proceed when done tagging
- Severity: P2 (minor)
- Spec Item: user_stories[2].acceptance_criteria[3] - "User can proceed to deep review when done tagging"
- Expected: Task in PHASE-3 TriageScreen tasks
- Problem: Implicitly covered by P3-T4 "Enter proceed" but not explicitly verified
- Fix: Ensure P3-T15 tests include verification of Enter proceeding to deep review

---

### PSC-005: Missing explicit coverage for TriageScreen component - progress indicator
- Severity: P1 (important)
- Spec Item: screens[1].components[3] - Progress indicator showing tagged/total sections ("3/12 sections tagged")
- Expected: Task in PHASE-3 for progress indicator component
- Problem: No task explicitly implements the progress indicator component
- Fix: Add task P3-Tx: "Implement TriageScreen progress indicator: show tagged/total count (screens[1].components[3])"

---

### PSC-006: Missing explicit coverage for TriageScreen component - section title and description header
- Severity: P2 (minor)
- Spec Item: screens[1].components[2] - Section title and description header
- Expected: Task in PHASE-3 for header component
- Problem: Not explicitly listed as a task (may be implicitly included in P3-T2/P3-T3)
- Fix: Clarify in P3-T2 or P3-T3 that section title and description header is included

---

### PSC-007: Missing explicit coverage for DeepReviewScreen navigation keybindings mismatch
- Severity: P2 (minor)
- Spec Item: screens[2].components[3] - Navigation hints "n: next unreviewed | p: previous | Enter: submit | q: quit"
- Expected: Task P3-T8 should match spec keybindings exactly
- Problem: P3-T8 says "n/p navigate, Enter submit, q quit" but spec says "n: next unreviewed | p: previous" (unreviewed is significant)
- Fix: Update P3-T8 to clarify "n: next unreviewed section, p: previous section"

---

### PSC-008: Missing explicit coverage for PseudocodeReviewScreen submitted state
- Severity: P1 (important)
- Spec Item: screens[3].screen_states.submitted - "Code visible, user text locked, AI response area appears below"
- Expected: Task in PHASE-3 covering the submitted state transition
- Problem: P3-T9 and P3-T10 don't explicitly mention the "submitted" state where user text is locked
- Fix: Add explicit handling in P3-T9 or P3-T10 for the submitted state transition

---

### PSC-009: Missing explicit coverage for PseudocodeReviewScreen error state
- Severity: P1 (important)
- Spec Item: screens[3].screen_states.error - "Error message shown, user text preserved, retry button visible (r to retry)"
- Expected: Task covering error state with retry
- Problem: P3-T11 covers "hypothesis preservation on error" but doesn't mention the retry button (r key)
- Fix: Expand P3-T11 to include "r key to retry" functionality per spec

---

### PSC-010: Missing explicit coverage for SummaryScreen exported state
- Severity: P2 (minor)
- Spec Item: screens[4].screen_states.exported - "Confirmation message with file path shown"
- Expected: Task in PHASE-3 covering export confirmation state
- Problem: P3-T13 mentions markdown export but not the confirmation message state
- Fix: Expand P3-T13 to include showing confirmation message with file path after export

---

### PSC-011: Missing explicit coverage for SummaryScreen session info component
- Severity: P2 (minor)
- Spec Item: screens[4].components[4] - Session info showing input and duration
- Expected: Task in PHASE-3 for session info component
- Problem: P3-T12 lists confidence breakdown, accuracy breakdown, misconceptions but not session info
- Fix: Expand P3-T12 to include "session info panel showing input and duration"

---

### PSC-012: Missing explicit coverage for flow edge case - all sections tagged "got it"
- Severity: P1 (important)
- Spec Item: flows[0].steps[1].edge_cases[0] - "All sections tagged 'got it' -> show summary directly"
- Expected: Task in PHASE-3 or PHASE-4 covering this flow shortcut
- Problem: No task explicitly handles skipping DeepReviewScreen when all sections are "got it"
- Fix: Add task in PHASE-4: "Implement flow shortcut: if all sections tagged 'got it', skip DeepReview to Summary"

---

### PSC-013: Missing explicit coverage for flow edge case - confirm abandonment on back
- Severity: P2 (minor)
- Spec Item: flows[1].steps[0].edge_cases[1] - "User presses back before submitting -> confirm abandonment or save draft"
- Expected: Task in PHASE-3 covering back navigation confirmation
- Problem: No task mentions confirmation dialog or draft saving on back before submit
- Fix: Add handling in P3-T14 or new task for back navigation confirmation in PseudocodeReviewScreen

---

### PSC-014: Missing explicit coverage for ephemeral state - text cursor position
- Severity: P2 (minor)
- Spec Item: state.ephemeral[2] - "Text cursor position in hypothesis input"
- Expected: Implicit in text input implementation
- Problem: Not explicitly mentioned but likely handled by Ratatui text widget
- Fix: Document reliance on Ratatui widget behavior or add explicit cursor handling note

---

### PSC-015: Missing explicit coverage for persistent state - draft hypothesis auto-save
- Severity: P1 (important)
- Spec Item: state.persistent[2] - "Draft hypothesis text (saved on each keystroke or on quit)"
- Expected: Task covering auto-save of draft hypothesis
- Problem: P1-T7 covers "atomic session save on quit" but not per-keystroke draft saving
- Fix: Add task or expand P1-T5/P1-T7: "Auto-save draft hypothesis on each keystroke"

---

### PSC-016: Missing explicit coverage for AI timeout behavior
- Severity: P2 (minor)
- Spec Item: ai.timeout - "None - wait indefinitely for response"
- Expected: Note in PHASE-2 about no timeout
- Problem: P2-T4 mentions "subprocess failure" but not the explicit "wait indefinitely" behavior
- Fix: Add note to P2-T1 or P2-T4: "No timeout - wait indefinitely per spec"

---

### PSC-017: Missing explicit coverage for AI manual retry behavior
- Severity: P2 (minor)
- Spec Item: ai.retry - "Manual via 'r' key on error; no automatic retry"
- Expected: Task in PHASE-2 covering retry mechanism
- Problem: Not explicitly mentioned in any PHASE-2 task
- Fix: Add to P2-T4: "Implement manual retry via 'r' key; no automatic retry"

---

### PSC-018: Missing explicit coverage for security considerations
- Severity: P2 (minor)
- Spec Item: security section - Local user file permissions, data sent to Claude
- Expected: Note or task about file permission handling
- Problem: Security section not referenced in any plan task
- Fix: Add note to P1-T5: "Ensure session files use standard local user file permissions"

---

### PSC-019: Missing explicit coverage for invariant - valid JSON schema
- Severity: P1 (important)
- Spec Item: invariants[1] - "Session file is valid JSON and matches expected schema; if_violated: treat as corrupted and offer fresh start"
- Expected: Task covering schema validation and corruption handling
- Problem: No task explicitly handles corrupted session detection and fresh start offer
- Fix: Add to P4-T5 or new task: "Detect corrupted session files, offer fresh start option"

---

### PSC-020: Missing explicit coverage for invariant - AI subprocess state
- Severity: P2 (minor)
- Spec Item: invariants[2] - "AI subprocess is either running OR completed, never both"
- Expected: Task ensuring proper state machine for AI subprocess
- Problem: Not explicitly verified in any task
- Fix: Add to P2-T5 or P2-T6 tests: "Verify AI subprocess state machine (running XOR completed)"

---

### PSC-021: Missing explicit coverage for flow metrics tracking
- Severity: P2 (minor)
- Spec Item: flows[0].metrics - Track flow_started, triage_completed, review_completed, export_triggered
- Expected: Task for metrics implementation
- Problem: No task mentions metrics tracking (may be intentionally deferred)
- Fix: Either add metrics task to PHASE-4 or document as out-of-scope for initial implementation

---

### PSC-022: Phantom reference check - no phantoms found
- Severity: N/A
- All spec references in the plan (screens[], flows[], user_stories[], testing_focus[], invariants[], session_matching, ai.method, ai.concurrency, export) correspond to actual spec items.

---

### Coverage Statistics

| Category | Total Items | Covered | Partially Covered | Missing |
|----------|-------------|---------|-------------------|---------|
| Screens | 5 | 5 | 0 | 0 |
| Screen Components | ~20 | 15 | 3 | 2 |
| Screen States | ~15 | 12 | 2 | 1 |
| User Stories | 6 | 4 | 2 | 0 |
| Acceptance Criteria | ~20 | 15 | 4 | 1 |
| Flow Edge Cases | 8 | 5 | 2 | 1 |
| Invariants | 4 | 2 | 1 | 1 |
| Testing Focus | 4 | 4 | 0 | 0 |

**Overall Assessment**: The plan provides good high-level coverage but lacks explicit task-level mapping for several acceptance criteria, screen components, and edge cases. The P0 issues should be addressed before implementation begins. P1 issues should be incorporated during implementation planning. P2 issues can be addressed during implementation or follow-up.

---

### Recommended Priority Fixes

**P1 (Important) - Address before implementation:**
1. PSC-001: Add file-path input verification
2. PSC-002: Add invalid input error handling task
3. PSC-003: Add section ordering requirement to AI chunking
4. PSC-005: Add progress indicator task for TriageScreen
5. PSC-008: Add submitted state handling for PseudocodeReviewScreen
6. PSC-009: Add retry button (r key) to error handling
7. PSC-012: Add "all got it" flow shortcut
8. PSC-015: Add draft hypothesis auto-save
9. PSC-019: Add session corruption detection and recovery

**P2 (Minor) - Address during implementation:**
- PSC-004, PSC-006, PSC-007, PSC-010, PSC-011, PSC-013, PSC-014, PSC-016, PSC-017, PSC-018, PSC-020, PSC-021

## Verification Review

### Summary

This review audits whether the verification strategies in the implementation plan would actually catch broken implementations. I analyzed each phase against the spec's edge cases, testing focus areas, invariants, and acceptance criteria.

---

### PVR-001: Phase 1 verification lacks specific CLI parsing scenarios
- **Severity**: P1
- **Phase**: PHASE-1 (Foundation)
- **Current Verification**: "cargo test - CLI parsing, session persistence, git diff reading all pass"
- **Problem**: The spec defines four distinct CLI invocation patterns (`sherpa HEAD~5..HEAD`, `sherpa --staged`, `sherpa <file-path>`, `sherpa --new <diff-range>`) plus "Invalid input shows helpful error message". The verification just says "CLI parsing tests pass" without specifying which scenarios are tested. A broken implementation that only handles `HEAD~X..HEAD` format would pass this verification.
- **Fix**: Replace with: "cargo test verifies: (1) diff-range parsing (HEAD~5..HEAD), (2) --staged flag handling, (3) file-path input, (4) --new flag, (5) invalid input returns error with actionable message"

---

### PVR-002: Phase 1 missing atomic save verification for crash scenarios
- **Severity**: P0
- **Phase**: PHASE-1 (Foundation)
- **Current Verification**: "Running `sherpa HEAD~1..HEAD` enters app shell (empty screen OK), session file created"
- **Problem**: Task P1-T7 implements "atomic session save on quit/Ctrl+C" which the spec identifies as a key risk (testing_focus[0]: "Session file corruption on unexpected quit"). The verification only checks that a session file is created, not that it survives corruption. A naive implementation writing directly to the session file would pass this verification but fail the spec's requirement for atomic writes (temp file + rename).
- **Fix**: Add: "Verify atomic save: (1) Kill process mid-save with SIGKILL, verify session file is valid JSON or previous version, (2) Verify temp file pattern used (write to .tmp, rename)"

---

### PVR-003: Phase 2 AI error handling verification is vague
- **Severity**: P1
- **Phase**: PHASE-2 (AI Integration)
- **Current Verification**: "AI chunking returns valid sections; assessment returns structured feedback"
- **Problem**: Task P2-T4 implements "Handle AI errors gracefully: non-JSON output, subprocess failure". The spec's testing_focus[1] states "handles non-JSON output gracefully". The verification only checks the happy path (valid sections, structured feedback). An implementation that crashes on malformed JSON would pass this verification.
- **Fix**: Add: "Verify error handling: (1) Non-JSON AI output shows user-friendly error, not panic, (2) Subprocess spawn failure shows error with retry option, (3) Partial/truncated JSON response handled gracefully"

---

### PVR-004: Phase 2 missing concurrency constraint verification
- **Severity**: P1
- **Phase**: PHASE-2 (AI Integration)
- **Current Verification**: "AI chunking returns valid sections; assessment returns structured feedback"
- **Problem**: Task P2-T5 implements "Ensure single concurrent AI call with navigation allowed during wait" (from ai.concurrency spec). The verification does not check this constraint. An implementation allowing multiple concurrent calls would pass.
- **Fix**: Add: "Verify concurrency: (1) Second AI request while first pending is queued or rejected, (2) User can navigate screens during AI wait, (3) AI response is associated with correct request"

---

### PVR-005: Phase 3 screen verification is too generic
- **Severity**: P0
- **Phase**: PHASE-3 (Screens)
- **Current Verification**: "cargo test - all screen tests pass"
- **Problem**: This is pure verification theater. "Screen tests pass" tells us nothing about what's being tested. Phase 3 implements 5 screens with multiple states, components, and keybindings. A test that renders each screen once in default state would pass this verification while missing:
  - TriageScreen empty state vs default state
  - DeepReviewScreen error state with preserved user input
  - PseudocodeReviewScreen ai_response state with three colored sections
  - Keybinding conflicts or missing bindings
- **Fix**: Replace with specific test requirements for each screen:
  - "TriageScreen: test all 4 states (default, all_tagged, empty, error); verify j/k navigation, 1/2/3 tagging, badge display"
  - "DeepReviewScreen: test all 4 states; verify navigation during waiting_ai state"
  - "PseudocodeReviewScreen: test all 4 states; verify user text preserved in error state"
  - "SummaryScreen: test default and exported states; verify export creates file at correct path"

---

### PVR-006: Phase 3 acceptance criteria lacks edge case coverage
- **Severity**: P1
- **Phase**: PHASE-3 (Screens)
- **Current Verification**: "Can navigate through all screens with stub data; all keybindings respond; export creates file"
- **Problem**: The spec defines critical edge cases that this acceptance criteria would not catch:
  - "All sections tagged 'got it' -> show summary directly" (flows[0].steps[1].edge_cases)
  - "User quits mid-triage -> session saved for resume" (not testable with stubs, but state should be saveable)
  - "Back from Pseudocode returns to section list in DeepReview" (backstack behavior)
  - Empty diff state
- **Fix**: Add: "Acceptance tests for edge cases: (1) Tagging all sections 'got it' skips DeepReview to Summary, (2) Back navigation from each screen works per backstack spec, (3) Empty sections state displays message, (4) Error retry keybinding (r) triggers retry action"

---

### PVR-007: Phase 3 missing hypothesis preservation verification
- **Severity**: P1
- **Phase**: PHASE-3 (Screens)
- **Current Verification**: See above
- **Problem**: Task P3-T11 implements "hypothesis preservation on error and retry" which maps to invariant[3]: "User hypothesis is preserved until AI response is received - if_violated: User loses their work on retry". The spec states this as critical in testing_focus[1] as well. No verification checks this.
- **Fix**: Add: "Verify hypothesis preservation: (1) Simulate AI error, verify hypothesis text remains in input, (2) User presses retry (r), hypothesis is resubmitted without re-entry"

---

### PVR-008: Phase 4 stub cleanup verification is incomplete
- **Severity**: P2
- **Phase**: PHASE-4 (Integration)
- **Current Verification**: "cleanup: All stubs removed, no TODO comments remaining"
- **Problem**: This is a manual check pattern that's easy to miss. The Phase 3 temporary_stubs says "AI responses stubbed with hardcoded sections and assessments" but there's no programmatic way to verify stubs are gone. A developer could leave a stub in a rarely-executed code path.
- **Fix**: Add: "Verification: (1) grep for 'stub', 'hardcoded', 'TODO' in source, (2) Run E2E test against real claude subprocess (not mocked), (3) Verify LoadingScreen makes actual AI call"

---

### PVR-009: Phase 4 missing large diff performance verification
- **Severity**: P1
- **Phase**: PHASE-4 (Integration)
- **Current Verification**: No performance criteria specified
- **Problem**: Task P4-T8 implements "Handle large diffs: verify chunking and smooth scrolling (testing_focus[2])". The spec's testing_focus states "UI scrolls smoothly with many sections". The verification says nothing about performance or what "large" means. An implementation that hangs on 100+ section diffs would pass.
- **Fix**: Add: "Performance verification: (1) Generate test diff with 50+ sections, verify UI remains responsive (< 100ms frame time), (2) Scroll through section list without visual stuttering, (3) AI chunking completes within reasonable time (log warning if > 30s)"

---

### PVR-010: Phase 4 missing special character verification
- **Severity**: P1
- **Phase**: PHASE-4 (Integration)
- **Current Verification**: "All edge cases handled gracefully" (too vague)
- **Problem**: Task P4-T9 implements "Handle special characters in code/hypothesis (testing_focus[3])". The spec states "Unicode, escape sequences, and long lines render correctly". The verification "all edge cases handled gracefully" provides no concrete check.
- **Fix**: Add: "Special character tests: (1) Unicode in hypothesis (emoji, CJK characters) displays correctly, (2) Escape sequences in code (\n, \t, \\) render as expected, (3) Lines > 200 chars wrap or scroll horizontally, (4) JSON-unsafe characters in hypothesis don't break session save"

---

### PVR-011: Phase 4 missing session resume edge cases
- **Severity**: P1
- **Phase**: PHASE-4 (Integration)
- **Current Verification**: "Session auto-resumes on same input"
- **Problem**: The spec defines nuanced session behavior:
  - Existing session auto-resumes to saved state (could be TriageScreen OR DeepReviewScreen)
  - Corrupted session offers fresh start (invariant[1])
  - Draft hypothesis saved on each keystroke or quit (state.persistent)
  
  The verification only checks that resume happens, not that it resumes to the correct state or handles corruption.
- **Fix**: Add: "Session resume tests: (1) Resume mid-triage restores section tags and selected index, (2) Resume mid-review restores to DeepReviewScreen with correct section, (3) Corrupted session file shows error and offers fresh start, (4) Draft hypothesis text restored on resume"

---

### PVR-012: Phase 4 missing backstack behavior verification
- **Severity**: P2
- **Phase**: PHASE-4 (Integration)
- **Current Verification**: Acceptance only mentions forward flow
- **Problem**: The spec defines specific backstack behavior (flows[0].backstack):
  - "From Summary, back returns to last Deep Review section"
  - "From Deep Review, back returns to previous section or Triage"
  - "From Triage, back exits the application (session saved)"
  
  None of this is verified in Phase 4.
- **Fix**: Add: "Backstack verification: (1) Back from Summary returns to DeepReview at last reviewed section, (2) Back from DeepReview with unreviewed sections returns to previous section, (3) Back from first section returns to Triage, (4) Back from Triage saves and exits"

---

### PVR-013: Export content verification missing
- **Severity**: P2
- **Phase**: PHASE-4 (Integration)
- **Current Verification**: "Export produces valid markdown"
- **Problem**: The spec defines export.content as "Full transcript of all sections, hypotheses, and AI responses". Verification only checks for "valid markdown" which could be an empty file or missing content.
- **Fix**: Add: "Export content verification: (1) Exported file contains all section titles, (2) Each section includes user hypothesis, (3) AI responses (Correct/Diverges/Missed) included, (4) Confidence tags included, (5) File path matches pattern docs/sherpa-review-<date>-<time>.md"

---

### Overall Assessment

**P0 Issues (2)**: Phase 1 atomic save and Phase 3 screen testing are both inadequate. An implementation could completely fail these requirements and pass verification.

**P1 Issues (9)**: Multiple edge cases from the spec have no verification coverage, particularly around error handling, special characters, and session state management.

**P2 Issues (2)**: Minor gaps in stub cleanup and backstack testing.

**Recommendation**: The verification strategy needs significant strengthening. Each phase should have explicit test cases that map to spec requirements, especially for edge cases marked in `testing_focus` and `edge_cases` sections of the spec.

## Plan Parallelization Review

**Reviewer**: Claude (Automated Parallelization Audit)
**Date**: 2026-02-04
**Plan**: PLAN-CODE-REVIEW-TUI

### Phase Dependency Analysis

```
PHASE-1 (Foundation)
    |
    +---> PHASE-2 (AI Integration)
    |
    +---> PHASE-3 (Screens)
              |
              v
         PHASE-4 (Integration)
              ^
              |
         PHASE-2
```

PHASE-2 and PHASE-3 are designed to run in parallel, both depending only on PHASE-1.

---

### PPR-001: AI Response Type Definitions Needed by Both Phases

- **Severity**: P1 (important)
- **Phases**: PHASE-2 and PHASE-3
- **Conflict Type**: interface_mismatch
- **Details**: PHASE-3 task P3-T10 "Implement PseudocodeReviewScreen AI response display: Correct/Diverges/Missed sections" requires knowledge of the exact structure of AI responses. PHASE-2 task P2-T3 "Implement assessment prompt: send hypothesis + code, receive Correct/Diverges/Missed" defines this structure. If implemented in parallel without coordination, the screen implementation may assume a different response structure than what the AI client produces.
- **Evidence**:
  - P3-T10 must display three sections: "Correct" (green), "Diverges" (yellow), "Missed" (dim) from screens[3].screen_states.ai_response
  - P2-T3 defines the AI assessment response format
  - No shared type definitions are established in PHASE-1 for AI responses
- **Fix**: Add a task to PHASE-1 that defines the `AIAssessmentResponse` struct with `correct: Vec<String>`, `diverges: Vec<String>`, `missed: Vec<String>` fields. Both PHASE-2 and PHASE-3 should import this shared type. Alternatively, document the expected interface in the plan for both phases to follow.

---

### PPR-002: Chunking Response Structure for LoadingScreen

- **Severity**: P1 (important)  
- **Phases**: PHASE-2 and PHASE-3
- **Conflict Type**: interface_mismatch
- **Details**: PHASE-3 task P3-T1 "Implement LoadingScreen: spinner, status text, error state with retry" needs to handle the result of AI chunking. The structure of this result (success with sections, error types) is defined by PHASE-2's P2-T2 and P2-T4. The LoadingScreen must know what success/error states are possible.
- **Evidence**:
  - P2-T2: "Implement chunking prompt: send diff, receive sections with titles/descriptions"
  - P2-T4: "Handle AI errors gracefully: non-JSON output, subprocess failure"
  - P3-T1: LoadingScreen error state needs "retry option (r)" per spec screens[0].screen_states
  - Error types from P2-T4 not defined in shared location
- **Fix**: Define in PHASE-1 a `ChunkingResult` enum (or similar) with variants for `Success(Vec<Section>)`, `ParseError`, `SubprocessError`, etc. Both phases use this shared definition.

---

### PPR-003: Section Data Model May Need Extension

- **Severity**: P2 (minor)
- **Phases**: PHASE-2 and PHASE-3
- **Conflict Type**: shared_state
- **Details**: P1-T4 defines "Section (id, title, description, code, tag)" but PHASE-2's chunking may discover additional fields needed (e.g., file paths, line numbers, ordering hints) while PHASE-3's screens may discover UI-specific fields needed. Parallel development could lead to both phases wanting to modify the Section struct.
- **Evidence**:
  - P1-T4 defines initial Section model
  - P2-T2 may need to add fields for chunking metadata
  - P3-T2/P3-T3 may need fields for display state (scroll position, highlight ranges)
- **Fix**: This is a minor concern because PHASE-1 should define a comprehensive Section model. Review P1-T4's implementation before starting parallel work to ensure it includes all fields both phases need. Consider separating display state (ephemeral) from section data (persistent) as spec already suggests.

---

### PPR-004: AI Client Interface Not Defined in Foundation

- **Severity**: P1 (important)
- **Phases**: PHASE-2 and PHASE-3
- **Conflict Type**: missing_dependency
- **Details**: PHASE-3's screens need to call AI functions (or at least know the interface for mocking). P3-T15 mentions "Write tests: each screen renders correctly" which requires stubbing AI responses. Without a defined interface trait from PHASE-1, PHASE-3 cannot create proper stubs that will be compatible with PHASE-2's implementation.
- **Evidence**:
  - P3 verification mentions: "temporary_stubs: AI responses stubbed with hardcoded sections and assessments"
  - No AI client trait/interface defined in PHASE-1
  - P3-T7 needs to handle "waiting_ai" state which requires knowing how to poll/await AI
- **Fix**: Add a task to PHASE-1 to define an `AiClient` trait with async methods for `chunk_diff()` and `assess_hypothesis()`. PHASE-2 implements this trait, PHASE-3 uses a mock implementation for testing.

---

### PPR-005: Concurrency State for AI Calls

- **Severity**: P2 (minor)
- **Phases**: PHASE-2 and PHASE-3
- **Conflict Type**: shared_state
- **Details**: The spec states "ai.concurrency: One AI call at a time (user can navigate while waiting)". P2-T5 implements "single concurrent AI call with navigation allowed during wait". P3-T7 implements "waiting_ai" state in DeepReviewScreen. Both need to coordinate on how this shared concurrency state is managed - is it in the AI client (PHASE-2) or in the app state (PHASE-1)?
- **Evidence**:
  - P2-T5: "Ensure single concurrent AI call with navigation allowed during wait"
  - P3-T7: "waiting_ai: Spinner overlay while AI processes, section still visible, user can navigate away"
  - Both phases need to read/write AI-in-progress state
- **Fix**: PHASE-1 should establish where AI call state lives (likely in app state as `Option<InFlightAiCall>` or similar). Both phases then interact with this shared location rather than duplicating state.

---

### Verified Parallelization Patterns

The following aspects of parallel execution between PHASE-2 and PHASE-3 are correctly structured:

1. **Module Independence**: PHASE-2 creates an `ai/` module, PHASE-3 creates a `screens/` module. These are distinct file hierarchies with no direct file conflicts.

2. **Screen Rendering Logic**: Individual screen implementations (P3-T2 through P3-T14) are purely UI code that can use hardcoded stub data without needing actual AI integration.

3. **Git Diff Reading**: P1-T3 establishes git diff parsing in PHASE-1, which both subsequent phases can use without modification.

4. **Session Persistence**: P1-T5 and P1-T7 establish session save/load in PHASE-1. Both PHASE-2 (saving AI responses) and PHASE-3 (saving screen state) write to the same session format, but this is coordinated through the shared Session model.

5. **Event Loop**: P1-T6 creates the app shell and event loop in PHASE-1. PHASE-3 registers screens with this routing. PHASE-2's AI client is independent of routing.

---

### Summary

| ID | Severity | Issue |
|----|----------|-------|
| PPR-001 | P1 | AI assessment response types need shared definition |
| PPR-002 | P1 | Chunking result types need shared definition |
| PPR-003 | P2 | Section model may need extensions from both phases |
| PPR-004 | P1 | AI client trait should be defined in PHASE-1 |
| PPR-005 | P2 | AI concurrency state location needs clarity |

**Recommendation**: Add 2-3 tasks to PHASE-1 to establish shared interfaces:
1. Define `AiClient` trait with method signatures
2. Define `ChunkingResult` and `AssessmentResult` types
3. Define where AI call-in-progress state lives in app state

This allows PHASE-2 and PHASE-3 to develop against these contracts independently and merge cleanly in PHASE-4.

