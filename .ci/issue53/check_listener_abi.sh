#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/../.."
test -s ffi/idevice.h
test -s cpp/include/idevice.h
cmp ffi/idevice.h cpp/include/idevice.h
"${CC:-cc}" -std=c11 -Wall -Wextra -Werror -fsyntax-only -Iffi .ci/issue53/assert_listener_abi.c
"${CXX:-c++}" -std=c++17 -Wall -Wextra -Werror -fsyntax-only -Icpp/include .ci/issue53/assert_listener_abi.cpp
