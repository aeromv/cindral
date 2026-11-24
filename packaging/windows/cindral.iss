; The Windows installer of Cindral (Inno Setup 6).
;
; Build it after `cargo build --release`:
;   ISCC.exe /DAppVersion=0.6.2 packaging\windows\cindral.iss
; The installer lands in target\installer.
;
; No Mojang file is packed: the last step of the install downloads the
; Minecraft 1.8.8 jar and sounds from Mojang's own servers and checks each one
; against Mojang's SHA-1. Without a connection the game downloads them on its
; first launch instead.

#ifndef AppVersion
  #define AppVersion "0.6.2"
#endif
#define AppName "Cindral"
#define AppPublisher "AeroMV"
#define AppUrl "https://github.com/aeromv/cindral"
#define Bin "..\..\target\release"

[Setup]
AppId={{B31C4798-7FBD-4020-A3F5-A0E8A822AD96}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppUrl}
AppSupportURL={#AppUrl}/issues
AppUpdatesURL={#AppUrl}/releases
AppCopyright=AeroMV
VersionInfoVersion={#AppVersion}
VersionInfoCompany={#AppPublisher}
VersionInfoDescription={#AppName} Setup
; Per-user install: no administrator prompt, updates in place.
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
DefaultDirName={autopf}\{#AppName}
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
DisableDirPage=auto
DisableReadyPage=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir=..\..\target\installer
OutputBaseFilename=cindral-setup-{#AppVersion}
SetupIconFile=cindral.ico
UninstallDisplayIcon={app}\cindral.exe
UninstallDisplayName={#AppName}
WizardStyle=modern
WizardSizePercent=100
WizardSmallImageFile=wizard-small.bmp,wizard-small-2x.bmp
Compression=lzma2/ultra64
SolidCompression=yes
LZMAUseSeparateProcess=yes
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "french"; MessagesFile: "compiler:Languages\French.isl"

[CustomMessages]
english.Downloading=Downloading the Minecraft 1.8.8 game files and sounds from Mojang (about 110 MB)...
french.Downloading=Téléchargement des fichiers et des sons de Minecraft 1.8.8 depuis Mojang (environ 110 Mo)...
english.Launch=Launch Cindral
french.Launch=Lancer Cindral

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"

[Files]
Source: "{#Bin}\cindral.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Bin}\dxcompiler.dll"; DestDir: "{app}"; Flags: ignoreversion skipifsourcedoesntexist
Source: "..\..\LICENSE-MIT"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE-APACHE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\cindral.exe"; WorkingDir: "{app}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\cindral.exe"; WorkingDir: "{app}"; Tasks: desktopicon

[Run]
; Skipped quietly when the files are already there, so updates are quick.
Filename: "{app}\cindral.exe"; Parameters: "--setup-assets"; StatusMsg: "{cm:Downloading}"; Flags: runhidden waituntilterminated
Filename: "{app}\cindral.exe"; Description: "{cm:Launch}"; Flags: nowait postinstall skipifsilent
