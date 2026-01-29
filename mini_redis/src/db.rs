use std::collections::HashMap;

use lazy_static::lazy_static;
use mini_runtime::UPSafeCell;

use crate::{request::Request, response::Response};

lazy_static! {
    static ref DB: UPSafeCell<HashMap<String, String>> = UPSafeCell::new(HashMap::new());
}

fn insert_db(key: String, val: String) {
    DB.exclusive_access().insert(key, val);
}

fn select_db(key: &String) -> Option<String> {
    DB.exclusive_access().get(key).map(|val| val.to_owned())
}

fn remove_db(key: &String) -> Option<String> {
    DB.exclusive_access().remove(key)
}

pub fn db_op(cmd: &Request) -> Response {
    match cmd {
        Request::Set(key, val) => {
            insert_db(key.to_owned(), val.to_owned());
            Response::Ok
        }
        Request::Get(key) => select_db(key).map_or(Response::NotFound, Response::Exist),
        Request::Del(key) => remove_db(key).map_or(Response::NotFound, |_| Response::Ok),
    }
}
