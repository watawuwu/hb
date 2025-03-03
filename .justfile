release-dry-run:
    #!/bin/bash
    workspace=$(just select-workspace)
    level=$(just select-level)
    just release-task $workspace $level

release:
    #!/bin/bash
    workspace=$(just select-workspace)
    level=$(just select-level)
    just release-task $workspace $level --execute

select-workspace:
    @cargo metadata --format-version 1 | \
        jq -r '.workspace_members[]' | \
        perl -ne 'print "$1\n" if /^.*#(.+?)@.+$/' | \
        fzf

select-level:
    #!/bin/bash
    echo -e "patch\nminor\nmajor" | fzf

release-task workspace level *args:
    cargo release -p {{ workspace }} {{ level }} {{ args }}

check-deny:
    cargo deny check licenses bans sources
