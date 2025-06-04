static REGISTRY_KEY_PATH: &str = "SOFTWARE\\SecureLink";
static REGISTRY_AUTH_TOKEN_VALUE: &str = "Auth Token";

use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::types::ToRegValue;
use winreg::RegKey;
fn get_service_reg_key() -> Result<RegKey, Box<dyn std::error::Error>> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let key = hklm.create_subkey(REGISTRY_KEY_PATH)?.0;

    Ok(key)
}

fn store_entry_in_registry<T: ToRegValue>(
    key: &str,
    entry: &T,
) -> Result<(), Box<dyn std::error::Error>> {
    let reg_key = get_service_reg_key()?;
    reg_key.set_value(key, entry)?;
    Ok(())
}

fn load_optional_entry_from_registry<T: winreg::types::FromRegValue>(
    key: &str,
) -> Result<Option<T>, Box<dyn std::error::Error>> {
    let reg_key = get_service_reg_key()?;
    match reg_key.get_value::<T, _>(key) {
        Ok(value) => Ok(Some(value)),
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Box::new(e)),
    }
}
pub fn load_auth_token() -> Result<Option<String>, Box<dyn std::error::Error>> {
    Ok(load_optional_entry_from_registry(
        REGISTRY_AUTH_TOKEN_VALUE,
    )?)
}

pub fn store_auth_token(auth_token: &str) -> Result<(), Box<dyn std::error::Error>> {
    store_entry_in_registry::<String>(REGISTRY_AUTH_TOKEN_VALUE, &auth_token.to_string())?;
    Ok(())
}
