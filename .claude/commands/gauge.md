Analyze the current git diff and create a structured gauge review session for the TUI.

Arguments: $ARGUMENTS

Parse the arguments for `--base <ref>`. The `<ref>` can be a branch name (e.g., `main`),
a commit hash (e.g., `abc123`), or a relative reference (e.g., `HEAD~2`). If `--base` is
not provided, default to `main`. Use this ref as `BASE_REF` throughout.

You are building a code review walkthrough. The goal is to break a branch's changes into
logical sections that a reviewer can step through one at a time in the TUI, understanding
each piece before moving to the next. Think of it like creating a guided tour of the PR.

## Step 0: Understand the changes

Before touching the CLI, get the full picture:

1. Run `git diff --stat $(git merge-base <BASE_REF> HEAD)..HEAD` to see which files changed and how much.
2. Run `git log --oneline $(git merge-base <BASE_REF> HEAD)..HEAD` to see the commit history.
3. Read changed files as needed to understand the intent — don't just look at diffs in isolation.
   Understand *why* the changes were made, not just *what* changed.

## Step 1: Initialize the session

Derive a session name from the branch name. Sanitize it to only contain alphanumeric chars,
hyphens, and underscores (e.g., `feature/add-auth` becomes `feature-add-auth`).

```
gauge init <session-name> --base <BASE_REF>
```

## Step 2: Preview diffs per file

For each changed file, preview its diff with numbered hunks:

```
gauge diff preview --only <file-path>
```

This shows each hunk with its index number, line counts, and position. Use this output to
decide how to group hunks into sections. Note which hunks are related to each other — hunks
from different files may belong in the same section if they serve the same purpose.

## Step 3: Plan your sections

Before creating sections, plan the grouping. Good sections are:

- **Thematic, not per-file**: Group by *what the change does*, not which file it's in.
  A "Add user validation" section might include hunks from `models/user.rs`, `handlers/auth.rs`,
  and `tests/user_test.rs` together because they all serve the same purpose.
- **Ordered for understanding**: Put foundational changes first (new types, data models),
  then the code that uses them (business logic, handlers), then tests/config last.
  A reviewer should be able to understand each section without having seen later ones.
- **Right-sized**: Each section should be reviewable in a few minutes. A section with 200+
  lines of diff is too big — split it. A section with 3 trivial lines may not need its own section.
- **Well-described**: The title should say what the section *does* (e.g., "Add rate limiting
  middleware"), not just where it is (e.g., "Changes to middleware.rs"). The description should
  explain *why* this change exists and what the reviewer should focus on.

Typical section patterns:
- Data model / type definitions
- Core logic / algorithm implementation
- Integration / wiring (connecting new code to existing systems)
- API / handler changes
- Configuration / infrastructure
- Tests

## Step 4: Create sections and add code blocks

For each planned section:

### Create the section
```
gauge section add --title "Section title" --description "What this section does and why"
```
This prints the section ID (e.g., `sec_1`). Use this ID for adding code blocks.

### Add code blocks from diffs
For diff content (changes between base branch and HEAD):
```
gauge code add <sec_id> --only <file-path>                     # Full file diff
gauge code add <sec_id> --only <file-path> --hunks 1,3         # Specific hunks (1-based)
gauge code add <sec_id> --only <file-path> --lines 10-50       # Specific line range
```

### Add code blocks from files (for context)
When the reviewer needs to see surrounding code that didn't change (e.g., a struct definition
that existing diff code references):
```
gauge code add <sec_id> --file <file-path>                     # Full file
gauge code add <sec_id> --file <file-path> --lines 1-30        # Line range
```

### Tips for code blocks
- **Use `--hunks` liberally**: Don't dump a whole file diff if only hunks 2 and 4 are relevant
  to this section. Be precise — it helps the reviewer focus.
- **Order code blocks within a section**: Add them in the order the reviewer should read them.
  Usually: types/structs first, then implementation, then callers.
- **Add file context when helpful**: If a diff adds a method call to `process_data()`, consider
  adding a `--file` block showing the `process_data` function so the reviewer can see what it does.
- **One file can appear in multiple sections**: If `main.rs` has hunks for both "Add logging" and
  "Add error handling", split its hunks across both sections using `--hunks`.

## Step 5: Verify and finalize

Review what you built:
```
gauge section list                    # See all sections with code block counts
gauge section show <sec_id>           # Check a section's details
gauge code list <sec_id>              # See code blocks in a section
gauge code show <sec_id> <code_id>    # View actual code content
```

If something looks wrong:
```
gauge section update <sec_id> --title "Better title" --description "Better description"
gauge section reorder sec_3 sec_1 sec_2       # Reorder sections
gauge code reorder <sec_id> code_2 code_1     # Reorder code blocks within a section
gauge code delete <sec_id> <code_id>          # Remove a code block
gauge section delete <sec_id>                 # Remove a section entirely
```

When everything looks good:
```
gauge done
```

## Step 6: Tell the user

Report what you created:
- Session name
- Number of sections and total code blocks
- A brief table of sections: ID, title, number of code blocks, files involved
- Tell the user to launch the TUI: `gauge open <session-name>`

## Important guidelines

- **Every changed hunk should appear in exactly one section.** Don't skip changes and don't
  duplicate them. Run `gauge diff preview --only <file>` for each file and track which hunks
  you've assigned.
- **Prefer smaller, focused sections over large catch-all ones.** "Miscellaneous changes" is a
  sign of poor grouping — find the common thread or split further.
- **Read the code before grouping.** Don't just guess from filenames. A change in `config.rs`
  might logically belong with "Add feature X" rather than in a "Configuration" section.
- **Shell-escape titles and descriptions** that contain quotes or special characters.
