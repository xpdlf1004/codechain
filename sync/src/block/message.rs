// Copyright 2018 Kodebox, Inc.
// This file is part of CodeChain.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use ccore::{BlockNumber, Header, UnverifiedTransaction};
use ctypes::{H256, U256};
use rlp::{Decodable, DecoderError, Encodable, RlpStream, UntrustedRlp};

const MESSAGE_ID_STATUS: u8 = 0x01;
const MESSAGE_ID_REQUEST_HEADERS: u8 = 0x02;
const MESSAGE_ID_HEADERS: u8 = 0x03;
const MESSAGE_ID_REQUEST_BODIES: u8 = 0x04;
const MESSAGE_ID_BODIES: u8 = 0x05;

#[derive(Debug, PartialEq)]
pub enum Message {
    Status {
        total_score: U256,
        best_hash: H256,
        genesis_hash: H256,
    },
    RequestHeaders {
        start_number: BlockNumber,
        max_count: u64,
    },
    Headers(Vec<Header>),
    RequestBodies(Vec<H256>),
    Bodies(Vec<Vec<UnverifiedTransaction>>),
}

impl Message {
    pub fn is_status(&self) -> bool {
        match self {
            &Message::Status {
                ..
            } => true,
            _ => false,
        }
    }
}

impl Encodable for Message {
    fn rlp_append(&self, s: &mut RlpStream) {
        s.begin_list(2);
        // add message id
        s.append(match self {
            &Message::Status {
                ..
            } => &MESSAGE_ID_STATUS,
            &Message::RequestHeaders {
                ..
            } => &MESSAGE_ID_REQUEST_HEADERS,
            &Message::Headers {
                ..
            } => &MESSAGE_ID_HEADERS,
            &Message::RequestBodies {
                ..
            } => &MESSAGE_ID_REQUEST_BODIES,
            &Message::Bodies {
                ..
            } => &MESSAGE_ID_BODIES,
        });
        // add body as rlp
        match self {
            &Message::Status {
                total_score,
                best_hash,
                genesis_hash,
            } => {
                s.begin_list(3);
                s.append(&total_score);
                s.append(&best_hash);
                s.append(&genesis_hash);
            }
            &Message::RequestHeaders {
                start_number,
                max_count,
            } => {
                s.begin_list(2);
                s.append(&start_number);
                s.append(&max_count);
            }
            &Message::Headers(ref headers) => {
                s.append_list(headers);
            }
            &Message::RequestBodies(ref hashes) => {
                s.append_list(hashes);
            }
            &Message::Bodies(ref bodies) => {
                s.begin_list(bodies.len());
                bodies.into_iter().for_each(|body| {
                    s.append_list(body);
                });
            }
        };
    }
}

impl Decodable for Message {
    fn decode(rlp: &UntrustedRlp) -> Result<Self, DecoderError> {
        if rlp.item_count()? != 2 {
            return Err(DecoderError::RlpIncorrectListLen)
        }
        let id = rlp.val_at(0)?;
        let message = rlp.at(1)?;
        Ok(match id {
            MESSAGE_ID_STATUS => {
                if message.item_count()? != 3 {
                    return Err(DecoderError::RlpIncorrectListLen)
                }
                Message::Status {
                    total_score: message.val_at(0)?,
                    best_hash: message.val_at(1)?,
                    genesis_hash: message.val_at(2)?,
                }
            }
            MESSAGE_ID_REQUEST_HEADERS => {
                if message.item_count()? != 2 {
                    return Err(DecoderError::RlpIncorrectListLen)
                }
                Message::RequestHeaders {
                    start_number: message.val_at(0)?,
                    max_count: message.val_at(1)?,
                }
            }
            MESSAGE_ID_HEADERS => Message::Headers(message.as_list()?),
            MESSAGE_ID_REQUEST_BODIES => Message::RequestBodies(message.as_list()?),
            MESSAGE_ID_BODIES => {
                let mut bodies = Vec::new();
                for item in message.into_iter() {
                    bodies.push(item.as_list()?);
                }
                Message::Bodies(bodies)
            }
            _ => return Err(DecoderError::Custom("Unknown message id detected")),
        })
    }
}

#[cfg(test)]
mod tests {
    use ccore::Header;
    use ctypes::{H256, U256};
    use rlp::Encodable;

    use super::Message;

    #[test]
    fn test_status_message_rlp() {
        let message = Message::Status {
            total_score: U256::default(),
            best_hash: H256::default(),
            genesis_hash: H256::default(),
        };
        assert_eq!(message, ::rlp::decode(message.rlp_bytes().as_ref()));
    }

    #[test]
    fn test_request_headers_message_rlp() {
        let message = Message::RequestHeaders {
            start_number: 100,
            max_count: 100,
        };
        assert_eq!(message, ::rlp::decode(message.rlp_bytes().as_ref()));
    }

    #[test]
    fn test_headers_message_rlp() {
        let headers = vec![Header::default()];
        headers.iter().for_each(|header| {
            header.hash();
        });

        let message = Message::Headers(headers);
        assert_eq!(message, ::rlp::decode(message.rlp_bytes().as_ref()));
    }

    #[test]
    fn test_request_bodies_message_rlp() {
        let message = Message::RequestBodies(vec![H256::default()]);
        assert_eq!(message, ::rlp::decode(message.rlp_bytes().as_ref()));
    }

    #[test]
    fn test_bodies_message_rlp() {
        let message = Message::Bodies(vec![vec![]]);
        assert_eq!(message, ::rlp::decode(message.rlp_bytes().as_ref()));
    }
}