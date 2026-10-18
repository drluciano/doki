#include <flutter/dart_project.h>
#include <flutter/flutter_view_controller.h>
#include <windows.h>

#include <cwchar>

#include "flutter_window.h"
#include "utils.h"

int APIENTRY wWinMain(_In_ HINSTANCE instance, _In_opt_ HINSTANCE prev,
                      _In_ wchar_t *command_line, _In_ int show_command) {
  // One Doki at a time: a second launch (for example from the Start menu
  // while Doki sits in the tray) brings the running one back instead.
  HANDLE single_instance =
      ::CreateMutexW(nullptr, TRUE, L"Local\\DokiSingleInstance");
  if (single_instance != nullptr && ::GetLastError() == ERROR_ALREADY_EXISTS) {
    HWND running = ::FindWindowW(L"FLUTTER_RUNNER_WIN32_WINDOW", L"doki");
    if (running != nullptr) {
      ::AllowSetForegroundWindow(ASFW_ANY);
      ::PostMessageW(running, ::RegisterWindowMessageW(kShowDokiMessage), 0, 0);
    }
    ::CloseHandle(single_instance);
    return EXIT_SUCCESS;
  }

  // Started with Windows (see FlutterWindow): open in the tray.
  bool start_hidden = command_line != nullptr &&
                      std::wcsstr(command_line, L"--hidden") != nullptr;

  // Attach to console when present (e.g., 'flutter run') or create a
  // new console when running with a debugger.
  if (!::AttachConsole(ATTACH_PARENT_PROCESS) && ::IsDebuggerPresent()) {
    CreateAndAttachConsole();
  }

  // Initialize COM, so that it is available for use in the library and/or
  // plugins.
  ::CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);

  flutter::DartProject project(L"data");

  std::vector<std::string> command_line_arguments = GetCommandLineArguments();

  project.set_dart_entrypoint_arguments(std::move(command_line_arguments));

  FlutterWindow window(project, start_hidden);
  Win32Window::Point origin(10, 10);
  Win32Window::Size size(1280, 720);
  if (!window.Create(L"doki", origin, size)) {
    return EXIT_FAILURE;
  }
  window.SetQuitOnClose(true);

  ::MSG msg;
  while (::GetMessage(&msg, nullptr, 0, 0)) {
    ::TranslateMessage(&msg);
    ::DispatchMessage(&msg);
  }

  ::CoUninitialize();
  if (single_instance != nullptr) {
    ::CloseHandle(single_instance);
  }
  return EXIT_SUCCESS;
}
