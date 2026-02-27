//! GML decompiler using LibGM's disassembler and fabricator's structure

pub mod cfg;

use std::{
    collections::{HashMap, HashSet, VecDeque},
    ops::Range,
};

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

fn create_instr_cfg_from_code(code: &GMCode) -> Result<ControlFlowGraph> {
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
            Instruction::BranchIf { jump_offset }
            | Instruction::BranchUnless { jump_offset }
            | Instruction::PushWithContext { jump_offset }
            | Instruction::PopWithContext { jump_offset } => {
                i_next.push_back((
                    i,
                    NodeRef(
                        get_index_from_byte_offset(&code.instructions, *i, *jump_offset * 4)
                            .context("decompile_one")?,
                    ),
                ));
                i_next.push_back((i, NodeRef(*i + 1)));
            }
            Instruction::Return | Instruction::Exit => {}
            _ => {
                i_next.push_back((i, NodeRef(*i + 1)));
            }
        };

        cfg.insert(parent, i);
    }

    Ok(cfg)
}

/// convert a per-instruction CFG into a list of blocks and a block CFG
fn instr_cfg_to_block_cfg(
    code: &GMCode,
    in_cfg: ControlFlowGraph,
) -> (Vec<Range<usize>>, ControlFlowGraph) {
    let mut leaders = Vec::new();
    let mut trailers = Vec::new();

    leaders.push(NodeRef(0));

    for node in in_cfg.iter() {
        match in_cfg.children_of(node).len() {
            0 => {
                trailers.push(node);
                if *node < in_cfg.len() - 1 {
                    eprintln!("{node}");
                    leaders.push(NodeRef(*node + 1));
                }
            }
            1 => {}
            _ => {
                trailers.push(node);
                if *node < in_cfg.len() - 1 {
                    eprintln!("{node}");
                    leaders.push(NodeRef(*node + 1));
                }
            }
        }

        if matches!(
            code.instructions[*node],
            Instruction::Branch { jump_offset: _ }
        ) {
            trailers.push(node);
            if *node < in_cfg.len() - 1 {
                eprintln!("{node}");
                leaders.push(NodeRef(*node + 1));
            }
        }
    }

    let blocks = trailers
        .into_iter()
        .zip(leaders.into_iter())
        .collect::<HashMap<_, _>>();

    let mut out_cfg = ControlFlowGraph::new(NodeRef(0));

    for (i, block) in blocks.iter().enumerate() {
        let start = *block.1;

        let parents = in_cfg
            .parents_of(start)
            .iter()
            .copied()
            .collect::<Vec<_>>(); // would be great to remove this allocation but rust gets made about aliasing
        for parent in parents {
            out_cfg.insert(blocks[&parent], NodeRef(i));
        }
    }

    (
        blocks.into_iter().map(|v| (*v.1)..(*v.0)).collect(),
        out_cfg,
    )
}

/// Decompile a single code entry.
pub fn decompile_one(code: &GMCode /* , data: &GMData */) -> Result<String> {
    let instr_cfg = create_instr_cfg_from_code(code)?;

    let (blocks, cfg) = instr_cfg_to_block_cfg(code, instr_cfg);

    Ok(cfg.to_dot())
}
