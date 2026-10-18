#include "flutter_window.h"

#include <flutter/standard_method_codec.h>

#include <optional>
#include <string>

#include "flutter/generated_plugin_registrant.h"
#include "resource.h"

namespace {

constexpr UINT kTrayCallbackMessage = WM_APP + 1;
constexpr UINT kTrayIconId = 1;
constexpr UINT kMenuOpen = 40001;
constexpr UINT kMenuSync = 40002;
constexpr UINT kMenuQuit = 40003;

constexpr const wchar_t kSettingsKey[] = L"Software\\Doki";
constexpr const wchar_t kKeepInTrayValue[] = L"KeepInTray";
constexpr const wchar_t kRunKey[] =
    L"Software\\Microsoft\\Windows\\CurrentVersion\\Run";
constexpr const wchar_t kRunValue[] = L"Doki";

// "Keep in tray" is on unless the user turned it off.
bool ReadKeepInTray() {
  DWORD value = 1;
  DWORD size = sizeof(value);
  if (RegGetValueW(HKEY_CURRENT_USER, kSettingsKey, kKeepInTrayValue,
                   RRF_RT_REG_DWORD, nullptr, &value, &size) != ERROR_SUCCESS) {
    return true;
  }
  return value != 0;
}

void WriteKeepInTray(bool enabled) {
  DWORD value = enabled ? 1 : 0;
  RegSetKeyValueW(HKEY_CURRENT_USER, kSettingsKey, kKeepInTrayValue, REG_DWORD,
                  &value, sizeof(value));
}

bool ReadStartWithWindows() {
  return RegGetValueW(HKEY_CURRENT_USER, kRunKey, kRunValue, RRF_RT_REG_SZ,
                      nullptr, nullptr, nullptr) == ERROR_SUCCESS;
}

// Starting with Windows runs this executable with --hidden, so Doki opens in
// the tray instead of on screen.
bool WriteStartWithWindows(bool enabled) {
  if (!enabled) {
    LSTATUS status = RegDeleteKeyValueW(HKEY_CURRENT_USER, kRunKey, kRunValue);
    return status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND;
  }
  wchar_t path[MAX_PATH];
  DWORD length = GetModuleFileNameW(nullptr, path, MAX_PATH);
  if (length == 0 || length == MAX_PATH) {
    return false;
  }
  std::wstring command = L"\"" + std::wstring(path, length) + L"\" --hidden";
  return RegSetKeyValueW(
             HKEY_CURRENT_USER, kRunKey, kRunValue, REG_SZ, command.c_str(),
             static_cast<DWORD>((command.size() + 1) * sizeof(wchar_t))) ==
         ERROR_SUCCESS;
}

std::optional<bool> EnabledArgument(const flutter::EncodableValue* arguments) {
  const auto* map = std::get_if<flutter::EncodableMap>(arguments);
  if (map == nullptr) {
    return std::nullopt;
  }
  auto it = map->find(flutter::EncodableValue("enabled"));
  if (it == map->end()) {
    return std::nullopt;
  }
  const bool* enabled = std::get_if<bool>(&it->second);
  if (enabled == nullptr) {
    return std::nullopt;
  }
  return *enabled;
}

}  // namespace

FlutterWindow::FlutterWindow(const flutter::DartProject& project,
                             bool start_hidden)
    : project_(project), start_hidden_(start_hidden) {}

FlutterWindow::~FlutterWindow() {}

