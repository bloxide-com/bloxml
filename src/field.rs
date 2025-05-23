use serde::{Deserialize, Serialize};

use crate::Link;

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone)]
pub struct Field {
    ident: String,
    ty: Link,
}

impl Field {
    pub fn new<L, S>(ident: S, ty: L) -> Self
    where
        L: Into<Link>,
        S: Into<String>,
    {
        Self {
            ident: ident.into(),
            ty: ty.into(),
        }
    }
}
