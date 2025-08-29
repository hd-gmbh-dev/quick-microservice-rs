use std::{
    borrow::Cow,
    future::{Future, IntoFuture},
    marker::PhantomData,
    pin::Pin,
    str::FromStr,
    sync::Arc,
};

use async_graphql::{Description, InputValueError, InputValueResult, Scalar, ScalarType, Value};
use chrono::{DateTime, Utc};
use futures::{StreamExt as _, TryStreamExt as _};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use qm_mongodb::{
    bson::{
        doc, oid::ObjectId, serde_helpers::chrono_datetime_as_bson_datetime, to_bson, Bson,
        Document, Uuid,
    },
    options::FindOptions,
    Collection, Database,
};

use crate::{
    error::EntityError,
    ids::{
        CustomerId, CustomerOrOrganization, CustomerResourceId, InstitutionId,
        InstitutionResourceId, OrganizationId, OrganizationOrInstitution, OrganizationResourceId,
        OwnerId,
    },
    model::ListFilter,
};

const EMPTY_ID: &str = "000000000000000000000000";
const DEFAULT_PAGE_LIMIT: i64 = 100;

/// External representation of Object ID type in MongoDB.
#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize, Description)]
pub struct Id(ObjectId);

impl FromStr for Id {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_object_id(s)?.ok_or(())
    }
}

fn parse_object_id(id: &str) -> Result<Option<Id>, ()> {
    if id == EMPTY_ID {
        Ok(None)
    } else {
        Ok(Some(ObjectId::from_str(id).map(Id).map_err(|_| ())?))
    }
}

#[Scalar]
impl ScalarType for Id {
    fn parse(value: Value) -> InputValueResult<Self> {
        if let Value::String(value) = &value {
            Ok(Self::from_str(value).map_err(|_| InputValueError::custom("parse error"))?)
        } else {
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.0.to_hex())
    }
}

type ID = Id;

/// Entity Id.
#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize, Description)]
#[graphql(name = "EntityId")]
pub struct GraphQLId {
    #[graphql(flatten)]
    id: ID,
}

impl FromStr for GraphQLId {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_object_id(s)?.map(|id| Self { id }).ok_or(())
    }
}

impl AsRef<ObjectId> for GraphQLId {
    fn as_ref(&self) -> &ObjectId {
        &self.id.0
    }
}

