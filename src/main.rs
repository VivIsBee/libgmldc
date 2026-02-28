//! tester/CLI for libgmldc

use std::error::Error;

use libgm::{
    gml::{
        GMCode, Instruction,
        instruction::{DataType, PushValue},
    },
    prelude::GMData,
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
                value: PushValue::Int64(256),
            },
            Instruction::Push {
                value: PushValue::Int64(256),
            },
            Instruction::Add {
                augend: DataType::Int64,
                addend: DataType::Int64,
            },
            Instruction::Convert {
                from: DataType::Int64,
                to: DataType::Variable,
            },
            Instruction::Return,
        ],
        modern_data: None,
    };

    let data = GMData {
        ..Default::default()
    };

    println!("{}", libgmldc::decompile_one(&code, &data)?);
    Ok(())
}
