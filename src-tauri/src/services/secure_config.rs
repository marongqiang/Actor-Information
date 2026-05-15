use crate::db;
use crate::utils::crypto;
use crate::utils::error::{CommandError, CommandResult};

/// Store a sensitive value (encrypted locally)
pub fn set_secure_config(key: &str, value: &str) -> CommandResult<()> {
    let machine_key = crypto::derive_machine_key();
    let encrypted = crypto::encrypt(value, &machine_key)
        .map_err(|e| CommandError::internal(&format!("加密配置失败: {}", e)))?;

    let now = db::now_ts();
    db::with_db(|conn| {
        conn.execute(
            "INSERT OR REPLACE INTO config (key, encrypted_value, use_system_credential, updated_at)
             VALUES (?1, ?2, 0, ?3)",
            rusqlite::params![key, encrypted.as_bytes(), now],
        )?;
        Ok(())
    })
}

/// Retrieve a sensitive value (decrypt from local storage)
pub fn get_secure_config(key: &str) -> CommandResult<Option<String>> {
    db::with_db(|conn| {
        let result: Result<Option<Vec<u8>>, rusqlite::Error> = conn.query_row(
            "SELECT encrypted_value FROM config WHERE key = ?1",
            [key],
            |row| row.get(0),
        );

        match result {
            Ok(Some(data)) => {
                let encrypted = String::from_utf8_lossy(&data).to_string();
                let machine_key = crypto::derive_machine_key();
                let decrypted = crypto::decrypt(&encrypted, &machine_key)
                    .map_err(|e| CommandError::internal(&format!("解密配置失败: {}", e)))?;
                Ok(Some(decrypted))
            }
            Ok(None) => Ok(None),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(CommandError::db(&format!("查询安全配置失败: {}", e))),
        }
    })
}

/// Clear a secure configuration value
pub fn clear_secure_config(key: &str) -> CommandResult<()> {
    db::with_db(|conn| {
        conn.execute(
            "UPDATE config SET encrypted_value = NULL, updated_at = ?1 WHERE key = ?2",
            rusqlite::params![db::now_ts(), key],
        )?;
        Ok(())
    })
}