#[Scalar]
impl ScalarType for GraphQLId {
    fn parse(value: Value) -> InputValueResult<Self> {
        if let Value::String(value) = &value {
            Ok(Self::from_str(value).map_err(|_| InputValueError::custom("parse error"))?)
        } else {
            Err(InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.id.0.to_hex())
    }
}

impl From<Id> for GraphQLId {
    fn from(id: Id) -> Self {
        Self { id }
    }
}

pub trait ToMongoFilterMany {
    fn to_mongo_filter_many(&self) -> Option<Document>;
}

impl ToMongoFilterMany for () {
    fn to_mongo_filter_many(&self) -> Option<Document> {
        None
    }
}

impl ToMongoFilterMany for Option<Document> {
    fn to_mongo_filter_many(&self) -> Option<Document> {
        self.clone()
    }
}

impl<T> ToMongoFilterMany for Option<T>
where
    T: ToMongoFilterMany,
{
    fn to_mongo_filter_many(&self) -> Option<Document> {
        self.as_ref().and_then(|v| v.to_mongo_filter_many())
    }
}

impl ToMongoFilterMany for CustomerId {
    fn to_mongo_filter_many(&self) -> Option<Document> {
        let cid = self.unzip();
        Some(doc! { "owner.cid": cid })
    }
}

impl ToMongoFilterMany for OrganizationId {
    fn to_mongo_filter_many(&self) -> Option<Document> {
        let (cid, oid) = self.unzip();
        Some(doc! { "owner.cid": cid, "owner.oid": oid })
    }
}

impl ToMongoFilterMany for InstitutionId {
    fn to_mongo_filter_many(&self) -> Option<Document> {
        let (cid, oid, iid) = self.unzip();
        Some(doc! { "owner.cid": cid, "owner.oid": oid, "owner.iid": iid })
    }
}

impl ToMongoFilterMany for CustomerOrOrganization {
    fn to_mongo_filter_many(&self) -> Option<Document> {
        match self {
            Self::Customer(v) => v.to_mongo_filter_many(),
            Self::Organization(v) => v.to_mongo_filter_many(),
        }
    }
}

impl ToMongoFilterMany for OrganizationOrInstitution {
    fn to_mongo_filter_many(&self) -> Option<Document> {
        match self {
            Self::Institution(v) => v.to_mongo_filter_many(),
            Self::Organization(v) => v.to_mongo_filter_many(),
        }
    }
}

pub trait ToMongoFilterOne {
    fn to_mongo_filter_one(&self) -> Document;
}

impl ToMongoFilterOne for Document {
    fn to_mongo_filter_one(&self) -> Document {
        self.clone()
    }
}

impl ToMongoFilterOne for CustomerResourceId {
    fn to_mongo_filter_one(&self) -> Document {
        let (.., id) = self.unzip();
        doc! { "_id": id }
    }
}

impl ToMongoFilterOne for OrganizationResourceId {
    fn to_mongo_filter_one(&self) -> Document {
        doc! { "_id": self.id() }
    }
}

impl ToMongoFilterOne for InstitutionResourceId {
    fn to_mongo_filter_one(&self) -> Document {
        let (.., id) = self.unzip();
        doc! { "_id": id }
    }
}

impl ToMongoFilterOne for CustomerId {
    fn to_mongo_filter_one(&self) -> Document {
        doc! { "_id": self.unzip() }
    }
}

impl ToMongoFilterOne for OrganizationId {
    fn to_mongo_filter_one(&self) -> Document {
        doc! { "_id": self.id() }
    }
}

impl ToMongoFilterOne for InstitutionId {
    fn to_mongo_filter_one(&self) -> Document {
        doc! { "_id": self.id() }
    }
}

impl ToMongoFilterOne for CustomerOrOrganization {
    fn to_mongo_filter_one(&self) -> Document {
        match self {
            Self::Customer(v) => v.to_mongo_filter_one(),
            Self::Organization(v) => v.to_mongo_filter_one(),
        }
    }
}

impl ToMongoFilterOne for OrganizationOrInstitution {
    fn to_mongo_filter_one(&self) -> Document {
        match self {
            Self::Institution(v) => v.to_mongo_filter_one(),
            Self::Organization(v) => v.to_mongo_filter_one(),
        }
    }
}

pub trait ToMongoFilterExact {
    fn to_mongo_filter_exact(&self) -> Result<Document, EntityError>;
}

pub struct ResourcesFilter<'a, I>(pub &'a [I])
where
    I: ToMongoFilterOne;
impl<I> ToMongoFilterExact for ResourcesFilter<'_, I>
where
    I: ToMongoFilterOne,
{
    fn to_mongo_filter_exact(&self) -> Result<Document, EntityError> {
        if self.0.is_empty() {
            return Err(EntityError::NotEmpty);
        }
        if self.0.len() == 1 {
            return Ok(self.0.first().unwrap().to_mongo_filter_one());
        }
        let items: Vec<Document> = self.0.iter().map(|v| v.to_mongo_filter_one()).collect();
        Ok(doc! {
            "$or": items,
        })
    }
}

pub trait AsMongoId {
    fn as_mongo_id(&self) -> ObjectId;
}

pub trait FromMongoId: Sized {
    fn from_mongo_id(old_id: Self, bson: Bson) -> Option<Self>;
}

