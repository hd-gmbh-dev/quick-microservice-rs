use std::{borrow::Cow, str::FromStr, sync::Arc};

use async_graphql::{Description, InputValueError, InputValueResult, Scalar, ScalarType, Value};
use chrono::{DateTime, Utc};
use futures::TryStreamExt as _;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use qm_mongodb::{
    bson::{doc, oid::ObjectId, serde_helpers::chrono_datetime_as_bson_datetime, Document, Uuid},
    Collection, Database,
};

use crate::{
    error::EntityError,
    ids::{InstitutionId, InstitutionResourceId, OrganizationId, OwnerId},
};

const EMPTY_ID: &str = "000000000000000000000000";

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

impl ToMongoFilterMany for InstitutionId {
    fn to_mongo_filter_many(&self) -> Option<Document> {
        let (cid, oid, iid) = self.unzip();
        Some(doc! { "owner.cid": cid, "owner.oid": oid, "owner.iid": iid })
    }
}

impl ToMongoFilterMany for OrganizationId {
    fn to_mongo_filter_many(&self) -> Option<Document> {
        let (cid, oid) = self.unzip();
        Some(doc! { "owner.cid": cid, "owner.oid": oid })
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

impl ToMongoFilterOne for InstitutionResourceId {
    fn to_mongo_filter_one(&self) -> Document {
        let (cid, oid, iid, id) = self.unzip();
        doc! { "owner.cid": cid, "owner.oid": oid, "owner.iid": iid, "_id": id }
    }
}

impl ToMongoFilterOne for OrganizationId {
    fn to_mongo_filter_one(&self) -> Document {
        let (cid, oid) = self.unzip();
        doc! { "owner.cid": cid, "_id": oid }
    }
}

impl ToMongoFilterOne for InstitutionId {
    fn to_mongo_filter_one(&self) -> Document {
        let (cid, oid, iid) = self.unzip();
        doc! { "owner.cid": cid, "owner.oid": oid, "_id": iid }
    }
}

pub trait AsMongoId {
    fn as_mongo_id(&self) -> ObjectId;
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Entity<T> {
    id: ID,
    #[serde(flatten)]
    fields: T,
    #[serde(flatten)]
    defaults: Defaults,
}

pub trait UpdateEntity<T: Clone> {
    fn update_entity(self, entity: &T) -> Result<Cow<T>, EntityError>;
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
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Defaults {
    created: UserModification,
    modified: UserModification,
}

impl Defaults {
    fn now(user_id: Uuid) -> Self {
        let modify = UserModification::now(user_id);
        Self {
            created: modify.clone(),
            modified: modify,
        }
    }

    fn update_by(&self, user_id: Uuid) -> Self {
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
    fn now(user_id: Uuid) -> Self {
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
