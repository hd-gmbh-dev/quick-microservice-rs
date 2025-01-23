use sea_orm::EntityName;
use strum::AsRefStr;
use async_graphql::Enum;

#[derive(Default, AsRefStr, Clone, Copy, PartialEq, Eq, Enum)]
#[strum(serialize_all = "lowercase")]
pub enum Op {
    #[default]
    Req,
    Res,
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

#[derive(Default, AsRefStr, Clone, Copy, PartialEq, Eq, Enum)]
#[strum(serialize_all = "UPPERCASE")]
pub enum ParentCtx {
    #[default]
    DE,
    BW,
    BY,
    BE,
    BB,
    HB,
    HH,
    HE,
    MV,
    NI,
    NW,
    RP,
    SL,
    SN,
    ST,
    SH,
    TH,
}

macro_rules! impl_from_for_parent_ctx {
    ($ty:ty) => {        
        impl From<$ty> for ParentCtx {
            fn from(value: $ty) -> Self {
                match value {
                    0x08 => ParentCtx::BW,
                    0x09 => ParentCtx::BY,
                    0x0B => ParentCtx::BE,
                    0x0C => ParentCtx::BB,
                    0x04 => ParentCtx::HB,
                    0x02 => ParentCtx::HH,
                    0x06 => ParentCtx::HE,
                    0x0D => ParentCtx::MV,
                    0x03 => ParentCtx::NI,
                    0x05 => ParentCtx::NW,
                    0x07 => ParentCtx::RP,
                    0x0A => ParentCtx::SL,
                    0x0E => ParentCtx::SN,
                    0x0F => ParentCtx::ST,
                    0x01 => ParentCtx::SH,
                    0x10 => ParentCtx::TH,
                    _ => ParentCtx::DE,
                }
            }
        }

        impl From<Option<$ty>> for ParentCtx {
            fn from(value: Option<$ty>) -> Self {
                let value = value.unwrap_or(0);
                ParentCtx::from(value)
            }
        }
    };
}

impl_from_for_parent_ctx!(i8);
impl_from_for_parent_ctx!(i16);
impl_from_for_parent_ctx!(i32);
impl_from_for_parent_ctx!(i64);

impl_from_for_parent_ctx!(u8);
impl_from_for_parent_ctx!(u16);
impl_from_for_parent_ctx!(u32);
impl_from_for_parent_ctx!(u64);

#[derive(Default, AsRefStr, Clone, Copy, PartialEq, Eq, Enum)]
#[strum(serialize_all = "UPPERCASE")]
pub enum CtxType {
    #[default]
    System,
    Olb,
    OlbBranch,
    Hdp,
    Wsp,
    WspBranch,
    Lab,
    LabBranch,
}

pub const NONE: &'static str = "none";

#[derive(Default)]
struct Event {
    op: Op,
    ty: Type,
    parent_ctx: ParentCtx,
    ctx_type: CtxType,
    ctx: Option<String>,
    request_id: Option<String>,
    actor: Option<String>,
    error: bool,
}

fn format(ev: &Event, resource_name: impl std::fmt::Display) -> String {
    let append = match ev.op {
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

pub struct Subject<E>(Event, E);
impl<E> Subject<E>
where
    E: Default,
{
    pub fn op(&self) -> Op {
        self.0.op
    }

    pub fn ty(&self) -> Type {
        self.0.ty
    }

    pub fn parent_ctx(&self) -> ParentCtx {
        self.0.parent_ctx
    }

    pub fn ctx_type(&self) -> CtxType {
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

    pub fn with_parent_ctx<T: Into<ParentCtx>>(mut self, ctx: T) -> Self {
        self.0.parent_ctx = ctx.into();
        self
    }

    pub fn with_ctx_type(mut self, ctx_type: CtxType) -> Self {
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

impl<E> Subject<E>
where
    E: EntityName,
{
    pub fn entity(&self) -> async_nats::Subject {
        format(&self.0, self.1.table_name()).into()
    }
}

impl<E> Subject<E>
where
    E: ResourceName,
{
    pub fn resource(&self) -> async_nats::Subject {
        format(&self.0, self.1.name()).into()
    }
}

impl<E> Subject<std::marker::PhantomData<E>>
where
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

impl<E> EventToSubject<Entity> for Subject<E>
where
    E: EntityName,
{
    fn event_to_subject(&self) -> async_nats::Subject {
        self.entity()
    }
}

impl<E> EventToSubject<Resource> for Subject<E>
where
    E: ResourceName,
{
    fn event_to_subject(&self) -> async_nats::Subject {
        self.resource()
    }
}

impl<E> EventToSubject<Resource> for Subject<std::marker::PhantomData<E>>
where
    E: StaticResourceName,
{
    fn event_to_subject(&self) -> async_nats::Subject {
        self.static_resource()
    }
}


#[cfg(test)]
mod tests {
    use super::ParentCtx;
    use super::{ResourceName, Subject};

    #[derive(Default)]
    struct TestEntity;
    impl ResourceName for TestEntity {
        fn name(&self) -> &str {
            "qr"
        }
    }

    #[test]
    fn test_subject_creation() {
        let mut subject = Subject::<TestEntity>::send()
            .with_parent_ctx(ParentCtx::BY)
            .with_ctx_type(super::CtxType::Lab)
            .with_ctx("R3425921760D")
            .with_request_id("165789548978")
            .with_actor("EA4DCDCA-1CFD-48B9-905A-60DAB47964CB");
        assert_eq!(
            subject.resource(),
            async_nats::Subject::from_static("ev.req.BY.LAB.R3425921760D.qr.send.165789548978.EA4DCDCA-1CFD-48B9-905A-60DAB47964CB"),
        );
        subject = subject.into_success();
        assert_eq!(
            subject.resource(),
            async_nats::Subject::from_static("ev.res.BY.LAB.R3425921760D.qr.send.165789548978.EA4DCDCA-1CFD-48B9-905A-60DAB47964CB.success"),
        );

        subject = subject.into_error();
        assert_eq!(
            subject.resource(),
            async_nats::Subject::from_static("ev.res.BY.LAB.R3425921760D.qr.send.165789548978.EA4DCDCA-1CFD-48B9-905A-60DAB47964CB.error"),
        );
    }
}
