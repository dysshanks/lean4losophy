// SPDX-License-Identifier: Apache-2.0
// Fixture for APORIA-1009. This file must PASS the no-allow rule.

#[expect(clippy::unwrap_used, reason = "APORIA-1002: fixture invariant; issue #42")]
fn suppressed_with_a_code_and_reason() {
    let _ = std::fs::read_to_string("note.md");
}
