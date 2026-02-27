//! tester/CLI for libgmldc

use std::error::Error;

use libgm::gml::{
    GMCode, Instruction,
    instruction::{DataType, PushValue},
};

fn main() -> Result<(), Box<dyn Error>> {
    let code = GMCode {
        name: "code1".to_string(),
        instructions: vec![
            Instruction::Push {
                value: PushValue::Boolean(true),
            },
            Instruction::BranchIf { jump_offset: -2 },
            Instruction::Push {
                value: PushValue::Boolean(true),
            },
            Instruction::Convert {
                from: DataType::Boolean,
                to: DataType::Variable,
            },
            Instruction::Return,
        ],
        modern_data: None,
    };

    println!("{}", libgmldc::decompile_one(&code)?);
    Ok(())
}
