; Doki's Windows installer (Inno Setup 6.3 or newer).
;
; Builds one setup .exe from the Flutter release build:
;   flutter build windows --release            (in app/)
;   iscc /DAppVersion=0.1.0 app\windows\installer\doki.iss
; CI does both (.github/workflows/app.yml) and uploads doki-windows-setup.
;
; What it does:
; - Installs for the current user only, in %LOCALAPPDATA%\Programs\Doki, so
;   it never asks for administrator rights.
; - Adds Doki to the Start menu, and optionally to the desktop and to
;   "start when I sign in" (the same HKCU Run value the app's Settings switch
;   writes, so the two always agree).
; - Installing a newer version over an older one quits Doki first, replaces
;   the program files and keeps everything else.
; - Uninstalling removes the program files, shortcuts and the sign-in entry
;   only. Doki's data folder (%APPDATA%\dev.doki\doki: checkpoints, paired
;   devices, settings) and every game save folder are left exactly as they
;   are, so nothing the user would miss can be lost by uninstalling.

#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif
#ifndef BuildDir
  #define BuildDir "..\..\build\windows\x64\runner\Release"
#endif
#ifndef OutputDir
  #define OutputDir "..\..\build\windows\installer"
#endif

; Never change AppId: Windows uses it to recognize an existing install, so
; a new AppId would install a second copy instead of upgrading.
#define AppId "{{99CB5D06-7F33-44C1-A5E4-186155482C71}"
#define UninstallKey "Software\Microsoft\Windows\CurrentVersion\Uninstall\{99CB5D06-7F33-44C1-A5E4-186155482C71}_is1"

