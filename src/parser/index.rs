use crate::{MigrationPlanner, SqlDiffer};

use super::{utils::create_diff, Index, RelationId};
use anyhow::Context;
use debug_ignore::DebugIgnore;
use pg_query::{protobuf::IndexStmt, NodeRef};
use std::str::FromStr;

impl FromStr for Index {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        let parsed = pg_query::parse(s).with_context(|| format!("Failed to parse: {}", s))?;
        let node = parsed.protobuf.nodes()[0].0;
        match node {
            NodeRef::IndexStmt(stmt) => Self::try_from(stmt),
            _ => anyhow::bail!("not an index: {}", s),
        }
    }
}

impl TryFrom<&IndexStmt> for Index {
    type Error = anyhow::Error;
    fn try_from(stmt: &IndexStmt) -> Result<Self, Self::Error> {
        let id = get_id(stmt);
        let node = pg_query::NodeEnum::IndexStmt(Box::new(stmt.clone()));
        Ok(Self {
            id,
            node: DebugIgnore(node),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndexDiff {
    pub id: RelationId,
    pub old: Option<Index>,
    pub new: Option<Index>,
    pub diff: String,
}

impl SqlDiffer for Index {
    type Delta = IndexDiff;
    fn diff(&self, remote: &Self) -> anyhow::Result<Option<Self::Delta>> {
        if self.id != remote.id {
            anyhow::bail!("can't diff {} and {}", self.id.name, remote.id.name);
        }

        if self != remote {
            let diff = create_diff(&self.node, &remote.node)?;
            Ok(Some(IndexDiff {
                id: self.id.clone(),
                old: Some(self.clone()),
                new: Some(remote.clone()),
                diff,
            }))
        } else {
            Ok(None)
        }
    }
}

impl MigrationPlanner for IndexDiff {
    type Migration = String;
    fn plan(&self) -> Vec<Self::Migration> {
        let mut migrations = vec![];
        if let Some(old) = &self.old {
            migrations.push(format!("DROP INDEX {};", old.id.name));
        }
        if let Some(new) = &self.new {
            migrations.push(format!("{};", new.node.deparse().unwrap()));
        }
        migrations
    }
}

fn get_id(stmt: &IndexStmt) -> RelationId {
    let name = stmt.idxname.clone();
    assert!(stmt.relation.is_some());
    let schema_id = stmt.relation.as_ref().unwrap().into();
    RelationId { name, schema_id }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_should_parse() {
        let sql = "CREATE INDEX foo ON bar (baz);";
        let index: Index = sql.parse().unwrap();
        assert_eq!(index.id.name, "foo");
        assert_eq!(index.id.schema_id.schema, "public");
        assert_eq!(index.id.schema_id.name, "bar");
    }

    #[test]
    fn unchanged_index_should_return_none() {
        let sql1 = "CREATE INDEX foo ON bar (baz);";
        let sql2 = "CREATE INDEX foo ON bar (baz);";
        let old: Index = sql1.parse().unwrap();
        let new: Index = sql2.parse().unwrap();
        let diff = old.diff(&new).unwrap();
        assert!(diff.is_none());
    }

    #[test]
    fn changed_index_should_generate_migration() {
        let sql1 = "CREATE INDEX foo ON bar (baz);";
        let sql2 = "CREATE INDEX foo ON bar (ooo);";
        let old: Index = sql1.parse().unwrap();
        let new: Index = sql2.parse().unwrap();
        let diff = old.diff(&new).unwrap().unwrap();
        let migrations = diff.plan();
        assert_eq!(migrations[0], "DROP INDEX foo;");
        assert_eq!(migrations[1], "CREATE INDEX foo ON bar USING btree (ooo);");
    }
}
