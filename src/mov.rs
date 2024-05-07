pub fn disassemble_register_to_from_register<'stream, S>(
    first_byte: u8,
    instruction_stream: &'_ mut S,
) -> Option<String>
where
    S: Iterator<Item = &'stream u8>,
{
    let direction = lookup_masked(&DIRECTIONS, first_byte, 0b0000_0010, 1);
    let operation_size = lookup_masked(&SIZES, first_byte, 0b0000_0001, 0);

    let second_byte = *(instruction_stream.next()?);
    let _mode = lookup_masked(&MODES, second_byte, 0b1100_0000, 6);

    let table = register_table(operation_size);
    let register = lookup_masked(table, second_byte, 0b0011_1000, 3);
    let register_or_memory = lookup_masked(table, second_byte, 0b0000_0111, 0);

    let string = match direction {
        Direction::FromRegister => format!("mov {register_or_memory}, {register}"),
        Direction::ToRegister => format!("mov {register}, {register_or_memory}"),
    };

    Some(string)
}

pub fn disassemble_immediate_to_register<'stream, S>(
    first_byte: u8,
    instruction_stream: &'_ mut S,
) -> Option<String>
where
    S: Iterator<Item = &'stream u8>,
{
    let mut data = [0u8; 2];
    data[0] = *(instruction_stream.next()?);
    let mut registers = BYTE_REGISTERS;

    let size = lookup_masked(&SIZES, first_byte, 0b0000_1000, 3);
    if size == Size::Word {
        data[1] = *(instruction_stream.next()?);
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
        let instruction_first_byte = 0b1000_1001;
        let instruction_second_byte = 0b1101_1110;
        let dissassembly = disassemble_register_to_from_register(
            instruction_first_byte,
            &mut [instruction_second_byte].iter(),
        );
        assert_eq!(dissassembly, Some("mov si, bx".into()));
    }

    #[test]
    fn register_to_register_byte() {
        let instruction_first_byte = 0b1000_1000;
        let instruction_second_byte = 0b1100_0110;
        let disassembly = disassemble_register_to_from_register(
            instruction_first_byte,
            &mut [instruction_second_byte].iter(),
        );
        assert_eq!(disassembly, Some("mov dh, al".into()));
    }

    #[test]
    fn immediate_to_register_8_bit_positive() {
        let instruction_first_byte = 0b1011_0001;
        let instruction_second_byte = 0b0000_1100;
        let disassembly = disassemble_immediate_to_register(
            instruction_first_byte,
            &mut [instruction_second_byte].iter(),
        );

        assert_eq!(disassembly, Some("mov cl, 12".into()));
    }

    #[test]
    fn immediate_to_register_8_bit_negative() {
        let instruction_first_byte = 0b1011_0101;
        let instruction_second_byte = 0b1111_0100;
        let disassembly = disassemble_immediate_to_register(
            instruction_first_byte,
            &mut [instruction_second_byte].iter(),
        );

        // Disassembler can't distinguish sign
        assert_eq!(disassembly, Some("mov ch, 244".into()));
    }

    #[test]
    fn immediate_to_register_16_bit_positive_8bit() {
        let disassembly =
            disassemble_immediate_to_register(0b1011_1001, &mut [0b0000_1100, 0].iter());

        assert_eq!(disassembly, Some("mov cx, 12".into()));
    }

    #[test]
    fn immediate_to_register_16_bit_negative_8bit() {
        let disassembly =
            disassemble_immediate_to_register(0b1011_1001, &mut [0b1111_0100, 0b1111_1111].iter());

        // Disassembler can't distinguish sign
        assert_eq!(disassembly, Some("mov cx, 65524".into()));
    }

    #[test]
    fn immediate_to_register_16_bit_positive() {
        let disassembly =
            disassemble_immediate_to_register(0b1011_1010, &mut [0b0110_1100, 0b0000_1111].iter());

        assert_eq!(disassembly, Some("mov dx, 3948".into()));
    }

    #[test]
    fn immediate_to_register_16_bit_negative() {
        let disassembly =
            disassemble_immediate_to_register(0b1011_1001, &mut [0b1001_0100, 0b1111_0000].iter());

        // Disassembler can't distinguish sign
        assert_eq!(disassembly, Some("mov cx, 61588".into()));
    }

    #[test]
    fn source_address_calculation_no_displacement_1() {
        let disassembly = disassemble_register_to_from_register(0b1000_1010, &mut [0].iter());

        assert_eq!(disassembly, Some("mov al, [bx + si]".into()));
    }

    #[test]
    fn source_address_calculation_no_displacement_2() {
        let disassembly =
            disassemble_register_to_from_register(0b1000_1011, &mut [0b0001_1011].iter());

        assert_eq!(disassembly, Some("mov bx, [bp + di]".into()));
    }

    #[test]
    fn source_address_calculation_no_displacement_3() {
        let disassembly =
            disassemble_register_to_from_register(0b1000_1011, &mut [0b0101_0110, 0].iter());

        assert_eq!(disassembly, Some("mov dx, [bp]".into()));
    }

    #[test]
    fn source_address_calculation_8_bit_displacement() {
        let disassembly = disassemble_register_to_from_register(
            0b1000_1010,
            &mut [0b0110_0000, 0b0000_0100].iter(),
        );

        assert_eq!(disassembly, Some("mov ah, [bx + si + 4]".into()));
    }

    #[test]
    fn source_address_calculation_16_bit_displacement() {
        let disassembly = disassemble_register_to_from_register(
            0b1000_1010,
            &mut [0b1000_0000, 0b1000_0111, 0b0001_0011].iter(),
        );

        assert_eq!(disassembly, Some("mov al, [bx + si + 4999]".into()));
    }

    #[test]
    fn destination_address_calculation_no_displacement_1() {
        let disassembly =
            disassemble_register_to_from_register(0b1000_1001, &mut [0b0000_1001].iter());

        assert_eq!(disassembly, Some("mov [bx + di], cx".into()));
    }

    #[test]
    fn destination_address_calculation_no_displacement_2() {
        let disassembly =
            disassemble_register_to_from_register(0b1000_1000, &mut [0b0000_1010].iter());

        assert_eq!(disassembly, Some("mov [bp + si], cl".into()));
    }

    #[test]
    fn destination_address_calculation_no_displacement_3() {
        let disassembly =
            disassemble_register_to_from_register(0b1000_1000, &mut [0b0110_1110, 0].iter());

        assert_eq!(disassembly, Some("mov [bp], ch".into()));
    }
}
