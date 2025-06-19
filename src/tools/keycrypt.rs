use magic_crypt::{new_magic_crypt, MagicCryptTrait};
use serde::{Deserialize, Serialize};

pub fn keycrypt() {
    

    let keycrypt = new_magic_crypt!("rust", 256);
    let encrypted = keycrypt.encrypt_str_to_base64("himitsudayo");
    let decrypted = keycrypt.decrypt_base64_to_string(&encrypted).unwrap();
    // println!("Encrypted: {}", encrypted);
    // println!("Decrypted: {}", decrypted);
}

pub fn checkvalid(target: String) -> bool {
    let mc = new_magic_crypt!("rust", 256);
    mc.decrypt_base64_to_string(target).unwrap_or_else(|err| {
        println!("{}", err);
        String::new()
    }) == "himitsudayo"
}
