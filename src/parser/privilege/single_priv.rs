use super::SinglePriv;
use crate::{
    parser::{utils::parsec::parse_single_priv, Privilege},
    DeltaItem, NodeItem,
};
use pg_query::{protobuf::AccessPriv, Node, NodeEnum};
use std::{collections::BTreeSet, str::FromStr};

impl FromStr for SinglePriv {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        let (_, p) =
            parse_single_priv(s).map_err(|_| anyhow::anyhow!("invalid single priv: {}", s))?;
        Ok(p)
    }
}

impl From<SinglePriv> for AccessPriv {
    fn from(p: SinglePriv) -> Self {
        let cols = p
            .cols
            .into_iter()
            .map(|s| NodeEnum::String(pg_query::protobuf::String { str: s }))
            .map(|n| Node { node: Some(n) })
            .collect::<Vec<_>>();
        AccessPriv {
            priv_name: p.name,
            cols,
        }
    }
}

impl SinglePriv {
    fn generate_change(self, item: &Privilege, is_grant: bool) -> anyhow::Result<NodeEnum> {
        let mut stmt = item.inner()?.clone();
        stmt.is_grant = is_grant;
        stmt.privileges = vec![self.into()];
        Ok(NodeEnum::GrantStmt(stmt))
    }
}

impl DeltaItem for SinglePriv {
    type SqlNode = Privilege;
    fn drop(self, item: &Self::SqlNode) -> anyhow::Result<Vec<String>> {
        let node = self.generate_change(item, false)?;
        Ok(vec![node.deparse()?])
    }

    fn create(self, item: &Self::SqlNode) -> anyhow::Result<Vec<String>> {
        let node = self.generate_change(item, true)?;
        Ok(vec![node.deparse()?])
    }

    fn alter(self, item: &Self::SqlNode, remote: Self) -> anyhow::Result<Vec<String>> {
        let mut migrations = vec![];
        let sql = self.drop(item)?;
        migrations.extend(sql);
        let sql = remote.create(item)?;
        migrations.extend(sql);
        Ok(migrations)
    }
}

impl From<SinglePriv> for Node {
    fn from(p: SinglePriv) -> Self {
        Node {
            node: Some(NodeEnum::AccessPriv(p.into())),
        }
    }
}

impl From<AccessPriv> for SinglePriv {
    fn from(p: AccessPriv) -> Self {
        let name = p.priv_name;
        let cols: BTreeSet<String> = p
            .cols
            .into_iter()
            .filter_map(|n| {
                n.node.and_then(|c| match c {
                    NodeEnum::String(s) => Some(s.str),
                    _ => None,
                })
            })
            .collect();
        Self { name, cols }
    }
}
