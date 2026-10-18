#ifndef RUNNER_FLUTTER_WINDOW_H_
#define RUNNER_FLUTTER_WINDOW_H_

#include <flutter/dart_project.h>
#include <flutter/encodable_value.h>
#include <flutter/flutter_view_controller.h>
#include <flutter/method_channel.h>
#include <windows.h>
// shellapi.h needs windows.h first.
#include <shellapi.h>

#include <memory>
#include <string>

#include "win32_window.h"

// Message another Doki launch posts to bring this window back from the tray.
// Registered by name in main.cpp and here.
constexpr const wchar_t kShowDokiMessage[] = L"DokiShowWindow";

// A window that hosts a Flutter view and, while "keep in tray" is on, a
// notification-area icon. Closing or minimizing then hides the window to the
// tray, so automatic checkpoints and sync keep running; Quit in the tray
// menu ends the app.
//
// Dart talks to it on the "dev.doki/window" channel:
//   getKeepInTray / setKeepInTray {enabled}     (stored in HKCU\Software\Doki)
//   getStartWithWindows / setStartWithWindows {enabled}   (HKCU Run key)
//   notify {key, title, body}  (a notification from the tray icon; false
//                               when the tray is off)
// and it calls "sync" on the same channel when Sync now is chosen.
class FlutterWindow : public Win32Window {
 public:
  // Creates a new FlutterWindow hosting a Flutter view running |project|.
  // With |start_hidden| (Doki started with Windows) the window stays in the
  // tray until opened, as long as "keep in tray" is on.
  FlutterWindow(const flutter::DartProject& project, bool start_hidden);
  virtual ~FlutterWindow();

 protected:
  // Win32Window:
  bool OnCreate() override;
  void OnDestroy() override;
  LRESULT MessageHandler(HWND window, UINT const message, WPARAM const wparam,
                         LPARAM const lparam) noexcept override;

 private:
  void AddTrayIcon();
  void RemoveTrayIcon();
  void SetKeepInTray(bool enabled);
  void ShowTrayMenu();
  void Open();
  void HideToTray();
  // Shows a notification from the tray icon. False when there is no icon.
  bool ShowBalloon(const std::wstring& title, const std::wstring& body,
                   bool quiet);
  void Quit();

  // The project to run.
  flutter::DartProject project_;
  bool start_hidden_;

  // The Flutter instance hosted by this window.
  std::unique_ptr<flutter::FlutterViewController> flutter_controller_;
  std::unique_ptr<flutter::MethodChannel<flutter::EncodableValue>> channel_;

  bool keep_in_tray_ = true;
  bool tray_icon_added_ = false;
  bool told_about_tray_ = false;
  bool quitting_ = false;
  NOTIFYICONDATAW tray_icon_ = {};
  UINT taskbar_created_message_ = 0;
  UINT show_message_ = 0;
};

#endif  // RUNNER_FLUTTER_WINDOW_H_
