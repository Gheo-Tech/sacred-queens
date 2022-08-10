#!/bin/bash
vyper=~/.local/pipx/venvs/vyper/bin/vyper
SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
TARGET_DIR="${SCRIPT_DIR}/target"
mkdir -p "${TARGET_DIR}"

compile_contract () {
  name="$1"
  echo Compiling contract $name
  $vyper -f abi "${SCRIPT_DIR}/${name}" | jq > "${TARGET_DIR}/${name}.abi"
  $vyper -f bytecode "${SCRIPT_DIR}/${name}" > "${TARGET_DIR}/${name}.bin"
}

contracts="$(ls $SCRIPT_DIR | grep -e '\.vy$')"

for c in $contracts; do
  compile_contract $c
done
