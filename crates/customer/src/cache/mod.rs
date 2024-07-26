use prometheus_client::metrics::gauge::Gauge;

use qm_entity::ids::PartialEqual;
use qm_entity::ids::{CustomerId, CustomerOrOrganization, InfraContext, InfraId};
use qm_entity::model::ListFilter;

use std::str::FromStr;
use std::sync::atomic::AtomicI64;
use std::sync::Arc;
use tokio::{runtime::Builder, task::LocalSet};

pub mod infra;
pub mod update;
pub mod user;

use crate::cache::infra::InfraDB;
use crate::cache::user::UserDB;
use crate::model::*;

struct Inner {
    infra: InfraDB,
    user: UserDB,
}

#[derive(Clone)]
pub struct CacheDB {
    inner: Arc<Inner>,
}

impl CacheDB {
    pub async fn new(
        customer_db: &qm_pg::DB,
        keycloak_db: &qm_pg::DB,
        realm: &str,
        realm_admin_username: &str,
    ) -> anyhow::Result<Self> {
        let infra = InfraDB::new(customer_db).await?;
        let user = UserDB::new(keycloak_db, realm, realm_admin_username).await?;
        Ok(Self {
            inner: Arc::new(Inner { infra, user }),
        })
    }

    pub fn user(&self) -> &UserDB {
        &self.inner.user
    }

    pub fn infra(&self) -> &InfraDB {
        &self.inner.infra
    }

    pub fn customers_total(&self) -> &Gauge<i64, AtomicI64> {
        &self.inner.infra.customers_total
    }

    pub fn organizations_total(&self) -> &Gauge<i64, AtomicI64> {
        &self.inner.infra.organizations_total
    }

    pub fn organization_units_total(&self) -> &Gauge<i64, AtomicI64> {
        &self.inner.infra.organization_units_total
    }

    pub fn institutions_total(&self) -> &Gauge<i64, AtomicI64> {
        &self.inner.infra.institutions_total
    }

    pub async fn customer_list(
        &self,
        filter: Option<ListFilter>,
        ty: Option<String>,
    ) -> CustomerList {
        let customers = self.inner.infra.customers.read().await;
        let iter = if let Some(ty) = ty.as_ref() {
            itertools::Either::Right(customers.values().filter(|c| c.ty.as_ref() == ty.as_str()))
        } else {
            itertools::Either::Left(customers.values())
        };
        if let Some(filter) = filter {
            let page = filter.page.unwrap_or(0);
            let limit = filter.limit.unwrap_or(100);
            let offset = page * limit;
            let items: Arc<[Arc<Customer>]> = iter.skip(offset).take(limit).cloned().collect();
            CustomerList {
                items,
                limit: Some(limit as i64),
                total: Some(self.inner.infra.customers_total.get()),
                page: Some(page as i64),
            }
        } else {
            let items: Arc<[Arc<Customer>]> = iter.cloned().collect();
            CustomerList {
                items,
                limit: None,
                total: Some(self.inner.infra.customers_total.get()),
                page: Some(0),
            }
        }
    }

    pub async fn organization_list(
        &self,
        customer_id: Option<CustomerId>,
        filter: Option<ListFilter>,
        ty: Option<String>,
    ) -> OrganizationList {
        let organizations = self.inner.infra.organizations.read().await;
        let iter = if let Some(ty) = ty.as_ref() {
            itertools::Either::Right(
                organizations
                    .values()
                    .filter(|c| c.ty.as_ref() == ty.as_str()),
            )
        } else {
            itertools::Either::Left(organizations.values())
        };
        let iter = if let Some(customer_id) = customer_id.as_ref() {
            itertools::Either::Right(iter.filter(|v| v.as_ref().partial_equal(customer_id)))
        } else {
            itertools::Either::Left(iter)
        };
        if let Some(filter) = filter {
            let page = filter.page.unwrap_or(0);
            let limit = filter.limit.unwrap_or(100);
            let offset = page * limit;
            let items: Arc<[Arc<Organization>]> = iter.skip(offset).take(limit).cloned().collect();
            OrganizationList {
                items,
                limit: Some(limit as i64),
                total: Some(self.inner.infra.organizations_total.get()),
                page: Some(page as i64),
            }
        } else {
            let items: Arc<[Arc<Organization>]> = iter.cloned().collect();
            OrganizationList {
                items,
                limit: None,
                total: Some(self.inner.infra.organizations_total.get()),
                page: Some(0),
            }
        }
    }

