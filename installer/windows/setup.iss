; Inno Setup Script for View Launcher
#define MyAppName "View Launcher"
#define MyAppVersion "0.2.0"
#define MyAppPublisher "Hieu Nguyen"
#define MyAppURL "https://github.com/hieunx1024/view-launcher"
#define MyAppExeName "view-launcher.exe"

[Setup]
AppId={{D37F7D28-854B-4E38-9FE7-99E8B62B23F1}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}
DefaultDirName={autopf}\view-launcher
DefaultGroupName={#MyAppName}
AllowNoIcons=yes
OutputDir=Output
OutputBaseFilename=view-launcher-setup
SetupIconFile=..\..\assets\view-launcher.ico
UninstallDisplayIcon={app}\view-launcher.ico
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ChangesEnvironment=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "startupicon"; Description: "Automatically start with Windows and register Ctrl+Alt+Space"; GroupDescription: "Startup Options"; Flags: checkedonce

[Files]
Source: "..\..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\assets\view-launcher.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\assets\view-launcher.png"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\README.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
; Start Menu
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\view-launcher.ico"; WorkingDir: "{app}"
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"

; Desktop
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\view-launcher.ico"; WorkingDir: "{app}"; Tasks: desktopicon

; Startup (with hotkey)
Name: "{userstartup}\ViewLauncher"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\view-launcher.ico"; WorkingDir: "{app}"; Tasks: startupicon

[Registry]
; Add install folder to User PATH
Root: HKCU; Subkey: "Environment"; ValueType: expandsz; ValueName: "Path"; ValueData: "{olddata};{app}"; Check: NeedsAddPath(ExpandConstant('{app}'))

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Launch View Launcher now"; Flags: postinstall nowait skipifsilent

[Code]
function NeedsAddPath(Param: string): boolean;
var
  OrigPath: string;
begin
  if not RegQueryStringValue(HKEY_CURRENT_USER, 'Environment', 'Path', OrigPath)
  then begin
    Result := True;
    exit;
  end;
  Result := Pos(';' + UpperCase(Param) + ';', ';' + UpperCase(OrigPath) + ';') = 0;
end;
