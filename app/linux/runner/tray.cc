#include "tray.h"

#include <gdk-pixbuf/gdk-pixbuf.h>

namespace {

constexpr char kItemPath[] = "/StatusNotifierItem";
constexpr char kMenuPath[] = "/StatusNotifierMenu";
constexpr char kWatcherName[] = "org.kde.StatusNotifierWatcher";
constexpr char kWatcherPath[] = "/StatusNotifierWatcher";

constexpr char kItemXml[] =
    "<node>"
    " <interface name='org.kde.StatusNotifierItem'>"
    "  <property name='Category' type='s' access='read'/>"
    "  <property name='Id' type='s' access='read'/>"
    "  <property name='Title' type='s' access='read'/>"
    "  <property name='Status' type='s' access='read'/>"
    "  <property name='WindowId' type='i' access='read'/>"
    "  <property name='IconName' type='s' access='read'/>"
    "  <property name='IconPixmap' type='a(iiay)' access='read'/>"
    "  <property name='ToolTip' type='(sa(iiay)ss)' access='read'/>"
    "  <property name='ItemIsMenu' type='b' access='read'/>"
    "  <property name='Menu' type='o' access='read'/>"
    "  <method name='ContextMenu'>"
    "   <arg name='x' type='i' direction='in'/>"
    "   <arg name='y' type='i' direction='in'/>"
    "  </method>"
    "  <method name='Activate'>"
    "   <arg name='x' type='i' direction='in'/>"
    "   <arg name='y' type='i' direction='in'/>"
    "  </method>"
    "  <method name='SecondaryActivate'>"
    "   <arg name='x' type='i' direction='in'/>"
    "   <arg name='y' type='i' direction='in'/>"
    "  </method>"
    "  <method name='Scroll'>"
    "   <arg name='delta' type='i' direction='in'/>"
    "   <arg name='orientation' type='s' direction='in'/>"
    "  </method>"
    "  <signal name='NewIcon'/>"
    "  <signal name='NewToolTip'/>"
    "  <signal name='NewStatus'><arg name='status' type='s'/></signal>"
    " </interface>"
    "</node>";

constexpr char kMenuXml[] =
    "<node>"
    " <interface name='com.canonical.dbusmenu'>"
    "  <property name='Version' type='u' access='read'/>"
    "  <property name='TextDirection' type='s' access='read'/>"
    "  <property name='Status' type='s' access='read'/>"
    "  <property name='IconThemePath' type='as' access='read'/>"
    "  <method name='GetLayout'>"
    "   <arg name='parentId' type='i' direction='in'/>"
    "   <arg name='recursionDepth' type='i' direction='in'/>"
    "   <arg name='propertyNames' type='as' direction='in'/>"
    "   <arg name='revision' type='u' direction='out'/>"
    "   <arg name='layout' type='(ia{sv}av)' direction='out'/>"
    "  </method>"
    "  <method name='GetGroupProperties'>"
    "   <arg name='ids' type='ai' direction='in'/>"
    "   <arg name='propertyNames' type='as' direction='in'/>"
    "   <arg name='properties' type='a(ia{sv})' direction='out'/>"
    "  </method>"
    "  <method name='GetProperty'>"
    "   <arg name='id' type='i' direction='in'/>"
    "   <arg name='name' type='s' direction='in'/>"
    "   <arg name='value' type='v' direction='out'/>"
    "  </method>"
    "  <method name='Event'>"
    "   <arg name='id' type='i' direction='in'/>"
    "   <arg name='eventId' type='s' direction='in'/>"
    "   <arg name='data' type='v' direction='in'/>"
    "   <arg name='timestamp' type='u' direction='in'/>"
    "  </method>"
    "  <method name='EventGroup'>"
    "   <arg name='events' type='a(isvu)' direction='in'/>"
    "   <arg name='idErrors' type='ai' direction='out'/>"
    "  </method>"
    "  <method name='AboutToShow'>"
    "   <arg name='id' type='i' direction='in'/>"
    "   <arg name='needUpdate' type='b' direction='out'/>"
    "  </method>"
    "  <method name='AboutToShowGroup'>"
    "   <arg name='ids' type='ai' direction='in'/>"
    "   <arg name='updatesNeeded' type='ai' direction='out'/>"
    "   <arg name='idErrors' type='ai' direction='out'/>"
    "  </method>"
    "  <signal name='LayoutUpdated'>"
    "   <arg name='revision' type='u'/>"
    "   <arg name='parent' type='i'/>"
    "  </signal>"
    " </interface>"
    "</node>";

// Menu items, in order. Id 0 is the menu itself.
struct MenuItem {
  gint32 id;
  const char* label;  // nullptr for a separator
  DokiTrayAction action;
};

constexpr MenuItem kMenu[] = {
    {1, "Open Doki", DOKI_TRAY_OPEN},
    {2, "Sync now", DOKI_TRAY_SYNC},
    {3, nullptr, DOKI_TRAY_OPEN},
    {4, "Quit", DOKI_TRAY_QUIT},
};

}  // namespace