bool FlutterWindow::OnCreate() {
  if (!Win32Window::OnCreate()) {
    return false;
  }

  RECT frame = GetClientArea();

  // The size here must match the window dimensions to avoid unnecessary surface
  // creation / destruction in the startup path.
  flutter_controller_ = std::make_unique<flutter::FlutterViewController>(
      frame.right - frame.left, frame.bottom - frame.top, project_);
  // Ensure that basic setup of the controller was successful.
  if (!flutter_controller_->engine() || !flutter_controller_->view()) {
    return false;
  }
  RegisterPlugins(flutter_controller_->engine());
  SetChildContent(flutter_controller_->view()->GetNativeWindow());

  channel_ = std::make_unique<flutter::MethodChannel<flutter::EncodableValue>>(
      flutter_controller_->engine()->messenger(), "dev.doki/window",
      &flutter::StandardMethodCodec::GetInstance());
  channel_->SetMethodCallHandler(
      [this](const flutter::MethodCall<flutter::EncodableValue>& call,
             std::unique_ptr<flutter::MethodResult<flutter::EncodableValue>>
                 result) {
        const std::string& method = call.method_name();
        if (method == "getKeepInTray") {
          result->Success(flutter::EncodableValue(keep_in_tray_));
        } else if (method == "getStartWithWindows") {
          result->Success(flutter::EncodableValue(ReadStartWithWindows()));
        } else if (method == "setKeepInTray" ||
                   method == "setStartWithWindows") {
          std::optional<bool> enabled = EnabledArgument(call.arguments());
          if (!enabled) {
            result->Error("InvalidInput", "expected {enabled: bool}");
          } else if (method == "setKeepInTray") {
            SetKeepInTray(*enabled);
            result->Success();
          } else if (WriteStartWithWindows(*enabled)) {
            result->Success();
          } else {
            result->Error("Io", "Windows did not save the startup setting");
          }
        } else {
          result->NotImplemented();
        }
      });

  taskbar_created_message_ = RegisterWindowMessageW(L"TaskbarCreated");
  show_message_ = RegisterWindowMessageW(kShowDokiMessage);
  keep_in_tray_ = ReadKeepInTray();
  if (keep_in_tray_) {
    AddTrayIcon();
  }

  flutter_controller_->engine()->SetNextFrameCallback([&]() {
    // Started with Windows: stay in the tray until opened.
    if (!(start_hidden_ && keep_in_tray_)) {
      this->Show();
    }
  });

  // Flutter can complete the first frame before the "show window" callback is
  // registered. The following call ensures a frame is pending to ensure the
  // window is shown. It is a no-op if the first frame hasn't completed yet.
  flutter_controller_->ForceRedraw();

  return true;
}

void FlutterWindow::OnDestroy() {
  RemoveTrayIcon();
  channel_ = nullptr;
  if (flutter_controller_) {
    flutter_controller_ = nullptr;
  }

  Win32Window::OnDestroy();
}

void FlutterWindow::AddTrayIcon() {
  HINSTANCE instance = GetModuleHandle(nullptr);
  tray_icon_ = {};
  tray_icon_.cbSize = sizeof(tray_icon_);
  tray_icon_.hWnd = GetHandle();
  tray_icon_.uID = kTrayIconId;
  tray_icon_.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
  tray_icon_.uCallbackMessage = kTrayCallbackMessage;
  tray_icon_.hIcon = static_cast<HICON>(LoadImageW(
      instance, MAKEINTRESOURCEW(IDI_APP_ICON), IMAGE_ICON,
      GetSystemMetrics(SM_CXSMICON), GetSystemMetrics(SM_CYSMICON), 0));
  wcscpy_s(tray_icon_.szTip, L"Doki");
  tray_icon_added_ = Shell_NotifyIconW(NIM_ADD, &tray_icon_) != FALSE;
}

void FlutterWindow::RemoveTrayIcon() {
  if (tray_icon_added_) {
    Shell_NotifyIconW(NIM_DELETE, &tray_icon_);
    tray_icon_added_ = false;
  }
  if (tray_icon_.hIcon != nullptr) {
    DestroyIcon(tray_icon_.hIcon);
    tray_icon_.hIcon = nullptr;
  }
}

void FlutterWindow::SetKeepInTray(bool enabled) {
  WriteKeepInTray(enabled);
  keep_in_tray_ = enabled;
  if (enabled && !tray_icon_added_) {
    AddTrayIcon();
  } else if (!enabled) {
    RemoveTrayIcon();
    // Never leave the window hidden with no icon to bring it back.
    Open();
  }
}

