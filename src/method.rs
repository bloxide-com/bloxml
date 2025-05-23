use crate::{Field, Link};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
pub struct Method {
    ident: String,
    args: Vec<Field>,
    ret: Link,
    body: String,
}

impl Method {
    pub fn new<S, L>(ident: S, args: &[Field], ret: L, body: S) -> Self
    where
        S: Into<String>,
        L: Into<Link>,
    {
        Self {
            ident: ident.into(),
            args: args.to_vec(),
            ret: ret.into(),
            body: body.into(),
        }
    }
}
