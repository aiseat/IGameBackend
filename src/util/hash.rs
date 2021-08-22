use blake3::{Hash, Hasher};
use rand::Rng;
use std::convert::TryInto;

use crate::error::ResponseError;

pub fn hash_password(password: &str) -> Vec<u8> {
    let salt = &rand::thread_rng().gen::<[u8; 32]>();

    let mut byted_password: Vec<u8> = Vec::new();
    byted_password.extend_from_slice(salt);
    byted_password.extend_from_slice(password.as_bytes());

    let mut hasher = Hasher::new();
    hasher.update(byted_password.as_slice());
    let hash = hasher.finalize();
    let hashed_password: &[u8; 32] = hash.as_bytes();

    let mut salted_password: Vec<u8> = Vec::new();
    salted_password.extend_from_slice(salt);
    salted_password.extend_from_slice(hashed_password);
    salted_password
}

pub fn compare_password(
    input_password: &str,
    salted_password: &Vec<u8>,
) -> Result<bool, ResponseError> {
    if salted_password.len() != 64 {
        return Err(ResponseError::unexpected_err(
            "比较密码失败",
            "salted_password的长度应该等于64比特",
        ));
    }
    let (salt, right) = salted_password.split_at(32);
    let hashed_password: [u8; 32] = right
        .try_into()
        .map_err(|e| ResponseError::unexpected_err("比较密码失败", &format!("{}", e)))?;
    let hash2 = Hash::from(hashed_password);

    let mut hasher = Hasher::new();
    let mut byted_password: Vec<u8> = Vec::new();
    byted_password.extend_from_slice(salt);
    byted_password.extend_from_slice(input_password.as_bytes());
    hasher.update(byted_password.as_slice());
    let hash1 = hasher.finalize();

    Ok(hash1 == hash2)
}
