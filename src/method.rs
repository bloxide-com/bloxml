use crate::{Field, Link, create::ToRust};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
pub struct Method {
    ident: String,
    #[serde(default)]
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

    pub fn ident(&self) -> &str {
        &self.ident
    }

    pub fn args(&self) -> &[Field] {
        &self.args
    }

    pub fn ret(&self) -> &Link {
        &self.ret
    }

    pub fn body(&self) -> &str {
        &self.body
    }
}

impl ToRust for Method {
    fn to_rust(&self) -> String {
        let args = self
            .args
            .iter()
            .map(|arg| match arg.ident().to_string().as_str() {
                slf @ ("self" | "&self" | "&mut self") => slf.to_string(),
                _ => format!("{}: {}", arg.ident(), arg.ty()),
            })
            .collect::<Vec<_>>()
            .join(", ");

        let ret = if self.ret.as_ref().is_empty() {
            "".to_string()
        } else {
            format!(" -> {}", self.ret)
        };

        format!(
            r#"pub fn {ident}({args}){ret} {{
        {body}
    }}
    "#,
            ident = self.ident,
            body = self.body,
        )
    }
}
