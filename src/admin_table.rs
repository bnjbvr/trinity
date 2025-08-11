use redb::ReadableTable;

use crate::ShareableDatabase;

/// Name of the admin table. Can be kept internal.
const ADMIN_TABLE: redb::TableDefinition<str, [u8]> = redb::TableDefinition::new("@admin");

/// Key for the `device_id` value in the admin table.
pub const DEVICE_ID_ENTRY: &str = "device_id";

/// Key for the `version` value in the admin table.
pub const VERSION_ENTRY: &str = "version";

/// Reads a given key in the admin table from the database.
///
/// Returns `Ok(None)` if the value wasn't present, `Ok(Some)` if it did exist.
pub fn read(db: &ShareableDatabase, key: &str) -> anyhow::Result<Option<Vec<u8>>> {
    let txn = db.begin_read()?;
    let table = match txn.open_table(ADMIN_TABLE) {
        Ok(table) => table,
        Err(err) => match err {
            redb::Error::DatabaseAlreadyOpen
            | redb::Error::InvalidSavepoint
            | redb::Error::Corrupted(_)
            | redb::Error::TableTypeMismatch(_)
            | redb::Error::DbSizeMismatch { .. }
            | redb::Error::TableAlreadyOpen(_, _)
            | redb::Error::OutOfSpace
            | redb::Error::Io(_)
            | redb::Error::LockPoisoned(_) => Err(err)?,
            redb::Error::TableDoesNotExist(_) => return Ok(None),
        },
    };
    Ok(table.get(key)?.map(|val| val.to_vec()))
}

/// Same as [`read`], but for a string value.
pub fn read_str(db: &ShareableDatabase, key: &str) -> anyhow::Result<Option<String>> {
    match read(db, key)? {
        Some(bytes) => Ok(Some(String::from_utf8(bytes)?)),
        None => Ok(None),
    }
}

/// Same as [`read`], but for a u64 value.
pub fn read_u64(db: &ShareableDatabase, key: &str) -> anyhow::Result<Option<u64>> {
    match read(db, key)? {
        Some(bytes) if bytes.len() == 8 => Ok(Some(u64::from_le_bytes(
            bytes.as_slice().try_into().unwrap(),
        ))),
        Some(_) => Err(anyhow::anyhow!(
            "Value for key '{}' is not a valid u64",
            key
        )),
        None => Ok(None),
    }
}

/// Writes a given key in the admin table from the database.
pub fn write(db: &ShareableDatabase, key: &str, value: &[u8]) -> anyhow::Result<()> {
    let txn = db.begin_write()?;
    {
        let mut table = txn.open_table(ADMIN_TABLE)?;
        table.insert(key, value)?;
    }
    txn.commit()?;
    Ok(())
}

/// Same as [`write`], but for a string ref.
pub fn write_str(db: &ShareableDatabase, key: &str, value: &str) -> anyhow::Result<()> {
    write(db, key, value.as_bytes())
}

/// Same as [`write`], but for a u64.
pub fn write_u64(db: &ShareableDatabase, key: &str, value: u64) -> anyhow::Result<()> {
    write(db, key, &value.to_le_bytes())
}

pub fn remove(db: &ShareableDatabase, key: &str) -> anyhow::Result<()> {
    let txn = db.begin_write()?;
    {
        let mut table = txn.open_table(ADMIN_TABLE)?;
        table.remove(key)?;
    }
    txn.commit()?;
    Ok(())
}
