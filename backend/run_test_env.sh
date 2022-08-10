#!/bin/bash

SCRIPT_DIR=$( cd -- "$( dirname -- "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )
cd "${SCRIPT_DIR}/../contracts/"
output=$(cargo run)
SQ_ADDR=$(echo $output | grep sacred_queens.vy | awk '{ print $NF }')
Q_ADDR=$(echo $output | grep queens.vy | awk '{ print $NF }')
export SQ_ADDR
export Q_ADDR
cd "$SCRIPT_DIR"
cargo run
