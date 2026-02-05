use crate::error::{AppError, Result};

/// A parsed hunk from a unified diff
#[derive(Debug, Clone)]
pub struct Hunk {
    /// 1-based index of the hunk within its diff
    pub index: usize,
    /// The @@ header line
    pub header: String,
    /// The full hunk content (header + lines)
    pub content: String,
    /// Starting line in the old file
    pub old_start: usize,
    /// Starting line in the new file
    pub new_start: usize,
    /// Number of lines in the new file
    pub new_count: usize,
}

/// Parse a unified diff into numbered hunks.
///
/// Uses the `unidiff` crate for robust parsing of @@ headers, including
/// proper handling of function context, edge cases, and malformed input.
/// Each hunk's content includes the @@ header and all diff lines.
/// The first hunk of each file includes the --- / +++ file headers.
pub fn parse_hunks(diff_text: &str) -> Vec<Hunk> {
    let mut patch = unidiff::PatchSet::new();
    if patch.parse(diff_text).is_err() {
        return Vec::new();
    }

    let mut hunks = Vec::new();
    let mut index = 0usize;

    for file in patch.files() {
        let preamble = format!("--- {}\n+++ {}\n", file.source_file, file.target_file);

        for (i, uh) in file.hunks().iter().enumerate() {
            index += 1;

            let header = if uh.section_header.is_empty() {
                format!(
                    "@@ -{},{} +{},{} @@",
                    uh.source_start, uh.source_length,
                    uh.target_start, uh.target_length,
                )
            } else {
                format!(
                    "@@ -{},{} +{},{} @@ {}",
                    uh.source_start, uh.source_length,
                    uh.target_start, uh.target_length,
                    uh.section_header,
                )
            };

            // Use unidiff's Display impl for faithful hunk text reconstruction
            let hunk_text = uh.to_string();
            let content = if i == 0 {
                format!("{}{}\n", preamble, hunk_text)
            } else {
                format!("{}\n", hunk_text)
            };

            hunks.push(Hunk {
                index,
                header,
                content,
                old_start: uh.source_start,
                new_start: uh.target_start,
                new_count: uh.target_length,
            });
        }
    }

    hunks
}

/// Filter hunks by 1-based indices, reassemble into a diff string
pub fn filter_by_hunks(hunks: &[Hunk], indices: &[usize]) -> Result<String> {
    let mut result = String::new();
    for &idx in indices {
        let hunk = hunks
            .iter()
            .find(|h| h.index == idx)
            .ok_or_else(|| AppError::Git(format!("Hunk index {} not found (have 1..{})", idx, hunks.len())))?;
        result.push_str(&hunk.content);
    }
    Ok(result)
}

/// Filter hunks to those overlapping a line range in the new file.
/// `start` and `end` are 1-based line numbers in the current (new) file.
pub fn filter_by_lines(hunks: &[Hunk], start: usize, end: usize) -> String {
    let mut result = String::new();
    for hunk in hunks {
        if hunk.new_count == 0 {
            continue; // deletion-only hunk has no new-file lines
        }
        let hunk_end = hunk.new_start + hunk.new_count.saturating_sub(1);
        // Check overlap: hunk range [new_start, hunk_end] vs [start, end]
        if hunk.new_start <= end && hunk_end >= start {
            result.push_str(&hunk.content);
        }
    }
    result
}

