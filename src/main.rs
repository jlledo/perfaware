mod mov;

const HEADER: &str = "bits 16";

fn main() -> color_eyre::eyre::Result<()> {
    let file = std::env::args().nth(1).unwrap();
    let machine_code = std::fs::read(&file)?;
    let dissassembly = disassemble(&machine_code);
    print!("{dissassembly}");
    Ok(())
}

fn disassemble(machine_code: &[u8]) -> String {
    let mut dissassembly = format!("{HEADER}\n\n");
    let mut instruction_stream = machine_code.iter();

    while let Some(asm_instruction) = dissassemble_instruction(&mut instruction_stream) {
        dissassembly += &asm_instruction;
        dissassembly.push('\n');
    }

    dissassembly
}

fn dissassemble_instruction<'stream, S>(instruction_stream: &'_ mut S) -> Option<String>
where
    S: Iterator<Item = &'stream u8>,
{
    let first_byte = *instruction_stream.next()?;
    match first_byte & 0b1111_0000 {
        0b1011_0000 => {
            return mov::disassemble_immediate_to_register(first_byte, instruction_stream)
        }
        _ => (),
    };

    match first_byte & 0b1111_1100 {
        0b1000_1000 => {
            return mov::disassemble_register_to_from_register(first_byte, instruction_stream)
        }
        _ => unimplemented!(),
    };
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use super::*;

    fn write_asm(dissassembly: &[u8], name: &str) -> PathBuf {
        let mut asm_out_path = std::env::temp_dir();
        asm_out_path.push(format!("{name}.asm"));
        std::fs::write(&asm_out_path, dissassembly).unwrap();
        asm_out_path
    }

    fn assemble_asm(asm_path: impl AsRef<Path>) -> PathBuf {
        let asm_path = asm_path.as_ref();
        let mut bin_out_path = std::env::temp_dir();
        bin_out_path.push(asm_path.file_stem().unwrap());
        Command::new("nasm")
            .arg(asm_path.as_os_str())
            .arg("-o")
            .arg(&bin_out_path)
            .output()
            .unwrap();
        bin_out_path
    }

    #[test]
    fn single_register_mov() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("listings/listing_0037_single_register_mov");
        let instruction = std::fs::read(path).unwrap();

        let dissassembly = disassemble(&instruction);

        let asm_path = write_asm(dissassembly.as_bytes(), "single_register_mov");
        let bin_path = assemble_asm(asm_path);
        assert_eq!(std::fs::read(&bin_path).unwrap(), instruction);
    }

    #[test]
    fn many_register_mov() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("listings/listing_0038_many_register_mov");
        let instructions = std::fs::read(path).unwrap();

        let dissassembly = disassemble(&instructions);

        let asm_path = write_asm(dissassembly.as_bytes(), "many_register_mov");
        let bin_path = assemble_asm(asm_path);
        assert_eq!(std::fs::read(&bin_path).unwrap(), instructions);
    }

    #[test]
    fn more_movs() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("listings/listing_0039_more_movs");
        let instructions = std::fs::read(path).unwrap();

        let dissassembly = disassemble(&instructions);

        let asm_path = write_asm(dissassembly.as_bytes(), "more_movs");
        let bin_path = assemble_asm(asm_path);
        assert_eq!(std::fs::read(&bin_path).unwrap(), instructions);
    }
}
