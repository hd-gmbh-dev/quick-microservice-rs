use std::sync::Arc;

use qm_pg::DB;

use crate::{
    cache::{
        update::{Op, Payload},
        User, UserEntityUpdate, UserMap,
    },
    query::fetch_users,
};

use super::realm::Realm;

#[derive(Default)]
pub struct Users {
    pub user_id_map: UserMap,
    pub users: UserMap,
    pub user_email_map: UserMap,
}

impl Users {
    pub async fn new(db: &DB, realm: &str) -> anyhow::Result<Self> {
        let user_id_map = fetch_users(db, realm)
            .await?
            .into_iter()
            .filter(|row| row.has_all_fields())
            .fold(UserMap::default(), |mut state, row| {
                let id: Arc<str> = Arc::from(row.id.unwrap());
                let firstname: Arc<str> = Arc::from(row.firstname.unwrap());
                let lastname: Arc<str> = Arc::from(row.lastname.unwrap());
                let username: Arc<str> = Arc::from(row.username.unwrap());
                let email: Arc<str> = Arc::from(row.email.unwrap());
                state.entry(id.clone()).or_insert(Arc::new(User {
                    id,
                    username,
                    email,
                    firstname,
                    lastname,
                    enabled: row.enabled,
                }));
                state
            });
        let users = UserMap::from_iter(
            user_id_map
                .values()
                .map(|v| (v.username.clone(), v.clone())),
        );
        let user_email_map =
            UserMap::from_iter(user_id_map.values().map(|v| (v.email.clone(), v.clone())));

        Ok(Self {
            user_id_map,
            users,
            user_email_map,
        })
    }

    pub fn total(&self) -> i64 {
        self.user_id_map.len() as i64
    }

    pub fn new_user(&mut self, user: Arc<User>) {
        self.user_id_map.insert(user.id.clone(), user.clone());
        self.users.insert(user.username.clone(), user.clone());
        self.user_email_map.insert(user.email.clone(), user);
    }

    pub fn list(&self) -> Arc<[Arc<User>]> {
        self.user_id_map.values().cloned().collect()
    }

    pub fn get(&self, user_id: &str) -> Option<&Arc<User>> {
        self.user_id_map.get(user_id)
    }

    pub fn by_username(&self, username: &str) -> Option<&Arc<User>> {
        self.users.get(username)
    }

    pub fn by_email(&self, email: &str) -> Option<&Arc<User>> {
        self.user_email_map.get(email)
    }

    pub fn contains(&self, user_id: &str) -> bool {
        self.user_id_map.contains_key(user_id)
    }

    pub fn update(&mut self, realm: &Realm, payload: &str) -> anyhow::Result<()> {
        let payload: Payload<UserEntityUpdate> = serde_json::from_str(payload)?;
        match (payload.op, payload.new, payload.old) {
            (Op::Insert, Some(new), None) => {
                if realm.equals(new.realm_id.as_deref()) && new.has_all_fields() {
                    let user = Arc::new(User {
                        id: new.id,
                        username: new.username,
                        email: new.email.unwrap(),
                        firstname: new.first_name.unwrap(),
                        lastname: new.last_name.unwrap(),
                        enabled: new.enabled,
                    });
                    self.new_user(user);
                }
            }
            (Op::Update, Some(new), Some(old)) => {
                if realm.equals(new.realm_id.as_deref())
                    && realm.equals(old.realm_id.as_deref())
                    && new.has_all_fields()
                {
                    let user = Arc::new(User {
                        id: new.id,
                        username: new.username,
                        email: new.email.unwrap(),
                        firstname: new.first_name.unwrap(),
                        lastname: new.last_name.unwrap(),
                        enabled: new.enabled,
                    });
                    self.user_id_map.remove(&user.id);
                    self.users.remove(&user.username);
                    self.user_email_map.remove(&user.email);
                    self.new_user(user);
                }
            }
            (Op::Delete, None, Some(old)) => {
                if realm.equals(old.realm_id.as_deref()) {
                    self.user_id_map.remove(&old.id);
                    self.users.remove(&old.username);
                    if old.email.is_some() {
                        self.user_email_map.remove(&old.email.unwrap());
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}
