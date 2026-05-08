@echo off
echo ============================================
echo   AI Novel Factory — Stopping Services
echo ============================================
echo.
cd /d D:\novel

echo Stopping n8n (Docker)...
docker compose down

echo Stopping writer-service + orchestrator...
taskkill /FI "WINDOWTITLE eq Writer Service" /F 2>nul
taskkill /FI "WINDOWTITLE eq Orchestrator" /F 2>nul

echo.
echo All services stopped.
pause
