#[cfg(feature="challenge")]
use rand_core::{RngCore, SeedableRng, CryptoRng};
#[cfg(feature="challenge")]
use rand_chacha::{ChaCha20Rng};

extern crate pest;
#[macro_use]
extern crate pest_derive;

use sha2::Sha256;
use hmac::{Hmac, Mac};
type HmacSha256 = Hmac<Sha256>;

#[derive(Parser)]
#[grammar = "command_parser.pest"]
pub struct CommandParser;

#[cfg(feature="challenge")]
pub struct GenericAuthState<Rng: RngCore + SeedableRng + CryptoRng>{
    rng: Rng
    ,salt: String
    ,secret: Vec<u8>
}
    
#[cfg(feature="challenge")]
pub type AuthState = GenericAuthState<ChaCha20Rng>;

#[cfg(not (feature="challenge"))]
pub struct GenericAuthState{
    salt: String
    ,secret: Vec<u8>
}

#[cfg(not (feature="challenge"))]
pub type AuthState = GenericAuthState;

#[cfg(feature="challenge")]
impl<Rng: RngCore + SeedableRng + CryptoRng> GenericAuthState<Rng>{
    pub fn new(secret: Vec<u8>) -> GenericAuthState<Rng>{
        let mut a = GenericAuthState{
            rng: Rng::from_entropy()
            ,salt: "".to_string()
            ,secret: secret
        };
        a.step();
        a
    }

    pub fn step(&mut self){
        let mut bytes = [0;8];
        self.rng.fill_bytes(&mut bytes);
        self.salt = hex::encode(bytes);
    }
}

impl AuthState{
    pub fn command_is_authentic(&self, command: &str, msg_salt: &str, given_mac: &str) -> bool{
        if self.salt != msg_salt{ return false; } // Didn't use the most recent challenge value
        let given_mac = hex::decode(given_mac);
        if given_mac.is_err(){return false;} // Mac is not even a valid hex string
        let mut mac = HmacSha256::new_from_slice(&self.secret).unwrap();
        mac.update(command.as_bytes());
        mac.verify_slice(&given_mac.unwrap()).is_ok()
    }
    
    fn authenticate_message(&self, message: &str) -> String{
        let mut mac = HmacSha256::new_from_slice(&self.secret).unwrap();
        mac.update(message.as_bytes());
        mac.update(self.salt.as_bytes());
        let sig = hex::encode(mac.finalize().into_bytes());
        format!("{}{}#{}\n", message, self.salt, sig)
    }
    
    pub fn construct_message(&self, command: &str, args: &Vec<&str>) -> String{
        let mut arg_list = String::new();
        for a in args{
            arg_list.push_str(a);
            arg_list.push(',');
        }
        let message = format!("{}:{}", command, arg_list);
        self.authenticate_message(&message)
    }

    pub fn signing_only(secret: Vec<u8>) -> AuthState {
        #[cfg(feature="challenge")]
        {Self::new(secret)}
        #[cfg(not(feature="challenge"))]
        {AuthState{
            salt: ""
            ,secret: secret
        }}
    }

    pub fn update_salt(&mut self, salt: String){
        self.salt = salt;
    }
}

#[cfg(test)]
mod tests {
    use crate::AuthState;

    fn auth_code_parts<'a>(msg: &'a String, expected_start: &str) -> (&'a str, &'a str, &'a str){
        let start = expected_start.to_string();
        assert_eq!(msg[0..start.len()], start);
        let auth_codes = &msg[start.len()..msg.len()];
        let checked_msg = &msg[0..start.len()+16];
        let salt = &auth_codes[0..16];
        let mac = &auth_codes[17..auth_codes.len()-1];
        (checked_msg, salt, mac)
    }

    #[cfg(feature="challenge")]
    #[test]
    fn differently_initialised_fails_auth() {
        // Initialise two authstates, see that they do not produce authentic messages (because the salt is randomly initialised)
        // even though the key is the same, they are out of sync with the salt value
        let auth1 = AuthState::new(b"Secret key".to_vec());
        let auth2 = AuthState::new(b"Secret key".to_vec());
        let msg = auth1.construct_message("test", &vec!["arg1", "arg2"]);
        let (checked_msg, salt, mac) = auth_code_parts(&msg, "test:arg1,arg2,");
        assert!(!auth2.command_is_authentic(checked_msg, salt, mac));
    }

    #[cfg(feature="challenge")]
    #[test]
    fn auth() {
        // an AuthState should always be in sync with itself
        let auth = AuthState::new(b"Secret key".to_vec());
        let msg = auth.construct_message("test", &vec!["arg1", "arg2"]);
        let (checked_msg, salt, mac) = auth_code_parts(&msg, "test:arg1,arg2,");
        assert!(auth.command_is_authentic(checked_msg, salt, mac));
        let mut auth = auth;
        auth.step();
        let msg = auth.construct_message("test2", &vec!["arg1", "arg2"]);
        let (checked_msg, salt, mac) = auth_code_parts(&msg, "test2:arg1,arg2,");
        assert!(auth.command_is_authentic(checked_msg, salt, mac));
    }

    #[test]
    fn sign_with_given_challenge(){
        let salt = "e6a7826851ce2d9a";
        let mut auth = AuthState::signing_only(b"Secret key".to_vec());
        auth.update_salt(salt.to_string());
        let result = auth.construct_message("test", &vec!["arg1", "arg2"]);
        let expected = "test:arg1,arg2,e6a7826851ce2d9a#77bfd3c375c76352ca52e0c6324637ee728d1fae65ab65aee9f3cd9aa3529f4d\n";
        assert_eq!(&result, expected);
    }
}
