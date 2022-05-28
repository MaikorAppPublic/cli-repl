use color_eyre::Result;
use crossterm::cursor::MoveToColumn;
use crossterm::event::Event::Key;
use crossterm::event::{KeyCode, KeyModifiers};
use crossterm::style::{Color, Print, ResetColor, SetForegroundColor};
use crossterm::terminal::{Clear, ClearType};
use crossterm::ExecutableCommand;
use maikor_asm_parser::parse_line_from_str;
use maikor_vm_core::VM;
use std::io::stdout;

pub fn run() -> Result<()> {
    //use new_test here as we don't care about audio
    let mut vm = VM::new_test();
    let mut mode_enter_command = false;
    let mut command = String::new();

    stdout().execute(Print("\n"))?;

    loop {
        if mode_enter_command {
            stdout()
                .execute(Clear(ClearType::CurrentLine))?
                .execute(MoveToColumn(1))?
                .execute(Print(format!("> {}\r", command)))?;
        } else {
            stdout()
                .execute(Clear(ClearType::CurrentLine))?
                .execute(MoveToColumn(1))?
                .execute(Print(
                    "(r) print registers  (f) flags   (return) enter command",
                ))?;
        }
        if let Key(key) = crossterm::event::read()? {
            if mode_enter_command {
                if key.code == KeyCode::Esc {
                    mode_enter_command = false;
                }
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    return Ok(());
                }
                if key.code == KeyCode::Backspace {
                    command.pop();
                }
                if key.code == KeyCode::Enter {
                    mode_enter_command = false;
                    let input = command.trim();
                    match parse_line_from_str(input) {
                        Ok(line) => {
                            stdout()
                                .execute(Clear(ClearType::CurrentLine))?
                                .execute(Print(format!(
                                    "Executing '{input}' {:?}\r\n",
                                    line.bytes
                                )))?;

                            let original_reg = vm.registers;
                            vm.execute_op(&line.bytes);
                            let post_reg = vm.registers;
                            let diff = calc_diff(original_reg, post_reg);
                            stdout().execute(Print(format!("({diff})\r\n\n")))?;
                        }
                        Err(err) => {
                            stdout()
                                .execute(SetForegroundColor(Color::Red))?
                                .execute(Print(format!("Error: {command}\r\n")))?
                                .execute(Print(format!("{:?}\r\n", err.to_string())))?
                                .execute(ResetColor)?;
                        }
                    }
                }
                if let KeyCode::Char(chr) = key.code {
                    command.push(chr);
                }
            } else {
                if key.code == KeyCode::Char('r') {
                    crossterm::execute!(
                        stdout(),
                        Clear(ClearType::CurrentLine),
                        MoveToColumn(1),
                        Print(format!(
                            "AH {:02X} AL {:02X} BH {:02X} BL {:02X}\r\n",
                            vm.registers[0], vm.registers[1], vm.registers[2], vm.registers[3]
                        )),
                        Print(format!(
                            "CH {:02X} CL {:02X} DH {:02X} DL {:02X}\r\n",
                            vm.registers[4], vm.registers[5], vm.registers[6], vm.registers[7]
                        )),
                        Print(format!(
                            "AX {:04X} BX {:04X} CX {:04X} DX {:04X}\r\n",
                            u16::from_be_bytes([vm.registers[0], vm.registers[1]]),
                            u16::from_be_bytes([vm.registers[2], vm.registers[3]]),
                            u16::from_be_bytes([vm.registers[4], vm.registers[5]]),
                            u16::from_be_bytes([vm.registers[6], vm.registers[7]])
                        )),
                        Print(format!(
                            "FLG {:08b} PC {:04X} SP {:04X} FP {:04X}\r\n\n",
                            vm.registers[8],
                            vm.pc,
                            vm.get_sp(),
                            vm.get_fp()
                        ))
                    )?;
                }
                if key.code == KeyCode::Char('f') {
                    stdout()
                        .execute(Clear(ClearType::CurrentLine))?
                        .execute(MoveToColumn(1))?
                        .execute(Print(decode_flags(vm.registers[8])))?
                        .execute(Print("\r\n\n"))?;
                }
                if key.code == KeyCode::Esc
                    || key.code == KeyCode::Char('c')
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                {
                    return Ok(());
                }
                if key.code == KeyCode::Enter {
                    mode_enter_command = true;
                    command = String::new();
                    stdout()
                        .execute(Clear(ClearType::CurrentLine))?
                        .execute(MoveToColumn(1))?;
                }
            }
        }
    }
}

fn calc_diff(old: [u8; 9], new: [u8; 9]) -> String {
    let reg = [
        ("AH", 0, 1),
        ("AL", 1, 1),
        ("BH", 2, 1),
        ("BL", 3, 1),
        ("CH", 4, 1),
        ("CL", 5, 1),
        ("DH", 6, 1),
        ("DL", 7, 1),
        ("AX", 0, 2),
        ("BX", 2, 2),
        ("CX", 4, 2),
        ("DX", 6, 2),
        ("FLG", 8, 1),
    ];
    let mut output = String::new();

    for (name, idx, size) in reg {
        if let Some((old, new)) = match size {
            1 => {
                if old[idx] != new[idx] {
                    Some((old[idx].to_string(), new[idx].to_string()))
                } else {
                    None
                }
            }
            2 => {
                let old = u16::from_be_bytes([old[idx], old[idx + 1]]);
                let new = u16::from_be_bytes([new[idx], new[idx + 1]]);
                if old != new {
                    Some((old.to_string(), new.to_string()))
                } else {
                    None
                }
            }
            _ => panic!("Invalid/unsupported size: {size}"),
        } {
            if !output.is_empty() {
                output.push_str(", ");
            }
            output.push_str(&format!("{name} = {old} -> {new}"));
        }
    }

    output
}

fn decode_flags(byte: u8) -> String {
    use maikor_platform::registers::flags::*;
    let regs = [
        ("CARRY", CARRY),
        ("OVERFLOW", OVERFLOW),
        ("LESS THAN", LESS_THAN),
        ("GREATER THAN", GREATER_THAN),
        ("ZERO", ZERO),
        ("SIGNED", SIGNED),
        ("INTERRUPTS", INTERRUPTS),
    ];
    let mut output = String::new();
    for (name, mask) in regs {
        if byte & mask == mask {
            if !output.is_empty() {
                output.push_str(", ");
            }
            output.push_str(name);
        }
    }
    if output.is_empty() {
        String::from("-")
    } else {
        output
    }
}
