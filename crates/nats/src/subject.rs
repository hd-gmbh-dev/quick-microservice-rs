use async_graphql::{Enum, OutputType, SimpleObject};
use strum::{AsRefStr, EnumString};

/// Operation type for events.
#[derive(Default, Debug, EnumString, AsRefStr, Clone, Copy, PartialEq, Eq, Enum)]
#[strum(serialize_all = "lowercase")]
pub enum Op {
    /// Request operation.
    #[default]
    Req,
    /// Response operation.
    Res,
    /// Mutation operation.
    Mut,
}

/// Event type.
#[derive(Default, Debug, EnumString, AsRefStr, Clone, Copy, PartialEq, Eq, Enum)]
#[strum(serialize_all = "lowercase")]
pub enum Type {
    /// Unknown type.
    #[default]
    Unknown,
    /// Create event.
    Create,
    /// Renew event.
    Renew,
    /// Update event.
    Update,
    /// Delete event.
    Delete,
    /// Download event.
    Download,
    /// Upload event.
    Upload,
    /// Import event.
    Import,
    /// Export event.
    Export,
    /// Send event.
    Send,
    /// Receive event.
    Recv,
    /// Assign event.
    Assign,
    /// Assign request event.
    AssignRequest,
    /// Reassign event.
    ReAssign,
    /// Assign update event.
    AssignUpdate,
    /// Unassign event.
    Unassign,
    /// Activate event.
    Activate,
    /// Deactivate event.
    Deactivate,
}

/// Constant for no value.
pub const NONE: &str = "_";

/// Generic event structure for NATS subjects.
#[derive(Default, Debug, Clone, SimpleObject)]
pub struct Event<V, P, C>
where
    V: Clone + OutputType,
    P: Clone + OutputType,
    C: Clone + OutputType,
{
    /// Event version.
    pub version: V,
    /// Operation type.
    pub op: Op,
    /// Event type.
    pub ty: Type,
    /// Parent context.
    pub parent_ctx: P,
    /// Context type.
    pub ctx_type: C,
    /// Context string.
    pub ctx: Option<String>,
    /// Request ID.
    pub request_id: Option<String>,
    /// Actor who triggered the event.
    pub actor: Option<String>,
    /// Whether this is an error event.
    pub error: bool,
}

