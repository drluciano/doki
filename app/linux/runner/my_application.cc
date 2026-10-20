#include "my_application.h"

#include <flutter_linux/flutter_linux.h>
#ifdef GDK_WINDOWING_X11
#include <gdk/gdkx.h>
#endif

#include "autostart.h"
#include "flutter/generated_plugin_registrant.h"
#include "tray.h"

// Started at sign-in (autostart.cc): open in the tray, not on screen.
constexpr char kHiddenArgument[] = "--hidden";

// How long a hidden start waits for a tray to show the icon before showing
// the window instead, so Doki never runs with no way to reach it.
constexpr guint kTrayWaitSeconds = 10;

struct _MyApplication {
  GtkApplication parent_instance;
  char** dart_entrypoint_arguments;
  GtkWindow* window;
  FlMethodChannel* channel;
  DokiTray* tray;
  gboolean keep_in_tray;
  gboolean start_hidden;
  gboolean told_about_tray;
};

G_DEFINE_TYPE(MyApplication, my_application, GTK_TYPE_APPLICATION)

// Linux settings live next to Doki's other settings ($XDG_CONFIG_HOME, which
// the Flatpak keeps across updates). "Keep in tray" is on unless turned off.
static gchar* settings_path() {
  return g_build_filename(g_get_user_config_dir(), "doki", "desktop.ini",
                          nullptr);
}

static gboolean read_keep_in_tray() {
  g_autofree gchar* path = settings_path();
  g_autoptr(GKeyFile) file = g_key_file_new();
  if (!g_key_file_load_from_file(file, path, G_KEY_FILE_NONE, nullptr)) {
    return TRUE;
  }
  g_autoptr(GError) error = nullptr;
  gboolean value = g_key_file_get_boolean(file, "tray", "keep", &error);
  return error != nullptr ? TRUE : value;
}

static gboolean write_keep_in_tray(gboolean enabled, GError** error) {
  g_autofree gchar* path = settings_path();
  g_autofree gchar* dir = g_path_get_dirname(path);
  g_mkdir_with_parents(dir, 0700);
  g_autoptr(GKeyFile) file = g_key_file_new();
  g_key_file_load_from_file(file, path, G_KEY_FILE_KEEP_COMMENTS, nullptr);
  g_key_file_set_boolean(file, "tray", "keep", enabled);
  return g_key_file_save_to_file(file, path, error);
}

static void open_window(MyApplication* self) {
  if (self->window != nullptr) {
    gtk_widget_show(GTK_WIDGET(self->window));
    gtk_window_present(self->window);
  }
}

static void quit(MyApplication* self) {
  g_application_quit(G_APPLICATION(self));
}

static void send_notification(MyApplication* self,
                              const gchar* key,
                              const gchar* title,
                              const gchar* body) {
  g_autoptr(GNotification) notification = g_notification_new(title);
  g_notification_set_body(notification, body);
  g_notification_set_default_action(notification, "app.open");
  // The same key replaces the older notification about the same game.
  g_application_send_notification(G_APPLICATION(self), key, notification);
}

// Closing hides Doki while a tray shows its icon; without one, closing quits,
// as on Windows, because nothing would be left to bring it back or quit it.
static gboolean delete_event_cb(GtkWidget* widget, GdkEvent*,
                                MyApplication* self) {
  if (!self->keep_in_tray || !doki_tray_is_shown(self->tray)) {
    return FALSE;
  }
  gtk_widget_hide(widget);
  if (!self->told_about_tray) {
    // Say once per run where Doki went, so closing does not look like quitting.
    self->told_about_tray = TRUE;
    send_notification(self, "tray", "Doki is still running",
                      "It keeps saving checkpoints and syncing from the tray. "
                      "Use the tray icon's menu to quit.");
  }
  return TRUE;
}

static void tray_cb(DokiTrayAction action, gpointer user_data) {
  MyApplication* self = MY_APPLICATION(user_data);
  switch (action) {
    case DOKI_TRAY_OPEN:
      open_window(self);
      break;
    case DOKI_TRAY_SYNC:
      if (self->channel != nullptr) {
        fl_method_channel_invoke_method(self->channel, "sync", nullptr, nullptr,
                                        nullptr, nullptr);
      }
      break;
    case DOKI_TRAY_QUIT:
      quit(self);
      break;
  }
}

