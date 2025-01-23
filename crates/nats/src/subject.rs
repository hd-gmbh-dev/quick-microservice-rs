use sea_orm::EntityName;
use strum::AsRefStr;
use async_graphql::Enum;

#[derive(Default, AsRefStr, Clone, Copy, PartialEq, Eq, Enum)]
#[strum(serialize_all = "lowercase")]
pub enum Op {
    #[default]
    Req,
    Res,
    Mut,
}

#[derive(Default, AsRefStr, Clone, Copy, PartialEq, Eq, Enum)]
#[strum(serialize_all = "lowercase")]
pub enum Type {
    #[default]
    Unknown,
    Create,
    Renew,
    Update,
    Delete,
    Download,
    Upload,
    Import,
    Export,
    Send,
    Recv,
    Assign,
    Unassign,
}

pub const NONE: &'static str = "_";

#[derive(Default)]
struct Event<P, C> {
    op: Op,
    ty: Type,
    parent_ctx: P,
    ctx_type: C,
    ctx: Option<String>,
    request_id: Option<String>,
    actor: Option<String>,
    error: bool,
}

fn format<P: AsRef<str>, C: AsRef<str>>(ev: &Event<P, C>, resource_name: impl std::fmt::Display) -> String {
    let append = match ev.op {
        Op::Mut => "",
        Op::Req => "",
        Op::Res => {
            if ev.error {
                ".error"
            } else {
                ".success"
            }
        }
    };
    format!(
        "ev.{}.{}.{}.{}.{}.{}.{}.{}{append}",
        ev.op.as_ref(),
        ev.parent_ctx.as_ref(),
        ev.ctx_type.as_ref(),
        ev.ctx.as_deref().unwrap_or(NONE),
        resource_name,
        ev.ty.as_ref(),
        ev.request_id.as_deref().unwrap_or(NONE),
        ev.actor.as_deref().unwrap_or(NONE),
    )
}

pub trait ResourceName {
    fn name(&self) -> &str;
}

pub trait StaticResourceName {
    fn name() -> &'static str;
}

pub struct Subject<P, C, E>(Event<P, C>, E);
impl<P, C, E> Subject<P, C, E>
where
    P: Default + AsRef<str> + Copy,
    C: Default + AsRef<str> + Copy,
    E: Default,
{
    pub fn op(&self) -> Op {
        self.0.op
    }

    pub fn ty(&self) -> Type {
        self.0.ty
    }

    pub fn parent_ctx(&self) -> P {
        self.0.parent_ctx
    }

    pub fn ctx_type(&self) -> C {
        self.0.ctx_type
    }

    pub fn ctx(&self) -> Option<&str> {
        self.0.ctx.as_deref()
    }

    pub fn request_id(&self) -> Option<&str> {
        self.0.request_id.as_deref()
    }

    pub fn actor(&self) -> Option<&str> {
        self.0.actor.as_deref()
    }

    pub fn error(&self) -> bool {
        self.0.error
    }

    fn factory(ty: Type) -> Self {
        let mut ev = Event::default();
        ev.ty = ty;
        Self(ev, E::default())
    }

    pub fn create() -> Self {
        Self::factory(Type::Create)
    }

    pub fn renew() -> Self {
        Self::factory(Type::Renew)
    }

    pub fn update() -> Self {
        Self::factory(Type::Update)
    }

    pub fn delete() -> Self {
        Self::factory(Type::Delete)
    }

    pub fn download() -> Self {
        Self::factory(Type::Download)
    }

    pub fn upload() -> Self {
        Self::factory(Type::Upload)
    }

    pub fn import() -> Self {
        Self::factory(Type::Import)
    }

    pub fn export() -> Self {
        Self::factory(Type::Export)
    }

    pub fn send() -> Self {
        Self::factory(Type::Send)
    }

    pub fn recv() -> Self {
        Self::factory(Type::Recv)
    }

    pub fn assign() -> Self {
        Self::factory(Type::Assign)
    }

    pub fn unassign() -> Self {
        Self::factory(Type::Unassign)
    }

    pub fn into_mut(mut self) -> Self {
        self.0.op = Op::Mut;
        self
    }

    pub fn into_response(mut self) -> Self {
        self.0.op = Op::Res;
        self
    }

    pub fn into_success(mut self) -> Self {
        self = self.into_response();
        self.0.error = false;
        self
    }

    pub fn into_error(mut self) -> Self {
        self = self.into_response();
        self.0.error = true;
        self
    }

    pub fn with_resource(mut self, resource: E) -> Self {
        self.1 = resource;
        self
    }
    
    pub fn with_type(mut self, ty: Type) -> Self {
        self.0.ty = ty;
        self
    }

    pub fn with_parent_ctx<T: Into<P>>(mut self, ctx: T) -> Self {
        self.0.parent_ctx = ctx.into();
        self
    }

    pub fn with_ctx_type(mut self, ctx_type: C) -> Self {
        self.0.ctx_type = ctx_type;
        self
    }

    pub fn with_ctx<S: Into<String>>(mut self, ctx: S) -> Self {
        self.0.ctx = Some(ctx.into());
        self
    }

    pub fn with_request_id<S: Into<String>>(mut self, request_id: S) -> Self {
        self.0.request_id = Some(request_id.into());
        self
    }

    pub fn with_actor<S: Into<String>>(mut self, actor: S) -> Self {
        self.0.actor = Some(actor.into());
        self
    }
}

