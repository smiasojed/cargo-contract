// Copyright 2018-2023 Parity Technologies (UK) Ltd.
// This file is part of cargo-contract.
//
// cargo-contract is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// cargo-contract is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with cargo-contract.  If not, see <http://www.gnu.org/licenses/>.

use super::{
    Balance,
    BalanceVariant,
    CodeHash,
    DefaultConfig,
    TokenMetadata,
};
use crate::DEFAULT_KEY_COL_WIDTH;
use colored::Colorize as _;
use contract_build::Verbosity;
use contract_transcode::{
    ContractMessageTranscoder,
    Hex,
    TranscoderBuilder,
    Value,
};

use anyhow::Result;
use scale_info::form::PortableForm;
use std::{
    fmt::Write,
    str::FromStr,
};
use subxt::{
    self,
    blocks::ExtrinsicEvents,
    events::StaticEvent,
    ext::{
        scale_decode,
        scale_encode,
    },
    Config,
};

/// A custom event emitted by the contract.
#[derive(
    scale::Decode,
    scale::Encode,
    scale_decode::DecodeAsType,
    scale_encode::EncodeAsType,
    Debug,
)]
#[decode_as_type(crate_path = "subxt::ext::scale_decode")]
#[encode_as_type(crate_path = "subxt::ext::scale_encode")]
pub struct ContractEmitted {
    pub contract: <DefaultConfig as Config>::AccountId,
    pub data: Vec<u8>,
}

impl StaticEvent for ContractEmitted {
    const PALLET: &'static str = "Contracts";
    const EVENT: &'static str = "ContractEmitted";
}

/// A contract was successfully instantiated.
#[derive(
    Debug,
    scale::Decode,
    scale::Encode,
    scale_decode::DecodeAsType,
    scale_encode::EncodeAsType,
)]
#[decode_as_type(crate_path = "subxt::ext::scale_decode")]
#[encode_as_type(crate_path = "subxt::ext::scale_encode")]
pub struct ContractInstantiated {
    /// Account id of the deployer.
    pub deployer: <DefaultConfig as Config>::AccountId,
    /// Account id where the contract was instantiated to.
    pub contract: <DefaultConfig as Config>::AccountId,
}

impl StaticEvent for ContractInstantiated {
    const PALLET: &'static str = "Contracts";
    const EVENT: &'static str = "Instantiated";
}

/// An event triggered by either the `instantiate_with_code` or the `upload_code` call.
#[derive(
    Debug,
    scale::Decode,
    scale::Encode,
    scale_decode::DecodeAsType,
    scale_encode::EncodeAsType,
)]
#[decode_as_type(crate_path = "subxt::ext::scale_decode")]
#[encode_as_type(crate_path = "subxt::ext::scale_encode")]
pub struct CodeStored {
    /// Hash under which the contract code was stored.
    pub code_hash: CodeHash,
}

impl StaticEvent for CodeStored {
    const PALLET: &'static str = "Contracts";
    const EVENT: &'static str = "CodeStored";
}

/// An event triggered by the `remove_code` call.
#[derive(
    Debug,
    scale::Decode,
    scale::Encode,
    scale_decode::DecodeAsType,
    scale_encode::EncodeAsType,
)]
#[decode_as_type(crate_path = "subxt::ext::scale_decode")]
#[encode_as_type(crate_path = "subxt::ext::scale_encode")]
pub struct CodeRemoved {
    pub code_hash: CodeHash,
    pub deposit_released: Balance,
    pub remover: <DefaultConfig as Config>::AccountId,
}

impl StaticEvent for CodeRemoved {
    const PALLET: &'static str = "Contracts";
    const EVENT: &'static str = "CodeRemoved";
}

/// Field that represent data of an event from invoking a contract extrinsic.
#[derive(serde::Serialize)]
pub struct Field {
    /// name of a field
    pub name: String,
    /// value of a field
    pub value: Value,
    /// The name of a type as defined in the pallet Source Code
    #[serde(skip_serializing)]
    pub type_name: Option<String>,
}

impl Field {
    pub fn new(name: String, value: Value, type_name: Option<String>) -> Self {
        Field {
            name,
            value,
            type_name,
        }
    }
}

/// An event produced from invoking a contract extrinsic.
#[derive(serde::Serialize)]
pub struct Event {
    /// name of a pallet
    pub pallet: String,
    /// name of the event
    pub name: String,
    /// data associated with the event
    pub fields: Vec<Field>,
}

/// Displays events produced from invoking a contract extrinsic.
#[derive(serde::Serialize)]
pub struct DisplayEvents(Vec<Event>);

