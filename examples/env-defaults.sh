# @describe An @env default written in the shell forms for a path

# @env TEST_EXA=${TEST_BASE:-$HOME/.local/state}/app  state directory
# @env TEST_EXB=~/app                                 home-relative
# @env TEST_EXC=$HOME/x                               bare variable
# @env TEST_EXD=${TEST_MISSING}/tail                  undefined, no fallback
# @env TEST_EXE=plain                                 no shell forms

main() {
    printenv | grep ^TEST_EX | sort
}

eval "$(argc --argc-eval "$0" "$@")"
