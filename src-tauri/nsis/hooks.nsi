; IDM No Ads — NSIS installer hooks.
;
; Run by the Tauri-generated NSIS installer. On install we:
;   * write the native-messaging host manifests (pointing at idmnoads-host.exe),
;   * register them in the registry for Chrome, Edge, Chromium, Brave & Firefox,
;   * add optional .torrent / .metalink file associations.
; On uninstall we remove everything we created.
;
; The Chromium extension ID and Firefox gecko ID below are PINNED — they match
; the published extensions and the keys committed in this repository.

!include "WordFunc.nsh"

!define IDM_CHROME_EXT_ID "ikbamigoaahjngjceemkppoimlphgmii"
!define IDM_CHROME_STORE_ID "hbommjcibnllhahdgcjkkbjikbifmenk"
!define IDM_FIREFOX_EXT_ID "idmnoads@hostingrakyat"
!define IDM_HOST_NAME "com.idmnoads.host"

!macro NSIS_HOOK_POSTINSTALL
  ; Escape backslashes in the host path for embedding in JSON.
  ${WordReplace} "$INSTDIR\idmnoads-host.exe" "\" "\\" "+" $R0

  ; --- Chrome / Edge / Chromium / Brave manifest (allowed_origins) ---
  FileOpen $0 "$INSTDIR\${IDM_HOST_NAME}.chrome.json" w
  FileWrite $0 '{$\r$\n'
  FileWrite $0 '  "name": "${IDM_HOST_NAME}",$\r$\n'
  FileWrite $0 '  "description": "IDM No Ads native messaging host",$\r$\n'
  FileWrite $0 '  "path": "$R0",$\r$\n'
  FileWrite $0 '  "type": "stdio",$\r$\n'
  FileWrite $0 '  "allowed_origins": [ "chrome-extension://${IDM_CHROME_EXT_ID}/", "chrome-extension://${IDM_CHROME_STORE_ID}/" ]$\r$\n'
  FileWrite $0 '}$\r$\n'
  FileClose $0

  ; --- Firefox manifest (allowed_extensions) ---
  FileOpen $0 "$INSTDIR\${IDM_HOST_NAME}.firefox.json" w
  FileWrite $0 '{$\r$\n'
  FileWrite $0 '  "name": "${IDM_HOST_NAME}",$\r$\n'
  FileWrite $0 '  "description": "IDM No Ads native messaging host",$\r$\n'
  FileWrite $0 '  "path": "$R0",$\r$\n'
  FileWrite $0 '  "type": "stdio",$\r$\n'
  FileWrite $0 '  "allowed_extensions": [ "${IDM_FIREFOX_EXT_ID}" ]$\r$\n'
  FileWrite $0 '}$\r$\n'
  FileClose $0

  ; --- Registry: native messaging host registration (per-user, HKCU) ---
  WriteRegStr HKCU "Software\Google\Chrome\NativeMessagingHosts\${IDM_HOST_NAME}" "" "$INSTDIR\${IDM_HOST_NAME}.chrome.json"
  WriteRegStr HKCU "Software\Chromium\NativeMessagingHosts\${IDM_HOST_NAME}" "" "$INSTDIR\${IDM_HOST_NAME}.chrome.json"
  WriteRegStr HKCU "Software\Microsoft\Edge\NativeMessagingHosts\${IDM_HOST_NAME}" "" "$INSTDIR\${IDM_HOST_NAME}.chrome.json"
  WriteRegStr HKCU "Software\BraveSoftware\Brave-Browser\NativeMessagingHosts\${IDM_HOST_NAME}" "" "$INSTDIR\${IDM_HOST_NAME}.chrome.json"
  WriteRegStr HKCU "Software\Mozilla\NativeMessagingHosts\${IDM_HOST_NAME}" "" "$INSTDIR\${IDM_HOST_NAME}.firefox.json"

  ; --- Optional file associations for torrent / metalink ---
  WriteRegStr HKCU "Software\Classes\IDMNoAds.torrent\shell\open\command" "" '"$INSTDIR\IDM No Ads.exe" "%1"'
  WriteRegStr HKCU "Software\Classes\.torrent\OpenWithProgids\IDMNoAds.torrent" "" ""
  WriteRegStr HKCU "Software\Classes\IDMNoAds.metalink\shell\open\command" "" '"$INSTDIR\IDM No Ads.exe" "%1"'
  WriteRegStr HKCU "Software\Classes\.metalink\OpenWithProgids\IDMNoAds.metalink" "" ""
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; Remove native messaging host registrations.
  DeleteRegKey HKCU "Software\Google\Chrome\NativeMessagingHosts\${IDM_HOST_NAME}"
  DeleteRegKey HKCU "Software\Chromium\NativeMessagingHosts\${IDM_HOST_NAME}"
  DeleteRegKey HKCU "Software\Microsoft\Edge\NativeMessagingHosts\${IDM_HOST_NAME}"
  DeleteRegKey HKCU "Software\BraveSoftware\Brave-Browser\NativeMessagingHosts\${IDM_HOST_NAME}"
  DeleteRegKey HKCU "Software\Mozilla\NativeMessagingHosts\${IDM_HOST_NAME}"

  ; Remove file associations.
  DeleteRegKey HKCU "Software\Classes\IDMNoAds.torrent"
  DeleteRegValue HKCU "Software\Classes\.torrent\OpenWithProgids" "IDMNoAds.torrent"
  DeleteRegKey HKCU "Software\Classes\IDMNoAds.metalink"
  DeleteRegValue HKCU "Software\Classes\.metalink\OpenWithProgids" "IDMNoAds.metalink"

  ; Remove generated manifests.
  Delete "$INSTDIR\${IDM_HOST_NAME}.chrome.json"
  Delete "$INSTDIR\${IDM_HOST_NAME}.firefox.json"
!macroend
