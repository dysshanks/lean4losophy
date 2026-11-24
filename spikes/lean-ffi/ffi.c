/*
 * GTK4 implementation of the Plan A spike's C surface.
 *
 * This file is deliberately Lean-free so it can be compiled and reviewed
 * without a Lean toolchain. See lean_bridge.c for the Lean ABI adapter.
 *
 * Spike shortcuts, all of which are production lint findings:
 *   - file reads and writes happen on the main loop (APORIA-2001);
 *   - errors go to a dialog, not to structured logs (APORIA-1008);
 *   - there is one window and no document model.
 * The point of the spike is the FFI and event-loop path, not the editor.
 */

#include "ffi.h"

#include <gtk/gtk.h>
#include <string.h>

struct AporiaApp {
    GtkApplication *application;
    GtkApplicationWindow *window;
    GtkTextView *text_view;
    GtkLabel *title_label;
    GtkLabel *status_label;
    GFile *current_file;
    AporiaSaveCallback save_callback;
    void *save_user_data;
};

static void show_error(AporiaApp *app, const char *message, const char *detail) {
    GtkAlertDialog *dialog = gtk_alert_dialog_new("%s", message);
    gtk_alert_dialog_set_detail(dialog, detail);
    gtk_alert_dialog_set_modal(dialog, TRUE);
    gtk_alert_dialog_show(dialog, GTK_WINDOW(app->window));
    g_object_unref(dialog);
}

static void set_status(AporiaApp *app, const char *text) {
    gtk_label_set_label(app->status_label, text);
}

static void notify_saved(AporiaApp *app) {
    if (app->save_callback == NULL) {
        return;
    }
    GtkTextBuffer *buffer = gtk_text_view_get_buffer(app->text_view);
    GtkTextIter start;
    GtkTextIter end;
    gtk_text_buffer_get_bounds(buffer, &start, &end);
    char *text = gtk_text_buffer_get_text(buffer, &start, &end, TRUE);
    app->save_callback(text, app->save_user_data);
    g_free(text);
}

static void load_into_editor(AporiaApp *app, GFile *file) {
    char *contents = NULL;
    gsize length = 0;
    GError *error = NULL;

    if (!g_file_load_contents(file, NULL, &contents, &length, NULL, &error)) {
        show_error(app, "Could not open note", error->message);
        g_error_free(error);
        return;
    }

    if (!g_utf8_validate(contents, (gssize)length, NULL)) {
        show_error(app, "Could not open note",
                   "The selected file is not valid UTF-8. Aporia stores notes as UTF-8.");
        g_free(contents);
        return;
    }

    GtkTextBuffer *buffer = gtk_text_view_get_buffer(app->text_view);
    gtk_text_buffer_set_text(buffer, contents, (int)length);
    g_free(contents);

    g_set_object(&app->current_file, file);

    char *name = g_file_get_basename(file);
    gtk_label_set_label(app->title_label, name);
    char *status = g_strdup_printf("Opened %s", name);
    set_status(app, status);
    g_free(status);
    g_free(name);
}

static void on_open_finished(GObject *source, GAsyncResult *result, gpointer user_data) {
    AporiaApp *app = user_data;
    GError *error = NULL;
    GFile *file = gtk_file_dialog_open_finish(GTK_FILE_DIALOG(source), result, &error);

    if (file == NULL) {
        if (!g_error_matches(error, GTK_DIALOG_ERROR, GTK_DIALOG_ERROR_DISMISSED) &&
            !g_error_matches(error, G_IO_ERROR, G_IO_ERROR_CANCELLED)) {
            show_error(app, "Could not open note", error->message);
        }
        g_clear_error(&error);
        return;
    }

    load_into_editor(app, file);
    g_object_unref(file);
}

static void on_open_clicked(GtkButton *button, gpointer user_data) {
    (void)button;
    AporiaApp *app = user_data;

    GtkFileDialog *dialog = gtk_file_dialog_new();
    gtk_file_dialog_set_title(dialog, "Open note");
    gtk_file_dialog_set_modal(dialog, TRUE);

    GListStore *filters = g_list_store_new(GTK_TYPE_FILE_FILTER);
    GtkFileFilter *markdown = gtk_file_filter_new();
    gtk_file_filter_set_name(markdown, "Markdown notes");
    gtk_file_filter_add_suffix(markdown, "md");
    gtk_file_filter_add_suffix(markdown, "markdown");
    g_list_store_append(filters, markdown);
    gtk_file_dialog_set_filters(dialog, G_LIST_MODEL(filters));
    gtk_file_dialog_set_default_filter(dialog, markdown);

    gtk_file_dialog_open(dialog, GTK_WINDOW(app->window), NULL, on_open_finished, app);

    g_object_unref(markdown);
    g_object_unref(filters);
    g_object_unref(dialog);
}

