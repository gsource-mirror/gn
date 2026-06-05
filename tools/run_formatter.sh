#!/bin/bash -eu

cd $(dirname $(dirname $0))

if [ "${1:-}" = "--diff" ]; then
  opts="--dry-run -Werror"
  fmt_opts="--check"
else
  opts="-i"
  fmt_opts=""
fi

if [ -z "${CLANG_FORMAT:-}" ]; then
  ensure_file=$(mktemp)
  # https://chrome-infra-packages.appspot.com/p/fuchsia/third_party/clang
  echo 'fuchsia/third_party/clang/${platform} integration' > $ensure_file
  cipd ensure -ensure-file $ensure_file -root clang
  CLANG_FORMAT="./clang/bin/clang-format"
fi

git ls-files | egrep '\.(h|cc)$' | xargs "$CLANG_FORMAT" $opts

if command -v cargo >/dev/null 2>&1; then
  cargo_cmd="cargo"
  extra_opts=""
  if cargo +nightly --version >/dev/null 2>&1; then
    cargo_cmd="cargo +nightly"
    extra_opts="-- --config-path rustfmt-nightly.toml"
  fi
  (cd src/gn/starlark && $cargo_cmd fmt --all $fmt_opts $extra_opts)
fi
