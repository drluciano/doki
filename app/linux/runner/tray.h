#ifndef RUNNER_TRAY_H_
#define RUNNER_TRAY_H_

#include <gio/gio.h>

// Doki's icon in the system tray, spoken directly over D-Bus as a
// StatusNotifierItem with a com.canonical.dbusmenu menu. KDE Plasma (SteamOS
// and Armada desktop mode) and most other desktops show these; plain GNOME
// needs the AppIndicator extension. No extra libraries are needed, which keeps
// the Flatpak to the GNOME runtime alone.

typedef enum {
  DOKI_TRAY_OPEN,
  DOKI_TRAY_SYNC,
  DOKI_TRAY_QUIT,
} DokiTrayAction;

typedef void (*DokiTrayCallback)(DokiTrayAction action, gpointer user_data);

typedef struct _DokiTray DokiTray;

// Starts showing the icon whenever a tray is running (one may start later or
// restart; the icon is registered again each time). [icon_path] is a PNG.
DokiTray* doki_tray_new(const char* icon_path,
                        DokiTrayCallback callback,
                        gpointer user_data);

// Whether a tray is showing the icon right now. When it is not, closing the
// window still keeps Doki running, and opening Doki again brings it back.
gboolean doki_tray_is_shown(DokiTray* tray);

void doki_tray_free(DokiTray* tray);

#endif  // RUNNER_TRAY_H_
