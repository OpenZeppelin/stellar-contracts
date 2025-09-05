mod storage;
#[cfg(test)]
mod test;

pub use self::storage::{increment_token_id, next_token_id};
