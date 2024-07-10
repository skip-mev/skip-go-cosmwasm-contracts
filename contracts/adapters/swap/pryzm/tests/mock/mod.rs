use cosmwasm_std::{
    ContractResult, Querier as CWStdQuerier, QuerierResult, SystemResult, to_json_binary,
    to_json_vec,
};
use cosmwasm_std::{Empty, QueryRequest};
use mockall::*;
use mockall::predicate::*;
use serde::Serialize;

mock! {
    StdQuerier {}
    impl CWStdQuerier for StdQuerier {
        fn raw_query(&self, bin_request: &[u8]) -> QuerierResult;
    }
}

pub struct MockQuerier {
    inner_mock: MockStdQuerier,
}

impl CWStdQuerier for MockQuerier {
    fn raw_query(&self, bin_request: &[u8]) -> QuerierResult {
        self.inner_mock.raw_query(bin_request)
    }
}

impl MockQuerier {
    pub fn new() -> Self {
        Self {
            inner_mock: MockStdQuerier::new(),
        }
    }

    pub fn mock_query<T>(&mut self, request: QueryRequest<Empty>, response: &T)
    where
        T: Serialize + ?Sized,
    {
        self.inner_mock
            .expect_raw_query()
            .with(eq(to_json_vec(&request).unwrap()))
            .return_const(SystemResult::Ok(ContractResult::Ok(
                to_json_binary(response).unwrap(),
            )));
    }
}
