use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

use crate::parser::ParseResult;

pub struct WriteResult<W> {
    _w: W,
}

pub struct Writer<W> {
    w: W,
}

impl<W> Writer<W> {
    pub fn from_writer(w: W) -> Self {
        Self { w }
    }
}

#[cfg(test)]
impl WriteResult<std::io::Cursor<Vec<u8>>> {
    pub fn into_inner(self) -> String {
        String::from_utf8(self._w.into_inner()).unwrap()
    }
}

#[cfg(test)]
impl Writer<std::io::Cursor<Vec<u8>>> {
    pub fn in_memory() -> Self {
        Self {
            w: std::io::Cursor::new(Vec::default()),
        }
    }
}

impl Writer<std::fs::File> {
    pub fn from_file<P>(p: P) -> std::io::Result<Self>
    where
        P: AsRef<Path>,
    {
        std::fs::File::create(p).map(Self::from_writer)
    }
}

const ENUM_DERIVE: &str =
    "#[derive(Clone, Debug, Copy, EnumString, EnumIter, AsRefStr, Ord, PartialOrd, Eq, PartialEq, Hash)]";
const ENUM_DERIVE_BUILT_IN_GROUP: &str =
    "#[derive(Clone, Debug, Copy, EnumString, async_graphql::Enum, AsRefStr, Ord, PartialOrd, Eq, PartialEq, Hash)]";