struct _DokiTray {
  GDBusConnection* bus;
  guint item_id;
  guint menu_id;
  guint watch_id;
  gboolean shown;
  GVariant* pixmap;  // a(iiay), floating reference sunk at creation
  DokiTrayCallback callback;
  gpointer user_data;
};

static GVariant* load_pixmap(const char* path) {
  GVariantBuilder icons;
  g_variant_builder_init(&icons, G_VARIANT_TYPE("a(iiay)"));
  g_autoptr(GError) error = nullptr;
  g_autoptr(GdkPixbuf) loaded = gdk_pixbuf_new_from_file(path, &error);
  if (loaded == nullptr) {
    g_warning("Doki tray icon not loaded: %s", error->message);
    return g_variant_ref_sink(g_variant_builder_end(&icons));
  }
  g_autoptr(GdkPixbuf) pixbuf = gdk_pixbuf_add_alpha(loaded, FALSE, 0, 0, 0);
  const int width = gdk_pixbuf_get_width(pixbuf);
  const int height = gdk_pixbuf_get_height(pixbuf);
  const int stride = gdk_pixbuf_get_rowstride(pixbuf);
  const guchar* pixels = gdk_pixbuf_read_pixels(pixbuf);
  // The tray wants ARGB32 in network byte order; GdkPixbuf holds RGBA.
  GByteArray* argb = g_byte_array_sized_new(width * height * 4);
  for (int y = 0; y < height; y++) {
    const guchar* row = pixels + y * stride;
    for (int x = 0; x < width; x++) {
      const guchar* p = row + x * 4;
      const guchar pixel[4] = {p[3], p[0], p[1], p[2]};
      g_byte_array_append(argb, pixel, 4);
    }
  }
  g_autoptr(GBytes) bytes = g_byte_array_free_to_bytes(argb);
  g_variant_builder_add(
      &icons, "(ii@ay)", width, height,
      g_variant_new_from_bytes(G_VARIANT_TYPE("ay"), bytes, TRUE));
  return g_variant_ref_sink(g_variant_builder_end(&icons));
}