void FlutterWindow::ShowTrayMenu() {
  HMENU menu = CreatePopupMenu();
  if (menu == nullptr) {
    return;
  }
  AppendMenuW(menu, MF_STRING, kMenuOpen, L"Open Doki");
  AppendMenuW(menu, MF_STRING, kMenuSync, L"Sync now");
  AppendMenuW(menu, MF_SEPARATOR, 0, nullptr);
  AppendMenuW(menu, MF_STRING, kMenuQuit, L"Quit");
  SetMenuDefaultItem(menu, kMenuOpen, FALSE);

  POINT cursor;
  GetCursorPos(&cursor);
  // Without this the menu does not close when the user clicks elsewhere.
  SetForegroundWindow(GetHandle());
  UINT choice =
      TrackPopupMenu(menu, TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
                     cursor.x, cursor.y, 0, GetHandle(), nullptr);
  DestroyMenu(menu);

  switch (choice) {
    case kMenuOpen:
      Open();
      break;
    case kMenuSync:
      if (channel_) {
        channel_->InvokeMethod("sync", nullptr);
      }
      break;
    case kMenuQuit:
      Quit();
      break;
  }
}

void FlutterWindow::Open() {
  HWND window = GetHandle();
  if (window == nullptr) {
    return;
  }
  ShowWindow(window, IsIconic(window) ? SW_RESTORE : SW_SHOW);
  SetForegroundWindow(window);
}

void FlutterWindow::HideToTray() {
  ShowWindow(GetHandle(), SW_HIDE);
  if (!told_about_tray_ && tray_icon_added_) {
    // Say once per run where Doki went, so closing does not look like quitting.
    told_about_tray_ = true;
    NOTIFYICONDATAW info = tray_icon_;
    info.uFlags = NIF_INFO;
    wcscpy_s(info.szInfoTitle, L"Doki is still running");
    wcscpy_s(info.szInfo,
             L"It keeps saving checkpoints and syncing from the tray. "
             L"Right-click the icon to quit.");
    info.dwInfoFlags = NIIF_INFO | NIIF_NOSOUND;
    Shell_NotifyIconW(NIM_MODIFY, &info);
  }
}

void FlutterWindow::Quit() {
  quitting_ = true;
  // Goes through WM_DESTROY, which removes the icon and quits (main.cpp sets
  // quit-on-close).
  DestroyWindow(GetHandle());
}

LRESULT
FlutterWindow::MessageHandler(HWND hwnd, UINT const message,
                              WPARAM const wparam,
                              LPARAM const lparam) noexcept {
  // Tray behaviour first, so closing and minimizing hide the window instead.
  if (message == kTrayCallbackMessage) {
    switch (LOWORD(lparam)) {
      case WM_LBUTTONUP:
      case WM_LBUTTONDBLCLK:
        Open();
        break;
      case WM_RBUTTONUP:
      case WM_CONTEXTMENU:
        ShowTrayMenu();
        break;
    }
    return 0;
  }
  if (message == show_message_ && show_message_ != 0) {
    Open();
    return 0;
  }
  if (message == taskbar_created_message_ && taskbar_created_message_ != 0) {
    // Explorer restarted and forgot every tray icon; add ours back.
    if (keep_in_tray_) {
      RemoveTrayIcon();
      AddTrayIcon();
    }
    return 0;
  }
  if (keep_in_tray_ && tray_icon_added_ && !quitting_) {
    if (message == WM_CLOSE) {
      HideToTray();
      return 0;
    }
    if (message == WM_SIZE && wparam == SIZE_MINIMIZED) {
      HideToTray();
    }
  }

  // Give Flutter, including plugins, an opportunity to handle window messages.
  if (flutter_controller_) {
    std::optional<LRESULT> result =
        flutter_controller_->HandleTopLevelWindowProc(hwnd, message, wparam,
                                                      lparam);
    if (result) {
      return *result;
    }
  }

  switch (message) {
    case WM_FONTCHANGE:
      flutter_controller_->engine()->ReloadSystemFonts();
      break;
  }

  return Win32Window::MessageHandler(hwnd, message, wparam, lparam);
}
