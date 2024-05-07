use crate::model::{MdTables, OptMdTables, Table};
use std::path::Path;

pub struct Reader<R> {
    r: R,
}

#[cfg(test)]
impl<'a> Reader<std::io::Cursor<&'a str>> {
    pub fn from_str(s: &'a str) -> Self {
        Self {
            r: std::io::Cursor::new(s),
        }
    }
}

impl Reader<std::io::BufReader<std::fs::File>> {
    pub fn from_file<P>(p: P) -> std::io::Result<Self>
    where
        P: AsRef<Path>,
    {
        Ok(Self {
            r: std::io::BufReader::new(std::fs::File::open(p)?),
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum CurrentTable {
    UserGroups,
    Roles,
    None,
}

fn set_table(rows: &mut Vec<Vec<String>>, tables: &mut OptMdTables, current_table: &CurrentTable) {
    let headers = rows.remove(0);
    let mut table_rows = Vec::with_capacity(rows.len());
    std::mem::swap(&mut table_rows, rows);
    let table = Table {
        headers,
        rows: table_rows,
    };
    match current_table {
        CurrentTable::UserGroups => {
            tables.user_groups = Some(table);
        }
        CurrentTable::Roles => {
            tables.roles = Some(table);
        }
        _ => {}
    }
    rows.clear();
}

impl<R> Reader<R>
where
    R: std::io::BufRead,
{
    pub fn read(self) -> anyhow::Result<MdTables> {
        let line_reader = self.r.lines();

        let mut rows = vec![];
        let mut current_table = CurrentTable::None;

        let mut tables = OptMdTables::default();

        for line in line_reader.into_iter().flatten() {

            if line.trim().starts_with('|') {
                let mut row = Vec::new();
                let s = line.split('|');
                let mut is_first = true;
                s.for_each(|col| {
                    if is_first {
                        is_first = false;
                    } else {
                        row.push(col.trim().to_string());
                    }
                });
                row.pop();
                let is_divider = row
                    .iter()
                    .all(|s| s.contains('-') && s.replace('-', "") == "");
                if !is_divider {
                    rows.push(row);
                }
            } else if !rows.is_empty() {
                set_table(&mut rows, &mut tables, &current_table);
            } else {
                if line.contains("`user_groups`") {
                    current_table = CurrentTable::UserGroups;
                }
                if line.contains("`roles`") {
                    current_table = CurrentTable::Roles;
                }
            }
        }

        if !rows.is_empty() {
            set_table(&mut rows, &mut tables, &current_table);
        }

        tables.try_into()
    }
}
