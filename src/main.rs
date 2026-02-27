//! tester/CLI for libgmldc

use std::error::Error;

use libgm::gml::{GMCode, Instruction, instruction::PushValue};

fn main() -> Result<(), Box<dyn Error>> {
    let code = GMCode {
        name: "code1".to_string(),
        instructions: vec![
            Instruction::Push {
                value: PushValue::Boolean(false),
            },
            Instruction::BranchIf { jump_offset: -2 },
            Instruction::Push {
                value: PushValue::Boolean(true),
            },
            Instruction::Push {
                value: PushValue::Boolean(false),
            },
            Instruction::Exit,
        ],
        modern_data: None,
    };

    println!("{}", libgmldc::decompile_one(&code)?);
    Ok(())
}