static GVariant* item_get_property(GDBusConnection*, const gchar*, const gchar*,
                                   const gchar*, const gchar* name, GError**,
                                   gpointer user_data) {
  DokiTray* tray = static_cast<DokiTray*>(user_data);
  if (g_strcmp0(name, "Category") == 0) {
    return g_variant_new_string("ApplicationStatus");
  }
  if (g_strcmp0(name, "Id") == 0) {
    return g_variant_new_string(APPLICATION_ID);
  }
  if (g_strcmp0(name, "Title") == 0) {
    return g_variant_new_string("Doki");
  }
  if (g_strcmp0(name, "Status") == 0) {
    return g_variant_new_string("Active");
  }
  if (g_strcmp0(name, "WindowId") == 0) {
    return g_variant_new_int32(0);
  }
  if (g_strcmp0(name, "IconName") == 0) {
    // Empty, so the tray uses IconPixmap: the icon theme name is not
    // exported when Doki runs outside the Flatpak.
    return g_variant_new_string("");
  }
  if (g_strcmp0(name, "IconPixmap") == 0) {
    return g_variant_ref(tray->pixmap);
  }
  if (g_strcmp0(name, "ToolTip") == 0) {
    return g_variant_new("(s@a(iiay)ss)", "", tray->pixmap, "Doki",
                         "Saving checkpoints and syncing");
  }
  if (g_strcmp0(name, "ItemIsMenu") == 0) {
    return g_variant_new_boolean(FALSE);
  }
  if (g_strcmp0(name, "Menu") == 0) {
    return g_variant_new_object_path(kMenuPath);
  }
  return nullptr;
}

static void item_method_call(GDBusConnection*, const gchar*, const gchar*,
                             const gchar*, const gchar* method, GVariant*,
                             GDBusMethodInvocation* invocation,
                             gpointer user_data) {
  DokiTray* tray = static_cast<DokiTray*>(user_data);
  g_dbus_method_invocation_return_value(invocation, nullptr);
  // A click opens Doki, like the Windows tray icon. The menu is shown by the
  // tray itself from the Menu object, so ContextMenu has nothing to do.
  if (g_strcmp0(method, "Activate") == 0 ||
      g_strcmp0(method, "SecondaryActivate") == 0) {
    tray->callback(DOKI_TRAY_OPEN, tray->user_data);
  }
}

static GVariant* menu_item_properties(const MenuItem* item) {
  GVariantBuilder props;
  g_variant_builder_init(&props, G_VARIANT_TYPE("a{sv}"));
  if (item == nullptr) {
    g_variant_builder_add(&props, "{sv}", "children-display",
                          g_variant_new_string("submenu"));
  } else if (item->label == nullptr) {
    g_variant_builder_add(&props, "{sv}", "type",
                          g_variant_new_string("separator"));
  } else {
    g_variant_builder_add(&props, "{sv}", "label",
                          g_variant_new_string(item->label));
  }
  return g_variant_builder_end(&props);
}

static const MenuItem* find_item(gint32 id) {
  for (const MenuItem& item : kMenu) {
    if (item.id == id) {
      return &item;
    }
  }
  return nullptr;
}

static GVariant* menu_layout(gint32 parent) {
  GVariantBuilder children;
  g_variant_builder_init(&children, G_VARIANT_TYPE("av"));
  if (parent == 0) {
    for (const MenuItem& item : kMenu) {
      g_variant_builder_add(
          &children, "v",
          g_variant_new("(i@a{sv}@av)", item.id, menu_item_properties(&item),
                        g_variant_new_array(G_VARIANT_TYPE_VARIANT, nullptr,
                                            0)));
    }
  }
  return g_variant_new("(i@a{sv}@av)", parent,
                       menu_item_properties(parent == 0 ? nullptr
                                                        : find_item(parent)),
                       g_variant_builder_end(&children));
}

static void menu_event(DokiTray* tray, gint32 id, const gchar* event) {
  const MenuItem* item = find_item(id);
  if (item != nullptr && item->label != nullptr &&
      g_strcmp0(event, "clicked") == 0) {
    tray->callback(item->action, tray->user_data);
  }
}

