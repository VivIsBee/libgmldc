//! tester/CLI for libgmldc

use std::error::Error;

use libgm::gml::{GMCode, Instruction};

fn main() -> Result<(), Box<dyn Error>> {
    let code = GMCode {
        name: "code1".to_string(),
        instructions: vec![
            Instruction::Exit,
            Instruction::BranchIf { jump_offset: -1 },
            Instruction::Exit,
            Instruction::Exit,
            Instruction::Exit,
        ],
        modern_data: None,
    };

    println!("{}", libgmldc::decompile_one(&code)?);
    Ok(())
}
