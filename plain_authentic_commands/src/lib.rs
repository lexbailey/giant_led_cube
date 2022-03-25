#[cfg(feature="challenge")]
use rand_core::{RngCore, SeedableRng, CryptoRng};
#[cfg(feature="challenge")]
use rand_chacha::{ChaCha20Rng};

extern crate pest;
#[macro_use]
extern crate pest_derive;
use pest::Parser;

use sha2::Sha256;
use hmac::{Hmac, Mac};
type HmacSha256 = Hmac<Sha256>;

#[derive(Parser)]
#[grammar = "command_parser.pest"]
pub struct CommandParser;

#[cfg(feature="challenge")]
pub struct GenericMessageHandler<Rng: RngCore + SeedableRng + CryptoRng>{
    rng: Rng
    ,salt: String
    ,secret: Vec<u8>
}
    
#[cfg(feature="challenge")]
pub type MessageHandler = GenericMessageHandler<ChaCha20Rng>;

#[cfg(not (feature="challenge"))]
pub struct GenericMessageHandler{
    salt: String
    ,secret: Vec<u8>
}

#[cfg(not (feature="challenge"))]
pub type MessageHandler = GenericMessageHandler;

#[cfg(feature="challenge")]
impl<Rng: RngCore + SeedableRng + CryptoRng> GenericMessageHandler<Rng>{
    pub fn new(secret: Vec<u8>) -> GenericMessageHandler<Rng>{
        let mut a = GenericMessageHandler{
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

pub enum ParseStatus {
    Success(String, Vec<String>)
    ,BadClient()
    ,Unauthorised()
}

impl MessageHandler{
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

    pub fn construct_reply(&self, command: &str, args: &Vec<&str>) -> String{
        "+".to_string() + &self.construct_message(command, args)
    }

    pub fn signing_only(secret: Vec<u8>) -> MessageHandler {
        #[cfg(feature="challenge")]
        {Self::new(secret)}
        #[cfg(not(feature="challenge"))]
        {MessageHandler{
            salt: "".to_string()
            ,secret: secret
        }}
    }

    #[cfg(test)]
    pub fn testing_only_update_salt(&mut self, salt: String){
        self.salt = salt;
    }

    pub fn get_salt(&self) -> &str{
        &self.salt
    }

    fn parse_message(&mut self, cmd: &Vec<u8>, is_reply: bool) -> ParseStatus{
        let s = std::str::from_utf8(cmd);
        if let Ok(cmd) = s {
            let result = if is_reply {
                CommandParser::parse(Rule::response, cmd)
            }
            else{
                CommandParser::parse(Rule::command, cmd)
            };
            match result{
                Err(cmd) => {
                    println!("Unparesable: Bad syntax: {}", cmd.to_string());
                    ParseStatus::BadClient()
                }
                ,Ok(cmd) => {
                    for c in cmd{
                        let mut c = c.into_inner();
                        let checked = c.next().unwrap();
                        let checked_string = checked.as_str();
                        let mut checked = checked.into_inner();
                        let command_name = checked.next().unwrap().as_str();
                        let command_args: Vec<String> = checked.next().unwrap().into_inner().map(|a|a.as_str().to_string()).collect();
                        let msg_salt = checked.next().unwrap().as_str();
                        let mac = c.next().unwrap().as_str();
                        return if !is_reply && command_name == "next_challenge" {
                            // Don't check if this is authentic, challenges can be requested by anyone
                            ParseStatus::Success(command_name.to_string(), command_args)
                        }
                        else {
                            if is_reply && command_name == "challenge" && command_args.len() >= 1 {
                                self.salt = command_args[0].clone();
                            }
                            if self.command_is_authentic(checked_string, msg_salt, mac){
                                ParseStatus::Success(command_name.to_string(), command_args)
                            }
                            else{
                                ParseStatus::Unauthorised()
                            }
                        }
                    }
                    // Strictly speaking this is an internal error, and this should be an unreachable!() instead of a BadClient
                    // but since this code is invoked by a network request, we don't want to panic if there's a bug here.
                    // Instead we just pretend the client is bad (which tbf it probably is if it exploited a bug in the server)
                    ParseStatus::BadClient()
                }
            }
        }
        else{
            println!("Unparseable: Command is not valid UTF8.");
            ParseStatus::BadClient()
        }
    }

    pub fn parse_command(&mut self, cmd: &Vec<u8>) -> ParseStatus{
        self.parse_message(cmd, false)
    }

    pub fn parse_response(&mut self, cmd: &Vec<u8>) -> ParseStatus{
        self.parse_message(cmd, true)
    }

}

#[cfg(test)]
mod tests {
    use crate::MessageHandler;

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
        let auth1 = MessageHandler::new(b"Secret key".to_vec());
        let auth2 = MessageHandler::new(b"Secret key".to_vec());
        let msg = auth1.construct_message("test", &vec!["arg1", "arg2"]);
        let (checked_msg, salt, mac) = auth_code_parts(&msg, "test:arg1,arg2,");
        assert!(!auth2.command_is_authentic(checked_msg, salt, mac));
    }

    #[cfg(feature="challenge")]
    #[test]
    fn auth() {
        // a MessageHandler should always be in sync with itself
        let auth = MessageHandler::new(b"Secret key".to_vec());
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
        let mut auth = MessageHandler::signing_only(b"Secret key".to_vec());
        auth.testing_only_update_salt(salt.to_string());
        let result = auth.construct_message("test", &vec!["arg1", "arg2"]);
        let expected = "test:arg1,arg2,e6a7826851ce2d9a#77bfd3c375c76352ca52e0c6324637ee728d1fae65ab65aee9f3cd9aa3529f4d\n";
        assert_eq!(&result, expected);
    }
}