    pub async fn organization_unit_list(
        &self,
        customer_or_organization: Option<CustomerOrOrganization>,
        filter: Option<ListFilter>,
        ty: Option<String>,
    ) -> OrganizationUnitList {
        let organization_units = self.inner.infra.organization_units.read().await;
        let iter = if let Some(ty) = ty.as_ref() {
            itertools::Either::Right(
                organization_units
                    .values()
                    .filter(|c| c.ty.as_ref() == ty.as_str()),
            )
        } else {
            itertools::Either::Left(organization_units.values())
        };

        let iter = match &customer_or_organization {
            Some(CustomerOrOrganization::Customer(customer_id)) => itertools::Either::Left(
                itertools::Either::Left(iter.filter(|v| v.as_ref().partial_equal(customer_id))),
            ),
            Some(CustomerOrOrganization::Organization(organization_id)) => {
                itertools::Either::Left(itertools::Either::Right(
                    iter.filter(|v| v.as_ref().partial_equal(organization_id)),
                ))
            }
            _ => itertools::Either::Right(iter),
        };
        if let Some(filter) = filter {
            let page = filter.page.unwrap_or(0);
            let limit = filter.limit.unwrap_or(100);
            let offset = page * limit;
            let items: Arc<[Arc<OrganizationUnit>]> =
                iter.skip(offset).take(limit).cloned().collect();
            OrganizationUnitList {
                items,
                limit: Some(limit as i64),
                total: Some(self.inner.infra.organization_units_total.get()),
                page: Some(page as i64),
            }
        } else {
            let items: Arc<[Arc<OrganizationUnit>]> = iter.cloned().collect();
            OrganizationUnitList {
                items,
                limit: None,
                total: Some(self.inner.infra.organization_units_total.get()),
                page: Some(0),
            }
        }
    }

    pub async fn institution_list(
        &self,
        customer_or_organization: Option<CustomerOrOrganization>,
        filter: Option<ListFilter>,
        ty: Option<String>,
    ) -> InstitutionList {
        let institutions = self.inner.infra.institutions.read().await;
        let iter = if let Some(ty) = ty.as_ref() {
            itertools::Either::Right(
                institutions
                    .values()
                    .filter(|c| c.ty.as_ref() == ty.as_str()),
            )
        } else {
            itertools::Either::Left(institutions.values())
        };
        let iter = match &customer_or_organization {
            Some(CustomerOrOrganization::Customer(customer_id)) => itertools::Either::Left(
                itertools::Either::Left(iter.filter(|v| v.as_ref().partial_equal(customer_id))),
            ),
            Some(CustomerOrOrganization::Organization(organization_id)) => {
                itertools::Either::Left(itertools::Either::Right(
                    iter.filter(|v| v.as_ref().partial_equal(organization_id)),
                ))
            }
            _ => itertools::Either::Right(iter),
        };
        if let Some(filter) = filter {
            let page = filter.page.unwrap_or(0);
            let limit = filter.limit.unwrap_or(100);
            let offset = page * limit;
            let items: Arc<[Arc<Institution>]> = iter.skip(offset).take(limit).cloned().collect();
            InstitutionList {
                items,
                limit: Some(limit as i64),
                total: Some(self.inner.infra.institutions_total.get()),
                page: Some(page as i64),
            }
        } else {
            let items: Arc<[Arc<Institution>]> = iter.cloned().collect();
            InstitutionList {
                items,
                limit: None,
                total: Some(self.inner.infra.institutions_total.get()),
                page: Some(0),
            }
        }
    }

    pub async fn user_list(
        &self,
        context: Option<InfraContext>,
        filter: Option<ListFilter>,
    ) -> UserList {
        let users = self.inner.user.users.read().await;
        let user_roles = self.inner.user.user_roles.read().await;
        let roles = self.inner.user.roles.read().await;
        let user_groups = self.inner.user.user_groups.read().await;
        let groups = self.inner.user.groups.read().await;
        let group_attributes = self.inner.user.group_attributes.read().await;
        let o = self.inner.infra.organization_unit_id_map.read().await;
        let institutions = match context {
            Some(InfraContext::OrganizationUnit(v)) => {
                let unit = o.get(&v.into());
                unit.map(|u| u.members.as_ref()).unwrap_or(&[])
            }
            _ => &[],
        };
        let user_list = users.list();
        let iter = user_list.iter().map(|u| {
            let context = user_roles
                .by_user_id(&u.id)
                .and_then(|r| r.iter().find_map(|r| roles.get(r).and_then(|r| r.context)));
            let access = user_roles.by_user_id(&u.id).and_then(|r| {
                r.iter().find_map(|r| {
                    roles
                        .get(r)
                        .and_then(|r| qm_role::Access::from_str(r.name.as_ref()).ok())
                })
            });
            let group = user_groups.by_user_id(&u.id).and_then(|g| {
                g.iter().find_map(|g| {
                    groups
                        .get(g)
                        .and_then(|r| group_attributes.get(&r.id).cloned())
                })
            });
            UserDetails {
                user: u.clone(),
                context,
                access,
                group,
            }
        });
        if let Some(filter) = filter {
            let page = filter.page.unwrap_or(0);
            let limit = filter.limit.unwrap_or(100);
            let offset = page * limit;
            let items: Vec<UserDetails> = if let Some(context) = context {
                iter.filter(|v| {
                    v.partial_equal(&context) || institutions.iter().any(|i| v.partial_equal(i))
                })
                .skip(offset)
                .take(limit)
                .collect::<Vec<UserDetails>>()
            } else {
                iter.skip(offset).take(limit).collect::<Vec<UserDetails>>()
            };
            UserList {
                items: Arc::from(items),
                limit: Some(limit as i64),
                total: Some(self.inner.user.users_total.get()),
                page: Some(page as i64),
            }
        } else {
            let items: Vec<UserDetails> = if let Some(context) = context {
                iter.filter(|v| {
                    v.partial_equal(&context) || institutions.iter().any(|i| v.partial_equal(i))
                })
                .collect::<Vec<UserDetails>>()
            } else {
                iter.collect::<Vec<UserDetails>>()
            };
            UserList {
                items: Arc::from(items),
                limit: None,
                total: Some(self.inner.user.users_total.get()),
                page: Some(0),
            }
        }
    }

