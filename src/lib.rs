//! GML decompiler using LibGM's disassembler and fabricator's structure

pub mod cfg;

use std::collections::{HashSet, VecDeque};

use libgm::{
    gml::{GMCode, Instruction},
    prelude::*,
};

use crate::cfg::{ControlFlowGraph, NodeRef};

/// Perform an early return with the specified formatted message.
/// This is a simple alias for `return Err(Error::new(format!(...));`.
macro_rules! bail {
    ($($arg:tt)*) => {
        return Err(libgm::error::Error::new(format!($($arg)*)))
    };
}

fn get_index_from_bytes(instructions: &[Instruction], byte_index: u32) -> Result<usize> {
    if byte_index == 0 {
        return Ok(0);
    }
    let mut index = 0;
    let mut offset = 0;
    while offset < byte_index {
        let instr = instructions.get(index).ok_or_else(|| 
            format!("Given byte offset {byte_index} is out of range in instructions with byte length {offset}"
        ))?;
        index += 1;
        offset += instr.size();
    }
    if offset != byte_index {
        bail!("Given byte offset {byte_index} is misaligned (reached offset {offset} instead)");
    }
    Ok(index)
}

fn code_byte_length(instructions: &[Instruction]) -> u32 {
    let mut size: u32 = 0;
    for instruction in instructions {
        size += instruction.size();
    }
    size
}

fn get_index_from_byte_offset(
    instructions: &[Instruction],
    index: usize,
    offset: i32,
) -> Result<usize> {
    if offset == 0 {
        return Ok(index);
    }
    if offset.is_negative() {
        let instructions = &instructions[..index];
        let pos_offset = code_byte_length(instructions)
            .checked_add_signed(offset)
            .ok_or("negative offset would send index before start of instructions")?;
        get_index_from_bytes(instructions, pos_offset)
    } else {
        get_index_from_bytes(&instructions[index..], offset as u32)
    }
}

fn create_cfg_from_code(code: &GMCode) -> Result<ControlFlowGraph> {
    let start_i = get_index_from_bytes(&code.instructions, code.execution_offset())
        .context("decompile_one")?;
    let mut cfg = cfg::ControlFlowGraph::new_rootless();

    let mut i_next: VecDeque<(NodeRef, NodeRef)> = vec![(NodeRef(0), NodeRef(start_i))].into();
    let mut seen_once = HashSet::new();
    loop {
        let i = &mut i_next.pop_front();
        if i.is_none() || *i.unwrap().1 >= code.instructions.len() {
            break;
        }
        let (parent, i) = i.unwrap();

        if cfg.has(i) && seen_once.contains(&i) {
            continue;
        } else if cfg.has(i) {
            seen_once.insert(i);
        }

        let instr = &code.instructions[*i];
        match instr {
            Instruction::Branch { jump_offset } => {
                i_next.push_back((
                    i,
                    NodeRef(
                        get_index_from_byte_offset(&code.instructions, *i, *jump_offset * 4)
                            .context("decompile_one")?,
                    ),
                ));
            }
            Instruction::BranchIf { jump_offset } | Instruction::BranchUnless { jump_offset } | Instruction::PushWithContext { jump_offset } | Instruction::PopWithContext { jump_offset } => {
                i_next.push_back((
                    i,
                    NodeRef(
                        get_index_from_byte_offset(&code.instructions, *i, *jump_offset * 4)
                            .context("decompile_one")?,
                    ),
                ));
                i_next.push_back((i, NodeRef(*i + 1)));
            },
            Instruction::Return | Instruction::Exit => {}
            _ => {
                i_next.push_back((i, NodeRef(*i + 1)));
            }
        };

        cfg.push(parent, i);
    }

    Ok(cfg)
}

/// Decompile a single code entry.
pub fn decompile_one(code: &GMCode /* , data: &GMData */) -> Result<String> {
    let cfg = create_cfg_from_code(code)?;

    Ok(format!("{cfg:#?}"))
}
