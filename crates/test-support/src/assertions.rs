//! Assertions that explain themselves.
//!
//! `assert_eq!` on a forty-field struct prints two walls of text and leaves you
//! to find the difference. Save/load and offline/live equivalence failures are
//! exactly that shape, and they are the failures most likely to matter, so they
//! get a comparison that points at the field.

use core::fmt::Debug;

/// Asserts equality, reporting the first differing line of the pretty-printed
/// structure rather than dumping both values in full.
///
/// # Panics
///
/// If the values differ.
#[track_caller]
pub fn assert_state_eq<T: Debug + PartialEq>(actual: &T, expected: &T) {
    if actual == expected {
        return;
    }
    panic!("{}", structural_diff(actual, expected));
}

/// Builds the diff report used by [`assert_state_eq`]. Exposed so a test can
/// assert on the message itself.
#[must_use]
pub fn structural_diff<T: Debug>(actual: &T, expected: &T) -> String {
    let actual_text = format!("{actual:#?}");
    let expected_text = format!("{expected:#?}");

    let actual_lines: Vec<&str> = actual_text.lines().collect();
    let expected_lines: Vec<&str> = expected_text.lines().collect();

    let first_difference = actual_lines
        .iter()
        .zip(expected_lines.iter())
        .position(|(left, right)| left != right)
        .unwrap_or_else(|| actual_lines.len().min(expected_lines.len()));

    const CONTEXT: usize = 4;
    let start = first_difference.saturating_sub(CONTEXT);
    let end = (first_difference + CONTEXT + 1).max(start + 1);

    let mut report = String::from("values differ\n");
    report.push_str(&format!(
        "first difference at line {} of the pretty-printed value\n\n",
        first_difference + 1
    ));

    for index in start..end {
        let actual_line = actual_lines.get(index);
        let expected_line = expected_lines.get(index);
        match (actual_line, expected_line) {
            (Some(left), Some(right)) if left == right => {
                report.push_str(&format!("   {left}\n"));
            }
            (left, right) => {
                if let Some(left) = left {
                    report.push_str(&format!(" - actual:   {left}\n"));
                }
                if let Some(right) = right {
                    report.push_str(&format!(" + expected: {right}\n"));
                }
            }
        }
    }

    if actual_lines.len() != expected_lines.len() {
        report.push_str(&format!(
            "\n(actual has {} lines, expected has {})\n",
            actual_lines.len(),
            expected_lines.len()
        ));
    }
    report
}

/// Asserts that `events` contains `expected` at least once.
///
/// # Panics
///
/// If no matching event is present.
#[track_caller]
pub fn assert_emitted<T: Debug + PartialEq>(events: &[T], expected: &T) {
    if events.iter().any(|event| event == expected) {
        return;
    }
    panic!("expected event was not emitted\n  wanted: {expected:#?}\n  got:    {events:#?}");
}

/// Asserts that nothing matching `unwanted` was emitted.
///
/// # Panics
///
/// If a matching event is present.
#[track_caller]
pub fn assert_not_emitted<T: Debug + PartialEq>(events: &[T], unwanted: &T) {
    if let Some(found) = events.iter().find(|event| *event == unwanted) {
        panic!("event should not have been emitted: {found:#?}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Sample {
        first: u32,
        second: &'static str,
        third: bool,
    }

    #[test]
    fn equal_values_do_not_panic() {
        let value = Sample {
            first: 1,
            second: "a",
            third: true,
        };
        assert_state_eq(
            &value,
            &Sample {
                first: 1,
                second: "a",
                third: true,
            },
        );
    }

    #[test]
    fn the_diff_points_at_the_changed_field() {
        let actual = Sample {
            first: 1,
            second: "a",
            third: true,
        };
        let expected = Sample {
            first: 1,
            second: "b",
            third: true,
        };
        let report = structural_diff(&actual, &expected);
        assert!(
            report.contains("\"a\""),
            "report should show the actual value:\n{report}"
        );
        assert!(
            report.contains("\"b\""),
            "report should show the expected value:\n{report}"
        );
        assert!(
            report.contains("second"),
            "report should name the field:\n{report}"
        );
    }

    #[test]
    fn the_diff_notes_a_length_mismatch() {
        let report = structural_diff(&vec![1, 2, 3], &vec![1, 2]);
        assert!(report.contains("lines"), "{report}");
    }

    #[test]
    #[should_panic(expected = "values differ")]
    fn differing_values_panic() {
        assert_state_eq(&1, &2);
    }

    #[test]
    fn emission_assertions_find_matches() {
        let events = vec!["hatched", "fed"];
        assert_emitted(&events, &"fed");
        assert_not_emitted(&events, &"died");
    }

    #[test]
    #[should_panic(expected = "was not emitted")]
    fn a_missing_event_panics() {
        assert_emitted(&["hatched"], &"fed");
    }
}