pub trait IsMongoInsert {
    fn is_mongo_insert(&self) -> bool;
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Entity<T> {
    id: ID,
    #[serde(flatten)]
    fields: T,
    #[serde(flatten)]
    defaults: Defaults,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Page<I> {
    pub items: Vec<I>,
    pub skip: u64,
    pub limit: Option<i64>,
    pub total: usize,
}

impl<I> Page<I> {
    /// Empty page.
    pub fn empty() -> Self {
        Self {
            items: vec![],
            total: 0,
            skip: 0,
            limit: Some(DEFAULT_PAGE_LIMIT),
        }
    }

    /// Returns page index.
    pub fn index(&self) -> u64 {
        if let Some(limit) = self.limit.filter(|l| *l > 0).map(|l| l as u64) {
            self.skip / limit
        } else {
            0
        }
    }

    /// Returns page count.
    pub fn count(&self) -> usize {
        if let Some(limit) = self.limit.filter(|l| *l > 0).map(|l| l as usize) {
            self.total.div_ceil(limit)
        } else {
            0
        }
    }
}

#[derive(Default)]
pub struct PageInfo {
    skip: Option<u64>,
    limit: Option<i64>,
}

impl TryFrom<ListFilter> for PageInfo {
    type Error = EntityError;

    fn try_from(value: ListFilter) -> Result<Self, Self::Error> {
        let limit = value.limit.map(|l| l as i64).unwrap_or(DEFAULT_PAGE_LIMIT);
        Ok(Self {
            skip: value.page.map(|page| limit as u64 * page as u64),
            limit: Some(limit),
        })
    }
}

impl TryFrom<Option<ListFilter>> for PageInfo {
    type Error = EntityError;

    fn try_from(value: Option<ListFilter>) -> Result<Self, Self::Error> {
        value
            .map(|v| v.try_into())
            .unwrap_or_else(|| Ok(Default::default()))
    }
}

pub trait UpdateEntity<T: Clone> {
    fn update_entity(self, entity: &T) -> Result<Cow<'_, T>, EntityError>;
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EntityOwned<T, ID = Id> {
    #[serde(rename = "_id")]
    pub id: ID,
    pub owner: Arc<OwnerId>,
    #[serde(flatten)]
    pub fields: T,
    #[serde(flatten)]
    pub defaults: Arc<Defaults>,
}

impl<T> EntityOwned<T>
where
    T: DeserializeOwned + Serialize + MongoCollection + Send + Sync + Unpin,
{
    pub async fn create(
        db: &Database,
        owner: impl Into<OwnerId>,
        fields: T,
        user_id: Uuid,
    ) -> Result<Self, EntityError> {
        #[derive(Serialize)]
        struct CreateOwnedEntity<'f, F> {
            owner: Arc<OwnerId>,
            #[serde(flatten)]
            fields: &'f F,
            #[serde(flatten)]
            defaults: Arc<Defaults>,
        }

        let owner = Arc::new(owner.into());
        let defaults = Arc::new(Defaults::now(user_id));

        T::mongo_collection(db)
            .insert_one(CreateOwnedEntity {
                owner: owner.clone(),
                fields: &fields,
                defaults: defaults.clone(),
            })
            .await?
            .inserted_id
            .as_object_id()
            .map(Id)
            .ok_or(EntityError::NoId)
            .map(|id| Self {
                id,
                owner,
                fields,
                defaults,
            })
    }
}

impl<T, ID> EntityOwned<T, ID>
where
    T: DeserializeOwned + Serialize + MongoCollection + Send + Sync + Unpin,
    ID: DeserializeOwned + Serialize + Send + Sync + Unpin,
{
    /// Query owned entities
    pub fn query(db: &Database) -> Query<'_, T, ID> {
        Query::new(db)
    }

    pub async fn list(
        db: &Database,
        filter: impl ToMongoFilterMany,
    ) -> Result<Vec<Self>, EntityError> {
        T::mongo_collection(db)
            .find(filter.to_mongo_filter_many().unwrap_or_default())
            .await?
            .try_collect()
            .await
            .map_err(From::from)
    }

    pub async fn page(
        db: &Database,
        filter: impl ToMongoFilterMany,
        page_selector: impl TryInto<PageInfo, Error = EntityError>,
    ) -> Result<Page<Self>, EntityError> {
        Self::page_filter(
            db,
            filter.to_mongo_filter_many().unwrap_or_default(),
            page_selector,
        )
        .await
    }

    pub async fn list_exact(
        db: &Database,
        filter: impl ToMongoFilterExact,
    ) -> Result<Vec<Self>, EntityError> {
        T::mongo_collection(db)
            .find(filter.to_mongo_filter_exact()?)
            .await?
            .try_collect()
            .await
            .map_err(From::from)
    }

    pub async fn page_exact(
        db: &Database,
        filter: impl ToMongoFilterExact,
        page_selector: impl TryInto<PageInfo, Error = EntityError>,
    ) -> Result<Page<Self>, EntityError> {
        Self::page_filter(db, filter.to_mongo_filter_exact()?, page_selector).await
    }

    pub async fn by_id(
        db: &Database,
        id: impl ToMongoFilterOne,
    ) -> Result<Option<Self>, EntityError> {
        T::mongo_collection(db)
            .find_one(id.to_mongo_filter_one())
            .await
            .map_err(From::from)
    }

    pub async fn update(
        db: &Database,
        context: impl ToMongoFilterOne,
        input: impl UpdateEntity<T>,
        user_id: Uuid,
    ) -> Result<Self, EntityError>
    where
        T: Clone,
    {
        let filter = context.to_mongo_filter_one();
        let Some(mut entity): Option<Self> =
            T::mongo_collection(db).find_one(filter.clone()).await?
        else {
            return Err(EntityError::NotFound);
        };

        if let Cow::Owned(updated) = input.update_entity(&entity.fields)? {
            entity.fields = updated;
            entity.defaults = Arc::new(entity.defaults.update_by(user_id));

            if let Some(filter) = filter.into() {
                T::mongo_collection::<Self>(db)
                    .replace_one(filter, &entity)
                    .await?;
            }
        }

        Ok(entity)
    }

    pub async fn save<C>(
        db: &Database,
        context: C,
        input: impl Into<T>,
        user_id: Uuid,
    ) -> Result<bool, EntityError>
    where
        T: Clone + std::fmt::Debug,
        C: ToMongoFilterOne + Into<OwnerId>,
    {
        let filter = context.to_mongo_filter_one();
        #[derive(Debug, Serialize)]
        struct SaveEntity<F> {
            owner: OwnerId,
            #[serde(flatten)]
            fields: F,
            #[serde(flatten)]
            defaults: Arc<Defaults>,
        }
        let defaults = Arc::new(Defaults::now(user_id));
        let entity = SaveEntity {
            owner: context.into(),
            fields: input.into(),
            defaults,
        };
        let result = T::mongo_collection::<SaveEntity<_>>(db)
            .replace_one(filter, &entity)
            .upsert(true)
            .await?;
        Ok(result.modified_count > 0 || result.upserted_id.is_some())
    }

    pub async fn save_with_id<C, I>(
        db: &Database,
        context: C,
        input: I,
        user_id: Uuid,
    ) -> Result<Option<C>, EntityError>
    where
        T: Clone + std::fmt::Debug,
        C: FromMongoId + IsMongoInsert + ToMongoFilterOne + Into<OwnerId> + Clone,
        I: Into<T> + Send + Sync,
    {
        let filter = context.to_mongo_filter_one();
        Ok(if context.is_mongo_insert() {
            #[derive(Debug, Serialize)]
            struct SaveEntity<F> {
                owner: OwnerId,
                #[serde(flatten)]
                fields: F,
                #[serde(flatten)]
                defaults: Defaults,
            }
            let defaults = Defaults::now(user_id);
            let entity = SaveEntity {
                owner: context.clone().into(),
                fields: input.into(),
                defaults,
            };
            let result = T::mongo_collection::<SaveEntity<T>>(db)
                .insert_one(&entity)
                .await?;
            C::from_mongo_id(context, result.inserted_id)
        } else {
            #[derive(Debug, Serialize)]
            struct SaveEntity<F> {
                owner: OwnerId,
                #[serde(flatten)]
                fields: F,
                modified: UserModification,
            }
            let entity = SaveEntity {
                owner: context.clone().into(),
                fields: input.into(),
                modified: UserModification::now(user_id),
            };
            let result = T::mongo_collection::<SaveEntity<T>>(db)
                .update_one(filter, doc!{ "$set": to_bson(&entity).map_err(|err| EntityError::Bson(err.to_string()))? })
                .await?;
            if result.matched_count == 0 {
                return Err(EntityError::NotFound);
            }
            if result.modified_count > 0 {
                Some(context)
            } else {
                None
            }
        })
    }

    pub async fn remove<I>(db: &Database, ids: I) -> Result<i32, EntityError>
    where
        I: ToMongoFilterExact,
    {
        let result = T::mongo_collection::<Document>(db)
            .delete_many(ids.to_mongo_filter_exact()?)
            .await?;
        Ok(result.deleted_count as i32)
    }

    pub async fn page_filter(
        db: &Database,
        filter: Document,
        page_selector: impl TryInto<PageInfo, Error = EntityError>,
    ) -> Result<Page<Self>, EntityError> {
        let page_info: PageInfo = page_selector.try_into()?;
        Self::page_filter_sort(db, filter, None, page_info).await
    }

    pub async fn page_filter_sort(
        db: &Database,
        filter: Document,
        sort: Option<Document>,
        page_info: PageInfo,
    ) -> Result<Page<Self>, EntityError> {
        let total = T::mongo_collection::<Self>(db)
            .find(filter.clone())
            .await?
            .count()
            .await;

        let limit = page_info.limit;

        if total == 0 {
            return Ok(if limit.is_some() {
                Page {
                    limit,
                    ..Page::empty()
                }
            } else {
                Page::empty()
            });
        }

        T::mongo_collection(db)
            .find(filter)
            .with_options(
                FindOptions::builder()
                    .limit(limit)
                    .sort(sort)
                    .skip(page_info.skip)
                    .build(),
            )
            .await?
            .try_collect()
            .await
            .map(|items| Page {
                items,
                total,
                skip: page_info.skip.unwrap_or_default(),
                limit,
            })
            .map_err(From::from)
    }
}

/// Represents a query for a collection of entities.
pub struct Query<'q, T, ID> {
    db: &'q Database,
    filter: Option<Document>,
    page: Option<PageInfo>,
    sort: Option<Document>,
    marker: PhantomData<(T, ID)>,
}

