#!/bin/bash

tag_name=$1

if [[ -z "$tag_name" ]]; then
    echo "Usage: $0 <tag_name>"
    exit 1
fi

git -C $WORKSPACE_ROOT cliff --unreleased --tag $tag_name --prepend $CRATE_ROOT/CHANGELOG.md
