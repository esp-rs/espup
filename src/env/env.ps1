# espup PowerShell setup
# affix semicolons on either side of $Env:PATH to simplify matching
$XTENSA_GCC = "{xtensa_gcc}"
if ($XTENSA_GCC -ne "") {
    if (-not ($Env:PATH -like "*;$XTENSA_GCC;*")) {
        # Prepending path
        $Env:PATH = "$XTENSA_GCC;$Env:PATH"
    }
}

$RISCV_GCC = "{riscv_gcc}"
if ($RISCV_GCC -ne "") {
    if (-not ($Env:PATH -like "*;$RISCV_GCC;*")) {
        # Prepending path
        $Env:PATH = "$RISCV_GCC;$Env:PATH"
    }
}

$Env:LIBCLANG_PATH = "{libclang_path}"
$LIBCLANG_BIN_PATH = "{libclang_bin_path}"
if ($LIBCLANG_BIN_PATH -ne "") {
    if (-not ($Env:PATH -like "*;$LIBCLANG_BIN_PATH;*")) {
        # Prepending path
        $Env:PATH = "$LIBCLANG_BIN_PATH;$Env:PATH"
    }
}

$Env:CLANG_PATH = "{clang_path}"