/// Format hunks for preview display with numbered indices
pub fn format_hunk_preview(hunks: &[Hunk]) -> String {
    let mut output = String::new();
    for hunk in hunks {
        output.push_str(&format!(
            "--- Hunk {} ---  {}\n",
            hunk.index, hunk.header
        ));
        let added = hunk.content.lines().filter(|l| l.starts_with('+') && !l.starts_with("+++")).count();
        let removed = hunk.content.lines().filter(|l| l.starts_with('-') && !l.starts_with("---")).count();
        output.push_str(&format!("  +{} -{} lines (new file: lines {}-{})\n", added, removed, hunk.new_start, hunk.new_start + hunk.new_count.saturating_sub(1)));
        output.push('\n');
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_DIFF: &str = "\
diff --git a/src/main.rs b/src/main.rs
index abcdef1..abcdef2 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,5 +1,6 @@
 use std::io;
+use std::fs;

 fn main() {
     println!(\"hello\");
@@ -10,3 +11,5 @@
 fn helper() {
+    // new comment
+    do_thing();
     old_thing();
 }
";

    #[test]
    fn test_parse_hunks() {
        let hunks = parse_hunks(SAMPLE_DIFF);
        assert_eq!(hunks.len(), 2);
        assert_eq!(hunks[0].index, 1);
        assert_eq!(hunks[0].new_start, 1);
        assert_eq!(hunks[0].new_count, 6);
        assert_eq!(hunks[1].index, 2);
        assert_eq!(hunks[1].new_start, 11);
        assert_eq!(hunks[1].new_count, 5);
    }

    #[test]
    fn test_filter_by_hunks() {
        let hunks = parse_hunks(SAMPLE_DIFF);
        let result = filter_by_hunks(&hunks, &[2]).unwrap();
        assert!(result.contains("new comment"));
        assert!(!result.contains("use std::fs"));
    }

    #[test]
    fn test_filter_by_hunks_invalid_index() {
        let hunks = parse_hunks(SAMPLE_DIFF);
        let result = filter_by_hunks(&hunks, &[99]);
        assert!(result.is_err());
    }

    #[test]
    fn test_filter_by_lines() {
        let hunks = parse_hunks(SAMPLE_DIFF);
        // Lines 11-15 should only match the second hunk
        let result = filter_by_lines(&hunks, 11, 15);
        assert!(result.contains("new comment"));
        assert!(!result.contains("use std::fs"));
    }

    #[test]
    fn test_filter_by_lines_overlap() {
        let hunks = parse_hunks(SAMPLE_DIFF);
        // Lines 1-15 should match both hunks
        let result = filter_by_lines(&hunks, 1, 15);
        assert!(result.contains("use std::fs"));
        assert!(result.contains("new comment"));
    }

    #[test]
    fn test_empty_diff() {
        let hunks = parse_hunks("");
        assert!(hunks.is_empty());
    }

    #[test]
    fn test_preamble_included_in_first_hunk() {
        let hunks = parse_hunks(SAMPLE_DIFF);
        // First hunk should include file headers
        assert!(hunks[0].content.contains("--- a/src/main.rs"));
        assert!(hunks[0].content.contains("+++ b/src/main.rs"));
        // Second hunk should not
        assert!(!hunks[1].content.contains("--- a/src/main.rs"));
    }

    #[test]
    fn test_format_hunk_preview() {
        let hunks = parse_hunks(SAMPLE_DIFF);
        let preview = format_hunk_preview(&hunks);
        assert!(preview.contains("Hunk 1"));
        assert!(preview.contains("Hunk 2"));
    }

    #[test]
    fn test_header_with_function_context() {
        // This was a bug with the hand-rolled parser: function context
        // after @@ starting with - or + would corrupt parsed line numbers
        let diff = "\
diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -100,5 +200,7 @@ fn process(-data: &str)
 fn helper() {
+    new_thing();
     old_thing();
 }
";
        let hunks = parse_hunks(diff);
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].old_start, 100);
        assert_eq!(hunks[0].new_start, 200);
        assert_eq!(hunks[0].new_count, 7);
    }

    #[test]
    fn test_single_line_hunk_header() {
        let diff = "\
diff --git a/f.rs b/f.rs
--- a/f.rs
+++ b/f.rs
@@ -1 +1 @@
-old
+new
";
        let hunks = parse_hunks(diff);
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].old_start, 1);
        assert_eq!(hunks[0].new_start, 1);
        assert_eq!(hunks[0].new_count, 1);
    }

    #[test]
    fn test_deletion_hunk_excluded_from_line_filter() {
        // A deletion-only hunk (new_count=0) should not match any line range
        let diff = "\
diff --git a/f.rs b/f.rs
--- a/f.rs
+++ b/f.rs
@@ -10,3 +10,0 @@
-removed1
-removed2
-removed3
";
        let hunks = parse_hunks(diff);
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].new_count, 0);
        // Should not match any line range since it's a pure deletion
        let result = filter_by_lines(&hunks, 1, 100);
        assert!(result.is_empty());
    }
}
