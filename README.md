# Time bomb

A time bomb game implemented using [SpacetimeDB](https://spacetimedb.com/home), [Rust](https://rust-lang.org/) and [Nuxt](https://nuxt.com/) / [Nuxt UI](https://ui.nuxt.com/).

## Development

1. Create `spacetime.local.json`:
```json
{
  "server": "http://127.0.0.1:3000"
}
```

2. Start the spacetimedb server
```shell
~> spacetime start
```

3. Connect to the server and run the project
```shell
~> spacetime login --server-issued-login 127.0.0.1:3000
~> spacetime dev --yes --delete-data=always
```

### Troubleshooting

#### Pre-publish check failed with status 403 Forbidden [...] update database

`spacetime dev` logs displays:
```log
Error: Pre-publish check failed with status 403 Forbidden: c200604a2d4f3d7e04f88c0a524eeb6af224122ccf8f8e09f073f6d7e85ed732 is not authorized to perform action on database c200f98cee0d412ae478270bd683a0fd350fee616c4e31e3fec9b6759458e66b: update database
```

Update the database name:
- `spacetime.json`: field `"database"`
- `client/.env`: field `NUXT_PUBLIC_SPACETIMEDB_DB_NAME`

## Deployment

Continuous deployment is configured in GitHub Actions.