static gboolean tray_wait_cb(gpointer user_data) {
  MyApplication* self = MY_APPLICATION(user_data);
  if (!doki_tray_is_shown(self->tray)) {
    open_window(self);
  }
  return G_SOURCE_REMOVE;
}

static void set_keep_in_tray(MyApplication* self, gboolean enabled) {
  self->keep_in_tray = enabled;
  if (!enabled) {
    // Never leave the window hidden with nothing to bring it back.
    open_window(self);
  }
}

static FlValue* string_argument(FlValue* args, const char* name) {
  if (args == nullptr || fl_value_get_type(args) != FL_VALUE_TYPE_MAP) {
    return nullptr;
  }
  FlValue* value = fl_value_lookup_string(args, name);
  return value != nullptr && fl_value_get_type(value) == FL_VALUE_TYPE_STRING
             ? value
             : nullptr;
}

static FlValue* enabled_argument(FlValue* args) {
  if (args == nullptr || fl_value_get_type(args) != FL_VALUE_TYPE_MAP) {
    return nullptr;
  }
  FlValue* value = fl_value_lookup_string(args, "enabled");
  return value != nullptr && fl_value_get_type(value) == FL_VALUE_TYPE_BOOL
             ? value
             : nullptr;
}

static void autostart_cb(gboolean ok, const char* error, gpointer user_data) {
  g_autoptr(FlMethodCall) call = FL_METHOD_CALL(user_data);
  if (ok) {
    fl_method_call_respond_success(call, nullptr, nullptr);
  } else {
    fl_method_call_respond_error(call, "Io", error, nullptr, nullptr);
  }
}

// The same calls as the Windows runner (windows/runner/flutter_window.cpp), so
// lib/backend.dart needs no Linux branch. "StartWithWindows" is starting at
// sign-in.
static void method_call_cb(FlMethodChannel*, FlMethodCall* call,
                           gpointer user_data) {
  MyApplication* self = MY_APPLICATION(user_data);
  const gchar* method = fl_method_call_get_name(call);
  FlValue* args = fl_method_call_get_args(call);
  if (g_strcmp0(method, "getKeepInTray") == 0) {
    g_autoptr(FlValue) value = fl_value_new_bool(self->keep_in_tray);
    fl_method_call_respond_success(call, value, nullptr);
  } else if (g_strcmp0(method, "getStartWithWindows") == 0) {
    g_autoptr(FlValue) value = fl_value_new_bool(doki_autostart_enabled());
    fl_method_call_respond_success(call, value, nullptr);
  } else if (g_strcmp0(method, "setKeepInTray") == 0) {
    FlValue* enabled = enabled_argument(args);
    g_autoptr(GError) error = nullptr;
    if (enabled == nullptr) {
      fl_method_call_respond_error(call, "InvalidInput",
                                   "expected {enabled: bool}", nullptr,
                                   nullptr);
    } else if (!write_keep_in_tray(fl_value_get_bool(enabled), &error)) {
      fl_method_call_respond_error(call, "Io", error->message, nullptr,
                                   nullptr);
    } else {
      set_keep_in_tray(self, fl_value_get_bool(enabled));
      fl_method_call_respond_success(call, nullptr, nullptr);
    }
  } else if (g_strcmp0(method, "setStartWithWindows") == 0) {
    FlValue* enabled = enabled_argument(args);
    if (enabled == nullptr) {
      fl_method_call_respond_error(call, "InvalidInput",
                                   "expected {enabled: bool}", nullptr,
                                   nullptr);
    } else {
      doki_autostart_set(fl_value_get_bool(enabled), autostart_cb,
                         g_object_ref(call));
    }
  } else if (g_strcmp0(method, "notify") == 0) {
    FlValue* key = string_argument(args, "key");
    FlValue* title = string_argument(args, "title");
    FlValue* body = string_argument(args, "body");
    if (key == nullptr || title == nullptr || body == nullptr) {
      fl_method_call_respond_error(call, "InvalidInput",
                                   "expected {key, title, body}", nullptr,
                                   nullptr);
    } else {
      send_notification(self, fl_value_get_string(key),
                        fl_value_get_string(title), fl_value_get_string(body));
      g_autoptr(FlValue) value = fl_value_new_bool(TRUE);
      fl_method_call_respond_success(call, value, nullptr);
    }
  } else {
    fl_method_call_respond_not_implemented(call, nullptr);
  }
}

