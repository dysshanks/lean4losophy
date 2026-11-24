#ifndef APORIA_FFI_H
#define APORIA_FFI_H

/*
 * Plain C surface of the Plan A spike.
 *
 * `ffi.c` implements this API with GTK4 only. `lean_bridge.c` adapts it to
 * Lean's `@[extern]` ABI. Keeping the two apart means the GTK half can be
 * compiled and reviewed without a Lean toolchain.
 *
 * Threading: every function must be called from the thread that runs
 * `aporia_app_run`. GTK is not thread-safe.
 *
 * Ownership:
 *   - `aporia_app_new` returns an owned pointer, released with
 *     `aporia_app_free`.
 *   - Strings passed in are borrowed for the duration of the call.
 *   - `aporia_app_get_text` returns an owned string, released with
 *     `aporia_string_free`.
 *   - The save callback's `text` is borrowed for the duration of the call.
 */

#include <stdbool.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct AporiaApp AporiaApp;

typedef void (*AporiaSaveCallback)(const char *text, void *user_data);

/* Initialises GTK. Returns false when no display is available. */
bool aporia_platform_init(void);

/* Creates the window and its widgets. Returns NULL on allocation failure. */
AporiaApp *aporia_app_new(const char *title);

/* Releases the app. Must not be called while the main loop is running. */
void aporia_app_free(AporiaApp *app);

/* Replaces the editor contents. */
void aporia_app_set_text(AporiaApp *app, const char *text);

/* Returns a copy of the editor contents; free with `aporia_string_free`. */
char *aporia_app_get_text(const AporiaApp *app);

void aporia_string_free(char *text);

/* Registers the callback invoked after a successful save. May be NULL. */
void aporia_app_set_save_callback(AporiaApp *app, AporiaSaveCallback callback,
                                  void *user_data);

/* Runs the GTK main loop. Returns the process exit status. */
int aporia_app_run(AporiaApp *app);

#ifdef __cplusplus
}
#endif

#endif /* APORIA_FFI_H */