impl DisplayEvents {
    /// Parses events and returns an object which can be serialised
    pub fn from_events(
        result: &ExtrinsicEvents<DefaultConfig>,
        transcoder: Option<&ContractMessageTranscoder>,
        subxt_metadata: &subxt::Metadata,
    ) -> Result<DisplayEvents> {
        let mut events: Vec<Event> = vec![];

        let events_transcoder = TranscoderBuilder::new(subxt_metadata.types())
            .with_default_custom_type_transcoders()
            .done();

        for event in result.iter() {
            let event = event?;
            tracing::debug!(
                "displaying event {}:{}",
                event.pallet_name(),
                event.variant_name()
            );

            let event_metadata = event.event_metadata();
            let event_fields = &event_metadata.variant.fields;

            let mut event_entry = Event {
                pallet: event.pallet_name().to_string(),
                name: event.variant_name().to_string(),
                fields: vec![],
            };

            let event_data = &mut event.field_bytes();
            let event_sig_topic = event.topics().iter().next();
            let mut unnamed_field_name = 0;
            for field_metadata in event_fields {
                if <ContractEmitted as StaticEvent>::is_event(
                    event.pallet_name(),
                    event.variant_name(),
                ) && field_metadata.name == Some("data".to_string())
                {
                    tracing::debug!("event data: {:?}", hex::encode(&event_data));
                    let field = contract_event_data_field(
                        transcoder,
                        field_metadata,
                        event_sig_topic,
                        event_data,
                    )?;
                    event_entry.fields.push(field);
                } else {
                    let field_name = field_metadata
                        .name
                        .as_ref()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| {
                            let name = unnamed_field_name.to_string();
                            unnamed_field_name += 1;
                            name
                        });

                    let decoded_field = events_transcoder.decode(
                        subxt_metadata.types(),
                        field_metadata.ty.id,
                        event_data,
                    )?;
                    let field = Field::new(
                        field_name,
                        decoded_field,
                        field_metadata.type_name.as_ref().map(|s| s.to_string()),
                    );
                    event_entry.fields.push(field);
                }
            }
            events.push(event_entry);
        }

        Ok(DisplayEvents(events))
    }

    /// Displays events in a human readable format
    pub fn display_events(
        &self,
        verbosity: Verbosity,
        token_metadata: &TokenMetadata,
    ) -> Result<String> {
        let event_field_indent: usize = DEFAULT_KEY_COL_WIDTH - 3;
        let mut out = format!(
            "{:>width$}\n",
            "Events".bright_purple().bold(),
            width = DEFAULT_KEY_COL_WIDTH
        );
        for event in &self.0 {
            let _ = writeln!(
                out,
                "{:>width$} {} ➜ {}",
                "Event".bright_green().bold(),
                event.pallet.bright_white(),
                event.name.bright_white().bold(),
                width = DEFAULT_KEY_COL_WIDTH
            );

            for field in &event.fields {
                if verbosity.is_verbose() {
                    let mut value: String = field.value.to_string();
                    if field.type_name == Some("T::Balance".to_string())
                        || field.type_name == Some("BalanceOf<T>".to_string())
                    {
                        if let Value::UInt(balance) = field.value {
                            value = BalanceVariant::from(balance, Some(token_metadata))?
                                .to_string();
                        }
                    }
                    let _ = writeln!(
                        out,
                        "{:width$}{}: {}",
                        "",
                        field.name.bright_white(),
                        value,
                        width = event_field_indent,
                    );
                }
            }
        }
        Ok(out)
    }

    /// Returns an event result in json format
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

/// Construct the contract event data field, attempting to decode the event using the
/// [`ContractMessageTranscoder`] if available.
fn contract_event_data_field(
    transcoder: Option<&ContractMessageTranscoder>,
    field_metadata: &scale_info::Field<PortableForm>,
    event_sig_topic: Option<&sp_core::H256>,
    event_data: &mut &[u8],
) -> Result<Field> {
    let event_value = if let Some(transcoder) = transcoder {
        if let Some(event_sig_topic) = event_sig_topic {
            match transcoder.decode_contract_event(event_sig_topic, event_data) {
                Ok(contract_event) => contract_event,
                Err(err) => {
                    tracing::warn!(
                        "Decoding contract event failed: {:?}. It might have come from another contract.",
                        err
                    );
                    Value::Hex(Hex::from_str(&hex::encode(&event_data))?)
                }
            }
        } else {
            tracing::info!("Anonymous event not decoded. Data displayed as raw hex.");
            Value::Hex(Hex::from_str(&hex::encode(event_data))?)
        }
    } else {
        Value::Hex(Hex::from_str(&hex::encode(event_data))?)
    };
    Ok(Field::new(
        String::from("data"),
        event_value,
        field_metadata.type_name.as_ref().map(|s| s.to_string()),
    ))
}
