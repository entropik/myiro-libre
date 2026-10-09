; Crochets de l'installateur NSIS de myiro-libre
; (tauri.conf.json : bundle > windows > nsis > installerHooks).
;
; A la desinstallation, retire la seule regle de pare-feu que l'application
; cree pour la detection du FD-9 (ticket #47, ADR 0005). Le nom doit rester
; celui de NOM_REGLE (app/src/instrument/parefeu.rs) : un test le verifie.
; Rien n'est demande si la regle n'existe pas ; sinon, si le desinstallateur
; n'a pas deja les droits d'administrateur, Windows les demande une fois.
; Lors d'une mise a jour ($UpdateMode = 1, option /UPDATE), la regle est gardee.

!define MYIRO_REGLE_FD9 "myiro-libre - FD-9 - detection (UDP 49152)"

!macro NSIS_HOOK_POSTUNINSTALL
  ${If} $UpdateMode <> 1
    nsExec::Exec '"$SYSDIR\netsh.exe" advfirewall firewall show rule name="${MYIRO_REGLE_FD9}"'
    Pop $0
    ${If} $0 == 0
      ExecShellWait "runas" "$SYSDIR\netsh.exe" 'advfirewall firewall delete rule name="${MYIRO_REGLE_FD9}"' SW_HIDE
    ${EndIf}
  ${EndIf}
!macroend
