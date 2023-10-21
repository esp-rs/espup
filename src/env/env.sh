#!/bin/sh
# espup shell setup
# affix colons on either side of $PATH to simplify matching
XTENSA_GCC="{xtensa_gcc}"
if [[ -n "${XTENSA_GCC}" ]]; then
    case ":${PATH}:" in
    *:"{xtensa_gcc}":*) ;;
    *)
        # Prepending path
        export PATH="{xtensa_gcc}:$PATH"
        ;;
    esac
fi
RISCV_GCC="{riscv_gcc}"
if [[ -n "${RISCV_GCC}" ]]; then
    case ":${PATH}:" in
    *:"{riscv_gcc}":*) ;;
    *)
        # Prepending path
        export PATH="{riscv_gcc}:$PATH"
        ;;
    esac
fi
export LIBCLANG_PATH="{libclang_path}"
export CLANG_PATH="{clang_path}"