    pub async fn customer_by_id(&self, id: &InfraId) -> Option<Arc<Customer>> {
        self.inner
            .infra
            .customer_id_map
            .read()
            .await
            .get(id)
            .cloned()
    }

    pub async fn customer_by_name(&self, name: &str) -> Option<Arc<Customer>> {
        self.inner.infra.customers.read().await.get(name).cloned()
    }

    pub async fn organization_by_id(&self, id: &InfraId) -> Option<Arc<Organization>> {
        self.inner
            .infra
            .organization_id_map
            .read()
            .await
            .get(id)
            .cloned()
    }

    pub async fn organization_unit_by_name(
        &self,
        cid: InfraId,
        oid: Option<InfraId>,
        name: Arc<str>,
    ) -> Option<Arc<OrganizationUnit>> {
        self.inner
            .infra
            .organization_units
            .read()
            .await
            .get(&(name, cid, oid))
            .cloned()
    }

    pub async fn organization_by_name(
        &self,
        cid: InfraId,
        name: Arc<str>,
    ) -> Option<Arc<Organization>> {
        self.inner
            .infra
            .organizations
            .read()
            .await
            .get(&(name, cid))
            .cloned()
    }

    pub async fn institution_by_name(
        &self,
        cid: InfraId,
        oid: InfraId,
        name: Arc<str>,
    ) -> Option<Arc<Institution>> {
        self.inner
            .infra
            .institutions
            .read()
            .await
            .get(&(name, cid, oid))
            .cloned()
    }

    pub async fn institution_by_id(&self, id: &InfraId) -> Option<Arc<Institution>> {
        self.inner
            .infra
            .institution_id_map
            .read()
            .await
            .get(id)
            .cloned()
    }

    pub async fn organization_unit_by_id(&self, id: &InfraId) -> Option<Arc<OrganizationUnit>> {
        self.inner
            .infra
            .organization_unit_id_map
            .read()
            .await
            .get(id)
            .cloned()
    }

    pub fn users_total(&self) -> &Gauge<i64, AtomicI64> {
        &self.inner.user.users_total
    }

    pub fn roles_total(&self) -> &Gauge<i64, AtomicI64> {
        &self.inner.user.roles_total
    }

    pub fn groups_total(&self) -> &Gauge<i64, AtomicI64> {
        &self.inner.user.groups_total
    }

    pub async fn group_detail_by_id(&self, id: &str) -> Option<Arc<GroupDetail>> {
        self.inner
            .user
            .group_attributes
            .read()
            .await
            .get(id)
            .cloned()
    }

    pub async fn user_by_id(&self, id: &str) -> Option<Arc<User>> {
        self.inner.user.users.read().await.get(id).cloned()
    }

    pub async fn user_details_by_id(&self, id: &str) -> Option<UserDetails> {
        let users = self.inner.user.users.read().await;
        let user_roles = self.inner.user.user_roles.read().await;
        let roles = self.inner.user.roles.read().await;
        let user_groups = self.inner.user.user_groups.read().await;
        let groups = self.inner.user.groups.read().await;
        let group_attributes = self.inner.user.group_attributes.read().await;
        users.get(id).map(|u| {
            let context = user_roles
                .by_user_id(&u.id)
                .and_then(|r| r.iter().find_map(|r| roles.get(r).and_then(|r| r.context)));
            let access = user_roles.by_user_id(&u.id).and_then(|r| {
                r.iter().find_map(|r| {
                    roles
                        .get(r)
                        .and_then(|r| qm_role::Access::from_str(r.name.as_ref()).ok())
                })
            });
            let group = user_groups.by_user_id(&u.id).and_then(|g| {
                g.iter().find_map(|g| {
                    groups
                        .get(g)
                        .and_then(|r| group_attributes.get(&r.id).cloned())
                })
            });
            UserDetails {
                user: u.clone(),
                context,
                access,
                group,
            }
        })
    }

