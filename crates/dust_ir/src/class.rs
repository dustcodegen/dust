use crate::{
    ConfigApplicationIr, ConstructorIr, FieldIr, MethodIr, SerdeClassConfigIr, SpanIr,
    TraitApplicationIr, TypeIr, TypeParamIr,
};

/// The lowered declaration kind of a Dart class-like target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ClassKindIr {
    /// A normal `class` declaration.
    Class,
    /// A `sealed class` declaration.
    SealedClass,
    /// A `mixin class` declaration.
    MixinClass,
}

/// One lowered class declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassIr {
    /// The declaration kind.
    pub kind: ClassKindIr,
    /// The class name.
    pub name: String,
    /// Class type parameters in declaration order.
    pub type_parameters: Vec<TypeParamIr>,
    /// Written mixin applications with arguments and source locations.
    pub mixins: Vec<MixinApplicationIr>,
    /// Whether the declaration is marked `abstract`.
    pub is_abstract: bool,
    /// Whether the declaration uses Dart's `interface class` form.
    pub is_interface: bool,
    /// The immediate superclass name, if one was declared.
    pub superclass_name: Option<String>,
    /// Written superclass application in this class's parameter scope.
    pub superclass: Option<SuperclassIr>,
    /// The source span for the class.
    pub span: SpanIr,
    /// The lowered fields in declaration order.
    pub fields: Vec<FieldIr>,
    /// The lowered constructors in declaration order.
    pub constructors: Vec<ConstructorIr>,
    /// The lowered methods in declaration order.
    pub methods: Vec<MethodIr>,
    /// Trait applications resolved for this class.
    pub traits: Vec<TraitApplicationIr>,
    /// Config applications resolved for this class.
    pub configs: Vec<ConfigApplicationIr>,
    /// Normalized serde-related class configuration.
    pub serde: Option<SerdeClassConfigIr>,
}

/// One type applied in a class's `with` clause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MixinApplicationIr {
    /// The applied type, including any type arguments.
    pub ty: TypeIr,
    /// The original application source span.
    pub span: SpanIr,
}

/// One type applied in a class's `extends` clause.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuperclassIr {
    /// The applied superclass type, retaining its actual arguments.
    pub ty: TypeIr,
    /// The original application source span.
    pub span: SpanIr,
}
