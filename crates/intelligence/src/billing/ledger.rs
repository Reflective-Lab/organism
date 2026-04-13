// Copyright 2024-2026 Reflective Labs
// SPDX-License-Identifier: MIT
// See LICENSE file in the project root for full license information.

//! Firestore-backed credit ledger for usage-based billing.
//!
//! Manages per-user credit balances with two Firestore collections:
//! - `credit_balances` — keyed by `user_id`, stores current balance (hot read path)
//! - `credit_transactions` — append-only audit log of all credit changes
//!
//! Idempotency keys prevent double-charge on retries and double-grant on webhooks.

use chrono::Utc;
use firestore::*;
use tracing::{debug, info};

use super::types::{BillingError, CreditBalance, CreditTransaction, CreditTransactionKind};

/// Generate a unique document ID (timestamp + random suffix).
fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let random: u32 = rand::random();
    format!("{timestamp:x}{random:08x}")
}

const BALANCES_COLLECTION: &str = "credit_balances";
const TRANSACTIONS_COLLECTION: &str = "credit_transactions";

/// Firestore-backed credit ledger.
pub struct CreditLedger {
    db: FirestoreDb,
}

impl CreditLedger {
    /// Create a new credit ledger backed by the given Firestore database.
    pub fn new(db: FirestoreDb) -> Self {
        Self { db }
    }

    /// Get the current credit balance for a user.
    ///
    /// Returns a zero balance if no record exists.
    pub async fn get_balance(&self, user_id: &str) -> Result<CreditBalance, BillingError> {
        let result: Option<CreditBalance> = self
            .db
            .fluent()
            .select()
            .by_id_in(BALANCES_COLLECTION)
            .obj()
            .one(user_id)
            .await
            .map_err(|e| BillingError::LedgerPersistence(e.to_string()))?;

        Ok(result.unwrap_or_else(|| CreditBalance {
            user_id: user_id.to_string(),
            balance: 0,
            updated_at: Utc::now().to_rfc3339(),
        }))
    }

    /// Check if a user has sufficient credits.
    pub async fn has_sufficient_credits(
        &self,
        user_id: &str,
        required: i64,
    ) -> Result<bool, BillingError> {
        let balance = self.get_balance(user_id).await?;
        Ok(balance.balance >= required)
    }

    /// Deduct credits from a user's balance.
    ///
    /// Returns the transaction record. Uses idempotency key to prevent double-charge.
    pub async fn deduct(
        &self,
        user_id: &str,
        amount: i64,
        description: &str,
        reference_id: Option<&str>,
        idempotency_key: Option<&str>,
    ) -> Result<CreditTransaction, BillingError> {
        // Check idempotency — if a transaction with this key already exists, return it
        if let Some(key) = idempotency_key {
            if let Some(existing) = self.find_by_idempotency_key(key).await? {
                debug!(
                    idempotency_key = key,
                    "Returning existing transaction (idempotent)"
                );
                return Ok(existing);
            }
        }

        // Get current balance
        let current = self.get_balance(user_id).await?;
        if current.balance < amount {
            return Err(BillingError::InsufficientCredits {
                required: amount,
                available: current.balance,
            });
        }

        let new_balance = current.balance - amount;

        // Create transaction record
        let tx = CreditTransaction {
            id: generate_id(),
            user_id: user_id.to_string(),
            kind: CreditTransactionKind::Deduction,
            amount: -amount,
            balance_after: new_balance,
            description: description.to_string(),
            reference_id: reference_id.map(String::from),
            idempotency_key: idempotency_key.map(String::from),
            created_at: Utc::now().to_rfc3339(),
        };

        // Write transaction
        self.db
            .fluent()
            .insert()
            .into(TRANSACTIONS_COLLECTION)
            .document_id(&tx.id)
            .object(&tx)
            .execute::<CreditTransaction>()
            .await
            .map_err(|e| BillingError::LedgerPersistence(e.to_string()))?;

        // Update balance
        let balance_doc = CreditBalance {
            user_id: user_id.to_string(),
            balance: new_balance,
            updated_at: Utc::now().to_rfc3339(),
        };
        self.upsert_balance(&balance_doc).await?;

        info!(
            user_id,
            amount,
            new_balance,
            tx_id = %tx.id,
            "Credits deducted"
        );

        Ok(tx)
    }

