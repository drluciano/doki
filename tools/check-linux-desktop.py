#!/usr/bin/env python3
"""Checks the Linux app's tray and single-instance behaviour.

Runs the built bundle on a private X display and session bus, playing the
tray (org.kde.StatusNotifierWatcher) and the notification server itself:

    tools/check-linux-desktop.sh app/build/linux/x64/release/bundle/doki

Checked: the tray icon registers with an icon and the Open / Sync now / Quit
menu; closing the window hides Doki to the tray and says so once; opening Doki
again brings the running one back; a start with --hidden stays in the tray
while one shows the icon and opens the window when none does; Quit in the menu
ends Doki. Needs python3-gi and python3-xlib (run through the .sh wrapper,
which starts Xvfb and the bus).
"""

import os
import shutil
import subprocess
import sys
import tempfile
import time

from gi.repository import Gio, GLib
from Xlib import X, display
from Xlib.protocol import event

WATCHER = "org.kde.StatusNotifierWatcher"
WATCHER_XML = """
<node><interface name='org.kde.StatusNotifierWatcher'>
  <method name='RegisterStatusNotifierItem'><arg type='s' direction='in'/></method>
</interface></node>"""
NOTIFY_XML = """
<node><interface name='org.freedesktop.Notifications'>
  <method name='Notify'>
    <arg type='s' direction='in'/><arg type='u' direction='in'/>
    <arg type='s' direction='in'/><arg type='s' direction='in'/>
    <arg type='s' direction='in'/><arg type='as' direction='in'/>
    <arg type='a{sv}' direction='in'/><arg type='i' direction='in'/>
    <arg type='u' direction='out'/>
  </method>
  <method name='GetCapabilities'><arg type='as' direction='out'/></method>
  <method name='GetServerInformation'>
    <arg type='s' direction='out'/><arg type='s' direction='out'/>
    <arg type='s' direction='out'/><arg type='s' direction='out'/>
  </method>
</interface></node>"""

bus = Gio.bus_get_sync(Gio.BusType.SESSION)
items = []  # (sender, path) registered with the tray
notifications = []  # (summary, body)
loop = GLib.MainContext.default()


def watcher_call(conn, sender, path, iface, method, params, invocation):
    items.append((sender, params.unpack()[0]))
    invocation.return_value(None)


def notify_call(conn, sender, path, iface, method, params, invocation):
    if method == "Notify":
        args = params.unpack()
        notifications.append((args[3], args[4]))
        invocation.return_value(GLib.Variant("(u)", (len(notifications),)))
    elif method == "GetCapabilities":
        invocation.return_value(GLib.Variant("(as)", (["body", "actions"],)))
    else:
        invocation.return_value(GLib.Variant("(ssss)", ("check", "doki", "1", "1.2")))


def serve(name, xml, handler):
    info = Gio.DBusNodeInfo.new_for_xml(xml)
    path = "/" + name.replace(".", "/") if "Notifications" in name else "/StatusNotifierWatcher"
    reg = bus.register_object(path, info.interfaces[0], handler, None, None)
    owned = []
    owner = Gio.bus_own_name_on_connection(
        bus, name, Gio.BusNameOwnerFlags.NONE, lambda *a: owned.append(1), None
    )
    wait(lambda: owned, f"owning {name}")
    return reg, owner


def wait(condition, what, timeout=90):
    end = time.monotonic() + timeout
    while time.monotonic() < end:
        while loop.iteration(False):
            pass
        if condition():
            return
        time.sleep(0.05)
    sys.exit(f"FAIL: timed out waiting for {what}")


def check(ok, what):
    if not ok:
        sys.exit(f"FAIL: {what}")
    print(f"ok - {what}")


x = display.Display()


def doki_windows():
    found = []

    def walk(win):
        try:
            cls = win.get_wm_class()
            if cls and "dev.doki.doki" in cls:
                found.append(win)
            for child in win.query_tree().children:
                walk(child)
        except Exception:
            pass

    walk(x.screen().root)
    return found


def window_shown():
    return any(w.get_attributes().map_state == X.IsViewable for w in doki_windows())


def close_window():
    # What a window manager's close button sends.
    delete = x.intern_atom("WM_DELETE_WINDOW")
    protocols = x.intern_atom("WM_PROTOCOLS")
    for w in doki_windows():
        if w.get_attributes().map_state == X.IsViewable:
            msg = event.ClientMessage(
                window=w, client_type=protocols, data=(32, [delete, X.CurrentTime, 0, 0, 0])
            )
            w.send_event(msg)
    x.flush()


