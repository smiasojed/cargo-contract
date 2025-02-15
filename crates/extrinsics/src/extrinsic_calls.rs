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
    CodeHash,
    DefaultConfig,
};
use crate::{
    upload::Determinism,
    WasmCode,
};
use subxt::{
    ext::{
        codec::Compact,
        scale_encode::EncodeAsType,
    },
    utils::MultiAddress,
    Config,
};

/// Copied from `sp_weight` to additionally implement `scale_encode::EncodeAsType`.
#[derive(Debug, EncodeAsType)]
#[encode_as_type(crate_path = "subxt::ext::scale_encode")]
pub(crate) struct Weight {
    #[codec(compact)]
    /// The weight of computational time used based on some reference hardware.
    ref_time: u64,
    #[codec(compact)]
    /// The weight of storage space used by proof of validity.
    proof_size: u64,
}

impl From<sp_weights::Weight> for Weight {
    fn from(weight: sp_weights::Weight) -> Self {
        Self {
            ref_time: weight.ref_time(),
            proof_size: weight.proof_size(),
        }
    }
}

impl core::fmt::Display for Weight {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Weight(ref_time: {}, proof_size: {})",
            self.ref_time, self.proof_size
        )
    }
}

/// A raw call to `pallet-contracts`'s `remove_code`.
#[derive(EncodeAsType)]
#[encode_as_type(crate_path = "subxt::ext::scale_encode")]
pub(crate) struct RemoveCode {
    code_hash: CodeHash,
}

impl RemoveCode {
    pub fn new(code_hash: CodeHash) -> Self {
        Self { code_hash }
    }

    pub fn build(self) -> subxt::tx::Payload<Self> {
        subxt::tx::Payload::new("Contracts", "remove_code", self)
    }
}

/// A raw call to `pallet-contracts`'s `upload_code`.
#[derive(Debug, EncodeAsType)]
#[encode_as_type(crate_path = "subxt::ext::scale_encode")]
pub(crate) struct UploadCode {
    code: Vec<u8>,
    storage_deposit_limit: Option<Compact<Balance>>,
    determinism: Determinism,
}

impl UploadCode {
    pub fn new(
        code: WasmCode,
        storage_deposit_limit: Option<Balance>,
        determinism: Determinism,
    ) -> Self {
        Self {
            code: code.0,
            storage_deposit_limit: storage_deposit_limit.map(Into::into),
            determinism,
        }
    }

    pub fn build(self) -> subxt::tx::Payload<Self> {
        subxt::tx::Payload::new("Contracts", "upload_code", self)
    }
}

/// A raw call to `pallet-contracts`'s `instantiate_with_code`.
#[derive(Debug, EncodeAsType)]
#[encode_as_type(crate_path = "subxt::ext::scale_encode")]
pub(crate) struct InstantiateWithCode {
    #[codec(compact)]
    value: Balance,
    gas_limit: Weight,
    storage_deposit_limit: Option<Compact<Balance>>,
    code: Vec<u8>,
    data: Vec<u8>,
    salt: Vec<u8>,
}

impl InstantiateWithCode {
    pub fn new(
        value: Balance,
        gas_limit: sp_weights::Weight,
        storage_deposit_limit: Option<Balance>,
        code: Vec<u8>,
        data: Vec<u8>,
        salt: Vec<u8>,
    ) -> Self {
        Self {
            value,
            gas_limit: gas_limit.into(),
            storage_deposit_limit: storage_deposit_limit.map(Into::into),
            code,
            data,
            salt,
        }
    }

    pub fn build(self) -> subxt::tx::Payload<Self> {
        subxt::tx::Payload::new("Contracts", "instantiate_with_code", self)
    }
}

/// A raw call to `pallet-contracts`'s `instantiate_with_code_hash`.
#[derive(Debug, EncodeAsType)]
#[encode_as_type(crate_path = "subxt::ext::scale_encode")]
pub(crate) struct Instantiate {
    #[codec(compact)]
    value: Balance,
    gas_limit: Weight,
    storage_deposit_limit: Option<Compact<Balance>>,
    code_hash: CodeHash,
    data: Vec<u8>,
    salt: Vec<u8>,
}

impl Instantiate {
    pub fn new(
        value: Balance,
        gas_limit: sp_weights::Weight,
        storage_deposit_limit: Option<Balance>,
        code_hash: CodeHash,
        data: Vec<u8>,
        salt: Vec<u8>,
    ) -> Self {
        Self {
            value,
            gas_limit: gas_limit.into(),
            storage_deposit_limit: storage_deposit_limit.map(Into::into),
            code_hash,
            data,
            salt,
        }
    }

    pub fn build(self) -> subxt::tx::Payload<Self> {
        subxt::tx::Payload::new("Contracts", "instantiate", self)
    }
}

/// A raw call to `pallet-contracts`'s `call`.
#[derive(EncodeAsType)]
#[encode_as_type(crate_path = "subxt::ext::scale_encode")]
pub(crate) struct Call {
    dest: MultiAddress<<DefaultConfig as Config>::AccountId, ()>,
    #[codec(compact)]
    value: Balance,
    gas_limit: Weight,
    storage_deposit_limit: Option<Compact<Balance>>,
    data: Vec<u8>,
}

impl Call {
    pub fn new(
        dest: MultiAddress<<DefaultConfig as Config>::AccountId, ()>,
        value: Balance,
        gas_limit: sp_weights::Weight,
        storage_deposit_limit: Option<Balance>,
        data: Vec<u8>,
    ) -> Self {
        Self {
            dest,
            value,
            gas_limit: gas_limit.into(),
            storage_deposit_limit: storage_deposit_limit.map(Into::into),
            data,
        }
    }

    pub fn build(self) -> subxt::tx::Payload<Self> {
        subxt::tx::Payload::new("Contracts", "call", self)
    }
}
