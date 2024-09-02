use std::{fmt, str::FromStr};
use borsh::{BorshDeserialize, BorshSerialize};
use hex::FromHex;
use near_primitives::types::AccountId;
use serde::{de::Visitor, Deserialize, Serialize};

#[derive(BorshDeserialize, BorshSerialize, Debug, Clone, PartialEq, Eq)]
pub struct H160(pub [u8; 20]);

impl FromStr for H160 {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = if let Some(stripped) = s.strip_prefix("0x") {
            stripped
        } else {
            s
        };
        let result = Vec::from_hex(s).map_err(|err| err.to_string())?;
        Ok(H160(
            result
                .try_into()
                .map_err(|err| format!("Invalid length: {err:?}"))?,
        ))
    }
}

impl fmt::Display for H160 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "0x{}", hex::encode(self.0))
    }
}

impl<'de> Deserialize<'de> for H160 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
    {
        struct HexVisitor;

        impl<'de> Visitor<'de> for HexVisitor {
            type Value = H160;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a hex string")
            }

            fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
            {
                Ok(s.parse().map_err(serde::de::Error::custom)?)
            }
        }

        deserializer.deserialize_str(HexVisitor)
    }
}

impl Serialize for H160 {
    fn serialize<S>(
        &self,
        serializer: S,
    ) -> Result<<S as serde::Serializer>::Ok, <S as serde::Serializer>::Error>
        where
            S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[derive(Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum ChainKind {
    Eth,
    Near,
    Sol,
}

impl From<&OmniAddress> for ChainKind {
    fn from(input: &OmniAddress) -> Self {
        match input {
            OmniAddress::Eth(_) => ChainKind::Eth,
            OmniAddress::Near(_) => ChainKind::Near,
            OmniAddress::Sol(_) => ChainKind::Sol,
        }
    }
}

pub type EthAddress = H160;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum OmniAddress {
    Eth(EthAddress),
    Near(String),
    Sol(String),
}

impl OmniAddress {
    pub fn get_chain(&self) -> ChainKind {
        match self {
            OmniAddress::Eth(_) => ChainKind::Eth,
            OmniAddress::Near(_) => ChainKind::Near,
            OmniAddress::Sol(_) => ChainKind::Sol,
        }
    }
}

impl FromStr for OmniAddress {
    type Err = ();

    fn from_str(input: &str) -> Result<OmniAddress, Self::Err> {
        let (chain, recipient) = input.split_once(':').ok_or(())?;

        match chain {
            "eth" => Ok(OmniAddress::Eth(recipient.parse().map_err(|_| ())?)),
            "near" => Ok(OmniAddress::Near(recipient.to_owned())),
            "sol" => Ok(OmniAddress::Sol(recipient.to_owned())), // TODO validate sol address
            _ => Err(()),
        }
    }
}

impl fmt::Display for OmniAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (chain_str, recipient) = match self {
            OmniAddress::Eth(recipient) => ("eth", recipient.to_string()),
            OmniAddress::Near(recipient) => ("near", recipient.to_string()),
            OmniAddress::Sol(recipient) => ("sol", recipient.clone()),
        };
        write!(f, "{}:{}", chain_str, recipient)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TransferMessagePayload {
    pub nonce: u128,
    pub token: AccountId,
    pub amount: u128,
    pub recipient: OmniAddress,
    pub relayer: Option<OmniAddress>,
}