    /// Top up credits for a user.
    ///
    /// Returns the transaction record. Uses idempotency key to prevent double-grant.
    pub async fn top_up(
        &self,
        user_id: &str,
        amount: i64,
        description: &str,
        reference_id: Option<&str>,
        idempotency_key: Option<&str>,
    ) -> Result<CreditTransaction, BillingError> {
        // Check idempotency
        if let Some(key) = idempotency_key {
            if let Some(existing) = self.find_by_idempotency_key(key).await? {
                debug!(
                    idempotency_key = key,
                    "Returning existing transaction (idempotent)"
                );
                return Ok(existing);
            }
        }

        let current = self.get_balance(user_id).await?;
        let new_balance = current.balance + amount;

        // Create transaction record
        let tx = CreditTransaction {
            id: generate_id(),
            user_id: user_id.to_string(),
            kind: CreditTransactionKind::TopUp,
            amount,
            balance_after: new_balance,
            description: description.to_string(),
            reference_id: reference_id.map(String::from),
            idempotency_key: idempotency_key.map(String::from),
            created_at: Utc::now().to_rfc3339(),
        };

        // Write transaction
        self.db
            .fluent()
            .insert()
            .into(TRANSACTIONS_COLLECTION)
            .document_id(&tx.id)
            .object(&tx)
            .execute::<CreditTransaction>()
            .await
            .map_err(|e| BillingError::LedgerPersistence(e.to_string()))?;

        // Update balance
        let balance_doc = CreditBalance {
            user_id: user_id.to_string(),
            balance: new_balance,
            updated_at: Utc::now().to_rfc3339(),
        };
        self.upsert_balance(&balance_doc).await?;

        info!(
            user_id,
            amount,
            new_balance,
            tx_id = %tx.id,
            "Credits topped up"
        );

        Ok(tx)
    }

    /// List recent transactions for a user.
    pub async fn list_transactions(
        &self,
        user_id: &str,
        limit: u32,
    ) -> Result<Vec<CreditTransaction>, BillingError> {
        let transactions: Vec<CreditTransaction> = self
            .db
            .fluent()
            .select()
            .from(TRANSACTIONS_COLLECTION)
            .filter(|q| q.field("user_id").eq(user_id))
            .order_by([("created_at", FirestoreQueryDirection::Descending)])
            .limit(limit)
            .obj()
            .query()
            .await
            .map_err(|e| BillingError::LedgerPersistence(e.to_string()))?;

        Ok(transactions)
    }

    /// Find a transaction by idempotency key.
    async fn find_by_idempotency_key(
        &self,
        key: &str,
    ) -> Result<Option<CreditTransaction>, BillingError> {
        let results: Vec<CreditTransaction> = self
            .db
            .fluent()
            .select()
            .from(TRANSACTIONS_COLLECTION)
            .filter(|q| q.field("idempotency_key").eq(key))
            .limit(1)
            .obj()
            .query()
            .await
            .map_err(|e| BillingError::LedgerPersistence(e.to_string()))?;

        Ok(results.into_iter().next())
    }

    /// Upsert a balance document (create or replace).
    async fn upsert_balance(&self, balance: &CreditBalance) -> Result<(), BillingError> {
        let _: CreditBalance = self
            .db
            .fluent()
            .update()
            .in_col(BALANCES_COLLECTION)
            .document_id(&balance.user_id)
            .object(balance)
            .execute()
            .await
            .map_err(|e| BillingError::LedgerPersistence(e.to_string()))?;
        Ok(())
    }
}