impl<'q, T, ID> Query<'q, T, ID> {
    fn new(db: &'q Database) -> Self {
        Self {
            db,
            filter: None,
            page: None,
            sort: None,
            marker: Default::default(),
        }
    }

    pub fn filter_exact(mut self, filter: impl ToMongoFilterExact) -> Result<Self, EntityError> {
        self.filter = Some(filter.to_mongo_filter_exact()?);
        Ok(self)
    }

    pub fn filter_many(mut self, filter: impl ToMongoFilterMany) -> Self {
        self.filter = filter.to_mongo_filter_many();
        self
    }

    pub fn filter(mut self, filter: Document) -> Self {
        self.filter = Some(filter);
        self
    }

    pub fn page_selector(
        mut self,
        page_selector: impl TryInto<PageInfo, Error = EntityError>,
    ) -> Result<Self, EntityError> {
        self.page = Some(page_selector.try_into()?);
        Ok(self)
    }

    pub fn page(mut self, page: PageInfo) -> Self {
        self.page = Some(page);
        self
    }

    pub fn sort(mut self, sort: impl Into<Option<Document>>) -> Self {
        self.sort = sort.into();
        self
    }
}

impl<'q, T, ID> IntoFuture for Query<'q, T, ID>
where
    T: DeserializeOwned + Serialize + MongoCollection + Send + Sync + Unpin + 'q,
    ID: DeserializeOwned + Serialize + Send + Sync + Unpin + 'q,
{
    type Output = Result<Page<EntityOwned<T, ID>>, EntityError>;

    type IntoFuture = Pin<Box<dyn Future<Output = Self::Output> + Send + 'q>>;

    fn into_future(self) -> Self::IntoFuture {
        Box::pin(EntityOwned::<T, ID>::page_filter_sort(
            self.db,
            self.filter.unwrap_or_default(),
            self.sort,
            self.page.unwrap_or_default(),
        ))
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Defaults {
    pub created: UserModification,
    pub modified: UserModification,
}

impl Defaults {
    pub fn now(user_id: Uuid) -> Self {
        let modify = UserModification::now(user_id);
        Self {
            created: modify.clone(),
            modified: modify,
        }
    }

    pub fn update_by(&self, user_id: Uuid) -> Self {
        let modified = UserModification::now(user_id);
        Self {
            created: self.created.clone(),
            modified,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserModification {
    #[serde(rename = "uid")]
    pub user_id: Uuid,
    #[serde(with = "chrono_datetime_as_bson_datetime")]
    pub at: DateTime<Utc>,
}

impl UserModification {
    pub fn now(user_id: Uuid) -> Self {
        Self {
            user_id,
            at: Utc::now(),
        }
    }
}

pub trait EntityField {
    type Field<T: Serialize + DeserializeOwned>: Serialize + DeserializeOwned;
}

#[derive(Default, Clone, PartialEq, Debug)]
pub struct Optional;
impl EntityField for Optional {
    type Field<T: Serialize + DeserializeOwned> = Option<T>;
}

#[derive(Default, Clone, PartialEq, Debug)]
pub struct Required;
impl EntityField for Required {
    type Field<T: Serialize + DeserializeOwned> = T;
}

pub trait MongoCollection {
    const COLLECTION: &'static str;

    fn mongo_collection<T>(db: &Database) -> Collection<T>
    where
        T: Send + Sync,
    {
        db.collection(Self::COLLECTION)
    }
}
