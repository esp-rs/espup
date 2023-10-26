@echo off
rem espup CMD setup

set XTENSA_GCC={xtensa_gcc}
if not "%XTENSA_GCC%" == "" (
    echo %PATH% | findstr /C:"%XTENSA_GCC%" 1>nul
    if errorlevel 1 (
        rem Prepending path
        set PATH=%XTENSA_GCC%;%PATH%
    )
)

set RISCV_GCC={riscv_gcc}
if not "%RISCV_GCC%" == "" (
    echo %PATH% | findstr /C:"%RISCV_GCC%" 1>nul
    if errorlevel 1 (
        rem Prepending path
        set PATH=%RISCV_GCC%;%PATH%
    )
)

set LIBCLANG_PATH={libclang_path}
set LIBCLANG_BIN_PATH={libclang_bin_path}
if not "%LIBCLANG_BIN_PATH%" == "" (
    echo %PATH% | findstr /C:"%LIBCLANG_BIN_PATH%" 1>nul
    if errorlevel 1 (
        rem Prepending path
        set PATH=%LIBCLANG_BIN_PATH%;%PATH%
    )
)

set CLANG_PATH={clang_path}
