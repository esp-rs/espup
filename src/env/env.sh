#!/bin/sh
# espup shell setup
# affix colons on either side of $PATH to simplify matching
case ":${PATH}:" in
*:"{xtensa_gcc}":*) ;;
*)
    # Prepending path in case a system-installed rustc needs to be overridden
    export PATH="{xtensa_gcc}:$PATH"
    ;;
esac

case ":${PATH}:" in
*:"{riscv_gcc}":*) ;;
*)
    # Prepending path in case a system-installed rustc needs to be overridden
    export PATH="{riscv_gcc}:$PATH"
    ;;
esac

export LIBCLANG_PATH="{libclang_path}"