impl<P, C, E> Subject<P, C, E>
where
    P: AsRef<str>,
    C: AsRef<str>,
    E: EntityName,
{
    pub fn entity(&self) -> async_nats::Subject {
        format(&self.0, self.1.table_name()).into()
    }
}

impl<P, C, E> Subject<P, C, E>
where
    P: AsRef<str>,
    C: AsRef<str>,
    E: ResourceName,
{
    pub fn resource(&self) -> async_nats::Subject {
        format(&self.0, self.1.name()).into()
    }
}

impl<P, C, E> Subject<P, C, std::marker::PhantomData<E>>
where
    P: AsRef<str>,
    C: AsRef<str>,
    E: StaticResourceName,
{
    pub fn static_resource(&self) -> async_nats::Subject {
        format(&self.0, E::name()).into()
    }
}

pub struct Entity;
pub struct Resource;
pub struct StaticResource;

use crate::EventToSubject;

impl<P, C, E> EventToSubject<Entity> for Subject<P, C, E>
where
    P: AsRef<str>,
    C: AsRef<str>,
    E: EntityName,
{
    fn event_to_subject(&self) -> async_nats::Subject {
        self.entity()
    }
}

impl<P, C, E> EventToSubject<Resource> for Subject<P, C, E>
where
    P: AsRef<str>,
    C: AsRef<str>,
    E: ResourceName,
{
    fn event_to_subject(&self) -> async_nats::Subject {
        self.resource()
    }
}

impl<P, C, E> EventToSubject<Resource> for Subject<P, C, std::marker::PhantomData<E>>
where
    P: AsRef<str>,
    C: AsRef<str>,
    E: StaticResourceName,
{
    fn event_to_subject(&self) -> async_nats::Subject {
        self.static_resource()
    }
}


#[cfg(test)]
mod tests {
    use async_graphql::Enum;
    use strum::AsRefStr;

    use super::{ResourceName, Subject};

    #[derive(Default, AsRefStr, Clone, Copy, PartialEq, Eq, Enum)]
    #[strum(serialize_all = "UPPERCASE")]
    pub enum ParentCtx {
        #[default]
        DE,
        BY,
    }

    #[derive(Default, AsRefStr, Clone, Copy, PartialEq, Eq, Enum)]
    #[strum(serialize_all = "snake_case")]
    pub enum CtxType {
        #[default]
        System,
        Test,
    }

    #[derive(Default, AsRefStr, Clone, Copy, PartialEq, Eq, Enum)]
    #[strum(serialize_all = "snake_case")]
    pub enum Resource {
        #[default]
        Unknown,
        User,
    }

    impl ResourceName for Resource {
        fn name(&self) -> &str {
            self.as_ref()
        }
    }

    #[derive(Default)]
    struct TestEntity;
    impl ResourceName for TestEntity {
        fn name(&self) -> &str {
            "qr"
        }
    }

    #[test]
    fn test_subject_empty_creation() {
        let mut subject = Subject::<ParentCtx, CtxType, Resource>::create()
            .with_resource(Resource::User);
        assert_eq!(
            subject.resource(),
            async_nats::Subject::from_static("ev.req.DE.system._.user.create._._"),
        );
        subject = subject.into_success();
        assert_eq!(
            subject.resource(),
            async_nats::Subject::from_static("ev.res.DE.system._.user.create._._.success"),
        );

        subject = subject.into_error();
        assert_eq!(
            subject.resource(),
            async_nats::Subject::from_static("ev.res.DE.system._.user.create._._.error"),
        );
    }

    #[test]
    fn test_subject_creation() {
        let mut subject = Subject::<ParentCtx, CtxType, TestEntity>::send()
            .with_parent_ctx(ParentCtx::BY)
            .with_ctx_type(CtxType::Test)
            .with_ctx("R3425921760D")
            .with_request_id("165789548978")
            .with_actor("EA4DCDCA-1CFD-48B9-905A-60DAB47964CB");
        assert_eq!(
            subject.resource(),
            async_nats::Subject::from_static("ev.req.BY.test.R3425921760D.qr.send.165789548978.EA4DCDCA-1CFD-48B9-905A-60DAB47964CB"),
        );
        subject = subject.into_success();
        assert_eq!(
            subject.resource(),
            async_nats::Subject::from_static("ev.res.BY.test.R3425921760D.qr.send.165789548978.EA4DCDCA-1CFD-48B9-905A-60DAB47964CB.success"),
        );

        subject = subject.into_error();
        assert_eq!(
            subject.resource(),
            async_nats::Subject::from_static("ev.res.BY.test.R3425921760D.qr.send.165789548978.EA4DCDCA-1CFD-48B9-905A-60DAB47964CB.error"),
        );
    }
}
