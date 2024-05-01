use std::fmt::Display;

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
    let (operation, first_byte) = decode_operation(instruction_stream)?;
    let decoded = match operation {
        Operation::Mov => decode_mov(first_byte, instruction_stream)?,
    };

    Some(decoded)
}

fn decode_operation<'stream, S>(instruction_stream: &'_ mut S) -> Option<(Operation, u8)>
where
    S: Iterator<Item = &'stream u8>,
{
    const OPCODE_MASK: u8 = 0b1111_1100;

    let first_byte = *instruction_stream.next()?;
    let operation = match first_byte & OPCODE_MASK {
        0b1000_1000 => Operation::Mov,
        _ => unimplemented!(),
    };

    Some((operation, first_byte))
}

fn decode_mov<'stream, S>(first_byte: u8, instruction_stream: &'_ mut S) -> Option<String>
where
    S: Iterator<Item = &'stream u8>,
{
    let second_byte = *instruction_stream.next()?;
    let mov = MovOperation::new(first_byte, second_byte);

    Some(mov.to_string())
}

#[derive(Debug)]
struct MovOperation {
    direction: Direction,
    mode: Mode,
    register: &'static str,
    register_or_memory: &'static str,
}

impl MovOperation {
    fn new(first_byte: u8, second_byte: u8) -> Self {
        let direction = Self::direction(first_byte);
        let size = Self::size(first_byte);
        let mode = Self::mode(second_byte);
        let register = Self::register(second_byte, size);
        let register_or_memory = Self::register_or_memory(second_byte, size);

        Self {
            direction,
            mode,
            register,
            register_or_memory,
        }
    }

    fn direction(byte: u8) -> Direction {
        const DIRECTIONS: [Direction; 2] = [Direction::FromRegister, Direction::ToRegister];
        const DIRECTION_MASK: u8 = 0b0000_0010;
        lookup_masked(&DIRECTIONS, byte, DIRECTION_MASK, 1)
    }

    fn size(byte: u8) -> Size {
        const SIZES: [Size; 2] = [Size::Byte, Size::Word];
        const SIZE_MASK: u8 = 0b0000_0001;
        lookup_masked(&SIZES, byte, SIZE_MASK, 0)
    }

    fn mode(byte: u8) -> Mode {
        const MODES: [Mode; 4] = [
            Mode::MemoryNoDisplacement,
            Mode::Memory8Bit,
            Mode::Memory16Bit,
            Mode::Register,
        ];
        const MODE_MASK: u8 = 0b1100_0000;
        lookup_masked(&MODES, byte, MODE_MASK, 6)
    }

    fn register(byte: u8, operation_size: Size) -> &'static str {
        const REGISTER_MASK: u8 = 0b0011_1000;

        let table = Self::register_table(operation_size);
        lookup_masked(table, byte, REGISTER_MASK, 3)
    }

    fn register_or_memory(byte: u8, operation_size: Size) -> &'static str {
        const REGISTER_OR_MEMORY_MASK: u8 = 0b0000_0111;

        let table = Self::register_table(operation_size);
        lookup_masked(table, byte, REGISTER_OR_MEMORY_MASK, 0)
    }

    fn register_table(operation_size: Size) -> &'static [&'static str; 8] {
        const BYTE_REGISTERS: [&str; 8] = ["al", "cl", "dl", "bl", "ah", "ch", "dh", "bh"];
        const WORD_REGISTERS: [&str; 8] = ["ax", "cx", "dx", "bx", "sp", "bp", "si", "di"];

        match operation_size {
            Size::Byte => &BYTE_REGISTERS,
            Size::Word => &WORD_REGISTERS,
        }
    }
}

impl Display for MovOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (first, second) = match self.direction {
            Direction::FromRegister => (self.register_or_memory, self.register),
            Direction::ToRegister => (self.register, self.register_or_memory),
        };
        write!(f, "mov {first}, {second}")
    }
}

fn lookup_masked<T, const N: usize>(table: &[T; N], byte: u8, mask: u8, shift: u8) -> T
where
    T: Copy,
{
    table[((byte & mask) >> shift) as usize]
}

#[derive(Clone, Copy, Debug)]
enum Operation {
    Mov,
}

#[derive(Clone, Copy, Debug)]
enum Direction {
    FromRegister,
    ToRegister,
}

#[derive(Clone, Copy, Debug)]
enum Size {
    Byte,
    Word,
}

#[derive(Clone, Copy, Debug)]
enum Mode {
    MemoryNoDisplacement,
    Memory8Bit,
    Memory16Bit,
    Register,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn single_register_mov() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("listings/listing_0037_single_register_mov");
        let instruction = std::fs::read(path).unwrap();

        let dissassembly = disassemble(&instruction);

        assert_eq!(
            dissassembly,
            "bits 16

mov cx, bx
"
            .to_string()
        )
    }

    #[test]
    fn many_register_mov() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("listings/listing_0038_many_register_mov");
        let instructions = std::fs::read(path).unwrap();

        let dissassembly = disassemble(&instructions);

        assert_eq!(
            dissassembly,
            "bits 16

mov cx, bx
mov ch, ah
mov dx, bx
mov si, bx
mov bx, di
mov al, cl
mov ch, ch
mov bx, ax
mov bx, si
mov sp, di
mov bp, ax
"
            .to_string()
        )
    }
}
