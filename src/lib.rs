//! GML decompiler using LibGM's disassembler

pub mod ast;
pub mod cfg;

use std::{
    collections::{HashMap, HashSet, VecDeque},
    ops::Range,
};

use libgm::{
    gml::{
        GMCode, Instruction,
        instruction::{AssetReference, DataType, PushValue},
    },
    prelude::*,
};

use crate::{
    ast::{BinaryOp, Constant},
    cfg::{ControlFlowGraph, NodeRef},
};

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

fn create_instr_cfg_from_code(code: &GMCode) -> Result<ControlFlowGraph<()>> {
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

        cfg.insert(parent, i, ());
    }

    Ok(cfg)
}

#[derive(Clone, Debug)]
struct BlockMeta {
    instr_range: Range<usize>,
    resolve_state: ResolveState,
}

/// convert a per-instruction CFG into a list of blocks and a block CFG
fn instr_cfg_to_block_cfg(
    code: &GMCode,
    in_cfg: ControlFlowGraph<()>,
) -> ControlFlowGraph<BlockMeta> {
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
        .enumerate()
        .map(|v| (v.1.0, (v.0, v.1.1)))
        .collect::<HashMap<_, _>>();

    let mut out_cfg = ControlFlowGraph::new_rootless();

    for (end, (i, start)) in &blocks {
        let parents = in_cfg
            .parents_of(*start)
            .iter()
            .copied()
            .collect::<Vec<_>>(); // would be great to remove this allocation but rust gets mad about aliasing
        for parent in parents {
            out_cfg.insert_parentless(
                NodeRef(blocks[&parent].0),
                BlockMeta {
                    instr_range: Range::default(),
                    resolve_state: ResolveState::Unresolved,
                },
            );
            out_cfg.insert(
                NodeRef(blocks[&parent].0),
                NodeRef(*i),
                BlockMeta {
                    instr_range: **start..(**end + 1),
                    resolve_state: ResolveState::Unresolved,
                },
            );
        }
    }

    out_cfg
}

/// Decompile a single code entry.
pub fn decompile_one(code: &GMCode, data: &GMData) -> Result<String> {
    let instr_cfg = create_instr_cfg_from_code(code)?;

    let cfg = instr_cfg_to_block_cfg(code, instr_cfg);

    let mut out = String::new();

    for i in 0..cfg.len() {
        if let Some(res) = StraightLineResolver::try_resolve(&cfg, code, data, NodeRef(i))? {
            out.push_str(&format!(
                "\n{}",
                match res.merged_into {
                    ResolveState::Resolved(v) => format!("{v:#?}"),
                    _ => unreachable!(),
                }
            ));
        }
    }

    Ok(out)
}

fn get_code_of_block<'a>(block: &BlockMeta, code: &'a GMCode) -> &'a [Instruction] {
    &code.instructions[block.instr_range.clone()]
}

#[derive(Clone, Debug)]
enum ResolveState {
    Unresolved,
    Resolved(ast::Block),
}

/// Removes all of the nodes listed in `nodes` and creates a node emcompassing
/// all of them with instructions with the resolve state of `merged_into` and
/// the listed children and parents.
#[derive(Clone, Debug)]
struct Resolution {
    nodes: HashSet<NodeRef>,
    merged_into: ResolveState,
    merged_children: HashSet<NodeRef>,
    merged_parents: HashSet<NodeRef>,
}

trait Resolver {
    /// How specific this is. Something that can encapsulate any construct would
    /// be [`i16::MIN`], something that only works for one very specific
    /// scenario (or is something like the straight-line resolver) would be
    /// [`i16::MAX`]
    const SPECIFICITY: i16;

    /// Resolve this construct into a block.
    ///
    /// If `None` is returned, this `Resolver` cannot resolve the construct at
    /// `entry`.
    fn try_resolve(
        block_cfg: &ControlFlowGraph<BlockMeta>,
        code: &GMCode,
        data: &GMData,
        entry: NodeRef,
    ) -> Result<Option<Resolution>>;
}

struct StraightLineResolver;

impl Resolver for StraightLineResolver {
    const SPECIFICITY: i16 = i16::MAX;

