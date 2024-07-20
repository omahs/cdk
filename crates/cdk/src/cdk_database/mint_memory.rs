//! Mint in memory database

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::{Mutex, RwLock};

use super::{Error, MintDatabase};
use crate::dhke::hash_to_curve;
use crate::mint::{self, MintKeySetInfo, MintQuote};
use crate::nuts::nut07::State;
use crate::nuts::{
    nut07, BlindSignature, CurrencyUnit, Id, MeltQuoteState, MintQuoteState, Proof, Proofs,
    PublicKey,
};

/// Mint Memory Database
#[derive(Debug, Clone)]
pub struct MintMemoryDatabase {
    active_keysets: Arc<RwLock<HashMap<CurrencyUnit, Id>>>,
    keysets: Arc<RwLock<HashMap<Id, MintKeySetInfo>>>,
    mint_quotes: Arc<RwLock<HashMap<String, MintQuote>>>,
    melt_quotes: Arc<RwLock<HashMap<String, mint::MeltQuote>>>,
    proofs: Arc<RwLock<HashMap<[u8; 33], Proof>>>,
    proof_state: Arc<Mutex<HashMap<[u8; 33], nut07::State>>>,
    blinded_signatures: Arc<RwLock<HashMap<[u8; 33], BlindSignature>>>,
}

impl MintMemoryDatabase {
    /// Create new [`MintMemoryDatabase`]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        active_keysets: HashMap<CurrencyUnit, Id>,
        keysets: Vec<MintKeySetInfo>,
        mint_quotes: Vec<MintQuote>,
        melt_quotes: Vec<mint::MeltQuote>,
        pending_proofs: Proofs,
        spent_proofs: Proofs,
        blinded_signatures: HashMap<[u8; 33], BlindSignature>,
    ) -> Result<Self, Error> {
        let mut proofs = HashMap::new();
        let mut proof_states = HashMap::new();

        for proof in pending_proofs {
            let y = hash_to_curve(&proof.secret.to_bytes())?.to_bytes();
            proofs.insert(y, proof);
            proof_states.insert(y, State::Pending);
        }

        for proof in spent_proofs {
            let y = hash_to_curve(&proof.secret.to_bytes())?.to_bytes();
            proofs.insert(y, proof);
            proof_states.insert(y, State::Spent);
        }

        Ok(Self {
            active_keysets: Arc::new(RwLock::new(active_keysets)),
            keysets: Arc::new(RwLock::new(
                keysets.into_iter().map(|k| (k.id, k)).collect(),
            )),
            mint_quotes: Arc::new(RwLock::new(
                mint_quotes.into_iter().map(|q| (q.id.clone(), q)).collect(),
            )),
            melt_quotes: Arc::new(RwLock::new(
                melt_quotes.into_iter().map(|q| (q.id.clone(), q)).collect(),
            )),
            proofs: Arc::new(RwLock::new(proofs)),
            proof_state: Arc::new(Mutex::new(proof_states)),
            blinded_signatures: Arc::new(RwLock::new(blinded_signatures)),
        })
    }
}

#[async_trait]
impl MintDatabase for MintMemoryDatabase {
    type Err = Error;

    async fn set_active_keyset(&self, unit: CurrencyUnit, id: Id) -> Result<(), Self::Err> {
        self.active_keysets.write().await.insert(unit, id);
        Ok(())
    }

    async fn get_active_keyset_id(&self, unit: &CurrencyUnit) -> Result<Option<Id>, Self::Err> {
        Ok(self.active_keysets.read().await.get(unit).cloned())
    }

    async fn get_active_keysets(&self) -> Result<HashMap<CurrencyUnit, Id>, Self::Err> {
        Ok(self.active_keysets.read().await.clone())
    }

    async fn add_keyset_info(&self, keyset: MintKeySetInfo) -> Result<(), Self::Err> {
        self.keysets.write().await.insert(keyset.id, keyset);
        Ok(())
    }

    async fn get_keyset_info(&self, keyset_id: &Id) -> Result<Option<MintKeySetInfo>, Self::Err> {
        Ok(self.keysets.read().await.get(keyset_id).cloned())
    }

    async fn get_keyset_infos(&self) -> Result<Vec<MintKeySetInfo>, Self::Err> {
        Ok(self.keysets.read().await.values().cloned().collect())
    }

    async fn add_mint_quote(&self, quote: MintQuote) -> Result<(), Self::Err> {
        self.mint_quotes
            .write()
            .await
            .insert(quote.id.clone(), quote);
        Ok(())
    }

    async fn get_mint_quote(&self, quote_id: &str) -> Result<Option<MintQuote>, Self::Err> {
        Ok(self.mint_quotes.read().await.get(quote_id).cloned())
    }

    async fn update_mint_quote_state(
        &self,
        quote_id: &str,
        state: MintQuoteState,
    ) -> Result<MintQuoteState, Self::Err> {
        let mut mint_quotes = self.mint_quotes.write().await;

        let mut quote = mint_quotes
            .get(quote_id)
            .cloned()
            .ok_or(Error::UnknownQuote)?;

        let current_state = quote.state;

        quote.state = state;

        mint_quotes.insert(quote_id.to_string(), quote.clone());

        Ok(current_state)
    }

