# migrate-redb-0.9

Migrates a trinity database created with redb 0.9.0 to the redb version trinity currently uses
(4.x), which can't open the old file format.

It reads every table from the old database, including the ones created by wasm modules, and
writes them all into a new database in a single transaction.

## Usage

Stop trinity first, then run:

```
cargo run --release -- <old.db> <new.db>
```

The old database is left untouched, because the tool reads from a temporary copy. `<new.db>`
must not exist yet. Once the migration succeeds, point `REDB_PATH` at the new file, or replace
the old file with it.
