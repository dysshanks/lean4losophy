#ifndef LEAN4LOSOPHY_FFI_H
#define LEAN4LOSOPHY_FFI_H

/*
 * Plain C surface of the Plan A spike.
 *
 * `ffi.c` implements this API with GTK4 only. `lean_bridge.c` adapts it to
 * Lean's `@[extern]` ABI. Keeping the two apart means the GTK half can be
 * compiled and reviewed without a Lean toolchain.
 *
 * Threading: every function must be called from the thread that runs
 * `lean4losophy_app_run`. GTK is not thread-safe.
 *
 * Ownership:
 *   - `lean4losophy_app_new` returns an owned pointer, released with
 *     `lean4losophy_app_free`.
 *   - Strings passed in are borrowed for the duration of the call.
 *   - `lean4losophy_app_get_text` returns an owned string, released with
 *     `lean4losophy_string_free`.
 *   - The save callback's `text` is borrowed for the duration of the call.
 */

#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct Lean4losophyApp Lean4losophyApp;

typedef void (*lean4losophySaveCallback)(const char *text, void *user_data);

/* Initialises GTK. Returns false when no display is available. */
bool lean4losophy_platform_init(void);

/* Creates the window and its widgets. Returns NULL on allocation failure. */
Lean4losophyApp *lean4losophy_app_new(const char *title);

/* Releases the app. Must not be called while the main loop is running. */
void lean4losophy_app_free(Lean4losophyApp *app);

/* Replaces the editor contents. */
void lean4losophy_app_set_text(Lean4losophyApp *app, const char *text);

/* Returns a copy of the editor contents; free with `lean4losophy_string_free`. */
char *lean4losophy_app_get_text(const Lean4losophyApp *app);

void lean4losophy_string_free(char *text);

/* Registers the callback invoked after a successful save. May be NULL. */
void lean4losophy_app_set_save_callback(Lean4losophyApp *app, lean4losophySaveCallback callback,
                                  void *user_data);

/* Runs the GTK main loop. Returns the process exit status. */
int lean4losophy_app_run(Lean4losophyApp *app);

#ifdef __cplusplus
}
#endif

#endif /* LEAN4LOSOPHY_FFI_H */
