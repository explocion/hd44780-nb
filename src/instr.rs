#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(transparent)]
pub struct CompiledInstr(pub(crate) u8);

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Deliverable {
    Instr(CompiledInstr),
    Data(u8),
}

#[derive(Debug)]
pub struct Clear;

impl Clear {
    pub const fn compile() -> CompiledInstr {
        CompiledInstr(0x01)
    }
}

#[derive(Debug)]
pub struct ReturnHome;

impl ReturnHome {
    pub const fn compile() -> CompiledInstr {
        CompiledInstr(0x02)
    }
}