static void write_to_file(AporiaApp *app, GFile *file) {
    GtkTextBuffer *buffer = gtk_text_view_get_buffer(app->text_view);
    GtkTextIter start;
    GtkTextIter end;
    gtk_text_buffer_get_bounds(buffer, &start, &end);
    char *text = gtk_text_buffer_get_text(buffer, &start, &end, TRUE);

    GError *error = NULL;
    gboolean ok = g_file_replace_contents(file, text, strlen(text), NULL, FALSE,
                                          G_FILE_CREATE_REPLACE_DESTINATION, NULL,
                                          NULL, &error);
    g_free(text);

    if (!ok) {
        show_error(app, "Could not save note", error->message);
        g_error_free(error);
        return;
    }

    g_set_object(&app->current_file, file);

    char *name = g_file_get_basename(file);
    gtk_label_set_label(app->title_label, name);
    char *status = g_strdup_printf("Saved %s", name);
    set_status(app, status);
    g_free(status);
    g_free(name);

    notify_saved(app);
}

static void on_save_finished(GObject *source, GAsyncResult *result, gpointer user_data) {
    AporiaApp *app = user_data;
    GError *error = NULL;
    GFile *file = gtk_file_dialog_save_finish(GTK_FILE_DIALOG(source), result, &error);

    if (file == NULL) {
        if (!g_error_matches(error, GTK_DIALOG_ERROR, GTK_DIALOG_ERROR_DISMISSED) &&
            !g_error_matches(error, G_IO_ERROR, G_IO_ERROR_CANCELLED)) {
            show_error(app, "Could not save note", error->message);
        }
        g_clear_error(&error);
        return;
    }

    write_to_file(app, file);
    g_object_unref(file);
}

static void on_save_clicked(GtkButton *button, gpointer user_data) {
    (void)button;
    AporiaApp *app = user_data;

    if (app->current_file != NULL) {
        write_to_file(app, app->current_file);
        return;
    }

    GtkFileDialog *dialog = gtk_file_dialog_new();
    gtk_file_dialog_set_title(dialog, "Save note");
    gtk_file_dialog_set_modal(dialog, TRUE);
    gtk_file_dialog_set_initial_name(dialog, "untitled.md");
    gtk_file_dialog_save(dialog, GTK_WINDOW(app->window), NULL, on_save_finished, app);
    g_object_unref(dialog);
}

static GtkWidget *build_sidebar(void) {
    GtkWidget *sidebar = gtk_box_new(GTK_ORIENTATION_VERTICAL, 8);
    gtk_widget_set_size_request(sidebar, 240, -1);
    gtk_widget_set_margin_top(sidebar, 12);
    gtk_widget_set_margin_bottom(sidebar, 12);
    gtk_widget_set_margin_start(sidebar, 12);
    gtk_widget_set_margin_end(sidebar, 12);

    GtkWidget *heading = gtk_label_new("Notes");
    gtk_widget_set_halign(heading, GTK_ALIGN_START);
    gtk_widget_add_css_class(heading, "heading");
    gtk_box_append(GTK_BOX(sidebar), heading);

    GtkWidget *list = gtk_list_box_new();
    gtk_list_box_set_selection_mode(GTK_LIST_BOX(list), GTK_SELECTION_NONE);
    const char *names[] = {"Welcome", "Sample note", "Reading list"};
    for (size_t i = 0; i < G_N_ELEMENTS(names); i++) {
        GtkWidget *row = gtk_label_new(names[i]);
        gtk_widget_set_halign(row, GTK_ALIGN_START);
        gtk_widget_set_margin_top(row, 6);
        gtk_widget_set_margin_bottom(row, 6);
        gtk_widget_set_margin_start(row, 8);
        gtk_widget_set_margin_end(row, 8);
        gtk_list_box_append(GTK_LIST_BOX(list), row);
    }

    GtkWidget *scroller = gtk_scrolled_window_new();
    gtk_scrolled_window_set_child(GTK_SCROLLED_WINDOW(scroller), list);
    gtk_widget_set_vexpand(scroller, TRUE);
    gtk_scrolled_window_set_has_frame(GTK_SCROLLED_WINDOW(scroller), TRUE);
    gtk_box_append(GTK_BOX(sidebar), scroller);

    GtkWidget *placeholder =
        gtk_label_new("Placeholder list. M1 replaces it with the notebook file tree.");
    gtk_label_set_wrap(GTK_LABEL(placeholder), TRUE);
    gtk_widget_set_halign(placeholder, GTK_ALIGN_START);
    gtk_widget_add_css_class(placeholder, "dim-label");
    gtk_box_append(GTK_BOX(sidebar), placeholder);

    return sidebar;
}

bool aporia_platform_init(void) {
    return gtk_init_check() != FALSE;
}