static void menu_method_call(GDBusConnection*, const gchar*, const gchar*,
                             const gchar*, const gchar* method,
                             GVariant* parameters,
                             GDBusMethodInvocation* invocation,
                             gpointer user_data) {
  DokiTray* tray = static_cast<DokiTray*>(user_data);
  if (g_strcmp0(method, "GetLayout") == 0) {
    gint32 parent;
    g_variant_get(parameters, "(ii@as)", &parent, nullptr, nullptr);
    g_dbus_method_invocation_return_value(
        invocation, g_variant_new("(u@(ia{sv}av))", 1u, menu_layout(parent)));
  } else if (g_strcmp0(method, "GetGroupProperties") == 0) {
    g_autoptr(GVariantIter) ids = nullptr;
    g_variant_get(parameters, "(ai@as)", &ids, nullptr);
    GVariantBuilder out;
    g_variant_builder_init(&out, G_VARIANT_TYPE("a(ia{sv})"));
    gint32 id;
    while (g_variant_iter_next(ids, "i", &id)) {
      const MenuItem* item = find_item(id);
      if (id == 0 || item != nullptr) {
        g_variant_builder_add(&out, "(i@a{sv})", id,
                              menu_item_properties(item));
      }
    }
    g_dbus_method_invocation_return_value(
        invocation, g_variant_new("(@a(ia{sv}))", g_variant_builder_end(&out)));
  } else if (g_strcmp0(method, "GetProperty") == 0) {
    gint32 id;
    const gchar* name;
    g_variant_get(parameters, "(i&s)", &id, &name);
    g_autoptr(GVariant) props =
        g_variant_ref_sink(menu_item_properties(find_item(id)));
    GVariant* value = g_variant_lookup_value(props, name, nullptr);
    if (value == nullptr) {
      g_dbus_method_invocation_return_dbus_error(
          invocation, "com.canonical.dbusmenu.Error", "No such property");
    } else {
      g_dbus_method_invocation_return_value(invocation,
                                            g_variant_new("(v)", value));
      g_variant_unref(value);
    }
  } else if (g_strcmp0(method, "Event") == 0) {
    gint32 id;
    const gchar* event;
    g_variant_get(parameters, "(i&svu)", &id, &event, nullptr, nullptr);
    g_dbus_method_invocation_return_value(invocation, nullptr);
    menu_event(tray, id, event);
  } else if (g_strcmp0(method, "EventGroup") == 0) {
    g_autoptr(GVariantIter) events = nullptr;
    g_variant_get(parameters, "(a(isvu))", &events);
    g_dbus_method_invocation_return_value(
        invocation,
        g_variant_new("(@ai)",
                      g_variant_new_array(G_VARIANT_TYPE_INT32, nullptr, 0)));
    gint32 id;
    const gchar* event;
    while (g_variant_iter_next(events, "(i&svu)", &id, &event, nullptr,
                               nullptr)) {
      menu_event(tray, id, event);
    }
  } else if (g_strcmp0(method, "AboutToShow") == 0) {
    g_dbus_method_invocation_return_value(invocation,
                                          g_variant_new("(b)", FALSE));
  } else if (g_strcmp0(method, "AboutToShowGroup") == 0) {
    GVariant* none = g_variant_new_array(G_VARIANT_TYPE_INT32, nullptr, 0);
    g_dbus_method_invocation_return_value(
        invocation,
        g_variant_new("(@ai@ai)", none,
                      g_variant_new_array(G_VARIANT_TYPE_INT32, nullptr, 0)));
  } else {
    g_dbus_method_invocation_return_dbus_error(
        invocation, "org.freedesktop.DBus.Error.UnknownMethod", method);
  }
}

static GVariant* menu_get_property(GDBusConnection*, const gchar*, const gchar*,
                                   const gchar*, const gchar* name, GError**,
                                   gpointer) {
  if (g_strcmp0(name, "Version") == 0) {
    return g_variant_new_uint32(3);
  }
  if (g_strcmp0(name, "TextDirection") == 0) {
    return g_variant_new_string("ltr");
  }
  if (g_strcmp0(name, "Status") == 0) {
    return g_variant_new_string("normal");
  }
  if (g_strcmp0(name, "IconThemePath") == 0) {
    return g_variant_new_strv(nullptr, 0);
  }
  return nullptr;
}

