@echo off

setlocal enabledelayedexpansion
SET /A linenum=0

rem echo "check tap network name"
FOR /F "tokens=1 delims=," %%A IN ('getmac /v /NH /FO CSV ^| findstr "TAP-Win"') DO (
echo %%A
SET /A linenum+=1
echo linenum is: !linenum!

rem echo interfaceName11 is %%A!linenum!
rem echo interfaceName111 is "%%~A!linenum!"
wmic nic where "netconnectionid = '%%~A'" set netconnectionid = "dnet"
)
goto EXIT

:EXIT
@ECHO on
