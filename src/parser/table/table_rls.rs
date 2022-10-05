use std::str::FromStr;

use crate::{
    parser::{AlterTable, AlterTableAction, SchemaId, TableRls},
    MigrationPlanner, MigrationResult, NodeDiff, NodeItem,
};
use pg_query::{protobuf::AlterTableStmt, NodeEnum, NodeRef};

impl NodeItem for TableRls {
    type Inner = AlterTableStmt;

    fn id(&self) -> String {
        self.id.to_string()
    }

    fn node(&self) -> &NodeEnum {
        &self.node
    }

    fn inner(&self) -> anyhow::Result<&Self::Inner> {
        match &self.node {
            NodeEnum::AlterTableStmt(stmt) => Ok(stmt),
            _ => anyhow::bail!("not a alter table statement"),
        }
    }

    /// we don't know what the old owner is, so we can only revert to session_user
    fn revert(&self) -> anyhow::Result<NodeEnum> {
        let sql = format!("ALTER TABLE {} DISABLE ROW LEVEL SECURITY", self.id);
        let parsed = pg_query::parse(&sql)?;
        let node = parsed.protobuf.nodes()[0].0;
        match node {
            NodeRef::AlterTableStmt(stmt) => Ok(NodeEnum::AlterTableStmt(stmt.clone())),
            _ => anyhow::bail!("not a alter table RLS statement"),
        }
    }
}

impl FromStr for TableRls {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        let parsed = pg_query::parse(s)?;
        let node = parsed.protobuf.nodes()[0].0;
        match node {
            NodeRef::AlterTableStmt(stmt) => AlterTable::try_from(stmt)?.try_into(),
            _ => anyhow::bail!("not a alter table RLS statement: {}", s),
        }
    }
}

impl TryFrom<AlterTable> for TableRls {
    type Error = anyhow::Error;
    fn try_from(AlterTable { id, action, node }: AlterTable) -> Result<Self, Self::Error> {
        match action {
            AlterTableAction::Rls => Ok(TableRls::new(id, node)),
            _ => anyhow::bail!("not an owner change"),
        }
    }
}

impl MigrationPlanner for NodeDiff<TableRls> {
    type Migration = String;

    fn drop(&self) -> MigrationResult<Self::Migration> {
        if let Some(old) = &self.old {
            let sql = old.revert()?.deparse()?;
            Ok(vec![sql])
        } else {
            Ok(vec![])
        }
    }

    fn create(&self) -> MigrationResult<Self::Migration> {
        if let Some(new) = &self.new {
            let sql = new.node.deparse()?;
            Ok(vec![sql])
        } else {
            Ok(vec![])
        }
    }

    fn alter(&self) -> MigrationResult<Self::Migration> {
        Ok(vec![])
    }
}

impl TableRls {
    fn new(id: SchemaId, node: NodeEnum) -> Self {
        Self { id, node }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_rls_should_parse() {
        let sql = "ALTER TABLE foo ENABLE ROW LEVEL SECURITY";
        let parsed = TableRls::from_str(sql).unwrap();
        assert_eq!(parsed.id, SchemaId::new("public", "foo"));
    }

    #[test]
    fn table_rls_should_revert() {
        let sql = "ALTER TABLE foo ENABLE ROW LEVEL SECURITY";
        let parsed = TableRls::from_str(sql).unwrap();
        let reverted = parsed.revert().unwrap().deparse().unwrap();
        assert_eq!(
            reverted,
            "ALTER TABLE public.foo DISABLE ROW LEVEL SECURITY"
        );
    }

    #[test]
    fn table_rls_should_generate_drop_create_migration() {
        let sql1 = "ALTER TABLE foo ENABLE ROW LEVEL SECURITY";

        let diff: NodeDiff<TableRls> = NodeDiff {
            old: Some(sql1.parse().unwrap()),
            new: None,
            diff: sql1.to_string(),
        };
        let plan = diff.plan().unwrap();
        assert_eq!(plan, &["ALTER TABLE public.foo DISABLE ROW LEVEL SECURITY"]);
    }
}
