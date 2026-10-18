; Windows installer for ArchiSim (Inno Setup 6).
;
;   iscc /DVersion=0.21.0 /DSource=ArchiSim scripts\windows_installer.iss
;
; Source is the folder with ArchiSim.exe, the demo house and the IFC
; examples (the continuous integration builds it). Writes
; dist\ArchiSim-<version>-Windows.exe.
;
; Installs for the current user, without asking for administrator rights
; (they can be chosen in the first page to install for everyone): the
; program, a Start menu entry, an optional desktop icon, the demo house and
; the IFC examples, and the file types: .archisim projects open with
; ArchiSim (with its icon), and .ifc models get ArchiSim in "Abrir con" and,
; if chosen, open with it on double click. The uninstaller removes all of it
; and leaves the user's projects alone.

#ifndef Version
  #define Version "0.0.0"
#endif
#ifndef Source
  #define Source "..\ArchiSim"
#endif

[Setup]
AppId={{8E1C2B6A-4A57-4D35-9C1F-5A2D0B7E3A11}
AppName=ArchiSim
AppVersion={#Version}
AppVerName=ArchiSim {#Version}
AppPublisher=AD5
AppComments=Herramienta BIM de autoría (IFC4)
DefaultDirName={autopf}\ArchiSim
DefaultGroupName=ArchiSim
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
OutputDir=..\dist
OutputBaseFilename=ArchiSim-{#Version}-Windows
SetupIconFile=..\crates\archisim-app\assets\icon.ico
UninstallDisplayIcon={app}\ArchiSim.exe
UninstallDisplayName=ArchiSim {#Version}
ChangesAssociations=yes
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Languages]
Name: "es"; MessagesFile: "compiler:Languages\Spanish.isl"

[Tasks]
Name: "desktopicon"; Description: "Crear un icono en el escritorio"; GroupDescription: "Iconos:"
Name: "ifcdefault"; Description: "Abrir los ficheros .ifc con ArchiSim al hacer doble clic"; GroupDescription: "Ficheros:"; Flags: unchecked

[Files]
Source: "{#Source}\ArchiSim.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#Source}\demo.archisim"; DestDir: "{app}\ejemplos"; Flags: ignoreversion
Source: "{#Source}\ejemplos-ifc\*.ifc"; DestDir: "{app}\ejemplos"; Flags: ignoreversion skipifsourcedoesntexist

[Icons]
Name: "{autoprograms}\ArchiSim"; Filename: "{app}\ArchiSim.exe"
Name: "{autoprograms}\Ejemplos de ArchiSim"; Filename: "{app}\ejemplos"
Name: "{autodesktop}\ArchiSim"; Filename: "{app}\ArchiSim.exe"; Tasks: desktopicon

[Registry]
; The program, for "Abrir con" and the file types.
Root: HKA; Subkey: "Software\Classes\Applications\ArchiSim.exe"; ValueType: string; ValueName: "FriendlyAppName"; ValueData: "ArchiSim"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\Applications\ArchiSim.exe\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\ArchiSim.exe"" ""%1"""
Root: HKA; Subkey: "Software\Classes\Applications\ArchiSim.exe\SupportedTypes"; ValueType: string; ValueName: ".archisim"; ValueData: ""
Root: HKA; Subkey: "Software\Classes\Applications\ArchiSim.exe\SupportedTypes"; ValueType: string; ValueName: ".ifc"; ValueData: ""

; .archisim projects belong to ArchiSim.
Root: HKA; Subkey: "Software\Classes\.archisim"; ValueType: string; ValueName: ""; ValueData: "ArchiSim.Proyecto"; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\.archisim"; ValueType: string; ValueName: "Content Type"; ValueData: "application/json"
Root: HKA; Subkey: "Software\Classes\ArchiSim.Proyecto"; ValueType: string; ValueName: ""; ValueData: "Proyecto ArchiSim"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\ArchiSim.Proyecto\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\ArchiSim.exe,0"
Root: HKA; Subkey: "Software\Classes\ArchiSim.Proyecto\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\ArchiSim.exe"" ""%1"""

; .ifc models: ArchiSim in "Abrir con" always, and the default if chosen.
Root: HKA; Subkey: "Software\Classes\.ifc\OpenWithProgids"; ValueType: string; ValueName: "ArchiSim.IFC"; ValueData: ""; Flags: uninsdeletevalue
Root: HKA; Subkey: "Software\Classes\ArchiSim.IFC"; ValueType: string; ValueName: ""; ValueData: "Modelo IFC"; Flags: uninsdeletekey
Root: HKA; Subkey: "Software\Classes\ArchiSim.IFC\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\ArchiSim.exe,0"
Root: HKA; Subkey: "Software\Classes\ArchiSim.IFC\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\ArchiSim.exe"" ""%1"""
Root: HKA; Subkey: "Software\Classes\.ifc"; ValueType: string; ValueName: ""; ValueData: "ArchiSim.IFC"; Tasks: ifcdefault; Flags: uninsdeletevalue

[Run]
Filename: "{app}\ArchiSim.exe"; Parameters: """{app}\ejemplos\demo.archisim"""; Description: "Abrir ArchiSim con la vivienda de prueba"; Flags: nowait postinstall skipifsilent
