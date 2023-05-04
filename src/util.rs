use std::hash::{SipHasher, Hasher};

use crate::vm::VariableIdentifier;

#[inline(always)]
pub fn variable_name_to_identifier(name: String) -> VariableIdentifier
{
    // Case insensitive
    let processed_string = name.to_lowercase();

    // FIXME: Unstable feature here? We need to ensure the hash algorithm remains static
    let mut hasher = SipHasher::new();
    hasher.write(processed_string.as_bytes());
    return hasher.finish();
}