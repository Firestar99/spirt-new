use smallvec::SmallVec;
use std::num::NonZeroU32;

pub mod lift;
pub mod lower;
pub mod print;
pub mod read;
pub mod spec;
pub mod write;

// TODO(eddyb) have a way to generate "parsed instructions" out of the module,
// basically whatever would be the the basis for outputting words back out.
pub struct ModuleLayout {
    // FIXME(eddyb) parse the version in the header.
    pub header_version: u32,

    pub original_generator_magic: u32,
    pub original_id_bound: u32,

    // FIXME(eddyb) this could be an `IndexSet` if not for duplicates.
    pub capabilities: Vec<u32>,
}

pub struct Inst {
    pub opcode: u16,

    // FIXME(eddyb) consider nesting "Result Type ID" in "Result ID".
    pub result_type_id: Option<Id>,
    pub result_id: Option<Id>,

    pub operands: SmallVec<[Operand; 2]>,
}

pub enum Operand {
    ShortImm(spec::OperandKind, u32),
    LongImmStart(spec::OperandKind, u32),
    LongImmCont(spec::OperandKind, u32),

    Id(spec::OperandKind, Id),

    // FIXME(eddyb) reduce uses of this by addressing the situations it can
    // appear in, with dedicated IR constructs instead.
    // FIXME(eddyb) if SPIR-T won't use this directly, is there a point in even
    // distinguishing between forward and other references? lowering would still
    // need to track that on its own anyway.
    ForwardIdRef(spec::OperandKind, Id),
}

pub type Id = NonZeroU32;
