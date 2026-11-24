# APORIA-2001 — No blocking I/O or CPU work on the UI thread

**Severity:** error
**Tool:** review, `ast-grep` rule `blocking-in-ui-handler`, profiling in CI
**Scope:** `crates/lean4losophy` and any code called from a GTK signal handler or the main context.

## Why

The GTK main loop runs typing, redraw, and input-method handling. Any blocking call on it freezes the window, and the spec sets a 16 ms typing budget and a 2 s paper preview. Reading a note on selection, running the SQLite FTS query over 10,000 notes, compiling Typst for the preview, or waiting for Lean diagnostics all break that budget. A freeze while a learner is mid-sentence loses keystrokes, and autosave may not have run yet.

## Alternatives

- Move file, SQLite, Lean, and Typst work to a worker and post results with `glib::MainContext::invoke` or `spawn_future_local`.
- Use an async task with `Loading`, `Ready`, and `Failed` state so the UI can show progress.
- Debounce preview compiles (300 ms) and cancel the previous job instead of queueing.
- Run Typst in a separate process and stream the PDF path back.

## Exceptions

- Reviewer approval for work with a measured budget under 1 ms, with the benchmark and reason in the `#[expect]`.
- No exception for file I/O, SQLite, Lean LSP calls, Typst compilation, or network on the UI thread.
- A whole-module exception needs an ADR.

## Examples

Bad:

```rust
open_button.connect_clicked(move |_| {
    let text = std::fs::read_to_string(&path)?; // blocks the main loop
    editor.buffer().set_text(&text);
});
```

Good:

```rust
open_button.connect_clicked(move |_| {
    let path = path.clone();
    let (tx, rx) = async_channel::bounded(1);
    std::thread::spawn(move || tx.send_blocking(read_note(&path)).ok());
    glib::spawn_future_local(async move {
        if let Ok(result) = rx.recv().await { show_note(result); }
    });
});
```
