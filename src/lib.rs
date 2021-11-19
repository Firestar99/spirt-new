use smallvec::SmallVec;
use std::collections::BTreeSet;

mod context;
pub use context::{AttrSet, Const, Context, InternedStr, Type};

pub mod print;
pub mod visit;

pub mod spv;

// HACK(eddyb) this only serves to disallow modifying the `cx` field of `Module`.
mod sealed {
    use super::*;
    use std::rc::Rc;

    pub struct Module {
        /// Context used for everything interned, in this module.
        ///
        /// Notable choices made for this field:
        /// * private to disallow switching the context of a module
        /// * `Rc` sharing to allow multiple modules to use the same context
        ///   (`Context: !Sync` because of the interners so it can't be `Arc`)
        cx: Rc<Context>,

        pub dialect: ModuleDialect,
        pub debug_info: ModuleDebugInfo,

        pub globals: Vec<Global>,
        pub funcs: Vec<Func>,
    }

    impl Module {
        pub fn new(cx: Rc<Context>, dialect: ModuleDialect, debug_info: ModuleDebugInfo) -> Self {
            Self {
                cx,

                dialect,
                debug_info,

                globals: vec![],
                funcs: vec![],
            }
        }

        // FIXME(eddyb) `cx_ref` might be the better default in situations where
        // the module doesn't need to be modified, figure out if that's common.
        pub fn cx(&self) -> Rc<Context> {
            self.cx.clone()
        }

        pub fn cx_ref(&self) -> &Rc<Context> {
            &self.cx
        }
    }
}
pub use sealed::Module;

pub enum ModuleDialect {
    Spv(spv::Dialect),
}

pub enum ModuleDebugInfo {
    Spv(spv::ModuleDebugInfo),
}

// FIXME(eddyb) maybe special-case some basic types like integers.
#[derive(PartialEq, Eq, Hash)]
pub struct TypeDef {
    pub ctor: TypeCtor,
    pub ctor_args: SmallVec<[TypeCtorArg; 2]>,
    pub attrs: AttrSet,
}

#[derive(PartialEq, Eq, Hash)]
pub enum TypeCtor {
    SpvInst(spv::spec::Opcode),
}

