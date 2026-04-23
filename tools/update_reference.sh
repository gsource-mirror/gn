#!/bin/bash

check=false
if [ "$1" = "--diff" ]; then
    check=true
fi

# Check for the existance of the AUTHORS file as an easy way to determine if
# it's being run from the correct directory.
if test -f "AUTHORS"; then
    echo Building gn...
    ninja -C out gn
    
    echo Generating new reference content...
    temp_ref=$(mktemp)
    out/gn help --markdown all > $temp_ref
    
    if "${check}"; then
        diff -u docs/reference.md $temp_ref
        has_diff=$?
        rm $temp_ref
        exit $has_diff
    else
        echo Overwriting docs/reference.md...
        mv $temp_ref docs/reference.md
    fi
else
    echo Please run this command from the GN checkout root directory.
    exit 1
fi
