use keyring::Entry;

const SERVICE_NAME: &str = "ai-novel-factory";

pub fn store_api_key(provider: &str, key: &str) -> Result<(), String> {
    let entry =
        Entry::new(SERVICE_NAME, provider).map_err(|e| format!("Keychain access error: {}", e))?;
    entry
        .set_secret(key.as_bytes())
        .map_err(|e| format!("Cannot store API key: {}", e))?;
    Ok(())
}

pub fn get_api_key(provider: &str) -> Result<String, String> {
    let entry =
        Entry::new(SERVICE_NAME, provider).map_err(|e| format!("Keychain access error: {}", e))?;
    let secret = entry.get_secret().map_err(|e| {
        format!(
            "Cannot retrieve API key for {}: {}. Go to Settings > Model Provider to set it.",
            provider, e
        )
    })?;
    String::from_utf8(secret).map_err(|e| format!("Invalid UTF-8 in stored key: {}", e))
}

pub fn delete_api_key(provider: &str) -> Result<(), String> {
    let entry =
        Entry::new(SERVICE_NAME, provider).map_err(|e| format!("Keychain access error: {}", e))?;
    entry
        .delete_credential()
        .map_err(|e| format!("Cannot delete API key: {}", e))?;
    Ok(())
}

pub fn mask_key(key: &str) -> String {
    if key.len() <= 10 {
        return "***".to_string();
    }
    format!("{}****{}", &key[..3], &key[key.len() - 4..])
}
