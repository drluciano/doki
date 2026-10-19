#ifndef RUNNER_AUTOSTART_H_
#define RUNNER_AUTOSTART_H_

#include <gio/gio.h>

// "Start Doki when you sign in" on Linux. Doki starts with --hidden, so it
// opens in the tray instead of on screen.
//
// Inside the Flatpak the Background portal is asked first: it writes the
// autostart entry and tells the desktop that Doki may keep running without a
// window. If the desktop has no Background portal, Doki writes the entry in
// ~/.config/autostart itself (the Flatpak can see the home folder for save
// folders anyway). Desktops start these entries in their normal session only;
// SteamOS Game Mode does not.

// Whether the autostart entry exists.
gboolean doki_autostart_enabled();

typedef void (*DokiAutostartCallback)(gboolean ok,
                                      const char* error,
                                      gpointer user_data);

// Turns starting at sign-in on or off and reports the outcome to [callback]
// (possibly after the user answers a prompt from the desktop).
void doki_autostart_set(gboolean enabled,
                        DokiAutostartCallback callback,
                        gpointer user_data);

#endif  // RUNNER_AUTOSTART_H_
