//! Migrates a database created with redb 0.9.0 to the latest redb version.
//!
//! All the tables are first exported from the old database into memory, then reimported into a
//! fresh database created with the latest redb version.
//!
//! Usage: `migrate-redb <old.db> <new.db>`

use anyhow::{Context as _, bail};

/// Key/value types supported for a table.
///
/// Trinity uses `str` keys for the admin table, and `[u8]` keys and values for the tables created
/// by wasm modules.
#[derive(Clone, Copy, Debug)]
enum Kind {
    Str,
    Bytes,
}

/// A table exported from the old database.
struct ExportedTable {
    name: String,
    key_kind: Kind,
    value_kind: Kind,
    entries: Vec<(Vec<u8>, Vec<u8>)>,
}

/// Tries to read all the entries of the table `$name` with the given key/value types.
///
/// Evaluates to `Ok(None)` if the table's types don't match the requested ones.
///
/// This is a macro, because redb 0.9.0 doesn't expose its key/value traits publicly.
macro_rules! try_export {
    ($txn:expr, $name:expr, $key:ty, $value:ty) => {{
        use redb_0_9::ReadableTable as _;

        let name: &str = $name;
        match $txn.open_table(redb_0_9::TableDefinition::<$key, $value>::new(name)) {
            Ok(table) => table
                .range::<std::ops::RangeFull, &$key>(..)
                .with_context(|| format!("iterating table {name}"))
                .map(|iter| {
                    Some(
                        iter.map(|(k, v)| (<[u8]>::to_vec(k.as_ref()), <[u8]>::to_vec(v.as_ref())))
                            .collect::<Vec<_>>(),
                    )
                }),
            Err(redb_0_9::Error::TableTypeMismatch(_)) => Ok(None),
            Err(err) => Err(err).with_context(|| format!("opening table {name}")),
        }
    }};
}

fn export(path: &str) -> anyhow::Result<Vec<ExportedTable>> {
    // SAFETY: the database file must not be modified concurrently by another process; this is a
    // private copy of the original database.
    let db = unsafe { redb_0_9::Database::open(path) }
        .with_context(|| format!("opening old database at {path}"))?;
    let txn = db.begin_read()?;

    let multimap_tables = txn.list_multimap_tables()?.collect::<Vec<_>>();
    if !multimap_tables.is_empty() {
        bail!("multimap tables aren't supported: {multimap_tables:?}");
    }

    let mut tables = Vec::new();

    for name in txn.list_tables()? {
        use Kind::*;

        let (key_kind, value_kind, entries) =
            if let Some(entries) = try_export!(txn, &name, [u8], [u8])? {
                (Bytes, Bytes, entries)
            } else if let Some(entries) = try_export!(txn, &name, str, [u8])? {
                (Str, Bytes, entries)
            } else if let Some(entries) = try_export!(txn, &name, str, str)? {
                (Str, Str, entries)
            } else if let Some(entries) = try_export!(txn, &name, [u8], str)? {
                (Bytes, Str, entries)
            } else {
                bail!("table {name} has an unsupported key/value type");
            };

        println!(
            "exported table {name:?} ({key_kind:?} -> {value_kind:?}): {} entries",
            entries.len()
        );

        tables.push(ExportedTable {
            name,
            key_kind,
            value_kind,
            entries,
        });
    }

    Ok(tables)
}

/// Inserts all the entries into the table `name`, with the given key/value types.
fn import_entries<K, V>(
    txn: &redb::WriteTransaction,
    name: &str,
    entries: &[(Vec<u8>, Vec<u8>)],
    to_key: impl for<'a> Fn(&'a [u8]) -> anyhow::Result<K::SelfType<'a>>,
    to_value: impl for<'a> Fn(&'a [u8]) -> anyhow::Result<V::SelfType<'a>>,
) -> anyhow::Result<()>
where
    K: redb::Key + 'static,
    V: redb::Value + 'static,
{
    let mut table = txn.open_table(redb::TableDefinition::<K, V>::new(name))?;
    for (k, v) in entries {
        table.insert(to_key(k)?, to_value(v)?)?;
    }
    Ok(())
}

fn as_bytes(b: &[u8]) -> anyhow::Result<&[u8]> {
    Ok(b)
}

fn as_str(b: &[u8]) -> anyhow::Result<&str> {
    Ok(std::str::from_utf8(b)?)
}

fn import(path: &str, tables: &[ExportedTable]) -> anyhow::Result<()> {
    let db =
        redb::Database::create(path).with_context(|| format!("creating new database at {path}"))?;

    let txn = db.begin_write()?;

    for table in tables {
        let name = &table.name;
        let entries = &table.entries;

        match (table.key_kind, table.value_kind) {
            (Kind::Bytes, Kind::Bytes) => {
                import_entries::<&[u8], &[u8]>(&txn, name, entries, as_bytes, as_bytes)
            }
            (Kind::Str, Kind::Bytes) => {
                import_entries::<&str, &[u8]>(&txn, name, entries, as_str, as_bytes)
            }
            (Kind::Str, Kind::Str) => {
                import_entries::<&str, &str>(&txn, name, entries, as_str, as_str)
            }
            (Kind::Bytes, Kind::Str) => {
                import_entries::<&[u8], &str>(&txn, name, entries, as_bytes, as_str)
            }
        }
        .with_context(|| format!("importing table {name}"))?;

        println!("imported table {name:?}: {} entries", entries.len());
    }

    txn.commit()?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let (Some(old_path), Some(new_path), None) = (args.next(), args.next(), args.next()) else {
        bail!("usage: migrate-redb <old.db> <new.db>");
    };

    if std::path::Path::new(&new_path).exists() {
        bail!("{new_path} already exists, refusing to overwrite it");
    }

    // redb 0.9.0 writes to the database file when opening it, so work on a copy to leave the
    // original untouched.
    let old_copy_path = format!("{new_path}.old-copy");
    std::fs::copy(&old_path, &old_copy_path)
        .with_context(|| format!("copying {old_path} to {old_copy_path}"))?;
    let tables = export(&old_copy_path);
    std::fs::remove_file(&old_copy_path)
        .with_context(|| format!("removing temporary copy {old_copy_path}"))?;
    let tables = tables?;

    import(&new_path, &tables)?;

    println!(
        "migrated {} tables from {old_path} to {new_path}",
        tables.len()
    );
    Ok(())
}
