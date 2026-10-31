#ifndef ReleaseVersion
  #error ReleaseVersion is required
#endif
[Setup]
AppId={{2C11CC72-539E-4EAA-A57B-CA7C5E4F94BB}
AppName=Ond
AppVersion={#ReleaseVersion}
AppPublisher=SUPER-SHRINE
DefaultDirName={autopf}\SUPER-SHRINE\Ond
DisableDirPage=no
UsePreviousAppDir=yes
UsePreviousTasks=yes
WizardStyle=modern
PrivilegesRequired=admin
ArchitecturesAllowed=x64os
ArchitecturesInstallIn64BitMode=x64os
UninstallDisplayIcon={app}\ond.exe
OutputDir={#OutputDirectory}
OutputBaseFilename=ond-{#ReleaseVersion}-windows-x86_64-setup
Compression=lzma2
SolidCompression=yes
ChangesEnvironment=yes
CloseApplications=yes
RestartApplications=no
SetupLogging=yes

[Languages]
Name: "japanese"; MessagesFile: "compiler:Languages\Japanese.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[CustomMessages]
japanese.AddToPath=OndをシステムのPATHに追加する
english.AddToPath=Add Ond to the system PATH
japanese.PathInvalid=PATHに追加する配置先にはセミコロン、二重引用符、パーセント記号を使用できません。
english.PathInvalid=An installation path added to PATH cannot contain semicolons, double quotes, or percent signs.
japanese.ClosePreviousApps=実行中のOnd、Ond Language Server、およびそれらを使用するエディターを閉じてから再実行してください。既存のインストールは変更していません。
english.ClosePreviousApps=Close Ond, Ond Language Server, and any editors using them, then retry. The existing installation has not been changed.

[Tasks]
Name: "systempath"; Description: "{cm:AddToPath}"

[Files]
Source: "{#PayloadDirectory}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Code]
const
  UninstallKey = 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{2C11CC72-539E-4EAA-A57B-CA7C5E4F94BB}_is1';
  EnvironmentKey = 'SYSTEM\CurrentControlSet\Control\Session Manager\Environment';
var
  RemoveOwnedPath: Boolean;
  PreviousUninstallError: String;

function RegOpenKeyEx(hKey: Integer; lpSubKey: String; ulOptions, samDesired: Cardinal; var phkResult: Integer): Integer;
  external 'RegOpenKeyExW@advapi32.dll stdcall';
function RegQueryValueEx(hKey: Integer; lpValueName: String; lpReserved: Integer; var lpType: Cardinal; lpData: Integer; var lpcbData: Cardinal): Integer;
  external 'RegQueryValueExW@advapi32.dll stdcall';
function RegCloseKey(hKey: Integer): Integer;
  external 'RegCloseKey@advapi32.dll stdcall';
function CreateFile(FileName: String; DesiredAccess, ShareMode: Cardinal; SecurityAttributes: Integer;
  CreationDisposition, FlagsAndAttributes: Cardinal; TemplateFile: Integer): Integer;
  external 'CreateFileW@kernel32.dll stdcall';
function CloseHandle(Handle: Integer): Boolean;
  external 'CloseHandle@kernel32.dll stdcall';

function PathType: Cardinal;
var Key: Integer; Size, Kind: Cardinal;
begin
  Result := 2;
  if RegOpenKeyEx(HKLM, EnvironmentKey, 0, $101, Key) <> 0 then
    RaiseException('Cannot open SYSTEM PATH.');
  try
    Size := 0;
    if RegQueryValueEx(Key, 'Path', 0, Kind, 0, Size) <> 0 then
      RaiseException('Cannot inspect SYSTEM PATH.');
    if (Kind <> 1) and (Kind <> 2) then RaiseException('Unsupported SYSTEM PATH type.');
    Result := Kind;
  finally
    RegCloseKey(Key);
  end;
end;

function RewritePath(Value, Directory: String; Remove: Boolean; var Found: Boolean): String;
var Start, I: Integer; Token: String;
begin
  Result := ''; Found := False; Start := 1;
  for I := 1 to Length(Value) + 1 do begin
    if (I > Length(Value)) or (Value[I] = ';') then begin
      Token := Copy(Value, Start, I - Start);
      if CompareText(Token, Directory) = 0 then begin
        if not Remove and not Found then Result := Result + Token + ';';
        Found := True;
      end else Result := Result + Token + ';';
      Start := I + 1;
    end;
  end;
  Delete(Result, Length(Result), 1);
end;

procedure WritePath(Value: String; Kind: Cardinal);
var OK: Boolean;
begin
  if Kind = 1 then OK := RegWriteStringValue(HKLM64, EnvironmentKey, 'Path', Value)
  else OK := RegWriteExpandStringValue(HKLM64, EnvironmentKey, 'Path', Value);
  if not OK then RaiseException('Cannot write SYSTEM PATH.');
end;

function IsAbsoluteDirectory(Value: String): Boolean;
begin
  { Only canonical absolute local drive paths are accepted from registration. }
  Result := (Length(Value) > 3) and (Value[2] = ':') and (Value[3] = '\') and
    (Pos('"', Value) = 0) and (CompareText(ExpandFileName(Value), Value) = 0);
end;

function HasInstallationFiles(Directory: String): Boolean;
begin
  Result := FileExists(Directory + '\ond.exe') or FileExists(Directory + '\ond-lsp.exe') or
    FileExists(Directory + '\unins000.exe');
end;

function CanReplaceBinary(FileName: String): Boolean;
var Handle: Integer;
begin
  Result := True;
  if not FileExists(FileName) then exit;
  { Check for running images and open files without modifying their contents. }
  Handle := CreateFile(FileName, $40000000, 0, 0, 3, 0, 0);
  Result := Handle <> -1;
  if Result then CloseHandle(Handle);
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var Command, ProgramName, Location, Destination: String; ClosingQuote, ExitCode, Attempt: Integer;
begin
  Result := '';
  Destination := RemoveBackslashUnlessRoot(ExpandConstant('{app}'));
  if not IsAbsoluteDirectory(Destination) then begin
    Result := 'Choose an absolute local installation directory.'; exit;
  end;
  { Validate choices before any previous installation is removed. }
  if WizardIsTaskSelected('systempath') and
     ((Pos(';', Destination) > 0) or (Pos('"', Destination) > 0) or (Pos('%', Destination) > 0)) then begin
    Result := CustomMessage('PathInvalid'); exit;
  end;
  if PreviousUninstallError <> '' then begin Result := PreviousUninstallError; exit; end;
  if not RegKeyExists(HKLM64, UninstallKey) then begin
    if HasInstallationFiles(Destination) then Result := 'Existing installation has no trusted uninstall registration.';
    exit;
  end;
  if not RegQueryStringValue(HKLM64, UninstallKey, 'InstallLocation', Location) or
     not RegQueryStringValue(HKLM64, UninstallKey, 'UninstallString', Command) then begin
    Result := 'Invalid existing uninstall registration.'; exit;
  end;
  Location := RemoveBackslashUnlessRoot(Location);
  if not IsAbsoluteDirectory(Location) then begin
    Result := 'Invalid registered installation directory.'; exit;
  end;
  if (CompareText(Destination, Location) <> 0) and HasInstallationFiles(Destination) then begin
    Result := 'The destination already contains an unregistered installation.'; exit;
  end;
  if (Length(Command) < 3) or (Command[1] <> '"') then begin
    Result := 'The registered uninstaller must be quoted.'; exit;
  end;
  Delete(Command, 1, 1); ClosingQuote := Pos('"', Command);
  if ClosingQuote = 0 then begin Result := 'Invalid uninstall command.'; exit; end;
  ProgramName := Copy(Command, 1, ClosingQuote - 1);
  if (Trim(Copy(Command, ClosingQuote + 1, MaxInt)) <> '') or
     (CompareText(ExpandFileName(ProgramName), ProgramName) <> 0) or
     (CompareText(ExtractFileDir(ProgramName), Location) <> 0) or
     (CompareText(ExtractFileName(ProgramName), 'unins000.exe') <> 0) or
     not FileExists(ProgramName) then begin
    Result := 'Invalid registered uninstaller path.'; exit;
  end;
  { PrepareToInstall runs before Inno Setup checks for applications using files. }
  if not CanReplaceBinary(Location + '\ond.exe') or not CanReplaceBinary(Location + '\ond-lsp.exe') then begin
    Result := CustomMessage('ClosePreviousApps'); exit;
  end;
  PreviousUninstallError := 'The existing uninstaller did not finish cleanly. Resolve removal or restart before retrying.';
  if not Exec(ProgramName, '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART', '', SW_HIDE,
      ewWaitUntilTerminated, ExitCode) then begin
    Result := PreviousUninstallError; exit;
  end;
  { The uninstaller clone removes its original executable after the process exits. }
  for Attempt := 1 to 50 do begin
    if not FileExists(ProgramName) then break;
    Sleep(100);
  end;
  if (ExitCode <> 0) or RegKeyExists(HKLM64, UninstallKey) or HasInstallationFiles(Location) then
    Result := PreviousUninstallError
  else PreviousUninstallError := '';
end;

procedure CurStepChanged(CurStep: TSetupStep);
var Value, Updated: String; Kind, Owned: Cardinal; Found: Boolean;
begin
  if CurStep <> ssPostInstall then exit;
  if not WizardIsTaskSelected('systempath') then begin
    if not RegWriteDWordValue(HKLM64, UninstallKey, 'OndPathOwned', 0) then
      RaiseException('Cannot record PATH ownership.');
    exit;
  end;
  Kind := PathType;
  if not RegQueryStringValue(HKLM64, EnvironmentKey, 'Path', Value) then
    RaiseException('Cannot read SYSTEM PATH.');
  Updated := RewritePath(Value, RemoveBackslashUnlessRoot(ExpandConstant('{app}')), False, Found);
  Owned := 0;
  if not Found then begin
    if (Updated <> '') and (Updated[Length(Updated)] <> ';') then Updated := Updated + ';';
    Updated := Updated + RemoveBackslashUnlessRoot(ExpandConstant('{app}'));
    Owned := 1;
  end;
  { Record ownership before changing PATH, so uninstall can recover after failure. }
  if not RegWriteDWordValue(HKLM64, UninstallKey, 'OndPathOwned', Owned) then
    RaiseException('Cannot record PATH ownership.');
  if Updated <> Value then WritePath(Updated, Kind);
end;

function InitializeUninstall: Boolean;
var Owned: Cardinal;
begin
  RemoveOwnedPath := RegQueryDWordValue(HKLM64, UninstallKey, 'OndPathOwned', Owned) and (Owned = 1);
  Result := True;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var Value, Updated: String; Kind: Cardinal; Found: Boolean;
begin
  if (CurUninstallStep <> usUninstall) or not RemoveOwnedPath then exit;
  Kind := PathType;
  if not RegQueryStringValue(HKLM64, EnvironmentKey, 'Path', Value) then
    RaiseException('Cannot read SYSTEM PATH.');
  Updated := RewritePath(Value, RemoveBackslashUnlessRoot(ExpandConstant('{app}')), True, Found);
  if Updated <> Value then WritePath(Updated, Kind);
end;