    async fn get_mint_quote_by_request_lookup_id(
        &self,
        request: &str,
    ) -> Result<Option<MintQuote>, Self::Err> {
        let quotes = self.get_mint_quotes().await?;

        let quote = quotes
            .into_iter()
            .filter(|q| q.request_lookup_id.eq(request))
            .collect::<Vec<MintQuote>>()
            .first()
            .cloned();

        Ok(quote)
    }
    async fn get_mint_quote_by_request(
        &self,
        request: &str,
    ) -> Result<Option<MintQuote>, Self::Err> {
        let quotes = self.get_mint_quotes().await?;

        let quote = quotes
            .into_iter()
            .filter(|q| q.request.eq(request))
            .collect::<Vec<MintQuote>>()
            .first()
            .cloned();

        Ok(quote)
    }

    async fn get_mint_quotes(&self) -> Result<Vec<MintQuote>, Self::Err> {
        Ok(self.mint_quotes.read().await.values().cloned().collect())
    }

    async fn remove_mint_quote(&self, quote_id: &str) -> Result<(), Self::Err> {
        self.mint_quotes.write().await.remove(quote_id);

        Ok(())
    }

    async fn add_melt_quote(&self, quote: mint::MeltQuote) -> Result<(), Self::Err> {
        self.melt_quotes
            .write()
            .await
            .insert(quote.id.clone(), quote);
        Ok(())
    }

    async fn get_melt_quote(&self, quote_id: &str) -> Result<Option<mint::MeltQuote>, Self::Err> {
        Ok(self.melt_quotes.read().await.get(quote_id).cloned())
    }

    async fn update_melt_quote_state(
        &self,
        quote_id: &str,
        state: MeltQuoteState,
    ) -> Result<MeltQuoteState, Self::Err> {
        let mut melt_quotes = self.melt_quotes.write().await;

        let mut quote = melt_quotes
            .get(quote_id)
            .cloned()
            .ok_or(Error::UnknownQuote)?;

        let current_state = quote.state;

        quote.state = state;

        melt_quotes.insert(quote_id.to_string(), quote.clone());

        Ok(current_state)
    }

    async fn get_melt_quotes(&self) -> Result<Vec<mint::MeltQuote>, Self::Err> {
        Ok(self.melt_quotes.read().await.values().cloned().collect())
    }

    async fn remove_melt_quote(&self, quote_id: &str) -> Result<(), Self::Err> {
        self.melt_quotes.write().await.remove(quote_id);

        Ok(())
    }

    async fn add_proofs(&self, proofs: Proofs) -> Result<(), Self::Err> {
        let mut db_proofs = self.proofs.write().await;

        for proof in proofs {
            let secret_point = hash_to_curve(&proof.secret.to_bytes())?;
            db_proofs.insert(secret_point.to_bytes(), proof);
        }
        Ok(())
    }

    async fn get_proofs_by_ys(&self, ys: &[PublicKey]) -> Result<Vec<Option<Proof>>, Self::Err> {
        let spent_proofs = self.proofs.read().await;

        let mut proofs = Vec::with_capacity(ys.len());

        for y in ys {
            let proof = spent_proofs.get(&y.to_bytes()).cloned();

            proofs.push(proof);
        }

        Ok(proofs)
    }

    async fn update_proofs_states(
        &self,
        ys: &[PublicKey],
        proof_state: State,
    ) -> Result<Vec<Option<State>>, Self::Err> {
        let mut proofs_states = self.proof_state.lock().await;

        let mut states = Vec::new();

        for y in ys {
            let state = proofs_states.insert(y.to_bytes(), proof_state);
            states.push(state);
        }

        Ok(states)
    }

    async fn get_proofs_states(&self, ys: &[PublicKey]) -> Result<Vec<Option<State>>, Self::Err> {
        let proofs_states = self.proof_state.lock().await;

        let mut states = Vec::new();

        for y in ys {
            let state = proofs_states.get(&y.to_bytes()).cloned();
            states.push(state);
        }

        Ok(states)
    }

    async fn add_blind_signatures(
        &self,
        blinded_message: &[PublicKey],
        blind_signatures: &[BlindSignature],
    ) -> Result<(), Self::Err> {
        let mut current_blinded_signatures = self.blinded_signatures.write().await;

        for (blinded_message, blind_signature) in blinded_message.iter().zip(blind_signatures) {
            current_blinded_signatures.insert(blinded_message.to_bytes(), blind_signature.clone());
        }

        Ok(())
    }

    async fn get_blinded_signatures(
        &self,
        blinded_messages: &[PublicKey],
    ) -> Result<Vec<Option<BlindSignature>>, Self::Err> {
        let mut signatures = Vec::with_capacity(blinded_messages.len());

        let blinded_signatures = self.blinded_signatures.read().await;

        for blinded_message in blinded_messages {
            let signature = blinded_signatures.get(&blinded_message.to_bytes()).cloned();

            signatures.push(signature)
        }

        Ok(signatures)
    }
}