static void registered_cb(GObject* source, GAsyncResult* result,
                          gpointer user_data) {
  DokiTray* tray = static_cast<DokiTray*>(user_data);
  g_autoptr(GError) error = nullptr;
  g_autoptr(GVariant) reply = g_dbus_connection_call_finish(
      G_DBUS_CONNECTION(source), result, &error);
  tray->shown = reply != nullptr;
  if (reply == nullptr) {
    g_warning("Doki tray icon not registered: %s", error->message);
  }
}

// The tray started (or restarted, which forgets every icon): register again.
// Registering by object path lets the tray use our unique bus name, so the
// Flatpak needs no well-known name of its own for the icon.
static void watcher_appeared(GDBusConnection* bus, const gchar*, const gchar*,
                             gpointer user_data) {
  g_dbus_connection_call(bus, kWatcherName, kWatcherPath, kWatcherName,
                         "RegisterStatusNotifierItem",
                         g_variant_new("(s)", kItemPath), nullptr,
                         G_DBUS_CALL_FLAGS_NONE, -1, nullptr, registered_cb,
                         user_data);
}

static void watcher_vanished(GDBusConnection*, const gchar*,
                             gpointer user_data) {
  static_cast<DokiTray*>(user_data)->shown = FALSE;
}

DokiTray* doki_tray_new(const char* icon_path,
                        DokiTrayCallback callback,
                        gpointer user_data) {
  g_autoptr(GError) error = nullptr;
  GDBusConnection* bus = g_bus_get_sync(G_BUS_TYPE_SESSION, nullptr, &error);
  if (bus == nullptr) {
    g_warning("Doki tray unavailable: %s", error->message);
    return nullptr;
  }
  DokiTray* tray = g_new0(DokiTray, 1);
  tray->bus = bus;
  tray->callback = callback;
  tray->user_data = user_data;
  tray->pixmap = load_pixmap(icon_path);

  static const GDBusInterfaceVTable item_vtable = {item_method_call,
                                                   item_get_property, nullptr};
  static const GDBusInterfaceVTable menu_vtable = {menu_method_call,
                                                   menu_get_property, nullptr};
  g_autoptr(GDBusNodeInfo) item_info =
      g_dbus_node_info_new_for_xml(kItemXml, nullptr);
  g_autoptr(GDBusNodeInfo) menu_info =
      g_dbus_node_info_new_for_xml(kMenuXml, nullptr);
  tray->item_id = g_dbus_connection_register_object(
      bus, kItemPath, item_info->interfaces[0], &item_vtable, tray, nullptr,
      &error);
  if (tray->item_id == 0) {
    g_warning("Doki tray icon not exported: %s", error->message);
    doki_tray_free(tray);
    return nullptr;
  }
  tray->menu_id = g_dbus_connection_register_object(
      bus, kMenuPath, menu_info->interfaces[0], &menu_vtable, tray, nullptr,
      &error);
  if (tray->menu_id == 0) {
    g_warning("Doki tray menu not exported: %s", error->message);
    doki_tray_free(tray);
    return nullptr;
  }
  tray->watch_id = g_bus_watch_name_on_connection(
      bus, kWatcherName, G_BUS_NAME_WATCHER_FLAGS_NONE, watcher_appeared,
      watcher_vanished, tray, nullptr);
  return tray;
}

gboolean doki_tray_is_shown(DokiTray* tray) {
  return tray != nullptr && tray->shown;
}

void doki_tray_free(DokiTray* tray) {
  if (tray == nullptr) {
    return;
  }
  if (tray->watch_id != 0) {
    g_bus_unwatch_name(tray->watch_id);
  }
  if (tray->item_id != 0) {
    g_dbus_connection_unregister_object(tray->bus, tray->item_id);
  }
  if (tray->menu_id != 0) {
    g_dbus_connection_unregister_object(tray->bus, tray->menu_id);
  }
  g_clear_pointer(&tray->pixmap, g_variant_unref);
  g_clear_object(&tray->bus);
  g_free(tray);
}
