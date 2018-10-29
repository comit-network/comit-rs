use bitcoin_support::MinedBlock as BitcoinBlock;
use block_processor::{Query, QueryMatchResult};
use http_api_problem::HttpApiProblem;
use link_factory::LinkFactory;
use query_repository::QueryRepository;
use query_result_repository::{QueryResult, QueryResultRepository};
use rocket::{
    response::{
        status::{Created, NoContent},
        Responder,
    },
    State,
};
use rocket_contrib::Json;
use std::sync::Arc;

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct BitcoinBlockQuery {
    pub min_height: Option<u32>,
}

#[post(
    "/queries/bitcoin/blocks",
    format = "application/json",
    data = "<query>"
)]
#[allow(clippy::needless_pass_by_value)] // Rocket passes by value
pub fn handle_new_query<'r>(
    query: Json<BitcoinBlockQuery>,
    link_factory: State<LinkFactory>,
    query_repository: State<Arc<QueryRepository<BitcoinBlockQuery>>>,
) -> Result<impl Responder<'r>, HttpApiProblem> {
    let query = query.into_inner();

    if let BitcoinBlockQuery {
        min_height: None, ..
    } = query
    {
        return Err(HttpApiProblem::with_title_from_status(400)
            .set_detail("Query needs at least one condition"));
    }

    let result = query_repository.save(query);

    match result {
        Ok(id) => Ok(created(
            link_factory.create_link(format!("/queries/bitcoin/blocks/{}", id)),
        )),
        Err(_) => {
            Err(HttpApiProblem::with_title_from_status(500)
                .set_detail("Failed to create new query"))
        }
    }
}

fn created(url: String) -> Created<Option<()>> {
    Created(url, None)
}

impl Query<BitcoinBlock> for BitcoinBlockQuery {
    fn matches(&self, block: &BitcoinBlock) -> QueryMatchResult {
        match self.min_height {
            Some(ref height) => {
                if *height <= block.height {
                    QueryMatchResult::yes()
                } else {
                    QueryMatchResult::no()
                }
            }
            None => {
                warn!("min_height not set, nothing to compare");
                QueryMatchResult::no()
            }
        }
    }
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct RetrieveBitcoinBlockQueryResponse {
    query: BitcoinBlockQuery,
    matching_blocks: QueryResult,
}

#[get("/queries/bitcoin/blocks/<id>")]
#[allow(clippy::needless_pass_by_value)] // Rocket passes by value
pub fn retrieve_query(
    id: u32,
    query_repository: State<Arc<QueryRepository<BitcoinBlockQuery>>>,
    query_result_repository: State<Arc<QueryResultRepository<BitcoinBlockQuery>>>,
) -> Result<Json<RetrieveBitcoinBlockQueryResponse>, HttpApiProblem> {
    let query = query_repository.get(id).ok_or_else(|| {
        HttpApiProblem::with_title_from_status(404).set_detail("The requested query does not exist")
    })?;

    let result = query_result_repository.get(id).unwrap_or_default();

    Ok(Json(RetrieveBitcoinBlockQueryResponse {
        query,
        matching_blocks: result,
    }))
}

#[delete("/queries/bitcoin/blocks/<id>")]
#[allow(clippy::needless_pass_by_value)] // Rocket passes by value
pub fn delete_query(
    id: u32,
    query_repository: State<Arc<QueryRepository<BitcoinBlockQuery>>>,
    query_result_repository: State<Arc<QueryResultRepository<BitcoinBlockQuery>>>,
) -> impl Responder<'static> {
    query_repository.delete(id);
    query_result_repository.delete(id);

    NoContent
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitcoin_support::{Block, BlockHeader, MinedBlock, Sha256dHash};
    use spectral::prelude::*;

    #[test]
    fn given_query_min_height_then_lesser_block_does_not_match() {
        let block_header = BlockHeader {
            version: 1,
            prev_blockhash: Sha256dHash::default(),
            merkle_root: Sha256dHash::default(),
            time: 0,
            bits: 1,
            nonce: 0,
        };

        let block = MinedBlock::new(
            Block {
                header: block_header,
                txdata: vec![],
            },
            40,
        );

        let query = BitcoinBlockQuery {
            min_height: Some(42),
        };

        assert_that(&query.matches(&block)).is_equal_to(QueryMatchResult::no());
    }

    #[test]
    fn given_query_min_height_then_exact_block_matches() {
        let block_header = BlockHeader {
            version: 1,
            prev_blockhash: Sha256dHash::default(),
            merkle_root: Sha256dHash::default(),
            time: 0,
            bits: 1,
            nonce: 0,
        };

        let block = MinedBlock::new(
            Block {
                header: block_header,
                txdata: vec![],
            },
            42,
        );

        let query = BitcoinBlockQuery {
            min_height: Some(42),
        };

        assert_that(&query.matches(&block)).is_equal_to(QueryMatchResult::yes());
    }

    #[test]
    fn given_query_min_height_then_greater_block_matches() {
        let block_header = BlockHeader {
            version: 1,
            prev_blockhash: Sha256dHash::default(),
            merkle_root: Sha256dHash::default(),
            time: 0,
            bits: 1,
            nonce: 0,
        };

        let block = MinedBlock::new(
            Block {
                header: block_header,
                txdata: vec![],
            },
            45,
        );

        let query = BitcoinBlockQuery {
            min_height: Some(42),
        };

        assert_that(&query.matches(&block)).is_equal_to(QueryMatchResult::yes());
    }

}
