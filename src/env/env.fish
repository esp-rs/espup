# espup shell setup
# TODO: WE NEED TO VERIFY THAT PLACEHOLDERS ARE REPLACED.
# EG: USING THE --NO-STD FLAG WONT INSTALL GCC, HENCE WE WILL WRITE THE PATH WITH THE PLACEHOLDER

if not contains "{xtensa_gcc}" $PATH
    # Prepending path in case a system-installed rustc needs to be overridden
    set -x PATH "{xtensa_gcc}" $PATH
end

if not contains "{riscv_gcc}" $PATH
    # Prepending path in case a system-installed rustc needs to be overridden
    set -x PATH "{riscv_gcc}" $PATH
end

set -x LIBCLANG_PATH "{libclang}"