[Setup]
AppId={#AppId}
AppName=Doki
AppVersion={#AppVersion}
AppVerName=Doki {#AppVersion}
AppPublisher=Doki
AppPublisherURL=https://github.com/drluciano/doki
AppSupportURL=https://github.com/drluciano/doki/wiki
AppUpdatesURL=https://github.com/drluciano/doki/wiki/Updating-Doki
VersionInfoVersion={#AppVersion}
VersionInfoProductName=Doki
VersionInfoDescription=Doki setup
; Per-user install: {autopf} becomes %LOCALAPPDATA%\Programs.
PrivilegesRequired=lowest
DefaultDirName={autopf}\Doki
DisableDirPage=yes
DisableProgramGroupPage=yes
DisableReadyPage=yes
DisableWelcomePage=no
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
WizardStyle=modern
WizardSizePercent=100
WizardImageFile=wizard-1x.bmp,wizard-2x.bmp
WizardSmallImageFile=wizard-small-1x.bmp,wizard-small-2x.bmp
SetupIconFile=..\runner\resources\app_icon.ico
UninstallDisplayIcon={app}\doki.exe
UninstallDisplayName=Doki
; [Code] quits Doki itself (it lives in the tray, where the Restart Manager
; would only ask it to close, which hides it again).
CloseApplications=no
RestartApplications=no
Compression=lzma2/max
SolidCompression=yes
OutputDir={#OutputDir}
OutputBaseFilename=doki-setup-{#AppVersion}

[Languages]
Name: "en"; MessagesFile: "compiler:Default.isl"

[Messages]
WelcomeLabel1=Welcome to Doki
WelcomeLabel2=Doki keeps checkpoints of your game saves and hands progress between your devices, without ever replacing a save behind your back.%n%nIt installs just for you, so no administrator password is needed.
SelectTasksLabel2=Pick the extras you would like. You can change both later.
FinishedHeadingLabel=Doki is ready
FinishedLabelNoIcons=Doki is installed.
FinishedLabel=Doki is installed. Find it in the Start menu, or in the tray once it is running.
ClickFinish=Choose Finish to close Setup.
ConfirmUninstall=Remove Doki from this computer?%n%nYour checkpoints, paired devices, settings and game saves are kept.

[CustomMessages]
en.Extras=Extras:
en.DesktopShortcut=Add a desktop shortcut
en.StartWithWindows=Start Doki when I sign in, so checkpoints happen on their own
en.OpenDoki=Open Doki
en.UpdateWelcome=This updates Doki to version {#AppVersion}.%n%nYour checkpoints, paired devices and settings are kept. Your game saves are not touched.
en.CloseDokiAsk=Doki is running. Setup will quit it now so it can carry on, and you can open it again afterwards.%n%nYour saves and checkpoints are not affected.
en.CloseDokiFailed=Doki is still running.%n%nRight-click the Doki icon in the tray, choose Quit, then choose Retry.
en.CloseDokiCancelled=Doki is still running, so nothing was changed. Quit Doki from its tray icon and run Setup again.
en.UninstallDone=Doki has been removed.%n%nYour checkpoints and settings are still in %1, in case you install Doki again. Your game saves were not touched.

[Tasks]
Name: "desktopicon"; Description: "{cm:DesktopShortcut}"; GroupDescription: "{cm:Extras}"; Flags: unchecked
; Only offered on a first install: after that the app's own Settings switch
; owns this choice, and an update should not undo it.
Name: "startup"; Description: "{cm:StartWithWindows}"; GroupDescription: "{cm:Extras}"; Check: not IsUpdate

[InstallDelete]
; Flutter's data folder holds only the app's code and assets (user data lives
; in %APPDATA%); clearing it stops files from an older version lingering.
Type: filesandordirs; Name: "{app}\data"

[Files]
Source: "{#BuildDir}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\Doki"; Filename: "{app}\doki.exe"
Name: "{autodesktop}\Doki"; Filename: "{app}\doki.exe"; Tasks: desktopicon

[Registry]
; Same value and format as FlutterWindow::WriteStartWithWindows.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "Doki"; ValueData: """{app}\doki.exe"" --hidden"; Tasks: startup

[Run]
Filename: "{app}\doki.exe"; Description: "{cm:OpenDoki}"; Flags: nowait postinstall skipifsilent

[Code]
const
  // Doki's single-instance mutex and window (app/windows/runner/main.cpp).
  DokiMutex = 'Local\DokiSingleInstance';
  DokiWindowClass = 'FLUTTER_RUNNER_WIN32_WINDOW';
  DokiWindowTitle = 'doki';
  // Handled by FlutterWindow like Quit in the tray menu.
  DokiQuitMessage = 'DokiQuit';
  RunKey = 'Software\Microsoft\Windows\CurrentVersion\Run';

function FindWindow(ClassName, WindowName: String): HWND;
  external 'FindWindowW@user32.dll stdcall';
function RegisterWindowMessage(Name: String): Cardinal;
  external 'RegisterWindowMessageW@user32.dll stdcall';

function IsUpdate(): Boolean;
begin
  Result := RegKeyExists(HKCU, '{#UninstallKey}');
end;

function DokiRunning(): Boolean;
begin
  Result := CheckForMutexes(DokiMutex);
end;

// Asks the running Doki to quit and waits up to five seconds for it to go.
// Builds from before the installer existed ignore the message; the caller
// then asks the user to quit from the tray.
function QuitDoki(): Boolean;
var
  Window: HWND;
  Waited: Integer;
begin
  Window := FindWindow(DokiWindowClass, DokiWindowTitle);
  if Window <> 0 then
    PostMessage(Window, RegisterWindowMessage(DokiQuitMessage), 0, 0);
  Waited := 0;
  while DokiRunning() and (Waited < 5000) do
  begin
    Sleep(250);
    Waited := Waited + 250;
  end;
  Result := not DokiRunning();
end;

// True once Doki is not running. Quitting Doki is safe at any moment: the
// engine journals every change and finishes or undoes it on the next start.
function EnsureDokiClosed(): Boolean;
begin
  Result := not DokiRunning();
  if Result then
    exit;
  if SuppressibleMsgBox(CustomMessage('CloseDokiAsk'), mbInformation, MB_OKCANCEL, IDOK) <> IDOK then
    exit;
  while not QuitDoki() do
    if SuppressibleMsgBox(CustomMessage('CloseDokiFailed'), mbError, MB_RETRYCANCEL, IDCANCEL) <> IDRETRY then
      exit;
  Result := True;
end;

procedure InitializeWizard();
begin
  if IsUpdate() then
    WizardForm.WelcomeLabel2.Caption := CustomMessage('UpdateWelcome');
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  if EnsureDokiClosed() then
    Result := ''
  else
    Result := CustomMessage('CloseDokiCancelled');
end;

function InitializeUninstall(): Boolean;
begin
  Result := EnsureDokiClosed();
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usUninstall then
    // Whether the installer or the app's Settings switch added it, the
    // sign-in entry would point at a removed program.
    RegDeleteValue(HKCU, RunKey, 'Doki');
  if (CurUninstallStep = usDone) and not UninstallSilent() then
    MsgBox(FmtMessage(CustomMessage('UninstallDone'), [ExpandConstant('{userappdata}\dev.doki\doki')]), mbInformation, MB_OK);
end;
