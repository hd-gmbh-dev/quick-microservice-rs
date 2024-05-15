use std::path::{Path, PathBuf};

mod model;
mod parser;
mod reader;
mod writer;

pub fn generate(input_file_path: &Path) -> anyhow::Result<()> {
    let out = input_file_path.with_extension("rs");
    let file_name = out
        .file_name()
        .ok_or(anyhow::anyhow!("invalid input filename"))?;
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);
    let out_file_path = out_dir.join(file_name);

    let tables = reader::Reader::from_file(input_file_path)?.read()?;
    let parse_result = crate::parser::parse(tables)?;

    writer::Writer::from_file(out_file_path)?.write(parse_result)?;

    Ok(())
}

pub fn generate_to_writer<W: std::io::Write>(
    input_file_path: &Path,
    writer: W,
) -> anyhow::Result<()> {
    let tables = reader::Reader::from_file(input_file_path)?.read()?;
    let parse_result = crate::parser::parse(tables)?;

    writer::Writer::from_writer(writer).write(parse_result)?;

    Ok(())
}

#[cfg(test)]
mod test {
    use crate::{
        model::{RoleMapping, Table},
        reader::Reader,
    };
    use std::rc::Rc;

    const TEST_INPUT: &str = r#"# User Groups `user_groups`

| Name                  | Path                  | Display Name         | Access Levels            | Allowed Types |
| --------------------- | --------------------- | -------------------- | ------------------------ | ------------- |
| Admin                 | /administration_owner | Admin                | Admin                    | none          |
| CustomerOwner         | /customer_owner       | Owner of Customer    | Customer                 | none          |
| InstitutionOwner      | /institution_owner    | Owner of Institution | Institution              | eco,state     |
| Reader                | /employee_reader      | Reader               | Customer, Institution    | eco           |

# Role Mappings `roles`

| Roles           | Admin   | InstitutionOwner | Reader |
| --------------- | ------- | ---------------- | ------ |
| administration  | x       |                  |        |
| user:list       |         | x                |        |
| user:view       |         | x                |        |
| user:update     |         | x                |        |
| user:create     |         | x                |        |
| user:delete     |         | x                |        |
| entity:list     |         | x                | x      |
| entity:view     |         | x                | x      |
| entity:update   |         | x                |        |
| entity:create   |         | x                |        |
| entity:delete   |         | x                |        |"#;

    #[test]
    fn test_md_table_reader() -> anyhow::Result<()> {
        let result = Reader::from_str(TEST_INPUT).read()?;
        assert_eq!(
            result.user_groups,
            Table {
                headers: vec![
                    "Name".to_string(),
                    "Path".to_string(),
                    "Display Name".to_string(),
                    "Access Levels".to_string(),
                    "Allowed Types".to_string(),
                ],
                rows: vec![
                    vec![
                        "Admin".to_string(),
                        "/administration_owner".to_string(),
                        "Admin".to_string(),
                        "Admin".to_string(),
                        "none".to_string(),
                    ],
                    vec![
                        "CustomerOwner".to_string(),
                        "/customer_owner".to_string(),
                        "Owner of Customer".to_string(),
                        "Customer".to_string(),
                        "none".to_string(),
                    ],
                    vec![
                        "InstitutionOwner".to_string(),
                        "/institution_owner".to_string(),
                        "Owner of Institution".to_string(),
                        "Institution".to_string(),
                        "eco,state".to_string(),
                    ],
                    vec![
                        "Reader".to_string(),
                        "/employee_reader".to_string(),
                        "Reader".to_string(),
                        "Customer, Institution".to_string(),
                        "eco".to_string(),
                    ],
                ],
            }
        );
        assert_eq!(
            result.roles,
            Table {
                headers: vec![
                    "Roles".to_string(),
                    "Admin".to_string(),
                    "InstitutionOwner".to_string(),
                    "Reader".to_string()
                ],
                rows: vec![
                    vec![
                        "administration".to_string(),
                        "x".to_string(),
                        "".to_string(),
                        "".to_string()
                    ],
                    vec![
                        "user:list".to_string(),
                        "".to_string(),
                        "x".to_string(),
                        "".to_string()
                    ],
                    vec![
                        "user:view".to_string(),
                        "".to_string(),
                        "x".to_string(),
                        "".to_string()
                    ],
                    vec![
                        "user:update".to_string(),
                        "".to_string(),
                        "x".to_string(),
                        "".to_string()
                    ],
                    vec![
                        "user:create".to_string(),
                        "".to_string(),
                        "x".to_string(),
                        "".to_string()
                    ],
                    vec![
                        "user:delete".to_string(),
                        "".to_string(),
                        "x".to_string(),
                        "".to_string()
                    ],
                    vec![
                        "entity:list".to_string(),
                        "".to_string(),
                        "x".to_string(),
                        "x".to_string()
                    ],
                    vec![
                        "entity:view".to_string(),
                        "".to_string(),
                        "x".to_string(),
                        "x".to_string()
                    ],
                    vec![
                        "entity:update".to_string(),
                        "".to_string(),
                        "x".to_string(),
                        "".to_string()
                    ],
                    vec![
                        "entity:create".to_string(),
                        "".to_string(),
                        "x".to_string(),
                        "".to_string()
                    ],
                    vec![
                        "entity:delete".to_string(),
                        "".to_string(),
                        "x".to_string(),
                        "".to_string()
                    ],
                ],
            },
        );
        Ok(())
    }

    #[test]
    fn test_md_table_parser() -> anyhow::Result<()> {
        let result = crate::parser::parse(Reader::from_str(TEST_INPUT).read()?)?;
        assert_eq!(
            &RoleMapping {
                user_group: Rc::from("Admin"),
                roles: Rc::from([Rc::from("administration")]),
            },
            &result.role_mappings[0]
        );
        assert_eq!(
            &RoleMapping {
                user_group: Rc::from("InstitutionOwner"),
                roles: Rc::from([
                    Rc::from("user:list"),
                    Rc::from("user:view"),
                    Rc::from("user:update"),
                    Rc::from("user:create"),
                    Rc::from("user:delete"),
                    Rc::from("entity:list"),
                    Rc::from("entity:view"),
                    Rc::from("entity:update"),
                    Rc::from("entity:create"),
                    Rc::from("entity:delete"),
                ]),
            },
            &result.role_mappings[1]
        );
        assert_eq!(
            &RoleMapping {
                user_group: Rc::from("Reader"),
                roles: Rc::from([Rc::from("entity:list"), Rc::from("entity:view"),]),
            },
            &result.role_mappings[2]
        );
        Ok(())
    }

    #[test]
    fn test_roles_writer() -> anyhow::Result<()> {
        let result = crate::parser::parse(Reader::from_str(TEST_INPUT).read()?)?;
        let code = crate::writer::Writer::in_memory()
            .write(result)?
            .into_inner();
        eprintln!("{code}");
        Ok(())
    }
}