// Called when first Flutter frame received.
static void first_frame_cb(MyApplication* self, FlView* view) {
  if (self->start_hidden && self->keep_in_tray) {
    g_timeout_add_seconds(kTrayWaitSeconds, tray_wait_cb, self);
    return;
  }
  gtk_widget_show(gtk_widget_get_toplevel(GTK_WIDGET(view)));
}

static gchar* tray_icon_path() {
  g_autofree gchar* self = g_file_read_link("/proc/self/exe", nullptr);
  if (self == nullptr) {
    return g_strdup("data/doki-tray.png");
  }
  g_autofree gchar* dir = g_path_get_dirname(self);
  return g_build_filename(dir, "data", "doki-tray.png", nullptr);
}

// Implements GApplication::activate.
static void my_application_activate(GApplication* application) {
  MyApplication* self = MY_APPLICATION(application);
  if (self->window != nullptr) {
    // Opened again (from the app menu, or a second launch): bring back the
    // running Doki, which may be hidden in the tray.
    open_window(self);
    return;
  }
  GtkWindow* window =
      GTK_WINDOW(gtk_application_window_new(GTK_APPLICATION(application)));
  self->window = window;

  // Use a header bar when running in GNOME as this is the common style used
  // by applications and is the setup most users will be using (e.g. Ubuntu
  // desktop).
  // If running on X and not using GNOME then just use a traditional title bar
  // in case the window manager does more exotic layout, e.g. tiling.
  // If running on Wayland assume the header bar will work (may need changing
  // if future cases occur).
  gboolean use_header_bar = TRUE;
#ifdef GDK_WINDOWING_X11
  GdkScreen* screen = gtk_window_get_screen(window);
  if (GDK_IS_X11_SCREEN(screen)) {
    const gchar* wm_name = gdk_x11_screen_get_window_manager_name(screen);
    if (g_strcmp0(wm_name, "GNOME Shell") != 0) {
      use_header_bar = FALSE;
    }
  }
#endif
  if (use_header_bar) {
    GtkHeaderBar* header_bar = GTK_HEADER_BAR(gtk_header_bar_new());
    gtk_widget_show(GTK_WIDGET(header_bar));
    gtk_header_bar_set_title(header_bar, "doki");
    gtk_header_bar_set_show_close_button(header_bar, TRUE);
    gtk_window_set_titlebar(window, GTK_WIDGET(header_bar));
  } else {
    gtk_window_set_title(window, "doki");
  }

  gtk_window_set_default_size(window, 1280, 720);
  gtk_window_set_icon_name(window, APPLICATION_ID);
  g_signal_connect(window, "delete-event", G_CALLBACK(delete_event_cb), self);

  g_autoptr(FlDartProject) project = fl_dart_project_new();
  fl_dart_project_set_dart_entrypoint_arguments(
      project, self->dart_entrypoint_arguments);

  FlView* view = fl_view_new(project);
  GdkRGBA background_color;
  // Background defaults to black, override it here if necessary, e.g. #00000000
  // for transparent.
  gdk_rgba_parse(&background_color, "#000000");
  fl_view_set_background_color(view, &background_color);
  gtk_widget_show(GTK_WIDGET(view));
  gtk_container_add(GTK_CONTAINER(window), GTK_WIDGET(view));

  // Show the window when Flutter renders.
  // Requires the view to be realized so we can start rendering.
  g_signal_connect_swapped(view, "first-frame", G_CALLBACK(first_frame_cb),
                           self);
  gtk_widget_realize(GTK_WIDGET(view));

  fl_register_plugins(FL_PLUGIN_REGISTRY(view));

  g_autoptr(FlStandardMethodCodec) codec = fl_standard_method_codec_new();
  self->channel = fl_method_channel_new(
      fl_engine_get_binary_messenger(fl_view_get_engine(view)),
      "dev.doki/window", FL_METHOD_CODEC(codec));
  fl_method_channel_set_method_call_handler(self->channel, method_call_cb, self,
                                            nullptr);

  self->keep_in_tray = read_keep_in_tray();
  g_autofree gchar* icon = tray_icon_path();
  self->tray = doki_tray_new(icon, tray_cb, self);

  gtk_widget_grab_focus(GTK_WIDGET(view));
}

