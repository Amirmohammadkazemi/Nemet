
// [mov]
//
// REX 89 "r" r/m16-64 r/16-64
// REX 8B "r" r16-64 r/m16-64
// REX b8+r r16-64 imm16-64
// REX c7 "0" r/m16-64 imm16-32
//
// 88 "r" r/m8 r8
// 8A "r" r8 r/m8
// B0+r r8 imm8
// c6 "0" r/m8 imm8

use super::instructions::Opr;


fn _mov8(op1: &Opr, op2: &Opr) -> Vec<u8> {
    vec![]
}

fn _mov(op1: &Opr,op2: &Opr) -> Vec<u8> {
    vec![]
}

pub fn mov(op1: &Opr, op2: &Opr) -> Vec<u8> {
    if op1.is_8bit() && op2.is_8bit() {
        return _mov8(op1,op2);
    }
    _mov(op1,op2)
}
