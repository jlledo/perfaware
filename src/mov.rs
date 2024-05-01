use std::fmt::Display;

fn lookup_masked<T, const N: usize>(table: &[T; N], byte: u8, mask: u8, shift: u8) -> T
where
    T: Copy,
{
    table[((byte & mask) >> shift) as usize]
}

#[derive(Debug)]
pub struct MovOperation {
    direction: Direction,
    mode: Mode,
    register: &'static str,
    register_or_memory: &'static str,
}

impl MovOperation {
    pub fn new(first_byte: u8, second_byte: u8) -> Self {
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
