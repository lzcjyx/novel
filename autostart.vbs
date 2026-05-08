' Auto-start AI Novel Factory on login
' Launches Tauri app which auto-starts writer-service + orchestrator internally
' Docker Desktop must already be running for n8n
CreateObject("WScript.Shell").Run """D:\novel\tauri-app\src-tauri\target\release\ai-novel-factory.exe""", 0, False
