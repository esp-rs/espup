# espup shell setup
set XTENSA_GCC "{xtensa_gcc}"
if test -n "$XTENSA_GCC"
    if not contains "{xtensa_gcc}" $PATH
        # Prepending path in case a system-installed rustc needs to be overridden
        set -x PATH "{xtensa_gcc}" $PATH
    end
end

set RISCV_GCC "{riscv_gcc}"
if test -n "$RISCV_GCC"
    if not contains "{riscv_gcc}" $PATH
        # Prepending path in case a system-installed rustc needs to be overridden
        set -x PATH "{riscv_gcc}" $PATH
    end
end

set -x LIBCLANG_PATH "{libclang}"
