@echo off
echo ============================================
echo   AI Novel Factory — Starting Services
echo ============================================
echo.
cd /d D:\novel

echo [1/3] Starting n8n (Docker)...
REM Wait for Docker Desktop (max 60s)
set /a W=0
:wait
docker info > nul 2>&1
if %ERRORLEVEL% NEQ 0 (
    if %W% GEQ 60 (
        echo ERROR: Docker Desktop not running. Start it first.
        pause
        exit /b 1
    )
    timeout /t 3 /nobreak > nul
    set /a W+=3
    goto wait
)
docker compose up -d

echo [2/3] Starting writer-service...
start "Writer Service" /min cmd /c "node writer-service\server.js >> writer-service.log 2>&1"
timeout /t 3 /nobreak > nul

echo [3/3] Starting orchestrator...
start "Orchestrator" /min cmd /c "cd orchestrator && node scheduler.js >> orchestrator.log 2>&1"
timeout /t 3 /nobreak > nul

echo.
echo Waiting for services...
timeout /t 8 /nobreak > nul
curl -s http://localhost:5678/healthz > nul 2>&1 && echo   n8n:            OK || echo   n8n:            WAITING
curl -s http://localhost:8787/health > nul 2>&1 && echo   writer-service: OK || echo   writer-service: WAITING
curl -s http://localhost:3001/status > nul 2>&1 && echo   orchestrator:   OK || echo   orchestrator:   WAITING

echo.
echo ============================================
echo   All up. stop.bat to shut down.
echo ============================================
pause
