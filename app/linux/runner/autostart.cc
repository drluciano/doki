#include "autostart.h"

#include <errno.h>
#include <glib/gstdio.h>

namespace {

constexpr char kPortalName[] = "org.freedesktop.portal.Desktop";
constexpr char kPortalPath[] = "/org/freedesktop/portal/desktop";
constexpr char kBackground[] = "org.freedesktop.portal.Background";
constexpr char kRequest[] = "org.freedesktop.portal.Request";

// The response code a portal sends when the user said no.
constexpr guint32 kPortalCancelled = 1;

struct Pending {
  gboolean enabled;
  DokiAutostartCallback callback;
  gpointer user_data;
  GDBusConnection* bus;
  gchar* handle;
  guint response_id;
};

}  // namespace

static gboolean in_flatpak() {
  return g_file_test("/.flatpak-info", G_FILE_TEST_EXISTS);
}

// The host's autostart entry. Inside the Flatpak $XDG_CONFIG_HOME points into
// the sandbox, so the real ~/.config is used; the portal writes the same file.
static gchar* entry_path() {
  return g_build_filename(g_get_home_dir(), ".config", "autostart",
                          APPLICATION_ID ".desktop", nullptr);
}

gboolean doki_autostart_enabled() {
  g_autofree gchar* path = entry_path();
  return g_file_test(path, G_FILE_TEST_EXISTS);
}

// Quotes a path for a desktop entry's Exec key: double quotes, with ", `, $
// and \ escaped, then every backslash doubled again for the string value.
static gchar* quote_exec(const gchar* path) {
  GString* out = g_string_new("\"");
  for (const gchar* c = path; *c != '\0'; c++) {
    if (*c == '"' || *c == '`' || *c == '$') {
      g_string_append(out, "\\\\");
    } else if (*c == '\\') {
      g_string_append(out, "\\\\\\");
    }
    g_string_append_c(out, *c);
  }
  g_string_append_c(out, '"');
  return g_string_free(out, FALSE);
}

// Used outside the Flatpak, and inside it when the desktop has no Background
// portal.
static gboolean write_entry(gboolean enabled, GError** error) {
  g_autofree gchar* path = entry_path();
  if (!enabled) {
    if (g_unlink(path) != 0 && errno != ENOENT) {
      g_set_error(error, G_IO_ERROR, g_io_error_from_errno(errno),
                  "could not remove %s", path);
      return FALSE;
    }
    return TRUE;
  }
  g_autofree gchar* dir = g_path_get_dirname(path);
  if (g_mkdir_with_parents(dir, 0700) != 0) {
    g_set_error(error, G_IO_ERROR, g_io_error_from_errno(errno),
                "could not create %s", dir);
    return FALSE;
  }
  g_autofree gchar* exec = nullptr;
  if (in_flatpak()) {
    exec = g_strdup("flatpak run --command=doki " APPLICATION_ID " --hidden");
  } else {
    g_autofree gchar* self = g_file_read_link("/proc/self/exe", error);
    if (self == nullptr) {
      return FALSE;
    }
    g_autofree gchar* quoted = quote_exec(self);
    exec = g_strdup_printf("%s --hidden", quoted);
  }
  g_autofree gchar* contents = g_strdup_printf(
      "[Desktop Entry]\n"
      "Type=Application\n"
      "Name=Doki\n"
      "Comment=Keeps saving checkpoints and syncing in the tray\n"
      "Exec=%s\n"
      "Icon=" APPLICATION_ID "\n"
      "%s"
      "X-GNOME-Autostart-enabled=true\n",
      exec, in_flatpak() ? "X-Flatpak=" APPLICATION_ID "\n" : "");
  return g_file_set_contents(path, contents, -1, error);
}

static void finish(Pending* pending, gboolean ok, const char* message) {
  if (pending->response_id != 0) {
    g_dbus_connection_signal_unsubscribe(pending->bus, pending->response_id);
  }
  pending->callback(ok, message, pending->user_data);
  g_clear_object(&pending->bus);
  g_free(pending->handle);
  g_free(pending);
}

static void finish_with_file(Pending* pending) {
  g_autoptr(GError) error = nullptr;
  if (write_entry(pending->enabled, &error)) {
    finish(pending, TRUE, nullptr);
  } else {
    finish(pending, FALSE, error->message);
  }
}