impl<W> Writer<W>
where
    W: std::io::Write,
{
    fn write_line(&mut self, depth: usize, line: &str) -> anyhow::Result<()> {
        self.w
            .write_all(format!("{}{}\n", ("    ").to_owned().repeat(depth), line).as_bytes())?;
        Ok(())
    }

    pub fn write(mut self, parse_result: ParseResult) -> anyhow::Result<WriteResult<W>> {
        let ParseResult {
            permissions,
            resources,
            role_mappings,
            user_group_name_mappings,
            ..
        } = parse_result;
        let user_group_name_mappings = BTreeMap::from_iter(
            user_group_name_mappings
                .into_iter()
                .map(|v| (v.user_group, (v.path, v.display_name, v.allowed_types))),
        );
        self.write_line(0, "use strum::{EnumString, EnumIter, AsRefStr};")?;
        self.write_line(0, "")?;
        self.write_line(0, ENUM_DERIVE)?;
        self.write_line(0, "pub enum Permission {")?;
        for permission in permissions.iter() {
            self.write_line(
                1,
                &format!("#[strum(serialize = \"{}\")]", permission.as_ref()),
            )?;
            self.write_line(
                1,
                &format!(
                    "{},",
                    inflector::cases::classcase::to_class_case(permission.as_ref())
                ),
            )?;
        }
        self.write_line(1, "#[strum(serialize = \"none\")]")?;
        self.write_line(1, "None,")?;
        self.write_line(0, "}")?;
        self.write_line(0, "")?;
        self.write_line(0, ENUM_DERIVE)?;
        self.write_line(0, "pub enum Resource {")?;
        for resource in resources.iter() {
            self.write_line(
                1,
                &format!("#[strum(serialize = \"{}\")]", resource.as_ref()),
            )?;
            self.write_line(
                1,
                &format!(
                    "{},",
                    inflector::cases::classcase::to_class_case(resource.as_ref())
                ),
            )?;
        }
        self.write_line(0, "}")?;
        self.write_line(0, "")?;

        let mut group_names = BTreeSet::new();
        let mut fn_names = vec![];
        for role_mapping in role_mappings.iter() {
            if let Some((user_group_name, user_group_display_name, allowed_types)) =
                user_group_name_mappings.get(&role_mapping.user_group)
            {
                group_names.insert(user_group_name);
                let cnst_name = inflector::cases::screamingsnakecase::to_screaming_snake_case(
                    role_mapping.user_group.as_ref(),
                );
                self.write_line(
                    0,
                    &format!(
                        "pub const {cnst_name}_PATH: &str = \"/app{}\";",
                        user_group_name.as_ref()
                    ),
                )?;
                let fn_name =
                    inflector::cases::snakecase::to_snake_case(role_mapping.user_group.as_ref());
                self.write_line(
                    0,
                    &format!(
                        "pub fn {fn_name}_group() -> qm::role::Group<Resource, Permission> {}",
                        "{"
                    ),
                )?;
                self.write_line(
                    1,
                    &format!(
                        "qm::role::Group::new(\"{}\".to_string(), {cnst_name}_PATH.to_string(), vec![{}], vec![",
                        user_group_display_name,
                        allowed_types.as_ref().split(',').filter(|&s| s != "none").map(|s| format!("{s:?}.into()")).collect::<Vec<String>>().join(","),
                    ),
                )?;
                for role in role_mapping.roles.iter() {
                    if let Some((resource, permission)) = role.as_ref().split_once(':') {
                        let resource = inflector::cases::classcase::to_class_case(resource);
                        let permission = inflector::cases::classcase::to_class_case(permission);
                        self.write_line(2, &format!("qm::role::Role::new(Resource::{resource}, Some(Permission::{permission})),"))?;
                    } else {
                        let resource = inflector::cases::classcase::to_class_case(role);
                        self.write_line(
                            2,
                            &format!("qm::role::Role::new(Resource::{resource}, None),"),
                        )?;
                    }
                }
                self.write_line(1, "])")?;
                self.write_line(0, "}")?;
                fn_names.push(fn_name);
            }
        }
        self.write_line(0, "")?;
        self.write_line(
            0,
            "pub fn groups() -> Vec<qm::role::Group<Resource, Permission>> {",
        )?;
        self.write_line(1, "vec![")?;
        for fn_name in fn_names {
            self.write_line(2, &format!("{fn_name}_group(),"))?;
        }
        self.write_line(1, "]")?;
        self.write_line(0, "}")?;

        self.write_line(0, "")?;

        self.write_line(0, "pub fn roles() -> std::collections::BTreeSet<String> {")?;
        self.write_line(1, "let mut map = std::collections::BTreeSet::default();")?;
        self.write_line(1, "for group in groups() {")?;
        self.write_line(2, "for resource in group.resources() {")?;
        self.write_line(3, "map.insert(resource);")?;
        self.write_line(2, "}")?;
        self.write_line(1, "}")?;
        self.write_line(1, "map")?;
        self.write_line(0, "}")?;

        self.write_line(0, "")?;
        self.write_line(
            0,
            &format!(
                "pub const BUILT_IN_GROUPS: [&str; {}] = [",
                group_names.len()
            ),
        )?;
        for group_name in group_names.iter() {
            let n =
                inflector::cases::screamingsnakecase::to_screaming_snake_case(group_name.as_ref());
            self.write_line(1, &format!("{n}_PATH,"))?;
        }
        self.write_line(0, "];")?;
        self.write_line(0, "")?;
        self.write_line(0, ENUM_DERIVE_BUILT_IN_GROUP)?;
        self.write_line(0, "pub enum BuiltInGroup {")?;
        for group_name in group_names.iter() {
            self.write_line(
                1,
                &format!(
                    "#[strum(serialize = \"/app/{}\")]",
                    inflector::cases::snakecase::to_snake_case(group_name.as_ref())
                ),
            )?;
            self.write_line(
                1,
                &format!(
                    "{},",
                    inflector::cases::classcase::to_class_case(group_name.as_ref())
                ),
            )?;
        }
        self.write_line(0, "}")?;
        self.write_line(0, "")?;
        self.write_line(
            0,
            "impl From<BuiltInGroup> for qm::role::Group<Resource, Permission> {",
        )?;
        self.write_line(1, "fn from(val: BuiltInGroup) -> Self {")?;
        self.write_line(2, "match val {")?;
        for r in role_mappings.iter() {
            let fn_name = inflector::cases::snakecase::to_snake_case(r.user_group.as_ref());
            self.write_line(
                3,
                &format!("BuiltInGroup::{} => {fn_name}_group(),", r.user_group),
            )?;
        }
        self.write_line(2, "}")?;
        self.write_line(1, "}")?;
        self.write_line(0, "}")?;
        Ok(WriteResult { _w: self.w })
    }
}
