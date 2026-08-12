//! Leo's Mulder/Ream algorithm for propagating public-file edits through
//! sentinelized private text.

use std::ops::Range;

/// Reinsert the sentinels from `old_private` into the edited public text.
///
/// `start_delimiter` and `end_delimiter` describe the sentinel comment. For a
/// line comment such as `#`, pass an empty end delimiter. For a block comment,
/// pass both delimiters, such as `/*` and `*/`.
pub fn propagate_clean_changes(
    new_public: &str,
    old_private: &str,
    start_delimiter: &str,
    end_delimiter: &str,
) -> String {
    let marker = Marker {
        start: start_delimiter,
        end: end_delimiter,
    };
    let (old_public, mut sentinels, trailing) = split_private(old_private, marker);
    let old_public = normalized_lines(&old_public.concat());
    let new_public = normalized_lines(new_public);
    let matches = lcs_matches(&old_public, &new_public);
    let mut result = Vec::new();

    // Leo puts leading sentinels first, then prevents them being emitted twice.
    put_sentinels(&mut result, &sentinels, 0);
    if let Some(first) = sentinels.first_mut() {
        first.clear();
    }

    let mut old_at = 0;
    let mut new_at = 0;
    for (old_match, new_match) in matches
        .into_iter()
        .chain(std::iter::once((old_public.len(), new_public.len())))
    {
        replace_region(
            &mut result,
            &sentinels,
            &new_public,
            old_at..old_match,
            new_at..new_match,
            marker,
        );
        if old_match < old_public.len() {
            put_sentinels(&mut result, &sentinels, old_match);
            put_plain_line(&mut result, &old_public[old_match], marker);
        }
        old_at = old_match + 1;
        new_at = new_match + 1;
    }
    result.extend(trailing);
    result.concat()
}

#[derive(Clone, Copy)]
struct Marker<'a> {
    start: &'a str,
    end: &'a str,
}

impl Marker<'_> {
    fn is_sentinel(self, line: &str) -> bool {
        let line = line.trim();
        line.starts_with(&format!("{}@", self.start))
            && (self.end.is_empty() || line.ends_with(self.end))
    }

    fn is_verbatim(self, line: &str) -> bool {
        let line = line.trim();
        line.starts_with(&format!("{}@verbatim", self.start))
            && (self.end.is_empty() || line.ends_with(self.end))
    }
}

fn normalized_lines(text: &str) -> Vec<String> {
    text.lines().map(|line| format!("{line}\n")).collect()
}

fn original_lines(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    text.split_inclusive('\n').map(str::to_owned).collect()
}

fn split_private(
    old_private: &str,
    marker: Marker<'_>,
) -> (Vec<String>, Vec<Vec<String>>, Vec<String>) {
    let lines = original_lines(old_private);
    let mut public = Vec::new();
    let mut sentinels = Vec::new();
    let mut pending = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        let line = &lines[index];
        index += 1;
        if marker.is_verbatim(line) {
            if let Some(line) = lines.get(index) {
                sentinels.push(std::mem::take(&mut pending));
                public.push(line.clone());
                index += 1;
            }
        } else if marker.is_sentinel(line) {
            pending.push(line.clone());
        } else {
            sentinels.push(std::mem::take(&mut pending));
            public.push(line.clone());
        }
    }
    (public, sentinels, pending)
}

fn replace_region(
    result: &mut Vec<String>,
    sentinels: &[Vec<String>],
    new_lines: &[String],
    old: Range<usize>,
    new: Range<usize>,
    marker: Marker<'_>,
) {
    let mut replacements = new_lines[new].iter();
    for old_index in old {
        put_sentinels(result, sentinels, old_index);
        if let Some(line) = replacements.next() {
            put_plain_line(result, line, marker);
        }
    }
    for line in replacements {
        put_plain_line(result, line, marker);
    }
}

fn put_sentinels(result: &mut Vec<String>, sentinels: &[Vec<String>], index: usize) {
    if let Some(lines) = sentinels.get(index) {
        result.extend(lines.iter().cloned());
    }
}

fn put_plain_line(result: &mut Vec<String>, line: &str, marker: Marker<'_>) {
    if marker.is_sentinel(line) {
        result.push(format!("{}@verbatim{}\n", marker.start, marker.end));
    }
    result.push(line.to_owned());
}

/// Return one increasing set of equal-line pairs. This is the set of anchors
/// needed by the propagation rules; regions between anchors are replacements.
fn lcs_matches(old: &[String], new: &[String]) -> Vec<(usize, usize)> {
    let mut lengths = vec![vec![0usize; new.len() + 1]; old.len() + 1];
    for i in (0..old.len()).rev() {
        for j in (0..new.len()).rev() {
            lengths[i][j] = if old[i] == new[j] {
                lengths[i + 1][j + 1] + 1
            } else {
                lengths[i + 1][j].max(lengths[i][j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut matches = Vec::new();
    while i < old.len() && j < new.len() {
        if old[i] == new[j] {
            matches.push((i, j));
            i += 1;
            j += 1;
        } else if lengths[i + 1][j] >= lengths[i][j + 1] {
            i += 1;
        } else {
            j += 1;
        }
    }
    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    const PRIVATE: &str = "#@+leo-ver=5-thin\n#@+node:root: * root\nroot 1\n#@+node:child: ** child\nchild 1\nchild 2\n#@-leo\n";

    #[test]
    fn propagates_insert_delete_and_replace_without_moving_sentinels() {
        let public = "root changed\ninserted\nchild 2\n";
        let expected = "#@+leo-ver=5-thin\n#@+node:root: * root\nroot changed\n#@+node:child: ** child\ninserted\nchild 2\n#@-leo\n";
        assert_eq!(propagate_clean_changes(public, PRIVATE, "#", ""), expected);
    }

    #[test]
    fn protects_new_lines_that_resemble_sentinels() {
        let public = "root 1\n#@not-a-sentinel-to-the-user\nchild 1\nchild 2\n";
        let result = propagate_clean_changes(public, PRIVATE, "#", "");
        assert!(result.contains("#@verbatim\n#@not-a-sentinel-to-the-user\n"));
    }

    #[test]
    fn supports_block_comment_sentinels() {
        let private = "/*@+leo-ver=5-thin*/\n/*@+node:r: * r*/\none\n/*@-leo*/\n";
        let expected = "/*@+leo-ver=5-thin*/\n/*@+node:r: * r*/\ntwo\n/*@-leo*/\n";
        assert_eq!(
            propagate_clean_changes("two\n", private, "/*", "*/"),
            expected
        );
    }
}
