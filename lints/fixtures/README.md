# Lint rule fixtures

Every rule in `lints/` has a pair here:

- `fixtures/bad/<rule-id>.rs` must produce a finding;
- `fixtures/good/<rule-id>.rs` must produce none.

`./scripts/lint` runs both and fails when a rule does not behave as documented,
so a rule cannot silently rot into a no-op. The fixture files are not compiled
and are not part of the workspace; they exist only for the rule tests.

The `<rule-id>` is the YAML file name without the extension, matching the `id`
field inside the rule.
