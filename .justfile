pkg := "hb"

release-dry-run $level:
    cargo release --no-publish -p {{ pkg }} $level

release $level:
    cargo release --no-publish -p {{ pkg }} $level --execute