/// Format an event as a NATS subject string.
fn format<V, P, C>(ev: &Event<V, P, C>, resource_name: impl std::fmt::Display) -> String
where
    V: AsRef<str> + Clone + OutputType,
    P: AsRef<str> + Clone + OutputType,
    C: AsRef<str> + Clone + OutputType,
{
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
        "ev.{}.{}.{}.{}.{}.{}.{}.{}.{}{append}",
        ev.version.as_ref(),
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

/// Trait for types that have a resource name.
pub trait ResourceName {
    /// Get the resource name.
    fn name(&self) -> &str;
}

/// Trait for types with a static resource name.
pub trait StaticResourceName {
    /// Get the static resource name.
    fn name() -> &'static str;
}

/// A Subject with an event and extra data.
#[derive(Debug, Clone)]
pub struct Subject<V, P, C, E>(pub Event<V, P, C>, pub E)
where
    V: Clone + OutputType,
    P: Clone + OutputType,
    C: Clone + OutputType;
impl<V, P, C, E> Subject<V, P, C, E>
where
    V: OutputType + Default + AsRef<str> + Copy,
    P: OutputType + Default + AsRef<str> + Copy,
    C: OutputType + Default + AsRef<str> + Copy,
    E: OutputType + Default,
{
    /// Get the version.
    pub fn version(&self) -> V {
        self.0.version
    }

    /// Get the operation.
    pub fn op(&self) -> Op {
        self.0.op
    }

    /// Get the type.
    pub fn ty(&self) -> Type {
        self.0.ty
    }

    /// Get the parent context.
    pub fn parent_ctx(&self) -> P {
        self.0.parent_ctx
    }

    /// Get the context type.
    pub fn ctx_type(&self) -> C {
        self.0.ctx_type
    }

    /// Get the context.
    pub fn ctx(&self) -> Option<&str> {
        self.0.ctx.as_deref()
    }

    /// Get the request ID.
    pub fn request_id(&self) -> Option<&str> {
        self.0.request_id.as_deref()
    }

    /// Get the actor.
    pub fn actor(&self) -> Option<&str> {
        self.0.actor.as_deref()
    }

    /// Check if this is an error.
    pub fn error(&self) -> bool {
        self.0.error
    }

    /// Create a factory for the given type.
    fn factory(ty: Type) -> Self {
        let ev = Event {
            ty,
            ..Default::default()
        };
        Self(ev, E::default())
    }

    /// Create a create subject.
    pub fn create() -> Self {
        Self::factory(Type::Create)
    }

    /// Create a renew subject.
    pub fn renew() -> Self {
        Self::factory(Type::Renew)
    }

    /// Create an update subject.
    pub fn update() -> Self {
        Self::factory(Type::Update)
    }

    /// Create a delete subject.
    pub fn delete() -> Self {
        Self::factory(Type::Delete)
    }

    /// Create a download subject.
    pub fn download() -> Self {
        Self::factory(Type::Download)
    }

    /// Create an upload subject.
    pub fn upload() -> Self {
        Self::factory(Type::Upload)
    }

    /// Create an import subject.
    pub fn import() -> Self {
        Self::factory(Type::Import)
    }

    /// Create an export subject.
    pub fn export() -> Self {
        Self::factory(Type::Export)
    }

    /// Create a send subject.
    pub fn send() -> Self {
        Self::factory(Type::Send)
    }

    /// Create a recv subject.
    pub fn recv() -> Self {
        Self::factory(Type::Recv)
    }

    /// Create an assign subject.
    pub fn assign() -> Self {
        Self::factory(Type::Assign)
    }

    /// Create an assign_request subject.
    pub fn assign_request() -> Self {
        Self::factory(Type::AssignRequest)
    }

    /// Create a re_assign subject.
    pub fn re_assign() -> Self {
        Self::factory(Type::ReAssign)
    }

    /// Create an assign_update subject.
    pub fn assign_update() -> Self {
        Self::factory(Type::AssignUpdate)
    }

    /// Create an unassign subject.
    pub fn unassign() -> Self {
        Self::factory(Type::Unassign)
    }

    /// Create an activate subject.
    pub fn activate() -> Self {
        Self::factory(Type::Activate)
    }

    /// Create a deactivate subject.
    pub fn deactivate() -> Self {
        Self::factory(Type::Deactivate)
    }

    /// Convert to a mutation operation.
    pub fn into_mut(mut self) -> Self {
        self.0.op = Op::Mut;
        self
    }

    /// Convert to a response operation.
    pub fn into_response(mut self) -> Self {
        self.0.op = Op::Res;
        self
    }

    /// Convert to a success response.
    pub fn into_success(mut self) -> Self {
        self = self.into_response();
        self.0.error = false;
        self
    }

    /// Convert to an error response.
    pub fn into_error(mut self) -> Self {
        self = self.into_response();
        self.0.error = true;
        self
    }

    /// Attach a resource to the subject.
    /// Attaches a resource to the subject.
    pub fn with_resource(mut self, resource: E) -> Self {
        self.1 = resource;
        self
    }

    /// Sets the event type.
    pub fn with_type(mut self, ty: Type) -> Self {
        self.0.ty = ty;
        self
    }

    /// Sets the parent context.
    pub fn with_parent_ctx<T: Into<P>>(mut self, ctx: T) -> Self {
        self.0.parent_ctx = ctx.into();
        self
    }

    /// Sets the context type.
    pub fn with_ctx_type(mut self, ctx_type: C) -> Self {
        self.0.ctx_type = ctx_type;
        self
    }

    /// Sets the context string.
    pub fn with_ctx<S: Into<String>>(mut self, ctx: S) -> Self {
        self.0.ctx = Some(ctx.into());
        self
    }

    /// Sets the request ID.
    pub fn with_request_id<S: Into<String>>(mut self, request_id: S) -> Self {
        self.0.request_id = Some(request_id.into());
        self
    }

    /// Sets the actor who triggered the event.
    pub fn with_actor<S: Into<String>>(mut self, actor: S) -> Self {
        self.0.actor = Some(actor.into());
        self
    }

    /// Sets the version.
    pub fn with_version(mut self, version: V) -> Self {
        self.0.version = version;
        self
    }
}

impl<V, P, C, E> Subject<V, P, C, E>
where
    V: Clone + OutputType + std::fmt::Debug + AsRef<str>,
    P: Clone + OutputType + std::fmt::Debug + AsRef<str>,
    C: Clone + OutputType + std::fmt::Debug + AsRef<str>,
    E: std::fmt::Debug + ResourceName,
{
    /// Returns the NATS subject for this event.
    pub fn resource(&self) -> async_nats::Subject {
        format(&self.0, self.1.name()).into()
    }
}

/// Marker type for Resource.
pub struct Resource;
use crate::EventToSubject;

impl<V, P, C, E> EventToSubject<Resource> for Subject<V, P, C, E>
where
    V: Clone + OutputType + std::fmt::Debug + AsRef<str>,
    P: Clone + OutputType + std::fmt::Debug + AsRef<str>,
    C: Clone + OutputType + std::fmt::Debug + AsRef<str>,
    E: std::fmt::Debug + ResourceName,
{
    fn event_to_subject(&self) -> async_nats::Subject {
        self.resource()
    }
}

impl<V, P, C, E> std::str::FromStr for Subject<V, P, C, E>
where
    V: Clone + OutputType + std::str::FromStr<Err = strum::ParseError>,
    P: Clone + OutputType + std::str::FromStr<Err = strum::ParseError>,
    C: Clone + OutputType + std::str::FromStr<Err = strum::ParseError>,
    E: std::str::FromStr<Err = strum::ParseError>,
{
    type Err = strum::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut s = s.split(".");
        let _ev_prefix = s.next();
        let version = s.next().unwrap_or_default().parse()?;
        let op = s.next().unwrap_or_default().parse()?;
        let parent_ctx = s.next().unwrap_or_default().parse()?;
        let ctx_type = s.next().unwrap_or_default().parse()?;
        let ctx = s.next().map(|s| s.to_string());
        let e: E = s.next().unwrap_or_default().parse()?;
        let ty = s.next().unwrap_or_default().parse()?;
        let request_id = s.next().map(|s| s.to_string());
        let actor = s.next().map(|s| s.to_string());
        let error = matches!(s.next(), Some("error"));
        Ok(Self(
            Event {
                version,
                op,
                ty,
                parent_ctx,
                ctx_type,
                ctx,
                request_id,
                actor,
                error,
            },
            e,
        ))
    }
}

