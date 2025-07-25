use serde::{Deserialize, Serialize};

use crate::{
    Link,
    create::{ActorGenerator, ToRust},
};

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

    pub fn ident(&self) -> &str {
        &self.ident
    }

    pub fn ty(&self) -> &Link {
        &self.ty
    }
}

impl ToRust for Field {
    fn to_rust(&self, _generator: &ActorGenerator) -> String {
        format!("pub {}: {}", self.ident, self.ty)
    }
}
