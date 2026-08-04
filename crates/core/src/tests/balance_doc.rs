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
const GRADES: [&str; 3] = ["DESIGNED", "MEASURED", "PLAYTESTED"];

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

/// Every row whose Value column is a plain number must agree with the
/// constant it describes.
///
/// **The two tests above check that a row *exists* and carries a grade.
/// Neither has ever checked that it says the right thing, and the cost
/// of that showed up the moment anybody looked.** Auditing the file
/// against measurements turned up a lighting figure 70% out, a meals
/// figure a third out, and a sails row comparing income against a stride
/// cost **six times** the constant printed two rows above it — that one
/// was arithmetic left over from a value the game no longer has.
///
/// Prose cannot be tested and is not tested here. The Value column can:
/// where it is a single number, it is a fact with an answer, and letting
/// it drift is how a reader ends up reasoning about a game that no
/// longer exists. Rows whose value is a list, a range, a per-terrain
/// breakdown or a sentence are skipped — this is a guard against silent
/// rot, not a formatting rule.
#[test]
fn a_numeric_row_says_what_the_constant_says() {
    let value = serde_json::to_value(&content().balance).expect("balance must serialise");
    let mut fields = Vec::new();
    collect_leaves(&value, &mut fields);

    let mut wrong = Vec::new();
    let mut checked = 0;
    for (field, actual) in fields {
        let Some(actual) = actual.as_i64() else {
            continue;
        };
        let needle = format!("`{field}`");
        for line in BALANCE_DOC.lines() {
            // The row has to be *about* this field: its name is the
            // first thing in the row, not a mention further along.
            let cells: Vec<&str> = line.split('|').collect();
            if cells.len() < 4 || !cells[1].contains(&needle) {
                continue;
            }
            // One row, one constant. A row naming two knobs is a
            // per-terrain breakdown or a paired cost, and its value
            // column is prose by necessity.
            if cells[1].matches('`').count() != 2 {
                continue;
            }
            let printed = cells[2].trim().replace([',', '`'], "");
            let Ok(printed) = printed.parse::<i64>() else {
                continue;
            };
            checked += 1;
            if printed != actual {
                wrong.push(format!(
                    "{field}: doc says {printed}, the pack says {actual}"
                ));
            }
        }
    }

    assert!(
        checked > 20,
        "only {checked} rows were checkable — the table's shape has changed and this test \
         has quietly stopped guarding anything"
    );
    assert!(
        wrong.is_empty(),
        "BALANCE.md disagrees with the content pack:\n  {}",
        wrong.join("\n  ")
    );
}

/// Every documented `build_cost` must name every item the pack charges.
///
/// **The sibling test above guards `balance.ron` and nothing guards the
/// content pack, which is where four rows had gone stale at once.** The
/// cell bank's row said "8 poles"; the pack charges **2 charge cells**,
/// a tier-two material, so that room moved from an opening build to one
/// gated behind a whole chain and its row still described the old game.
/// The dumbwaiter, the elevator and the dart battery each said poles
/// only and each charge rope as well — which is not a detail, because
/// rope needs a ropery, a ropery needs fiber, and fiber needs a comb. A
/// reader pricing the elevator off that row would conclude vertical
/// transport is eighteen poles away when it is three rooms and a chain
/// away.
///
/// Checks presence rather than exact wording: a row has to mention each
/// item's amount and a recognisable piece of its name. Prose stays free,
/// facts do not.
#[test]
fn a_documented_build_cost_names_everything_the_pack_charges() {
    let content = content();
    let mut wrong = Vec::new();
    let mut checked = 0;

    let costs = content
        .rooms
        .iter()
        .map(|def| &def.name)
        .zip(content.room_runtime.iter().map(|rt| &rt.build_cost))
        .chain(
            content
                .shafts
                .iter()
                .map(|def| &def.name)
                .zip(content.shaft_runtime.iter().map(|rt| &rt.build_cost)),
        );

    for (name, cost) in costs {
        let needle = format!("| {} `build_cost`", name.to_lowercase());
        let Some(line) = BALANCE_DOC
            .lines()
            .find(|line| line.to_lowercase().starts_with(&needle))
        else {
            continue;
        };
        let cells: Vec<&str> = line.split('|').collect();
        let Some(value) = cells.get(2) else { continue };
        checked += 1;
        for (item, amount) in cost {
            // The last word of the item id — `item.charge_cells` is
            // "cells", `item.poles` is "poles" — which is what a row
            // would naturally call it.
            let word = content.items[item.get()]
                .id
                .rsplit(['.', '_'])
                .next()
                .unwrap_or("")
                .to_lowercase();
            if !value.contains(&amount.to_string()) || !value.to_lowercase().contains(&word) {
                wrong.push(format!(
                    "{name}: pack charges {amount} {word}, row's value column reads \"{}\"",
                    value.trim()
                ));
            }
        }
    }

    assert!(
        checked > 8,
        "only {checked} build-cost rows were found — the table's shape has changed and this \
         test has quietly stopped guarding anything"
    );
    assert!(
        wrong.is_empty(),
        "BALANCE.md build costs disagree with the content pack:\n  {}",
        wrong.join("\n  ")
    );
}

/// Leaf fields with their values, for the row-agreement check above.
fn collect_leaves(value: &serde_json::Value, out: &mut Vec<(String, serde_json::Value)>) {
    let serde_json::Value::Object(fields) = value else {
        return;
    };
    for (key, child) in fields {
        if child.is_object() {
            collect_leaves(child, out);
        } else {
            out.push((key.clone(), child.clone()));
        }
    }
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
