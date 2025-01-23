use anyhow::Context as _;

mod wit {
    wit_bindgen::generate!("kv-world" in "../../wit/kv.wit");
    pub use self::trinity::api::kv::*;
}

pub fn get<K: serde::Serialize + ?Sized, V: for<'a> serde::Deserialize<'a>>(
    key: &K,
) -> anyhow::Result<Option<V>> {
    let key = serde_json::to_vec(key).context("couldn't serialize get key")?;
    let val = wit::get(&key)?;
    if let Some(val) = val {
        let deser = serde_json::from_slice(&val).context("couldn't deserialize get value")?;
        Ok(Some(deser))
    } else {
        Ok(None)
    }
}

pub fn remove<T: serde::Serialize + ?Sized>(key: &T) -> anyhow::Result<()> {
    let key = serde_json::to_vec(key).context("couldn't serialize remove key")?;
    wit::remove(&key)?;
    Ok(())
}

pub fn set<T: serde::Serialize + ?Sized, V: serde::Serialize + ?Sized>(
    key: &T,
    val: &V,
) -> anyhow::Result<()> {
    let key = serde_json::to_vec(key).context("couldn't serialize set key")?;
    let val = serde_json::to_vec(val).context("couldn't serialize set value")?;
    wit::set(&key, &val)?;
    Ok(())
}