    pub async fn user_by_username(&self, username: &str) -> Option<Arc<User>> {
        self.inner
            .user
            .users
            .read()
            .await
            .by_username(username)
            .cloned()
    }

    pub async fn user_by_email(&self, email: &str) -> Option<Arc<User>> {
        self.inner.user.users.read().await.by_email(email).cloned()
    }

    pub async fn users(&self) -> Arc<[Arc<User>]> {
        self.inner.user.users.read().await.list()
    }

    pub async fn roles(&self) -> Arc<[Arc<Role>]> {
        self.inner.user.roles.read().await.list()
    }

    pub async fn group_by_id(&self, group_id: &str) -> Option<Arc<Group>> {
        self.inner.user.groups.read().await.get(group_id).cloned()
    }

    pub async fn group_id_by_path(&self, path: &str) -> Option<String> {
        if path.is_empty() {
            return None;
        }
        let mut s = path[1..].split('/');
        if let Some((parent, name)) = s.next().zip(s.next()) {
            let m = self.inner.user.groups.read().await;
            m.by_parent(parent)
                .and_then(|v| v.get(name))
                .map(|g| g.id.to_string())
        } else {
            None
        }
    }

    pub async fn groups_by_parent(&self, parent_name: &str) -> Vec<Arc<Group>> {
        self.inner
            .user
            .groups
            .read()
            .await
            .by_parent(parent_name)
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default()
    }

    pub async fn role_by_name(&self, name: &str) -> Option<Arc<Role>> {
        self.inner.user.roles.read().await.by_name(name).cloned()
    }

    pub async fn roles_by_user_id(&self, user_id: &str) -> Option<Arc<[Arc<Role>]>> {
        let roles = self.inner.user.roles.read().await;
        let user_roles = self.inner.user.user_roles.read().await;
        user_roles.by_user_id(user_id).map(|v| {
            v.iter()
                .filter_map(|role_id| roles.get(role_id).cloned())
                .collect()
        })
    }

    pub async fn roles_by_group_id(&self, group_id: &str) -> Option<Arc<[Arc<Role>]>> {
        let roles = self.inner.user.roles.read().await;
        let group_roles = self.inner.user.group_roles.read().await;
        group_roles.by_group_id(group_id).map(|v| {
            v.iter()
                .filter_map(|role_id| roles.get(role_id).cloned())
                .collect()
        })
    }

    pub async fn groups_by_user_id(&self, user_id: &str) -> Option<Arc<[UserGroup]>> {
        let group_attributes = self.inner.user.group_attributes.read().await;
        let user_groups = self.inner.user.user_groups.read().await;
        user_groups.by_user_id(user_id).map(|v| {
            v.iter()
                .filter_map(|group_id| {
                    group_attributes.get(group_id).cloned().map(|v| UserGroup {
                        group_id: group_id.clone(),
                        group_detail: v,
                    })
                })
                .collect()
        })
    }
}

pub fn subscribe(keycloak_db: qm_pg::DB, customer_db: qm_pg::DB, listener_instance: CacheDB) {
    let keycloak_listener_instance = listener_instance.clone();
    std::thread::spawn(move || {
        let rt = Builder::new_current_thread().enable_all().build().unwrap();
        let local = LocalSet::new();
        local.spawn_local(async move {
            if let Err(err) = listener_instance.inner.infra.listen(&customer_db).await {
                if !err
                    .to_string()
                    .contains("A Tokio 1.x context was found, but it is being shutdown.")
                {
                    log::error!("{err:#?}");
                    std::process::exit(1)
                }
            }
        });
        rt.block_on(local);
    });
    std::thread::spawn(move || {
        let rt = Builder::new_current_thread().enable_all().build().unwrap();
        let local = LocalSet::new();
        local.spawn_local(async move {
            if let Err(err) = keycloak_listener_instance
                .inner
                .user
                .listen(&keycloak_db)
                .await
            {
                if !err
                    .to_string()
                    .contains("A Tokio 1.x context was found, but it is being shutdown.")
                {
                    log::error!("{err:#?}");
                    std::process::exit(1)
                }
            }
        });
        rt.block_on(local);
    });
}