def menu_call(sender, method, params, reply):
    return bus.call_sync(
        sender, "/StatusNotifierMenu", "com.canonical.dbusmenu", method, params,
        GLib.VariantType(reply), Gio.DBusCallFlags.NONE, 5000, None,
    ).unpack()


def main():
    exe = os.path.abspath(sys.argv[1])
    home = tempfile.mkdtemp(prefix="doki-home-")
    env = dict(os.environ, HOME=home, XDG_CONFIG_HOME=f"{home}/.config",
               XDG_DATA_HOME=f"{home}/.local/share", GDK_BACKEND="x11")

    def launch(*args):
        return subprocess.Popen([exe, *args], env=env)

    def launch_again(*args):
        code = subprocess.run([exe, *args], env=env, timeout=30).returncode
        check(code == 0, f"a second launch {' '.join(args)} hands over and exits")

    serve("org.freedesktop.Notifications", NOTIFY_XML, notify_call)

    # 1. No tray yet: a start with --hidden opens the window after a while.
    doki = launch("--hidden")
    wait(window_shown, "the window to open with no tray to hide in", timeout=60)
    check(True, "a --hidden start with no tray opens the window")
    close_window()
    wait(lambda: doki.poll() is not None, "Doki to quit on close with no tray")
    check(doki.returncode == 0, "closing with no tray quits")

    # 2. With a tray.
    serve(WATCHER, WATCHER_XML, watcher_call)
    doki = launch()
    wait(lambda: items, "the tray icon to register")
    sender, path = items[-1]
    check(path == "/StatusNotifierItem", "the icon registers by object path")
    props = bus.call_sync(
        sender, path, "org.freedesktop.DBus.Properties", "GetAll",
        GLib.Variant("(s)", ("org.kde.StatusNotifierItem",)),
        GLib.VariantType("(a{sv})"), Gio.DBusCallFlags.NONE, 5000, None,
    ).unpack()[0]
    pixmaps = props["IconPixmap"]
    check(pixmaps and pixmaps[0][0] == 64 and len(pixmaps[0][2]) == 64 * 64 * 4,
          "the icon carries a 64 px picture")
    check(props["Menu"] == "/StatusNotifierMenu", "the icon names its menu")
    _, layout = menu_call(sender, "GetLayout", GLib.Variant("(iias)", (0, -1, [])), "(u(ia{sv}av))")
    labels = [child[1].get("label") for child in layout[2]]
    check(labels == ["Open Doki", "Sync now", None, "Quit"], f"the menu reads {labels}")

    wait(window_shown, "the window to open")
    close_window()
    wait(lambda: not window_shown(), "closing to hide the window")
    time.sleep(1)
    check(doki.poll() is None, "closing hides Doki in the tray instead of quitting")
    wait(lambda: notifications, "the still-running notification")
    check(notifications[0][0] == "Doki is still running", "closing says Doki is still running")

    launch_again("--hidden")
    time.sleep(2)
    check(not window_shown(), "a start at sign-in leaves the running Doki in the tray")
    launch_again()
    wait(window_shown, "a second launch to bring the window back")
    check(True, "opening Doki again brings back the running one")

    close_window()
    wait(lambda: not window_shown(), "closing to hide the window again")
    time.sleep(1)
    check(len(notifications) == 1, "the still-running notification is shown once per run")
    menu_call(sender, "Event", GLib.Variant("(isvu)", (1, "clicked", GLib.Variant("s", ""), 0)), "()")
    wait(window_shown, "Open Doki to show the window")
    check(True, "Open Doki in the menu shows the window")

    menu_call(sender, "Event", GLib.Variant("(isvu)", (4, "clicked", GLib.Variant("s", ""), 0)), "()")
    wait(lambda: doki.poll() is not None, "Quit to end Doki")
    check(doki.returncode == 0, "Quit in the menu ends Doki")

    # 3. A start at sign-in with a tray stays hidden.
    items.clear()
    doki = launch("--hidden")
    wait(lambda: items, "the tray icon to register on a hidden start")
    time.sleep(15)
    check(not window_shown(), "a --hidden start with a tray stays in the tray")
    sender, _ = items[-1]
    menu_call(sender, "Event", GLib.Variant("(isvu)", (4, "clicked", GLib.Variant("s", ""), 0)), "()")
    wait(lambda: doki.poll() is not None, "Quit to end the hidden Doki")
    shutil.rmtree(home, ignore_errors=True)
    print("all checks passed")


if __name__ == "__main__":
    main()