    fn try_resolve(
        block_cfg: &ControlFlowGraph<BlockMeta>,
        code: &GMCode,
        data: &GMData,
        entry: NodeRef,
    ) -> Result<Option<Resolution>> {
        let range = block_cfg.meta_of(entry).instr_range.clone();
        if range.len() <= 1 {
            return Ok(None);
        }

        let code = &code.instructions[range];

        let mut out = Vec::new();
        let mut stack = Vec::new();

        let mut i = 0usize;

        loop {
            if i >= code.len() {
                break;
            }
            let instr = code[i].clone();
            match instr {
                Instruction::Push { value } => {
                    stack.push(match value {
                        PushValue::Boolean(v) => ast::Expr::Constant(Constant::Boolean(v)),
                        PushValue::Int16(v) => ast::Expr::Constant(Constant::Integer(v as i64)),
                        PushValue::Int32(v) => ast::Expr::Constant(Constant::Integer(v as i64)),
                        PushValue::Int64(v) => ast::Expr::Constant(Constant::Integer(v as i64)),
                        PushValue::Double(v) => ast::Expr::Constant(Constant::Float(v)),
                        PushValue::String(v) => ast::Expr::Constant(Constant::String(v)),
                        PushValue::Function(v) => ast::Expr::Ident(
                            v.resolve(&data.functions.functions).unwrap().name.clone(),
                        ),
                        PushValue::Variable(v) => ast::Expr::Ident(
                            v.variable
                                .resolve(&data.variables.variables)
                                .unwrap()
                                .name
                                .clone(),
                        ),
                    });
                }
                Instruction::Add {
                    augend: _,
                    addend: _,
                }
                | Instruction::And { lhs: _, rhs: _ }
                | Instruction::Divide {
                    dividend: _,
                    divisor: _,
                }
                | Instruction::Modulus {
                    dividend: _,
                    divisor: _,
                }
                | Instruction::Or { lhs: _, rhs: _ }
                | Instruction::Remainder {
                    dividend: _,
                    divisor: _,
                }
                | Instruction::ShiftLeft {
                    value: _,
                    shift_amount: _,
                }
                | Instruction::ShiftRight {
                    value: _,
                    shift_amount: _,
                }
                | Instruction::Subtract {
                    minuend: _,
                    subtrahend: _,
                }
                | Instruction::Xor { lhs: _, rhs: _ }
                | Instruction::Multiply {
                    multiplicand: _,
                    multiplier: _,
                } => {
                    let (arg2, arg1) = (
                        stack.pop().ok_or(err!("stack underflow while attempting to resolve straight-line block {entry}"))?,
                        stack.pop().ok_or(err!("stack underflow while attempting to resolve straight-line block {entry}"))?
                    );

                    stack.push(ast::Expr::Binary{
                        lhs: Box::new(arg1),
                        rhs: Box::new(arg2),
                        op: match instr {
                            Instruction::Add {
                                augend: _,
                                addend: _,
                            } => BinaryOp::Add,
                            Instruction::And {
                                lhs: DataType::Boolean,
                                rhs: _,
                            } => BinaryOp::And,
                            Instruction::And { lhs: _, rhs: _ } => BinaryOp::BitAnd,
                            Instruction::Divide {
                                dividend: DataType::Int16 | DataType::Int32 | DataType::Int64,
                                divisor: _,
                            } => BinaryOp::IDiv,
                            Instruction::Divide {
                                dividend: _,
                                divisor: _,
                            } => BinaryOp::Div,
                            Instruction::Modulus {
                                dividend: _,
                                divisor: _,
                            }
                            | Instruction::Remainder {
                                dividend: _,
                                divisor: _,
                            } => BinaryOp::Rem,
                            Instruction::Or {
                                lhs: DataType::Boolean,
                                rhs: _,
                            } => BinaryOp::Or,
                            Instruction::Or { lhs: _, rhs: _ } => BinaryOp::BitOr,
                            Instruction::ShiftLeft {
                                value: _,
                                shift_amount: _,
                            } => BinaryOp::BitShiftLeft,
                            Instruction::ShiftRight {
                                value: _,
                                shift_amount: _,
                            } => BinaryOp::BitShiftRight,
                            Instruction::Subtract {
                                minuend: _,
                                subtrahend: _,
                            } => BinaryOp::Sub,
                            Instruction::Xor {
                                lhs: DataType::Boolean,
                                rhs: _,
                            } => BinaryOp::Xor,
                            Instruction::Xor { lhs: _, rhs: _ } => BinaryOp::BitXor,
                            Instruction::Multiply {
                                multiplicand: _,
                                multiplier: _,
                            } => BinaryOp::Mult,
                            _ => unreachable!(),
                        },
                    });
                }
                Instruction::Call {
                    function,
                    argument_count,
                } => {
                    let mut args = Vec::new();
                    for _ in 0..argument_count {
                        args.push(stack.pop().unwrap());
                    }
                    stack.push(ast::Expr::Call(ast::Call {
                        base: Box::new(ast::Expr::Ident(
                            function
                                .resolve(&data.functions.functions)
                                .unwrap()
                                .name
                                .clone(),
                        )),
                        arguments: args,
                        has_new: false,
                    }));
                }
                Instruction::PushReference { asset_reference } => {
                    stack.push(ast::Expr::Ident(match asset_reference {
                        AssetReference::Object(gmref) => gmref
                            .resolve(&data.game_objects.game_objects)
                            .unwrap()
                            .name
                            .clone(),
                        AssetReference::Sprite(gmref) => {
                            gmref.resolve(&data.sprites.sprites).unwrap().name.clone()
                        }
                        AssetReference::Sound(gmref) => {
                            gmref.resolve(&data.sounds.sounds).unwrap().name.clone()
                        }
                        AssetReference::Room(gmref) => {
                            gmref.resolve(&data.rooms.rooms).unwrap().name.clone()
                        }
                        AssetReference::Path(gmref) => {
                            gmref.resolve(&data.paths.paths).unwrap().name.clone()
                        }
                        AssetReference::Script(gmref) => {
                            gmref.resolve(&data.scripts.scripts).unwrap().name.clone()
                        }
                        AssetReference::Font(gmref) => {
                            gmref.resolve(&data.fonts.fonts).unwrap().name.clone()
                        }
                        AssetReference::Timeline(gmref) => gmref
                            .resolve(&data.timelines.timelines)
                            .unwrap()
                            .name
                            .clone(),
                        AssetReference::Shader(gmref) => {
                            gmref.resolve(&data.shaders.shaders).unwrap().name.clone()
                        }
                        AssetReference::Sequence(gmref) => gmref
                            .resolve(&data.sequences.sequences)
                            .unwrap()
                            .name
                            .clone(),
                        AssetReference::AnimCurve(gmref) => gmref
                            .resolve(&data.animation_curves.animation_curves)
                            .unwrap()
                            .name
                            .clone(),
                        AssetReference::ParticleSystem(gmref) => gmref
                            .resolve(&data.particle_systems.particle_systems)
                            .unwrap()
                            .name
                            .clone(),
                        AssetReference::Background(gmref) => gmref
                            .resolve(&data.backgrounds.backgrounds)
                            .unwrap()
                            .name
                            .clone(),
                        AssetReference::RoomInstance(v) => format!("inst_{v:X}"),
                        AssetReference::Function(gmref) => gmref
                            .resolve(&data.functions.functions)
                            .unwrap()
                            .name
                            .clone(),
                    }))
                }
                Instruction::Exit => {
                    out.push(ast::Statement::Return(None));
                }
                Instruction::Return => {
                    let val = stack.pop().unwrap();
                    out.push(ast::Statement::Return(Some(Box::new(val))));
                }
                Instruction::Pop {
                    variable,
                    type1: _,
                    type2: _,
                } => {
                    let val = stack.pop().unwrap();
                    out.push(ast::Statement::Assignment{
                        target: ast::MutableExpr::Ident(
                            variable
                                .variable
                                .resolve(&data.variables.variables)?
                                .name
                                .clone(),
                        ),
                        op: ast::AssignmentOp::Equal,
                        value: Box::new(val),
                    });
                }
                Instruction::Branch { jump_offset } => {
                    i = get_index_from_byte_offset(&code, i, jump_offset)?;
                }
                Instruction::BranchIf { jump_offset: _ }
                | Instruction::BranchUnless { jump_offset: _ } => {
                    stack.pop();
                }
                Instruction::Convert { from: _, to: _ } => {}
                _ => todo!("{instr:#?}"),
            }
            i += 1;
        }

        Ok(Some(Resolution {
            nodes: [entry].into_iter().collect(),
            merged_into: ResolveState::Resolved(ast::Block(out)),
            merged_children: block_cfg.children_of(entry).clone(),
            merged_parents: block_cfg.parents_of(entry).clone(),
        }))
    }
}
