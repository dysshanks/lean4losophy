// SPDX-License-Identifier: Apache-2.0
// Fixture for APORIA-1009. This file must FAIL the no-allow rule.

#[allow(clippy::unwrap_used)]
fn suppressed_without_a_code() {
    let _ = std::fs::read_to_string("note.md");
}
