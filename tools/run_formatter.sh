#!/bin/bash

cd $(dirname $(dirname $0))

check=false
if [ "$1" = "--diff" ]; then
  check=true
fi

if [ -z "${CLANG_FORMAT:-}" ]; then
  ensure_file=$(mktemp)
  # https://chrome-infra-packages.appspot.com/p/fuchsia/third_party/clang
  echo 'fuchsia/third_party/clang/${platform} integration' > $ensure_file
  cipd ensure -ensure-file $ensure_file -root clang
  CLANG_FORMAT="./clang/bin/clang-format"
fi

has_diff=0
for f in $(git ls-files | egrep '\.(h|cc)$' | fgrep -v 'third_party'); do
    if "${check}"; then
        diff -u "$f" <("$CLANG_FORMAT" "$f") || has_diff=1
    else
        "$CLANG_FORMAT" -i "$f"
    fi
done
exit $has_diff