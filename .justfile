pkg := "hb"

release-dry-run $level:
    cargo release -p {{ pkg }} $level

release $level:
    cargo release -p {{ pkg }} $level --execute

check-deny:
    cargo deny check licenses bans sources
