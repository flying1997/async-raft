use std::{num, str::FromStr};

use thiserror::Error;
/// /127.0.0.1:60001/0
pub struct NetworkAddress{
    id: u64, 
    address: String,
}
impl NetworkAddress{
    pub fn new(vec: Vec<String>) -> Self{
        Self{
            id: vec[1].parse::<u64>().unwrap(),
            address: vec[0].clone(),
        }
    }
    pub fn get_id(&self) -> u64{
        self.id
    }
    pub fn get_address(&self) -> String{
        self.address.clone()
    }
}
impl FromStr for NetworkAddress{
    type Err = ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(ParseError::EmptyProtocolString);
        }

        let mut protocols :Vec<String> = Vec::new();
        let mut parts_iter = s.split('/');

        // the first character must be '/'
        if parts_iter.next() != Some("") {
            return Err(ParseError::InvalidProtocolString);
        }

        // parse all `Protocol`s
        while let Some(s) = parts_iter.next() {
            protocols.push(String::from(s));
        }

        Ok(NetworkAddress::new(protocols))
    }
}

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("unexpected end of protocol string")]
    UnexpectedEnd,
    #[error("unknown protocol type: '{0}'")]
    UnknownProtocolType(String),
    #[error("network address cannot be empty")]
    EmptyProtocolString,
    #[error("protocol string must start with '/'")]
    InvalidProtocolString,
    #[error("Network address must have node id.")]
    MissingNodeId,
    #[error("error parsing integer: {0}")]
    ParseIntError(#[from] num::ParseIntError),
}