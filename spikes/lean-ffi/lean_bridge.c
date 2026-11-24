/*
 * Lean ABI adapter for the Plan A spike.
 *
 * `ffi.c` knows nothing about Lean; this file translates Lean's `@[extern]`
 * calls into the plain C API and translates the save callback back into an
 * exported Lean function.
 *
 * Ownership rules used here:
 *   - `lean_object *` arguments from Lean are borrowed (`@&`) or owned as
 *     declared in Main.lean; strings are read with `lean_string_cstr`, which
 *     must not be stored past the call.
 *   - Every `lean_object *` returned by a Lean call is decremented exactly
 *     once.
 *   - The GTK loop runs on the Lean main thread, so callbacks into Lean stay
 *     on that thread. Do not call these from a worker thread.
 *
 * This file was not compiled in the spike environment because Lean is not
 * installed, so `lean/lean.h` was unavailable. Treat it as a design sketch to
 * validate with the pinned toolchain.
 */

#include <lean/lean.h>

#include "ffi.h"

/* Implemented in Main.lean with @[export aporia_on_save]. */
extern lean_object *aporia_on_save(lean_object *text);

static AporiaApp *app_of_handle(lean_object *handle) {
    return (AporiaApp *)lean_unbox(handle);
}

static lean_object *ok_unit(void) {
    return lean_io_result_mk_ok(lean_box(0));
}

static lean_object *io_error(const char *message) {
    return lean_io_result_mk_error(lean_mk_io_user_error(lean_mk_string(message)));
}

/* Called by GTK after a successful save. */
static void on_save(const char *text, void *user_data) {
    (void)user_data;
    lean_object *arg = lean_mk_string(text);
    lean_object *result = aporia_on_save(arg);
    lean_dec(result);
}

lean_object *aporia_gtk_init(void) {
    if (!aporia_platform_init()) {
        return io_error("GTK could not open a display");
    }
    return ok_unit();
}

lean_object *aporia_window_new(lean_object *title) {
    AporiaApp *app = aporia_app_new(lean_string_cstr(title));
    if (app == NULL) {
        return io_error("could not create the GTK window");
    }
    aporia_app_set_save_callback(app, on_save, NULL);
    return lean_io_result_mk_ok(lean_box((size_t)app));
}

lean_object *aporia_window_set_text(lean_object *handle, lean_object *text) {
    AporiaApp *app = app_of_handle(handle);
    if (app == NULL) {
        return io_error("window handle is null");
    }
    aporia_app_set_text(app, lean_string_cstr(text));
    return ok_unit();
}

lean_object *aporia_window_get_text(lean_object *handle) {
    AporiaApp *app = app_of_handle(handle);
    if (app == NULL) {
        return io_error("window handle is null");
    }
    char *text = aporia_app_get_text(app);
    lean_object *result = lean_mk_string(text);
    aporia_string_free(text);
    return lean_io_result_mk_ok(result);
}

lean_object *aporia_window_run(lean_object *handle) {
    AporiaApp *app = app_of_handle(handle);
    if (app == NULL) {
        return io_error("window handle is null");
    }
    int status = aporia_app_run(app);
    return lean_io_result_mk_ok(lean_box_uint32((uint32_t)status));
}

lean_object *aporia_window_free(lean_object *handle) {
    AporiaApp *app = app_of_handle(handle);
    aporia_app_free(app);
    return ok_unit();
}
