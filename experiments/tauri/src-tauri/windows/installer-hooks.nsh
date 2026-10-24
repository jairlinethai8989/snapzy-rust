!define SNAPZY_HOOK_SOURCE_DIR "${__FILEDIR__}"

!macro NSIS_HOOK_PREINSTALL
  InitPluginsDir
  File /oname=$PLUGINSDIR\migrate-legacy.ps1 "${SNAPZY_HOOK_SOURCE_DIR}\..\resources\migration\migrate-legacy.ps1"

snapzy_process_check:
  nsExec::ExecToStack '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$PLUGINSDIR\migrate-legacy.ps1" -Mode CheckRunning'
  Pop $0
  Pop $1
  ${If} $0 == 20
    MessageBox MB_RETRYCANCEL|MB_ICONEXCLAMATION "SnapZy is still running. Close SnapZy, including its System Tray icon, then choose Retry.$\r$\n$\r$\nSnapZy ยังทำงานอยู่ กรุณาปิดโปรแกรมรวมถึงไอคอนใน System Tray แล้วกด Retry" IDRETRY snapzy_process_check
    Abort
  ${ElseIf} $0 != 0
    MessageBox MB_OK|MB_ICONSTOP "SnapZy could not verify that the previous version is closed. Setup will stop to protect your work.$\r$\n$\r$\nไม่สามารถตรวจสอบว่ารุ่นเดิมปิดแล้ว การติดตั้งจะหยุดเพื่อป้องกันข้อมูลงาน"
    Abort
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
  nsExec::ExecToLog '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$PLUGINSDIR\migrate-legacy.ps1" -Mode Migrate'
  Pop $0
  ${If} $0 != 0
    DetailPrint "Legacy shortcut cleanup returned exit code $0; the new SnapZy installation remains available."
  ${EndIf}
!macroend
