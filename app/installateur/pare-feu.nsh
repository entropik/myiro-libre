; Crochets de l'installateur NSIS de myiro-libre
; (tauri.conf.json : bundle > windows > nsis > installerHooks).
;
; La seule regle de pare-feu de l'application, pour la detection du FD-9
; (tickets #47 et #85, ADR 0005) : entrante, UDP, port local 49152, reseau
; local seulement (remoteip=localsubnet), tous profils, SANS programme. Elle
; vaut quel que soit l'emplacement du pont lance. Le nom et les arguments
; doivent rester ceux de app/src/instrument/parefeu.rs : un test le verifie.
;
; L'installateur s'installe pour l'utilisateur courant, sans droits
; d'administrateur (installMode de Tauri : currentUser). Le pare-feu en
; demande : Windows les demande une fois, par la fenetre de controle de
; compte, et seulement si la regle manque ou n'est pas conforme.

!define MYIRO_REGLE_FD9 "myiro-libre - FD-9 - detection (UDP 49152)"

; A l'installation et a chaque mise a jour : lecture de la regle, sans droits
; (netsh ecrit son detail dans un fichier temporaire, lu ligne a ligne). Les
; libelles LocalPort, RemoteIP et Program restent en anglais dans le Windows
; francais ; Protocol y devient Protocole. Chaque regle de ce nom doit avoir
; le port 49152, UDP, l'etendue LocalSubnet et aucun programme (Any, Tout).
; Sinon : une seule demande de droits, retrait de toute regle de ce nom, puis
; ajout d'une seule. Un refus ne bloque pas l'installation : l'application
; redemandera la regle a la premiere detection vide.
!macro NSIS_HOOK_POSTINSTALL
  Push $0
  Push $1
  Push $2
  Push $3
  Push $4
  Push $5
  Push $6
  Push $7
  Push $8
  Push $9
  InitPluginsDir
  StrCpy $6 "$PLUGINSDIR\myiro-regle-fd9.txt"
  nsExec::Exec '"$SYSDIR\cmd.exe" /d /s /c ""$SYSDIR\netsh.exe" advfirewall firewall show rule name="${MYIRO_REGLE_FD9}" verbose > "$6""'
  Pop $0
  StrCpy $1 0 ; regles de ce nom
  StrCpy $2 0 ; port local 49152
  StrCpy $3 0 ; etendue LocalSubnet
  StrCpy $4 0 ; protocole UDP
  StrCpy $5 0 ; liees a un programme
  ClearErrors
  FileOpen $7 $6 r
  ${IfNot} ${Errors}
    ${Do}
      ClearErrors
      FileRead $7 $8
      ${If} ${Errors}
        ${ExitDo}
      ${EndIf}
      ; Fin de ligne retiree.
      ${Do}
        StrCpy $9 $8 1 -1
        ${If} $9 == "$\r"
        ${OrIf} $9 == "$\n"
          StrCpy $8 $8 -1
        ${Else}
          ${ExitDo}
        ${EndIf}
      ${Loop}
      ; Chaque regle commence par son nom, souligne d'une ligne de tirets.
      StrCpy $9 $8 10
      ${If} $9 == "----------"
        IntOp $1 $1 + 1
      ${EndIf}
      StrCpy $9 $8 9
      ${If} $9 == "LocalPort"
        StrCpy $9 $8 "" -6
        ${If} $9 == " 49152"
          IntOp $2 $2 + 1
        ${EndIf}
      ${EndIf}
      StrCpy $9 $8 8
      ${If} $9 == "RemoteIP"
        StrCpy $9 $8 "" -12
        ${If} $9 == " LocalSubnet"
          IntOp $3 $3 + 1
        ${EndIf}
      ${EndIf}
      StrCpy $9 $8 8
      ${If} $9 == "Protocol"
        StrCpy $9 $8 "" -4
        ${If} $9 == " UDP"
          IntOp $4 $4 + 1
        ${EndIf}
      ${EndIf}
      StrCpy $9 $8 7
      ${If} $9 == "Program"
        StrCpy $9 $8 "" -4
        ${If} $9 != " Any"
          StrCpy $9 $8 "" -5
          ${If} $9 != " Tout"
            IntOp $5 $5 + 1
          ${EndIf}
        ${EndIf}
      ${EndIf}
    ${Loop}
    FileClose $7
  ${EndIf}
  Delete $6
  ${If} $1 > 0
  ${AndIf} $2 = $1
  ${AndIf} $3 = $1
  ${AndIf} $4 = $1
  ${AndIf} $5 = 0
    DetailPrint "Pare-feu : la regle ${MYIRO_REGLE_FD9} est deja en place."
  ${Else}
    ExecShellWait "runas" "$SYSDIR\cmd.exe" '/d /s /c ""$SYSDIR\netsh.exe" advfirewall firewall delete rule name="${MYIRO_REGLE_FD9}" & "$SYSDIR\netsh.exe" advfirewall firewall add rule name="${MYIRO_REGLE_FD9}" dir=in action=allow protocol=UDP localport=49152 remoteip=localsubnet profile=any"' SW_HIDE
  ${EndIf}
  Pop $9
  Pop $8
  Pop $7
  Pop $6
  Pop $5
  Pop $4
  Pop $3
  Pop $2
  Pop $1
  Pop $0
!macroend

; A la desinstallation, la regle est retiree. Rien n'est demande si elle
; n'existe pas ; sinon, si le desinstallateur n'a pas deja les droits
; d'administrateur, Windows les demande une fois. Lors d'une mise a jour
; ($UpdateMode = 1, option /UPDATE), la regle est gardee.
!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    nsExec::Exec '"$SYSDIR\netsh.exe" advfirewall firewall show rule name="${MYIRO_REGLE_FD9}"'
    Pop $0
    ${If} $0 == 0
      ExecShellWait "runas" "$SYSDIR\netsh.exe" 'advfirewall firewall delete rule name="${MYIRO_REGLE_FD9}"' SW_HIDE
    ${EndIf}
  ${EndIf}
!macroend
