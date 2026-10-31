@echo off
REM Knox unified launcher for Windows terminal sessions (CMD & PowerShell).
REM Ensures synchronous execution when running headless CLI / MCP subcommands.
"%~dp0target\release\knox.exe" %*

