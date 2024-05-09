use std::borrow::Cow;

pub fn disassemble_register_to_from_register<I>(instruction_stream: &'_ mut I) -> Option<String>
where
    I: Iterator<Item = u8>,
{
    let first_byte = instruction_stream.next()?;
    let direction = lookup_masked(&DIRECTIONS, first_byte, 0b0000_0010, 1);
    let operation_size = lookup_masked(&SIZES, first_byte, 0b0000_0001, 0);

    let second_byte = instruction_stream.next()?;
    let register_table = register_table(operation_size);
    let register = lookup_masked(register_table, second_byte, 0b0011_1000, 3);
    let register_or_memory = register_or_memory(operation_size, second_byte, instruction_stream)?;

    let string = match direction {
        Direction::FromRegister => format!("mov {register_or_memory}, {register}"),
        Direction::ToRegister => format!("mov {register}, {register_or_memory}"),
    };

    Some(string)
}

fn register_or_memory<I>(
    size: Size,
    second_byte: u8,
    instruction_stream: &'_ mut I,
) -> Option<Cow<'static, str>>
where
    I: Iterator<Item = u8>,
{
    let mode = lookup_masked(&MODES, second_byte, 0b1100_0000, 6);
    let r_m = match mode {
        Mode::MemoryNoDisplacement => r_m_format_no_displacement(second_byte, instruction_stream)?,
        Mode::Memory8Bit => r_m_format_8_bit_displacement(second_byte, instruction_stream.next()?),
        Mode::Memory16Bit => {
            let third_byte = instruction_stream.next()?;
            let fourth_byte = instruction_stream.next()?;
            r_m_format_16_bit_displacement(second_byte, third_byte, fourth_byte)
        }
        Mode::Register => {
            let register_table = register_table(size);
            Cow::from(lookup_masked(register_table, second_byte, 0b0000_0111, 0))
        }
    };

    Some(r_m)
}

pub fn disassemble_immediate_to_register_memory<I>(instruction_stream: &'_ mut I) -> Option<String>
where
    I: Iterator<Item = u8>,
{
    let first_byte = instruction_stream.next()?;
    let operation_size = lookup_masked(&SIZES, first_byte, 0b1, 0);

    let second_byte = instruction_stream.next()?;
    let register_or_memory = register_or_memory(operation_size, second_byte, instruction_stream)?;

    let mut data = [0u8; 2];
    data[0] = instruction_stream.next()?;
    if operation_size == Size::Word {
        data[1] = instruction_stream.next()?;
    };
    let data = u16::from_le_bytes(data);

    let disassembly = format!(
        "mov {register_or_memory}, {} {data}",
        operation_size.as_immediate_str()
    );

    Some(disassembly)
}

const MEMORY_STRINGS: [&str; 8] = [
    "[bx + si]",
    "[bx + di]",
    "[bp + si]",
    "[bp + di]",
    "[si]",
    "[di]",
    "[bp]",
    "[bx]",
];

fn r_m_format_no_displacement<I>(
    second_byte: u8,
    instruction_stream: &'_ mut I,
) -> Option<Cow<'static, str>>
where
    I: Iterator<Item = u8>,
{
    let second_byte = second_byte & 0b111;
    if second_byte == 6 {
        let address = direct_address(instruction_stream)?;
        return Some(Cow::from(address));
    }

    Some(Cow::from(MEMORY_STRINGS[second_byte as usize]))
}

fn direct_address<I>(instruction_stream: &'_ mut I) -> Option<String>
where
    I: Iterator<Item = u8>,
{
    let memory_lo = instruction_stream.next()?;
    let memory_hi = instruction_stream.next()?;
    let displacement = u16::from_le_bytes([memory_lo, memory_hi]);
    Some(format!("[{}]", displacement.to_string()))
}

fn r_m_format_8_bit_displacement(second_byte: u8, third_byte: u8) -> Cow<'static, str> {
    let second_byte = second_byte & 0b111;
    if third_byte == 0 {
        Cow::from(MEMORY_STRINGS[second_byte as usize])
    } else {
        let displacement = u8::from_le(third_byte);
        Cow::from(r_m_format_displacement_inner(
            second_byte,
            displacement as i8 as i16,
        ))
    }
}

fn r_m_format_16_bit_displacement(
    second_byte: u8,
    third_byte: u8,
    fourth_byte: u8,
) -> Cow<'static, str> {
    let second_byte = second_byte & 0b111;
    let displacement = i16::from_le_bytes([third_byte, fourth_byte]);
    if displacement == 0 {
        Cow::from(MEMORY_STRINGS[second_byte as usize])
    } else {
        Cow::from(r_m_format_displacement_inner(second_byte, displacement))
    }
}

fn r_m_format_displacement_inner(second_byte: u8, displacement: i16) -> String {
    let second_byte = second_byte & 0b111;
    if displacement > 0 {
        match second_byte {
            0 => format!("[bx + si + {displacement}]"),
            1 => format!("[bx + di + {displacement}]"),
            2 => format!("[bp + si + {displacement}]"),
            3 => format!("[bp + di + {displacement}]"),
            4 => format!("[si + {displacement}]"),
            5 => format!("[di + {displacement}]"),
            6 => format!("[bp + {displacement}]"),
            7 => format!("[bx + {displacement}]"),
            _ => unreachable!(),
        }
    } else {
        let displacement = -displacement;
        match second_byte {
            0 => format!("[bx + si - {displacement}]"),
            1 => format!("[bx + di - {displacement}]"),
            2 => format!("[bp + si - {displacement}]"),
            3 => format!("[bp + di - {displacement}]"),
            4 => format!("[si - {displacement}]"),
            5 => format!("[di - {displacement}]"),
            6 => format!("[bp - {displacement}]"),
            7 => format!("[bx - {displacement}]"),
            _ => unreachable!(),
        }
    }
}

pub fn disassemble_immediate_to_register<I>(instruction_stream: &'_ mut I) -> Option<String>
where
    I: Iterator<Item = u8>,
{
    let first_byte = instruction_stream.next()?;

    let mut data = [0u8; 2];
    data[0] = instruction_stream.next()?;
    let mut registers = BYTE_REGISTERS;

    let size = lookup_masked(&SIZES, first_byte, 0b0000_1000, 3);
    if size == Size::Word {
        data[1] = instruction_stream.next()?;
        registers = WORD_REGISTERS;
    };

    let register = lookup_masked(&registers, first_byte, 0b0000_0111, 0);
    let data = u16::from_le_bytes(data);

    Some(format!("mov {register}, {data}"))
}

const DIRECTIONS: [Direction; 2] = [Direction::FromRegister, Direction::ToRegister];
const SIZES: [Size; 2] = [Size::Byte, Size::Word];
const MODES: [Mode; 4] = [
    Mode::MemoryNoDisplacement,
    Mode::Memory8Bit,
    Mode::Memory16Bit,
    Mode::Register,
];
const BYTE_REGISTERS: [&str; 8] = ["al", "cl", "dl", "bl", "ah", "ch", "dh", "bh"];
const WORD_REGISTERS: [&str; 8] = ["ax", "cx", "dx", "bx", "sp", "bp", "si", "di"];

fn register_table(operation_size: Size) -> &'static [&'static str; 8] {
    match operation_size {
        Size::Byte => &BYTE_REGISTERS,
        Size::Word => &WORD_REGISTERS,
    }
}

fn lookup_masked<T, const N: usize>(table: &[T; N], byte: u8, mask: u8, shift: u8) -> T
where
    T: Copy,
{
    table[((byte & mask) >> shift) as usize]
}

#[derive(Clone, Copy, Debug)]
enum Direction {
    FromRegister,
    ToRegister,
}

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum Size {
    Byte,
    Word,
}

impl Size {
    fn as_immediate_str(&self) -> &'static str {
        match self {
            Size::Byte => "byte",
            Size::Word => "word",
        }
    }
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
    use super::*;

    #[test]
    fn register_to_register_word() {
        let dissassembly =
            disassemble_register_to_from_register(&mut [0b1000_1001, 0b1101_1110].into_iter());
        assert_eq!(dissassembly, Some("mov si, bx".into()));
    }

    #[test]
    fn register_to_register_byte() {
        let disassembly =
            disassemble_register_to_from_register(&mut [0b1000_1000, 0b1100_0110].into_iter());
        assert_eq!(disassembly, Some("mov dh, al".into()));
    }

    #[test]
    fn immediate_to_register_8_bit_positive() {
        let disassembly =
            disassemble_immediate_to_register(&mut [0b1011_0001, 0b0000_1100].into_iter());

        assert_eq!(disassembly, Some("mov cl, 12".into()));
    }

    #[test]
    fn immediate_to_register_8_bit_negative() {
        let disassembly =
            disassemble_immediate_to_register(&mut [0b1011_0101, 0b1111_0100].into_iter());

        // Disassembler can't distinguish sign
        assert_eq!(disassembly, Some("mov ch, 244".into()));
    }

    #[test]
    fn immediate_to_register_16_bit_positive_8bit() {
        let disassembly =
            disassemble_immediate_to_register(&mut [0b1011_1001, 0b0000_1100, 0].into_iter());

        assert_eq!(disassembly, Some("mov cx, 12".into()));
    }

    #[test]
    fn immediate_to_register_16_bit_negative_8bit() {
        let disassembly = disassemble_immediate_to_register(
            &mut [0b1011_1001, 0b1111_0100, 0b1111_1111].into_iter(),
        );

        // Disassembler can't distinguish sign
        assert_eq!(disassembly, Some("mov cx, 65524".into()));
    }

    #[test]
    fn immediate_to_register_16_bit_positive() {
        let disassembly = disassemble_immediate_to_register(
            &mut [0b1011_1010, 0b0110_1100, 0b0000_1111].into_iter(),
        );

        assert_eq!(disassembly, Some("mov dx, 3948".into()));
    }

    #[test]
    fn immediate_to_register_16_bit_negative() {
        let disassembly = disassemble_immediate_to_register(
            &mut [0b1011_1001, 0b1001_0100, 0b1111_0000].into_iter(),
        );

        // Disassembler can't distinguish sign
        assert_eq!(disassembly, Some("mov cx, 61588".into()));
    }

    #[test]
    fn source_address_calculation_no_displacement_1() {
        let disassembly = disassemble_register_to_from_register(&mut [0b1000_1010, 0].into_iter());

        assert_eq!(disassembly, Some("mov al, [bx + si]".into()));
    }

    #[test]
    fn source_address_calculation_no_displacement_2() {
        let disassembly =
            disassemble_register_to_from_register(&mut [0b1000_1011, 0b0001_1011].into_iter());

        assert_eq!(disassembly, Some("mov bx, [bp + di]".into()));
    }

    #[test]
    fn source_address_calculation_no_displacement_3() {
        let disassembly =
            disassemble_register_to_from_register(&mut [0b1000_1011, 0b0101_0110, 0].into_iter());

        assert_eq!(disassembly, Some("mov dx, [bp]".into()));
    }

    #[test]
    fn source_address_calculation_8_bit_displacement() {
        let disassembly = disassemble_register_to_from_register(
            &mut [0b1000_1010, 0b0110_0000, 0b0000_0100].into_iter(),
        );

        assert_eq!(disassembly, Some("mov ah, [bx + si + 4]".into()));
    }

    #[test]
    fn source_address_calculation_16_bit_displacement() {
        let disassembly = disassemble_register_to_from_register(
            &mut [0b1000_1010, 0b1000_0000, 0b1000_0111, 0b0001_0011].into_iter(),
        );

        assert_eq!(disassembly, Some("mov al, [bx + si + 4999]".into()));
    }

    #[test]
    fn destination_address_calculation_no_displacement_1() {
        let disassembly =
            disassemble_register_to_from_register(&mut [0b1000_1001, 0b0000_1001].into_iter());

        assert_eq!(disassembly, Some("mov [bx + di], cx".into()));
    }

    #[test]
    fn destination_address_calculation_no_displacement_2() {
        let disassembly =
            disassemble_register_to_from_register(&mut [0b1000_1000, 0b0000_1010].into_iter());

        assert_eq!(disassembly, Some("mov [bp + si], cl".into()));
    }

    #[test]
    fn destination_address_calculation_no_displacement_3() {
        let disassembly =
            disassemble_register_to_from_register(&mut [0b1000_1000, 0b0110_1110, 0].into_iter());

        assert_eq!(disassembly, Some("mov [bp], ch".into()));
    }

    #[test]
    fn signed_displacement_1() {
        let disassembly = disassemble_register_to_from_register(
            &mut [0b1000_1011, 0b0100_0001, 0b1101_1011].into_iter(),
        );

        assert_eq!(disassembly, Some("mov ax, [bx + di - 37]".into()));
    }

    #[test]
    fn signed_displacement_2() {
        let disassembly = disassemble_register_to_from_register(
            &mut [0b1000_1001, 0b1000_1100, 0b1101_0100, 0b1111_1110].into_iter(),
        );

        assert_eq!(disassembly, Some("mov [si - 300], cx".into()));
    }

    #[test]
    fn signed_displacement_3() {
        let disassembly = disassemble_register_to_from_register(
            &mut [0b1000_1011, 0b0101_0111, 0b1110_0000].into_iter(),
        );

        assert_eq!(disassembly, Some("mov dx, [bx - 32]".into()));
    }

    #[test]
    fn explicit_size_byte() {
        let disassembly = disassemble_immediate_to_register_memory(
            &mut [0b1100_0110, 0b0000_0011, 0b0000_0111].into_iter(),
        );

        assert_eq!(disassembly, Some("mov [bp + di], byte 7".into()));
    }

    #[test]
    fn explicit_size_word() {
        let disassembly = disassemble_immediate_to_register_memory(
            &mut [
                0b1100_0111,
                0b1000_0101,
                0b1000_0101,
                0b0000_0011,
                0b0101_1011,
                0b0000_0001,
            ]
            .into_iter(),
        );

        assert_eq!(disassembly, Some("mov [di + 901], word 347".into()));
    }

    #[test]
    fn direct_address_byte() {
        let disassembly = disassemble_register_to_from_register(
            &mut [0b1000_1011, 0b0010_1110, 0b0000_0101, 0].into_iter(),
        );

        assert_eq!(disassembly, Some("mov bp, [5]".into()));
    }

    #[test]
    fn direct_address_word() {
        let disassembly = disassemble_register_to_from_register(
            &mut [0b1000_1011, 0b0001_1110, 0b1000_0010, 0b0000_1101].into_iter(),
        );

        assert_eq!(disassembly, Some("mov bx, [3458]".into()));
    }

    #[test]
    fn memory_to_accumulator_word_16_bit() {
        let disassembly = disassemble_memory_to_from_accumulator(
            &mut [0b1010_0001, 0b1111_1011, 0b0000_1001].into_iter(),
        );

        assert_eq!(disassembly, Some("mov ax, [2555]".into()));
    }

    #[test]
    fn memory_to_accumulator_word_8_bit() {
        let disassembly =
            disassemble_memory_to_from_accumulator(&mut [0b1010_0001, 0b0001_0000, 0].into_iter());

        assert_eq!(disassembly, Some("mov ax, [16]".into()));
    }

    #[test]
    fn accumulator_to_memory_word_16_bit() {
        let disassembly = disassemble_memory_to_from_accumulator(
            &mut [0b1010_0011, 0b1111_1010, 0b0000_1001].into_iter(),
        );

        assert_eq!(disassembly, Some("mov [2554], ax".into()));
    }

    #[test]
    fn accumulator_to_memory_word_8_bit() {
        let disassembly =
            disassemble_memory_to_from_accumulator(&mut [0b1010_0011, 0b0000_1111, 0].into_iter());

        assert_eq!(disassembly, Some("mov [15], ax".into()));
    }
}
