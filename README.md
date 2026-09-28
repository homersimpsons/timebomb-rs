# Time bomb

A time bomb game implemented using [SpacetimeDB](https://spacetimedb.com/home), [Rust](https://rust-lang.org/) and [Nuxt](https://nuxt.com/) / [Nuxt UI](https://ui.nuxt.com/).

## Development

1. Create `spacetime.local.json`:
```json
{
  "server": "http://127.0.0.1:3000"
}
```

```shell
~> spacetime start
~> spacetime login --server-issued-login 127.0.0.1:3000
~> spacetime dev --yes --delete-data=always
```

## Deployment

```sh
~> spacetime login
~> spacetime publish --server https://maincloud.spacetimedb.com <db_name>
```
