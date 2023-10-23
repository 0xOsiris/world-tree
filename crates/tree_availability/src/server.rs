use std::sync::atomic::Ordering;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use ethers::providers::Middleware;
use semaphore::poseidon_tree::Proof;
use semaphore::Field;
use serde::{Deserialize, Serialize};

use crate::error::TreeError;
use crate::world_tree::{Hash, WorldTree};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InclusionProofRequest {
    pub identity_commitment: Hash,
    pub root: Option<Hash>,
}

impl InclusionProofRequest {
    pub fn new(
        identity_commitment: Hash,
        root: Option<Hash>,
    ) -> InclusionProofRequest {
        Self {
            identity_commitment,
            root,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InclusionProof {
    pub root: Field,
    pub proof: Proof,
    pub message: Option<String>,
}

impl InclusionProof {
    pub fn new(
        root: Field,
        proof: Proof,
        message: Option<String>,
    ) -> InclusionProof {
        Self {
            root,
            proof,
            message,
        }
    }
}

pub async fn inclusion_proof<M: Middleware>(
    State(world_tree): State<Arc<WorldTree<M>>>,
    Json(req): Json<InclusionProofRequest>,
) -> Result<(StatusCode, Json<Option<InclusionProof>>), TreeError> {
    if world_tree.tree_updater.synced.load(Ordering::Relaxed) {
        let inclusion_proof = world_tree
            .tree_data
            .get_inclusion_proof(req.identity_commitment, req.root)
            .await;

        Ok((StatusCode::OK, inclusion_proof.into()))
    } else {
        Err(TreeError::TreeNotSynced)
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResponse {
    pub synced: bool,
    pub block_number: Option<u64>,
}

impl SyncResponse {
    pub fn new(synced: bool, block_number: Option<u64>) -> SyncResponse {
        Self {
            synced,
            block_number,
        }
    }
}

pub async fn synced<M: Middleware>(
    State(world_tree): State<Arc<WorldTree<M>>>,
) -> (StatusCode, Json<SyncResponse>) {
    if world_tree.tree_updater.synced.load(Ordering::Relaxed) {
        (StatusCode::OK, SyncResponse::new(true, None).into())
    } else {
        let latest_synced_block = Some(
            world_tree
                .tree_updater
                .latest_synced_block
                .load(Ordering::SeqCst),
        );
        (
            StatusCode::OK,
            SyncResponse::new(false, latest_synced_block).into(),
        )
    }
}

impl TreeError {
    fn to_status_code(&self) -> StatusCode {
        //TODO: update this
        StatusCode::BAD_REQUEST
    }
}
impl IntoResponse for TreeError {
    fn into_response(self) -> axum::response::Response {
        let status_code = self.to_status_code();
        let response_body = self.to_string();
        (status_code, response_body).into_response()
    }
}