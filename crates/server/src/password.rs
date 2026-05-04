use argon2::password_hash::{
    rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use argon2::Argon2;

const ARGON2_PREFIX: &str = "$argon2";

pub fn hash_password(password: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| format!("failed to hash password: {error}"))
}

pub fn verify_password(password: &str, password_hash: &str) -> bool {
    let Ok(parsed_hash) = PasswordHash::new(password_hash) else {
        return false;
    };

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok()
}

pub fn is_password_hash(value: &str) -> bool {
    value.starts_with(ARGON2_PREFIX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_verify_only_the_original_password() {
        let hash = hash_password("correct horse battery staple").expect("hash password");

        assert!(is_password_hash(&hash));
        assert!(verify_password("correct horse battery staple", &hash));
        assert!(!verify_password("wrong password", &hash));
        assert_ne!(hash, "correct horse battery staple");
    }
}