impl TypeCtor {
    pub fn name(&self) -> &'static str {
        match self {
            Self::SpvInst(opcode) => opcode.name(),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum TypeCtorArg {
    Type(Type),
    Const(Const),

    // FIXME(eddyb) reconsider whether flattening "long immediates" is a good idea.
    // FIXME(eddyb) it might be worth investingating the performance implications
    // of interning "long immediates", compared to the flattened representation.
    SpvImm(spv::Imm),
}

// FIXME(eddyb) maybe special-case some basic consts like integer literals.
#[derive(PartialEq, Eq, Hash)]
pub struct ConstDef {
    pub ty: Type,
    pub ctor: ConstCtor,
    pub ctor_args: SmallVec<[ConstCtorArg; 2]>,
    pub attrs: AttrSet,
}

#[derive(PartialEq, Eq, Hash)]
pub enum ConstCtor {
    SpvInst(spv::spec::Opcode),
}

impl ConstCtor {
    pub fn name(&self) -> &'static str {
        match self {
            Self::SpvInst(opcode) => opcode.name(),
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum ConstCtorArg {
    Const(Const),

    // FIXME(eddyb) reconsider whether flattening "long immediates" is a good idea.
    // FIXME(eddyb) it might be worth investingating the performance implications
    // of interning "long immediates", compared to the flattened representation.
    SpvImm(spv::Imm),

    // FIXME(eddyb) this is really bad because it's interned and shouldn't have
    // an ID, but it's hard to determine ahead of time whether an `OpVariable`
    // can plausibly be considered "constant data", instead of having an identity,
    // and even then, there may be an usecase for pointers to mutable global vars
    // in constants. Probably the only way to make it work in the most general
    // case is to make it point to a `Module` global variable, by either:
    // * having different "link-time constants" that are kept in `Module` instead
    // * introducing a notion of "generics", with the real variable pointer being
    //   "passed in" into the "generic" constant as a "generic parameter"
    // FIXME(eddyb) consider introducing a "deferred error" system, where the
    // producer (or `spv::lower`) can keep around errors in the SPIR-T IR, and
    // still have the opportunity of silencing them e.g. by removing dead code.
    SpvUntrackedGlobalVarId(spv::Id),
}

pub enum Global {
    Misc(Misc),
}

pub struct Func {
    pub insts: Vec<Misc>,
}

pub struct Misc {
    pub kind: MiscKind,

    // FIXME(eddyb) track this entirely as a def-use graph.
    pub output: Option<MiscOutput>,

    // FIXME(eddyb) maybe split inputs into "params" and "value inputs"?
    // (would "params" only contain immediates, or also e.g. types?)
    pub inputs: SmallVec<[MiscInput; 2]>,

    pub attrs: AttrSet,
}

pub enum MiscKind {
    SpvInst(spv::spec::Opcode),
}

impl MiscKind {
    pub fn name(&self) -> &'static str {
        match self {
            Self::SpvInst(opcode) => opcode.name(),
        }
    }
}

#[derive(Copy, Clone)]
pub enum MiscOutput {
    SpvResult {
        result_type: Option<Type>,
        result_id: spv::Id,
    },
}

#[derive(Copy, Clone)]
pub enum MiscInput {
    Type(Type),
    Const(Const),

    // FIXME(eddyb) reconsider whether flattening "long immediates" is a good idea.
    // FIXME(eddyb) it might be worth investingating the performance implications
    // of interning "long immediates", compared to the flattened representation.
    SpvImm(spv::Imm),

    // FIXME(eddyb) get rid of this by tracking all entities SPIR-V uses ID for.
    SpvUntrackedId(spv::Id),

    SpvExtInstImport(InternedStr),
}

#[derive(Default, PartialEq, Eq, Hash)]
pub struct AttrSetDef {
    // FIXME(eddyb) use `BTreeMap<Attr, AttrValue>` and split some of the params
    // between the `Attr` and `AttrValue` based on specified uniquness.
    // FIXME(eddyb) don't put debuginfo in here, but rather at use sites
    // (for e.g. types, with component types also having the debuginfo
    // bundled at the use site of the composite type) in order to allow
    // deduplicating definitions that only differ in debuginfo, in SPIR-T,
    // and still lift SPIR-V with duplicate definitions, out of that.
    pub attrs: BTreeSet<Attr>,
}

// FIXME(eddyb) consider interning individual attrs, not just `AttrSet`s.
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Attr {
    // FIXME(eddyb) de-special-case this by recomputing the interface IDs.
    SpvEntryPoint {
        params: SmallVec<[spv::Imm; 2]>,
        interface_ids: SmallVec<[spv::Id; 4]>,
    },

    SpvAnnotation {
        // FIXME(eddyb) determine this based on the annotation.
        opcode: spv::spec::Opcode,
        // FIXME(eddyb) this cannot represent IDs - is that desirable?
        // (for now we don't support `Op{ExecutionMode,Decorate}Id`)
        params: SmallVec<[spv::Imm; 2]>,
    },

    SpvDebugLine {
        file_path: OrdAssertEq<InternedStr>,
        line: u32,
        col: u32,
    },
}

// HACK(eddyb) wrapper to limit `Ord` for interned index types (e.g. `InternedStr`)
// to only situations where the interned index reflects contents (i.e. equality).
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct OrdAssertEq<T>(pub T);

impl<T: Eq> PartialOrd for OrdAssertEq<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Eq> Ord for OrdAssertEq<T> {
    #[track_caller]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        assert!(
            self == other,
            "OrdAssertEq<{}>::cmp called with unequal values",
            std::any::type_name::<T>(),
        );
        std::cmp::Ordering::Equal
    }
}
