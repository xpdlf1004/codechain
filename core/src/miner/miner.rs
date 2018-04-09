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

use std::sync::Arc;

use cbytes::Bytes;
use ckeys::Private;
use ctypes::{Address, U256};
use parking_lot::RwLock;

use super::super::client::{AccountData, BlockChain, MiningBlockChainClient};
use super::super::consensus::CodeChainEngine;
use super::super::error::Error;
use super::super::state::State;
use super::super::transaction::{SignedTransaction, TransactionError, UnverifiedTransaction};
use super::super::types::TransactionId;
use super::transaction_queue::{AccountDetails, TransactionDetailsProvider as TransactionQueueDetailsProvider, TransactionOrigin, TransactionQueue};
use super::{MinerService, MinerStatus, TransactionImportResult};

pub struct Miner {
    transaction_queue: Arc<RwLock<TransactionQueue>>,
    author: RwLock<Address>,
    extra_data: RwLock<Bytes>,
    engine: Arc<CodeChainEngine>,
}

impl Miner {
    fn add_transactions_to_queue<C: AccountData + BlockChain>(
        &self,
        client: &C,
        transactions: Vec<UnverifiedTransaction>,
        default_origin: TransactionOrigin,
        transaction_queue: &mut TransactionQueue,
    ) -> Vec<Result<TransactionImportResult, Error>> {
        let best_block_header = client.best_block_header().decode();
        let insertion_time = client.chain_info().best_block_number;
        let mut inserted = Vec::with_capacity(transactions.len());

        let results = transactions
            .into_iter()
            .map(|tx| {
                let hash = tx.hash();
                if client.transaction_block(TransactionId::Hash(hash)).is_some() {
                    debug!(target: "miner", "Rejected tx {:?}: already in the blockchain", hash);
                    return Err(Error::Transaction(TransactionError::AlreadyImported))
                }
                match self.engine
                    .verify_transaction_basic(&tx, &best_block_header)
                    .and_then(|_| self.engine.verify_transaction_unordered(tx, &best_block_header))
                {
                    Err(e) => {
                        debug!(target: "miner", "Rejected tx {:?} with invalid signature: {:?}", hash, e);
                        Err(e)
                    }
                    Ok(transaction) => {
                        // This check goes here because verify_transaction takes SignedTransaction parameter
                        self.engine.machine().verify_transaction(&transaction, &best_block_header, client)?;

                        // FIXME: Determine the origin from transaction.sender().
                        let origin = default_origin;
                        let details_provider = TransactionDetailsProvider::new(client);
                        let hash = transaction.hash();
                        let result = transaction_queue.add(transaction, origin, insertion_time, &details_provider)?;

                        inserted.push(hash);
                        Ok(result)
                    }
                }
            })
            .collect();

        results
    }
}

impl MinerService for Miner {
    type State = State<::state_db::StateDB>;

    fn status(&self) -> MinerStatus {
        let status = self.transaction_queue.read().status();
        MinerStatus {
            transactions_in_pending_queue: status.pending,
            transactions_in_future_queue: status.future,
            // FIXME: Fill in transactions_in_pending_block.
            transactions_in_pending_block: 0,
        }
    }

    fn author(&self) -> Address {
        *self.author.read()
    }

    fn set_author(&self, author: Address) {
        *self.author.write() = author;
    }

    fn extra_data(&self) -> Bytes {
        self.extra_data.read().clone()
    }

    fn set_extra_data(&self, extra_data: Bytes) {
        *self.extra_data.write() = extra_data;
    }

    fn set_engine_signer(&self, address: Address, private: Private) {
        if self.engine.seals_internally().is_some() {
            self.engine.set_signer(address, private)
        }
    }

    fn minimal_fee(&self) -> U256 {
        *self.transaction_queue.read().minimal_fee()
    }

    fn set_minimal_fee(&self, min_fee: U256) {
        self.transaction_queue.write().set_minimal_fee(min_fee);
    }

    fn transactions_limit(&self) -> usize {
        self.transaction_queue.read().limit()
    }

    fn set_transactions_limit(&self, limit: usize) {
        self.transaction_queue.write().set_limit(limit)
    }

    fn import_external_transactions<C: MiningBlockChainClient>(
        &self,
        client: &C,
        transactions: Vec<UnverifiedTransaction>,
    ) -> Vec<Result<TransactionImportResult, Error>> {
        trace!(target: "external_tx", "Importing external transactions");
        let mut transaction_queue = self.transaction_queue.write();
        self.add_transactions_to_queue(client, transactions, TransactionOrigin::External, &mut transaction_queue)
    }

    fn import_own_transaction<C: MiningBlockChainClient>(
        &self,
        chain: &C,
        transaction: SignedTransaction,
    ) -> Result<TransactionImportResult, Error> {
        trace!(target: "own_tx", "Importing transaction: {:?}", transaction);

        // Be sure to release the lock before we call prepare_work_sealing
        let mut transaction_queue = self.transaction_queue.write();
        // We need to re-validate transactions
        let import = self.add_transactions_to_queue(
            chain,
            vec![transaction.into()],
            TransactionOrigin::Local,
            &mut transaction_queue,
        ).pop()
            .expect("one result returned per added transaction; one added => one result; qed");

        match import {
            Ok(_) => {
                trace!(target: "own_tx", "Status: {:?}", transaction_queue.status());
            }
            Err(ref e) => {
                trace!(target: "own_tx", "Status: {:?}", transaction_queue.status());
                warn!(target: "own_tx", "Error importing transaction: {:?}", e);
            }
        }
        import
    }

    fn ready_transactions(&self) -> Vec<SignedTransaction> {
        self.transaction_queue.read().top_transactions()
    }

    /// Get a list of all future transactions.
    fn future_transactions(&self) -> Vec<SignedTransaction> {
        self.transaction_queue.read().future_transactions()
    }
}

struct TransactionDetailsProvider<'a, C: 'a> {
    client: &'a C,
}

impl<'a, C> TransactionDetailsProvider<'a, C> {
    pub fn new(client: &'a C) -> Self {
        TransactionDetailsProvider {
            client,
        }
    }
}

impl<'a, C> TransactionQueueDetailsProvider for TransactionDetailsProvider<'a, C>
where
    C: AccountData,
{
    fn fetch_account(&self, address: &Address) -> AccountDetails {
        AccountDetails {
            nonce: self.client.latest_nonce(address),
            balance: self.client.latest_balance(address),
        }
    }
}
