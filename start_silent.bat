@echo off
cd /d D:\novel

echo [%date% %time%] Starting AI Novel Factory... >> startup.log

REM Wait for Docker
:wait
docker info > nul 2>&1
if %ERRORLEVEL% NEQ 0 ( timeout /t 5 /nobreak > nul & goto wait )

docker compose up -d >> startup.log 2>&1
timeout /t 8 /nobreak > nul

REM Use wscript to launch hidden (no console window)
wscript.exe "run_hidden.vbs" "node writer-service\server.js" "writer-service.log"
timeout /t 3 /nobreak > nul

wscript.exe "run_hidden.vbs" "cd /d D:\novel\orchestrator && node scheduler.js" "orchestrator.log"

echo [%date% %time%] All services started. >> startup.log
exit