#[cfg(test)]
mod tests {
    use async_graphql::Enum;
    use strum::AsRefStr;
    use strum::EnumString;

    use super::{ResourceName, Subject};

    #[derive(Default, Debug, EnumString, AsRefStr, Clone, Copy, PartialEq, Eq, Enum)]
    #[strum(serialize_all = "UPPERCASE")]
    pub enum Version {
        #[default]
        V1,
    }

    #[derive(Default, Debug, EnumString, AsRefStr, Clone, Copy, PartialEq, Eq, Enum)]
    #[strum(serialize_all = "UPPERCASE")]
    pub enum ParentCtx {
        #[default]
        DE,
        BY,
    }

    #[derive(Default, Debug, EnumString, AsRefStr, Clone, Copy, PartialEq, Eq, Enum)]
    #[strum(serialize_all = "snake_case")]
    pub enum CtxType {
        #[default]
        System,
        Test,
    }

    #[derive(Default, Debug, EnumString, AsRefStr, Clone, Copy, PartialEq, Eq, Enum)]
    #[strum(serialize_all = "snake_case")]
    pub enum Resource {
        #[default]
        Unknown,
        User,
        Qr,
    }

    impl ResourceName for Resource {
        fn name(&self) -> &str {
            self.as_ref()
        }
    }

    #[test]
    fn test_subject_empty_creation() {
        let mut subject = Subject::<Version, ParentCtx, CtxType, Resource>::create()
            .with_resource(Resource::User);
        assert_eq!(
            subject.resource(),
            async_nats::Subject::from_static("ev.V1.req.DE.system._.user.create._._"),
        );
        subject = subject.into_success();
        assert_eq!(
            subject.resource(),
            async_nats::Subject::from_static("ev.V1.res.DE.system._.user.create._._.success"),
        );

        subject = subject.into_error();
        assert_eq!(
            subject.resource(),
            async_nats::Subject::from_static("ev.V1.res.DE.system._.user.create._._.error"),
        );
    }

    #[test]
    fn test_subject_creation() {
        let mut subject = Subject::<Version, ParentCtx, CtxType, Resource>::send()
            .with_parent_ctx(ParentCtx::BY)
            .with_ctx_type(CtxType::Test)
            .with_ctx("R3425921760D")
            .with_request_id("165789548978")
            .with_actor("EA4DCDCA-1CFD-48B9-905A-60DAB47964CB")
            .with_resource(Resource::Qr);
        assert_eq!(
            subject.resource(),
            async_nats::Subject::from_static("ev.V1.req.BY.test.R3425921760D.qr.send.165789548978.EA4DCDCA-1CFD-48B9-905A-60DAB47964CB"),
        );
        subject = subject.into_success();
        assert_eq!(
            subject.resource(),
            async_nats::Subject::from_static("ev.V1.res.BY.test.R3425921760D.qr.send.165789548978.EA4DCDCA-1CFD-48B9-905A-60DAB47964CB.success"),
        );

        subject = subject.into_error();
        assert_eq!(
            subject.resource(),
            async_nats::Subject::from_static("ev.V1.res.BY.test.R3425921760D.qr.send.165789548978.EA4DCDCA-1CFD-48B9-905A-60DAB47964CB.error"),
        );
    }
}