// Implements GApplication::local_command_line.
static gboolean my_application_local_command_line(GApplication* application,
                                                  gchar*** arguments,
                                                  int* exit_status) {
  MyApplication* self = MY_APPLICATION(application);
  // Strip out the first argument as it is the binary name.
  self->dart_entrypoint_arguments = g_strdupv(*arguments + 1);
  self->start_hidden =
      g_strv_contains(const_cast<const gchar* const*>(*arguments + 1),
                      kHiddenArgument);

  g_autoptr(GError) error = nullptr;
  if (!g_application_register(application, nullptr, &error)) {
    g_warning("Failed to register: %s", error->message);
    *exit_status = 1;
    return TRUE;
  }

  // One Doki at a time: a second launch brings the running one back, except
  // a start at sign-in, which leaves it where it is.
  if (!(g_application_get_is_remote(application) && self->start_hidden)) {
    g_application_activate(application);
  }
  *exit_status = 0;

  return TRUE;
}

static void open_action_cb(GSimpleAction*, GVariant*, gpointer user_data) {
  g_application_activate(G_APPLICATION(user_data));
}

// Implements GApplication::startup.
static void my_application_startup(GApplication* application) {
  G_APPLICATION_CLASS(my_application_parent_class)->startup(application);

  // Clicking a notification opens Doki.
  g_autoptr(GSimpleAction) open = g_simple_action_new("open", nullptr);
  g_signal_connect(open, "activate", G_CALLBACK(open_action_cb), application);
  g_action_map_add_action(G_ACTION_MAP(application), G_ACTION(open));
}

// Implements GApplication::shutdown.
static void my_application_shutdown(GApplication* application) {
  MyApplication* self = MY_APPLICATION(application);
  g_clear_pointer(&self->tray, doki_tray_free);
  g_clear_object(&self->channel);

  G_APPLICATION_CLASS(my_application_parent_class)->shutdown(application);
}

// Implements GObject::dispose.
static void my_application_dispose(GObject* object) {
  MyApplication* self = MY_APPLICATION(object);
  g_clear_pointer(&self->dart_entrypoint_arguments, g_strfreev);
  G_OBJECT_CLASS(my_application_parent_class)->dispose(object);
}

static void my_application_class_init(MyApplicationClass* klass) {
  G_APPLICATION_CLASS(klass)->activate = my_application_activate;
  G_APPLICATION_CLASS(klass)->local_command_line =
      my_application_local_command_line;
  G_APPLICATION_CLASS(klass)->startup = my_application_startup;
  G_APPLICATION_CLASS(klass)->shutdown = my_application_shutdown;
  G_OBJECT_CLASS(klass)->dispose = my_application_dispose;
}

static void my_application_init(MyApplication* self) {}

MyApplication* my_application_new() {
  // Set the program name to the application ID, which helps various systems
  // like GTK and desktop environments map this running application to its
  // corresponding .desktop file. This ensures better integration by allowing
  // the application to be recognized beyond its binary name.
  g_set_prgname(APPLICATION_ID);

  // Unique (not G_APPLICATION_NON_UNIQUE): the bus name dev.doki.doki makes a
  // second launch hand over to the running Doki (local_command_line).
  return MY_APPLICATION(g_object_new(my_application_get_type(),
                                     "application-id", APPLICATION_ID, "flags",
                                     G_APPLICATION_DEFAULT_FLAGS, nullptr));
}
