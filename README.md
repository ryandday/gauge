# gauge

Structured code review sessions, built by AI agents, reviewed by humans.

## The problem

GitHub shows diffs alphabetically by filename. You see `api/handlers.rs` before the data model it depends on. Tests before the implementation they test. There's no narrative, no ordering for understanding.

Large PRs become a maze. You jump between files trying to piece together what changed and why. The diff is all there — it's just in the wrong order.

## How gauge works

An AI agent reads your diff, groups changes thematically, orders them for understanding, and writes descriptions. You open the TUI and walk through the sections like a guided tour.

**Build phase** — an agent uses the gauge CLI to create a review session. It previews diffs, plans sections by theme (not by file), selects specific hunks, and orders everything so foundational changes come first.

**Review phase** — you open the TUI and triage each section: *got it*, *shaky*, or *lost*. For sections you don't fully understand, you write what you *think* the code does. An AI assesses your understanding and tells you what you got right, where you diverged, and what you missed.

## Quick start

On a feature branch, have an agent build the session:

```
/gauge
```

Then open it:

```
gauge open <session-name>
```

The `/gauge` slash command analyzes your branch's diff against `main`, creates themed sections, adds the right code blocks, and hands you a ready-to-review session.

## CLI reference

The CLI is the agent's API for building review sessions. Every command is designed to be called programmatically.

### Sessions

```
gauge init <name> [--base <ref>]    # Create session (default base: merge-base with main)
gauge open <name>                   # Launch TUI
gauge list                          # List all sessions
gauge done                          # Mark session ready for triage
```

### Sections

```
gauge section add --title "..." --description "..."   # Create section, prints sec_id
gauge section list                                     # List all sections
gauge section show <sec_id>                            # Show section details
gauge section update <sec_id> [--title "..."] [--description "..."]
gauge section reorder <sec_id> <sec_id> ...            # Set section order
gauge section delete <sec_id>
```

### Code blocks

Each section contains one or more code blocks. A code block is either a diff (from `git diff base_ref..HEAD`) or a file read (for context).

```
# From diff
gauge code add <sec_id> --only <path>                  # Full file diff
gauge code add <sec_id> --only <path> --hunks 1,3      # Specific hunks (1-based)
gauge code add <sec_id> --only <path> --lines 10-50    # Line range

# From file (for surrounding context)
gauge code add <sec_id> --file <path>                  # Full file
gauge code add <sec_id> --file <path> --lines 1-30     # Line range
```

```
gauge code list <sec_id>
gauge code show <sec_id> <code_id>
gauge code update <sec_id> <code_id> [--only|--file ...] [--hunks ...] [--lines ...]
gauge code reorder <sec_id> <code_id> <code_id> ...
gauge code delete <sec_id> <code_id>
```

### Diff preview

Agents use this to plan hunk grouping:

```
gauge diff preview --only <path>     # Show diff with numbered hunks
```

## The TUI

Three stages, each with keybindings shown in the footer:

**Triage** — scroll through sections, read the code with syntax highlighting, tag each one: `1` got it, `2` shaky, `3` lost. Auto-advances to the next section after each tag.

**Deep review** — for sections tagged shaky or lost, write what you think the code does in your own words. Submit your hypothesis and get an AI assessment: what you got right (*correct*), where your interpretation differs (*diverges*), and what you didn't mention (*missed*).

**Summary** — confidence breakdown, accuracy stats, and a list of all misconceptions. Export the full review to markdown.

Sessions are saved automatically. Quit and resume anytime with `gauge open <name>`.

## Writing agent commands

The `/gauge` slash command is a prompt template at `.claude/commands/gauge.md` that drives the CLI. It instructs the agent to:

1. Understand the full diff (`git diff --stat`, `git log`, read changed files)
2. Preview each file's diff with numbered hunks
3. Plan sections by theme and order for understanding
4. Create sections and add code blocks with precise hunk selection
5. Verify the session and finalize

You can customize this prompt or write your own. Any tool that can run shell commands can build gauge sessions.

## Install

```
cargo install --path .
```

Requires [Claude Code](https://docs.anthropic.com/en/docs/claude-code) for AI hypothesis assessment in the TUI and for the `/gauge` agent command.

## License

Apache 2.0
