//! Balance constants must be documented and graded.
//!
//! `v2-plan.md` §10.6 asks for constants that live in data with a
//! provenance grade, so you know which numbers have earned trust and
//! which are still a guess. Documentation drifts unless something
//! fails, so this is that something: every field in `balance.ron` has
//! to appear in `BALANCE.md` carrying a grade.

use crate::tests::content;

const BALANCE_DOC: &str = include_str!("../../../../docs/BALANCE.md");

/// The grades a constant may carry, in increasing order of trust.
const GRADES: [&str; 2] = ["DESIGNED", "PLAYTESTED"];

#[test]
fn every_balance_field_is_documented_with_a_grade() {
    let value = serde_json::to_value(&content().balance).expect("balance must serialise");
    let mut fields = Vec::new();
    collect_leaf_fields(&value, &mut fields);
    assert!(!fields.is_empty(), "balance has no fields to check");

    let missing: Vec<&String> = fields
        .iter()
        .filter(|field| !documented_with_grade(field))
        .collect();

    assert!(
        missing.is_empty(),
        "these balance constants are missing a graded row in docs/BALANCE.md: {missing:?}"
    );
}

#[test]
fn the_doc_does_not_grade_constants_that_no_longer_exist() {
    let value = serde_json::to_value(&content().balance).expect("balance must serialise");
    let mut fields = Vec::new();
    collect_leaf_fields(&value, &mut fields);

    let stale: Vec<&str> = BALANCE_DOC
        .lines()
        .filter(|line| line.trim_start().starts_with("| `"))
        .filter_map(|line| line.split('`').nth(1))
        // The legend at the top of the doc names the grades themselves
        // in backticks; those are not constants.
        .filter(|name| !GRADES.contains(name))
        .filter(|name| !fields.iter().any(|field| field == name))
        .collect();

    assert!(
        stale.is_empty(),
        "docs/BALANCE.md grades constants that balance.ron no longer defines: {stale:?}"
    );
}

/// A row counts as documented when the line naming the field in
/// backticks also carries one of the grades.
fn documented_with_grade(field: &str) -> bool {
    let needle = format!("`{field}`");
    BALANCE_DOC
        .lines()
        .filter(|line| line.contains(&needle))
        .any(|line| GRADES.iter().any(|grade| line.contains(grade)))
}

/// Field names of every tuning knob, ignoring the nesting path — the
/// doc is a flat table and the names are unique.
///
/// A list-valued field (a cost, a starting stock) is one knob, not one
/// per element: `floor_cost` gets a row, `item` and `amount` do not.
fn collect_leaf_fields(value: &serde_json::Value, out: &mut Vec<String>) {
    let serde_json::Value::Object(fields) = value else {
        return;
    };
    for (key, child) in fields {
        if child.is_object() {
            collect_leaf_fields(child, out);
        } else {
            out.push(key.clone());
        }
    }
}