static void response_cb(GDBusConnection*, const gchar*, const gchar*,
                        const gchar*, const gchar*, GVariant* parameters,
                        gpointer user_data) {
  Pending* pending = static_cast<Pending*>(user_data);
  guint32 response;
  g_autoptr(GVariant) results = nullptr;
  g_variant_get(parameters, "(u@a{sv})", &response, &results);
  if (response == kPortalCancelled) {
    finish(pending, FALSE, "Your desktop did not allow Doki to start on its own");
    return;
  }
  gboolean autostart = FALSE;
  if (response == 0 &&
      g_variant_lookup(results, "autostart", "b", &autostart) &&
      autostart == pending->enabled) {
    finish(pending, TRUE, nullptr);
    return;
  }
  // The portal answered but did not do it (an older or partial portal):
  // write the entry ourselves.
  finish_with_file(pending);
}

static guint subscribe(Pending* pending, const gchar* handle) {
  g_free(pending->handle);
  pending->handle = g_strdup(handle);
  return g_dbus_connection_signal_subscribe(
      pending->bus, kPortalName, kRequest, "Response", handle, nullptr,
      G_DBUS_SIGNAL_FLAGS_NO_MATCH_RULE, response_cb, pending, nullptr);
}

static void request_cb(GObject* source, GAsyncResult* result,
                       gpointer user_data) {
  Pending* pending = static_cast<Pending*>(user_data);
  g_autoptr(GError) error = nullptr;
  g_autoptr(GVariant) reply = g_dbus_connection_call_finish(
      G_DBUS_CONNECTION(source), result, &error);
  if (reply == nullptr) {
    // No Background portal on this desktop.
    finish_with_file(pending);
    return;
  }
  // The answer arrives as the Request's Response signal. Portals older than
  // handle_token pick their own path; listen there instead.
  const gchar* handle;
  g_variant_get(reply, "(&o)", &handle);
  if (g_strcmp0(handle, pending->handle) != 0) {
    g_dbus_connection_signal_unsubscribe(pending->bus, pending->response_id);
    pending->response_id = subscribe(pending, handle);
  }
}

void doki_autostart_set(gboolean enabled,
                        DokiAutostartCallback callback,
                        gpointer user_data) {
  Pending* pending = g_new0(Pending, 1);
  pending->enabled = enabled;
  pending->callback = callback;
  pending->user_data = user_data;
  if (!in_flatpak()) {
    finish_with_file(pending);
    return;
  }
  g_autoptr(GError) error = nullptr;
  pending->bus = g_bus_get_sync(G_BUS_TYPE_SESSION, nullptr, &error);
  if (pending->bus == nullptr) {
    finish_with_file(pending);
    return;
  }

  // Subscribe to the reply before asking, at the path the portal will use,
  // so a fast answer is not missed.
  g_autofree gchar* token =
      g_strdup_printf("doki%u", g_random_int_range(0, G_MAXINT32));
  g_autofree gchar* sender =
      g_strdup(g_dbus_connection_get_unique_name(pending->bus) + 1);
  g_strdelimit(sender, ".", '_');
  g_autofree gchar* handle = g_strdup_printf(
      "/org/freedesktop/portal/desktop/request/%s/%s", sender, token);
  pending->response_id = subscribe(pending, handle);

  GVariantBuilder options;
  g_variant_builder_init(&options, G_VARIANT_TYPE_VARDICT);
  g_variant_builder_add(&options, "{sv}", "handle_token",
                        g_variant_new_string(token));
  g_variant_builder_add(
      &options, "{sv}", "reason",
      g_variant_new_string("Doki keeps saving checkpoints and syncing in the "
                           "tray after you sign in."));
  g_variant_builder_add(&options, "{sv}", "autostart",
                        g_variant_new_boolean(enabled));
  const gchar* commandline[] = {"doki", "--hidden", nullptr};
  g_variant_builder_add(&options, "{sv}", "commandline",
                        g_variant_new_strv(commandline, -1));
  g_dbus_connection_call(
      pending->bus, kPortalName, kPortalPath, kBackground, "RequestBackground",
      g_variant_new("(sa{sv})", "", &options), G_VARIANT_TYPE("(o)"),
      G_DBUS_CALL_FLAGS_NONE, -1, nullptr, request_cb, pending);
}
