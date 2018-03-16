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

use super::{SoloAuthority, Tendermint};

/// Engine deserialization.
#[derive(Debug, PartialEq, Deserialize)]
pub enum Engine {
    #[serde(rename="solo")]
    Solo,
    #[serde(rename="soloAuthority")]
    SoloAuthority(SoloAuthority),
    #[serde(rename="tendermint")]
    Tendermint(Tendermint)
}

#[cfg(test)]
mod tests {
    use serde_json;
    use super::Engine;

    #[test]
    fn engine_deserialization() {
        let s = r#"{
			"solo": null
		}"#;

        let deserialized: Engine = serde_json::from_str(s).unwrap();
        match deserialized {
            Engine::Solo => {},	// solo is unit tested in its own file.
            _ => panic!(),
        };

        let s = r#"{
			"soloAuthority": {
				"params": {
					"durationLimit": "0x0d",
					"validators" : ["0xc6d9d2cd449a754c494264e1809c50e34d64562b"]
				}
			}
		}"#;
        let deserialized: Engine = serde_json::from_str(s).unwrap();
        match deserialized {
            Engine::SoloAuthority(_) => {}, // solo authority is unit tested in its own file.
            _ => panic!(),
        };

        let s = r#"{
			"tendermint": {
				"params": {
					"validators": ["0xc6d9d2cd449a754c494264e1809c50e34d64562b"]
				}
			}
		}"#;
        let deserialized: Engine = serde_json::from_str(s).unwrap();
        match deserialized {
            Engine::Tendermint(_) => {}, // Tendermint is unit tested in its own file.
            _ => panic!(),
        };
    }
}