AporiaApp *aporia_app_new(const char *title) {
    AporiaApp *app = g_new0(AporiaApp, 1);
    if (app == NULL) {
        return NULL;
    }

    app->application = gtk_application_new("io.github.aporia.Notebook.ffi.spike",
                                           G_APPLICATION_DEFAULT_FLAGS);
    if (app->application == NULL) {
        g_free(app);
        return NULL;
    }

    app->window = GTK_APPLICATION_WINDOW(gtk_application_window_new(app->application));
    gtk_window_set_title(GTK_WINDOW(app->window), title);
    gtk_window_set_default_size(GTK_WINDOW(app->window), 1100, 720);

    app->title_label = GTK_LABEL(gtk_label_new("Untitled"));
    app->status_label = GTK_LABEL(gtk_label_new("No note open"));
    gtk_widget_set_halign(GTK_WIDGET(app->status_label), GTK_ALIGN_START);
    gtk_widget_add_css_class(GTK_WIDGET(app->status_label), "dim-label");

    GtkWidget *header = gtk_header_bar_new();
    gtk_header_bar_set_title_widget(GTK_HEADER_BAR(header), GTK_WIDGET(app->title_label));

    GtkWidget *open_button = gtk_button_new_from_icon_name("document-open-symbolic");
    gtk_widget_set_tooltip_text(open_button, "Open note");
    g_signal_connect(open_button, "clicked", G_CALLBACK(on_open_clicked), app);
    gtk_header_bar_pack_start(GTK_HEADER_BAR(header), open_button);

    GtkWidget *save_button = gtk_button_new_from_icon_name("document-save-symbolic");
    gtk_widget_set_tooltip_text(save_button, "Save note");
    gtk_widget_add_css_class(save_button, "suggested-action");
    g_signal_connect(save_button, "clicked", G_CALLBACK(on_save_clicked), app);
    gtk_header_bar_pack_end(GTK_HEADER_BAR(header), save_button);

    gtk_window_set_titlebar(GTK_WINDOW(app->window), header);

    GtkTextBuffer *buffer = gtk_text_buffer_new(NULL);
    app->text_view = GTK_TEXT_VIEW(gtk_text_view_new_with_buffer(buffer));
    gtk_text_view_set_monospace(app->text_view, TRUE);
    gtk_text_view_set_wrap_mode(app->text_view, GTK_WRAP_WORD);
    gtk_text_view_set_left_margin(app->text_view, 24);
    gtk_text_view_set_right_margin(app->text_view, 24);
    gtk_text_view_set_top_margin(app->text_view, 16);
    gtk_text_view_set_bottom_margin(app->text_view, 16);
    g_object_unref(buffer);

    GtkWidget *editor_scroller = gtk_scrolled_window_new();
    gtk_scrolled_window_set_child(GTK_SCROLLED_WINDOW(editor_scroller),
                                  GTK_WIDGET(app->text_view));
    gtk_widget_set_hexpand(editor_scroller, TRUE);
    gtk_widget_set_vexpand(editor_scroller, TRUE);

    GtkWidget *paned = gtk_paned_new(GTK_ORIENTATION_HORIZONTAL);
    gtk_paned_set_start_child(GTK_PANED(paned), build_sidebar());
    gtk_paned_set_end_child(GTK_PANED(paned), editor_scroller);
    gtk_paned_set_position(GTK_PANED(paned), 260);
    gtk_paned_set_resize_start_child(GTK_PANED(paned), FALSE);
    gtk_paned_set_shrink_start_child(GTK_PANED(paned), FALSE);

    GtkWidget *status_bar = gtk_box_new(GTK_ORIENTATION_HORIZONTAL, 0);
    gtk_widget_set_margin_top(status_bar, 6);
    gtk_widget_set_margin_bottom(status_bar, 6);
    gtk_widget_set_margin_start(status_bar, 16);
    gtk_widget_set_margin_end(status_bar, 16);
    gtk_box_append(GTK_BOX(status_bar), GTK_WIDGET(app->status_label));

    GtkWidget *root = gtk_box_new(GTK_ORIENTATION_VERTICAL, 0);
    gtk_box_append(GTK_BOX(root), paned);
    gtk_box_append(GTK_BOX(root), gtk_separator_new(GTK_ORIENTATION_HORIZONTAL));
    gtk_box_append(GTK_BOX(root), status_bar);
    gtk_window_set_child(GTK_WINDOW(app->window), root);

    return app;
}

void aporia_app_free(AporiaApp *app) {
    if (app == NULL) {
        return;
    }
    g_clear_object(&app->current_file);
    g_clear_object(&app->application);
    g_free(app);
}

void aporia_app_set_text(AporiaApp *app, const char *text) {
    GtkTextBuffer *buffer = gtk_text_view_get_buffer(app->text_view);
    gtk_text_buffer_set_text(buffer, text, -1);
}

char *aporia_app_get_text(const AporiaApp *app) {
    GtkTextBuffer *buffer = gtk_text_view_get_buffer(app->text_view);
    GtkTextIter start;
    GtkTextIter end;
    gtk_text_buffer_get_bounds(buffer, &start, &end);
    return gtk_text_buffer_get_text(buffer, &start, &end, TRUE);
}

void aporia_string_free(char *text) {
    g_free(text);
}

void aporia_app_set_save_callback(AporiaApp *app, AporiaSaveCallback callback,
                                  void *user_data) {
    app->save_callback = callback;
    app->save_user_data = user_data;
}

int aporia_app_run(AporiaApp *app) {
    gtk_window_present(GTK_WINDOW(app->window));
    char *argv[] = {(char *)"aporia-ffi-spike", NULL};
    return g_application_run(G_APPLICATION(app->application), 1, argv);
}
